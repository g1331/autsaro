use crate::generator;
use std::ffi::OsStr;
use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Use the actual fixed compiler/linker to check the complete translation-unit
/// namespace. This catches source, type, macro and symbol collisions without
/// imposing a speculative prefix blacklist on valid application identifiers.
/// Nothing is installed in a caller's destination and no ECU is executed.
pub(super) fn verify(files: &[(String, Vec<u8>)]) -> Result<(), String> {
    let parent = std::env::temp_dir();
    let stage = generator::reserve_directory(&parent, "ecu-linkcheck", OsStr::new("source"))?;
    let source = stage.join("source");
    for (name, bytes) in files {
        let path = source.join(name);
        fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
        fs::write(path, bytes).map_err(|error| error.to_string())?;
    }
    for mode in ["probe", "host-batch"] {
        let build = stage.join(mode);
        let stdout_path = stage.join(format!("{mode}.stdout"));
        let stderr_path = stage.join(format!("{mode}.stderr"));
        let mut command = Command::new("powershell.exe");
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(source.join("build.ps1"))
            .arg("-OutputDirectory")
            .arg(&build)
            .stdout(Stdio::from(
                fs::File::create(&stdout_path).map_err(|error| error.to_string())?,
            ))
            .stderr(Stdio::from(
                fs::File::create(&stderr_path).map_err(|error| error.to_string())?,
            ));
        if mode == "host-batch" {
            command.arg("-HostBatch");
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = command.spawn().map_err(|error| {
            format!(
                "Cannot start the fixed target link check: {error}; source retained at {}",
                source.display()
            )
        })?;
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
                break status;
            }
            if started.elapsed() >= Duration::from_secs(180) {
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    // Only this still-running compiler parent and its descendants.
                    // Terminating the entire tree prevents an orphaned GCC process
                    // from continuing to write the retained private diagnostics.
                    let stopped = Command::new("taskkill.exe")
                        .args(["/PID", &child.id().to_string(), "/T", "/F"])
                        .creation_flags(0x0800_0000)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .map_err(|error| {
                            format!(
                                "Cannot close the timed-out compiler tree: {error}; diagnostics at {}",
                                stage.display()
                            )
                        })?;
                    if !stopped.success()
                        && child
                            .try_wait()
                            .map_err(|error| error.to_string())?
                            .is_none()
                    {
                        return Err(format!(
                            "Cannot close the timed-out compiler tree; diagnostics at {}",
                            stage.display()
                        ));
                    }
                }
                #[cfg(not(windows))]
                child.kill().map_err(|error| error.to_string())?;
                child.wait().map_err(|error| error.to_string())?;
                return Err(format!(
                    "Target link check exceeded its host watchdog; source/logs retained at {}",
                    stage.display()
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        if !status.success() {
            return Err(format!(
                "The complete ECU source does not compile/link with the fixed target; no destination was changed.\n{}\n{}\nSource/logs retained at {}",
                fs::read_to_string(&stdout_path).unwrap_or_else(|error| error.to_string()),
                fs::read_to_string(&stderr_path).unwrap_or_else(|error| error.to_string()),
                stage.display(),
            ));
        }
    }
    // Remove only this newly reserved build/check tree. Verify the resolved
    // absolute path is still a direct child of the intended temporary root.
    let resolved_parent = fs::canonicalize(&parent).map_err(|error| error.to_string())?;
    let resolved_stage = fs::canonicalize(&stage).map_err(|error| error.to_string())?;
    if resolved_stage.parent() != Some(resolved_parent.as_path()) {
        return Err("The private link-check cleanup path escaped its reserved parent.".into());
    }
    fs::remove_dir_all(resolved_stage).map_err(|error| error.to_string())?;
    Ok(())
}
