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
