//! The Linux sanitizer lane (`AUTOSAR_SANITIZE`) reports deliberate faults.
use super::*;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

#[test]
fn sanitizer_lane_reports_deliberate_faults() {
    // Normal lanes build without instrumentation; there is nothing to prove.
    let Ok(kinds) = std::env::var("AUTOSAR_SANITIZE") else {
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = Scratch::new();
    let version = tooling::run_public_command(
        Command::new(tooling::execution_settings().compiler).arg("--version"),
        &scratch.0,
        "compiler-version",
        Duration::from_secs(30),
    );
    assert!(version.status.success(), "{version:?}");
    eprintln!(
        "detector compiler: {}; AUTOSAR_SANITIZE={kinds}; TSAN_OPTIONS={:?}; ASAN_OPTIONS={:?}",
        String::from_utf8_lossy(&version.stdout),
        std::env::var_os("TSAN_OPTIONS"),
        std::env::var_os("ASAN_OPTIONS"),
    );
    for optimization in ["-O0", "-O2"] {
        check_faults(root, &scratch.0, &kinds, optimization);
    }
}

fn check_faults(root: &Path, scratch: &Path, kinds: &str, optimization: &str) {
    let binary = tooling::native_binary(scratch, &format!("sanitizer-faults{optimization}"));
    let mut compiler = tooling::c_compiler();
    compiler
        .arg(optimization)
        .args([
            "-std=c99",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-pthread",
        ])
        .arg(root.join("core/tests/fixtures/sanitizers/sanitizer_faults.c"))
        .arg("-o")
        .arg(&binary);
    eprintln!("detector compile: {compiler:?}");
    let output = tooling::run_public_command(
        &mut compiler,
        scratch,
        &format!("compile{optimization}"),
        Duration::from_secs(30),
    );
    assert!(output.status.success(), "{output:?}");
    let mut checked = 0;
    // (sanitizer, fault, faulty argument, clean argument, report)
    for (kind, fault, faulty, clean, report) in [
        (
            "thread",
            "race",
            "0",
            "1",
            "WARNING: ThreadSanitizer: data race",
        ),
        (
            "address",
            "overflow",
            "16",
            "8",
            "ERROR: AddressSanitizer: global-buffer-overflow",
        ),
        (
            "undefined",
            "undefined",
            "40",
            "1",
            "runtime error: shift exponent 40",
        ),
    ] {
        if !kinds.split(',').any(|name| name == kind) {
            continue;
        }
        let output = tooling::run_public_command(
            Command::new(&binary).args([fault, faulty]),
            scratch,
            &format!("{fault}{optimization}"),
            Duration::from_secs(30),
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{kind}: {output:?}");
        assert!(stderr.contains(report), "{kind}: {output:?}");
        // The report comes from the fault, not from the instrumented program.
        let output = tooling::run_public_command(
            Command::new(&binary).args([fault, clean]),
            scratch,
            &format!("clean-{fault}{optimization}"),
            Duration::from_secs(30),
        );
        assert!(
            output.status.success() && output.stderr.is_empty(),
            "{kind}: {output:?}"
        );
        if kind == "thread" {
            assert_eq!(output.stdout, b"1\n", "{kind}: {output:?}");
        }
        checked += 1;
    }
    assert!(
        checked > 0,
        "AUTOSAR_SANITIZE names no checked sanitizer: {kinds}"
    );
}
