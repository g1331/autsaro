use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

pub(crate) fn python_command() -> Command {
    let executable = PathBuf::from(
        std::env::var_os("AUTOSAR_PYTHON")
            .expect("Set AUTOSAR_PYTHON to the absolute Python interpreter"),
    );
    assert!(
        executable.is_absolute() && executable.is_file(),
        "AUTOSAR_PYTHON must name an existing absolute interpreter"
    );
    Command::new(executable)
}

pub(crate) fn execution_owner() -> autosar_config_core::execution::ProcessOwner {
    let command = python_command();
    autosar_config_core::execution::ProcessOwner::with_python(Path::new(command.get_program()))
        .expect("configured owner interpreter")
}

/// Waits for an owned child and fails with its output, not only its log paths.
pub(crate) fn wait_success(
    mut process: autosar_config_core::execution::OwnedProcess,
    context: &str,
) -> autosar_config_core::execution::ProcessResult {
    let result = process.wait().unwrap();
    assert!(
        result.success(),
        "{context}: {result:?}\n{}",
        result.output_tail()
    );
    result
}

pub(crate) fn native_target() -> autosar_config_core::target::BuildTarget {
    use autosar_config_core::target::BuildTarget;
    if cfg!(target_os = "linux") {
        BuildTarget::LinuxX64ControlledV1
    } else {
        BuildTarget::WindowsX64ControlledV1
    }
}

pub(crate) fn execution_settings() -> autosar_config_core::target::ExecutionSettings {
    autosar_config_core::target::ExecutionSettings::from_environment().unwrap()
}

/// The configured C compiler for a test executable. `AUTOSAR_SANITIZE` names
/// the GCC sanitizers of the Linux sanitizer lane (`thread`, or
/// `address,undefined`); each report then ends the program with an error.
pub(crate) fn c_compiler() -> Command {
    let mut command = Command::new(execution_settings().compiler);
    if let Some(kinds) = std::env::var_os("AUTOSAR_SANITIZE").filter(|kinds| !kinds.is_empty()) {
        let mut sanitize = std::ffi::OsString::from("-fsanitize=");
        sanitize.push(kinds);
        command
            .arg(sanitize)
            .args(["-fno-sanitize-recover=all", "-fno-omit-frame-pointer", "-g"]);
    }
    command
}

pub(crate) fn ecu_build_command(
    project: &std::path::Path,
    output: &std::path::Path,
    mode: &str,
    control: Option<&std::path::Path>,
) -> Command {
    if let Ok(kinds) = std::env::var("AUTOSAR_SANITIZE")
        && matches!(kinds.as_str(), "thread" | "address,undefined")
    {
        return ecu_sanitizer_build_command(
            project,
            output,
            mode,
            control,
            std::ffi::OsStr::new(&kinds),
        );
    }
    ecu_production_build_command(project, output, mode, control)
}

pub(crate) fn ecu_production_build_command(
    project: &Path,
    output: &Path,
    mode: &str,
    control: Option<&Path>,
) -> Command {
    let settings = execution_settings();
    let mut command = Command::new(&settings.python);
    command
        .args(["-I", "-S"])
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

/// Test-only instrumentation reuses the sealed package's production build owner.
pub(crate) fn ecu_sanitizer_build_command(
    project: &Path,
    output: &Path,
    mode: &str,
    control: Option<&Path>,
    kinds: &std::ffi::OsStr,
) -> Command {
    let settings = execution_settings();
    let mut command = Command::new(&settings.python);
    command
        .args(["-I", "-S", "-B"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/sealed_sanitizers.py"))
        .arg(project)
        .arg(output)
        .arg(mode)
        .env("AUTOSAR_CC", settings.compiler)
        .env("AUTOSAR_OBJDUMP", settings.objdump)
        .env("AUTOSAR_GIT", settings.git)
        .env("AUTOSAR_SANITIZE", kinds);
    if let Some(source) = control {
        command.arg(source);
    }
    command
}

pub(crate) fn ecu_verify_command(project: &std::path::Path, output: &std::path::Path) -> Command {
    let settings = execution_settings();
    let mut command = Command::new(&settings.python);
    command
        .args(["-I", "-S"])
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

pub(crate) fn native_binary(directory: &std::path::Path, stem: &str) -> PathBuf {
    directory.join(if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    })
}

/// Section names defining each symbol, read with the target `objdump`.
/// ELF symbol tables name the section; PE/COFF ones give a 1-based
/// `(sec N)` index into the `objdump -h` section headers.
pub(crate) fn symbol_sections(objdump: &Path, binary: &Path) -> Vec<(String, String)> {
    let binary = std::path::absolute(binary).unwrap();
    let run = |flag: &str| {
        let output = run_public_command(
            Command::new(objdump).arg(flag).arg(&binary),
            binary.parent().unwrap(),
            &format!("objdump-{flag}"),
            std::time::Duration::from_secs(30),
        );
        assert!(
            output.status.success(),
            "objdump {flag} {}",
            binary.display()
        );
        String::from_utf8(output.stdout).unwrap()
    };
    let headers: Vec<String> = run("-h")
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            fields.next()?.parse::<usize>().ok()?;
            fields.next().map(str::to_owned)
        })
        .collect();
    run("-t")
        .lines()
        .filter_map(|line| {
            let symbol = line.split_whitespace().last()?.to_owned();
            let section = match line.split_once("(sec") {
                Some((_, rest)) => {
                    let index: usize = rest.split(')').next()?.trim().parse().ok()?;
                    headers.get(index.checked_sub(1)?)?.clone()
                }
                None => line
                    .split_whitespace()
                    .find(|field| field.starts_with('.'))?
                    .to_owned(),
            };
            Some((symbol, section))
        })
        .collect()
}
pub(crate) fn run_public_command(
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
            "{name} terminated by signal: {exit_code}; {}{}",
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr),
        );
        std::process::ExitStatus::from_raw(exit_code << 8)
    };
    Output {
        status,
        stdout,
        stderr,
    }
}
