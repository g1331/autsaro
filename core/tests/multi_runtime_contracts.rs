//! Independently authored consumers of the separate standard multi runtime.
#![cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]

#[allow(dead_code)]
#[path = "support/tooling.rs"]
mod tooling;
#[allow(dead_code)]
#[path = "support/workspace.rs"]
mod workspace;

use std::path::{Path, PathBuf};
use std::process::Command;
use workspace::Scratch;

fn det_consumer(scratch: &Path, fixture: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    autosar_config_core::resources::AssetInventory::from_directory(root).unwrap();
    let executable = tooling::native_binary(scratch, "det-consumer");
    let settings = tooling::execution_settings();
    let mut command = Command::new(settings.compiler);
    if cfg!(target_os = "linux") {
        command.arg("-pthread");
    }
    let output = command
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(root.join("runtime/multi/include"))
        .arg("-I")
        .arg(root.join("runtime/multi/src"))
        .arg(root.join("runtime/multi/src/Det.c"))
        .arg(root.join("runtime/multi/src/Det_Host.c"))
        .arg(root.join("core/tests/fixtures/multi-runtime").join(fixture))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

#[test]
fn det_standard_consumer_executes_runtime_and_halt_contracts() {
    let scratch = Scratch::new();
    let executable = det_consumer(&scratch.0, "det_contract.c");
    for (mode, expected, halted) in [
        ("runtime", "runtime-ok\n", false),
        ("before-init", "", true),
        (
            "development",
            "development-first\ndevelopment-second\n",
            true,
        ),
        ("recursive-development", "development-first\n", true),
        ("invalid-config", "", true),
    ] {
        let output = Command::new(&executable)
            .arg(mode)
            .current_dir(&scratch.0)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected, "{mode}");
        assert!(output.stderr.is_empty(), "{mode}: {:?}", output);
        if halted {
            // A failed assertion or ordinary rejection cannot satisfy halt.
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::process::ExitStatusExt;
                assert_eq!(output.status.signal(), Some(6), "{mode}: {:?}", output);
            }
            #[cfg(windows)]
            assert!(
                !output.status.success()
                    && output.status.code() != Some(90)
                    && output.status.code() != Some(91)
                    && output.status.code() != Some(92)
                    && output.status.code() != Some(93),
                "{mode}: {:?}",
                output
            );
        } else {
            assert!(output.status.success(), "{mode}: {:?}", output);
        }
    }
}

#[test]
fn det_reports_concurrently_without_changing_callout_results() {
    let scratch = Scratch::new();
    let executable = det_consumer(&scratch.0, "det_parallel.c");
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{:?}",
        output
    );
}
