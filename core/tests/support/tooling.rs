use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

pub(super) fn python_command() -> Command {
    let executable = PathBuf::from(
        std::env::var_os("AUTOSAR_PYTHON")
            .expect("Set AUTOSAR_PYTHON to the locked absolute CPython interpreter"),
    );
    assert!(
        executable.is_absolute() && executable.is_file(),
        "AUTOSAR_PYTHON must name an existing absolute interpreter"
    );
    Command::new(executable)
}

pub(super) fn execution_owner() -> autosar_config_core::execution::ProcessOwner {
    let command = python_command();
    autosar_config_core::execution::ProcessOwner::with_python(Path::new(command.get_program()))
        .expect("configured owner interpreter")
}

pub(super) fn native_target() -> autosar_config_core::target::BuildTarget {
    use autosar_config_core::target::BuildTarget;
    if cfg!(target_os = "linux") {
        BuildTarget::LinuxX64ControlledV1
    } else {
        BuildTarget::WindowsX64ControlledV1
    }
}

pub(super) fn execution_settings() -> autosar_config_core::target::ExecutionSettings {
    autosar_config_core::target::ExecutionSettings::from_environment().unwrap()
}

pub(super) fn host_build_directory(project: &std::path::Path) -> PathBuf {
    project.with_extension("native-build")
}

#[cfg(windows)]
pub(super) fn host_binary(project: &std::path::Path) -> PathBuf {
    host_build_directory(project).join(if cfg!(windows) {
        "ecu_host.exe"
    } else {
        "ecu_host"
    })
}

pub(super) fn build_host(
    project: &std::path::Path,
) -> Result<autosar_config_core::BuildReport, String> {
    autosar_config_core::generator::build(
        project,
        &host_build_directory(project),
        &execution_settings(),
        &super::tooling::execution_owner(),
    )
}

#[cfg(windows)]
pub(super) fn run_hosts(
    first: &std::path::Path,
    second: &std::path::Path,
) -> Result<autosar_config_core::RunReport, String> {
    autosar_config_core::host::run(
        first,
        &host_binary(first),
        second,
        &host_binary(second),
        &super::tooling::execution_owner(),
    )
}

#[cfg(windows)]
pub(super) fn run_diagnostic(
    project: &std::path::Path,
) -> Result<autosar_config_core::RunReport, String> {
    autosar_config_core::host::run_diagnostic(
        project,
        &host_binary(project),
        &super::tooling::execution_owner(),
    )
}

pub(super) fn ecu_build_command(
    project: &std::path::Path,
    output: &std::path::Path,
    mode: &str,
    control: Option<&std::path::Path>,
) -> Command {
    let settings = execution_settings();
    let mut command = Command::new(&settings.python);
    command
        .arg(project.join("tools/ecu-tool.py"))
        .arg("build")
        .arg("--project")
        .arg(project)
        .arg("--output")
        .arg(output)
        .args(["--mode", mode])
        .env("AUTOSAR_CC", settings.compiler)
        .env("AUTOSAR_OBJDUMP", settings.objdump)
        .env("AUTOSAR_GIT", settings.git);
    if let Some(source) = control {
        command.arg("--control-source").arg(source);
    }
    command
}

#[cfg(windows)]
pub(super) fn ecu_verify_command(project: &std::path::Path, output: &std::path::Path) -> Command {
    let settings = execution_settings();
    let mut command = Command::new(&settings.python);
    command
        .arg(project.join("tools/ecu-tool.py"))
        .arg("verify")
        .arg("--project")
        .arg(project)
        .arg("--build-directory")
        .arg(output)
        .env("AUTOSAR_CC", settings.compiler)
        .env("AUTOSAR_OBJDUMP", settings.objdump)
        .env("AUTOSAR_GIT", settings.git);
    command
}

