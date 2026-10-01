use std::path::PathBuf;
use std::process::Command;

pub(super) fn python_command() -> Command {
    let executable = PathBuf::from(
        std::env::var_os("AUTOSAR_PYTHON")
            .expect("Set AUTOSAR_PYTHON to the locked absolute CPython interpreter"),
    );
    assert!(
        executable.is_absolute() && executable.is_file(),
        "AUTOSAR_PYTHON must name an existing absolute interpreter"
    );
    let mut command = Command::new(executable);
    command.args(["-m", "autosar_tooling"]);
    command
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
    )
}

pub(super) fn run_hosts(
    first: &std::path::Path,
    second: &std::path::Path,
) -> Result<autosar_config_core::RunReport, String> {
    autosar_config_core::host::run(first, &host_binary(first), second, &host_binary(second))
}

pub(super) fn run_diagnostic(
    project: &std::path::Path,
) -> Result<autosar_config_core::RunReport, String> {
    autosar_config_core::host::run_diagnostic(project, &host_binary(project))
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
