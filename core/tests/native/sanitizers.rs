//! The Linux sanitizer lane (`AUTOSAR_SANITIZE`) reports deliberate faults.
use super::*;
use std::path::Path;
use std::process::Command;

#[test]
fn sanitizer_lane_reports_deliberate_faults() {
    // Normal lanes build without instrumentation; there is nothing to prove.
    let Ok(kinds) = std::env::var("AUTOSAR_SANITIZE") else {
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = Scratch::new();
    let binary = tooling::native_binary(&scratch.0, "sanitizer-faults");
    let output = tooling::c_compiler()
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
        .arg(&binary)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let mut checked = 0;
    // (sanitizer, fault, faulty argument, clean argument, report)
    for (kind, fault, faulty, clean, report) in [
        (
            "thread",
            "race",
            "0",
            None,
            "WARNING: ThreadSanitizer: data race",
        ),
        (
            "address",
            "overflow",
            "16",
            Some("8"),
            "ERROR: AddressSanitizer: global-buffer-overflow",
        ),
        (
            "undefined",
            "undefined",
            "40",
            Some("1"),
            "runtime error: shift exponent 40",
        ),
    ] {
        if !kinds.split(',').any(|name| name == kind) {
            continue;
        }
        let output = Command::new(&binary)
            .args([fault, faulty])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{kind}: {output:?}");
        assert!(stderr.contains(report), "{kind}: {stderr}");
        // The report comes from the fault, not from the instrumented program.
        if let Some(clean) = clean {
            let output = Command::new(&binary).args([fault, clean]).output().unwrap();
            assert!(
                output.status.success() && output.stderr.is_empty(),
                "{kind}: {output:?}"
            );
        }
        checked += 1;
    }
    assert!(
        checked > 0,
        "AUTOSAR_SANITIZE names no checked sanitizer: {kinds}"
    );
}