pub(super) fn native_binary(directory: &std::path::Path, stem: &str) -> PathBuf {
    directory.join(if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    })
}
#[cfg(any(windows, target_os = "linux"))]
pub(super) fn run_native_os_suite(suite: &str) {
    use std::ffi::OsString;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    static NONCE: AtomicU64 = AtomicU64::new(0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let python = python_command().get_program().to_os_string();
    let logs = std::env::temp_dir().join(format!(
        "autosar-os-suite-{}-{}-{}",
        std::process::id(),
        autosar_config_core::execution::monotonic_ns().unwrap(),
        NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&logs, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let target = if cfg!(windows) {
        "windows-x64-controlled-v1"
    } else {
        "linux-x64-controlled-v1"
    };
    let spec = autosar_config_core::execution::ProcessSpec::for_duration(
        vec![
            python,
            OsString::from("-m"),
            OsString::from("autosar_tooling"),
            OsString::from("os"),
            OsString::from("--target"),
            OsString::from(target),
            OsString::from("--suite"),
            OsString::from(suite),
        ],
        root.to_path_buf(),
        vec![],
        Duration::from_secs(1800),
        logs.clone(),
    )
    .unwrap();
    let result = autosar_config_core::execution::run_bounded(spec);
    if let Err(error) = result {
        panic!(
            "native OS suite {suite} failed: {error}; retained_logs={}",
            logs.display()
        );
    }
    fs::remove_dir_all(logs).unwrap();
}
pub(super) fn run_public_command(
    command: &mut Command,
    directory: &Path,
    name: &str,
    watchdog: Duration,
) -> Output {
    use autosar_config_core::execution::{ProcessOwner, ProcessSpec, ProcessStatus};
    let program = match command.get_program().to_str() {
        Some("gcc") => execution_settings().compiler,
        Some("objdump") => execution_settings().objdump,
        _ => {
            let path = std::path::PathBuf::from(command.get_program());
            if path.is_absolute() {
                path
            } else {
                std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                    .find_map(|root| {
                        let candidate = root.join(&path);
                        if candidate.is_file() {
                            Some(candidate)
                        } else if cfg!(windows) && path.extension().is_none() {
                            let executable = candidate.with_extension("exe");
                            executable.is_file().then_some(executable)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        panic!("Test command executable is unavailable: {}", path.display())
                    })
            }
        }
    };
    let logs = directory.join(format!("{name}.owned-logs"));
    fs::create_dir_all(&logs).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&logs, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut argv = vec![program.as_os_str().to_owned()];
    argv.extend(command.get_args().map(std::ffi::OsStr::to_owned));
    let environment = command
        .get_envs()
        .map(|(name, value)| {
            (
                name.to_owned(),
                value
                    .expect("Test commands do not remove environment variables")
                    .to_owned(),
            )
        })
        .collect();
    let spec = ProcessSpec::for_duration(
        argv,
        command.get_current_dir().unwrap_or(directory).to_path_buf(),
        environment,
        watchdog,
        logs,
    )
    .unwrap();
    let owner = ProcessOwner::new().unwrap();
    let result = owner.spawn(spec, None).unwrap().wait().unwrap();
    let stdout = fs::read(&result.stdout).unwrap();
    let stderr = fs::read(&result.stderr).unwrap();
    fs::write(directory.join(format!("{name}.stdout")), &stdout).unwrap();
    fs::write(directory.join(format!("{name}.stderr")), &stderr).unwrap();
    assert_eq!(
        result.status,
        ProcessStatus::Exited,
        "{name} exceeded host watchdog or scope closure failed: {result:?}; {}{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr),
    );
    let exit_code = result
        .exit_code
        .expect("A normally exited root has an observed exit code");
    #[cfg(windows)]
    let status = {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(exit_code as u32)
    };
    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        assert!(
            exit_code >= 0,
            "Native test command terminated by signal: {exit_code}"
        );
        std::process::ExitStatus::from_raw(exit_code << 8)
    };
    Output {
        status,
        stdout,
        stderr,
    }
}

pub(super) fn verify_protocol_oracles() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = super::Scratch::new();
    let output = run_public_command(
        python_command()
            .args(["-m", "autosar_tooling", "protocol-oracles"])
            .current_dir(root),
        &scratch.0,
        "protocol-oracles",
        Duration::from_secs(30),
    );
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
