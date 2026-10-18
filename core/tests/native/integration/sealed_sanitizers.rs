//! Linux sealed ECU sanitizers and focused OS thread owners, without changing delivery builds.
use super::*;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;

fn enabled() -> bool {
    cfg!(target_os = "linux")
        && matches!(
            std::env::var("AUTOSAR_SANITIZE").as_deref(),
            Ok("thread" | "address,undefined")
        )
}

fn require_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn instrument(project: &Path, output: &Path, mode: &str, control: Option<&Path>) {
    require_success(&tooling::run_public_command(
        &mut tooling::ecu_sanitizer_build_command(
            project,
            output,
            mode,
            control,
            OsStr::new(&std::env::var("AUTOSAR_SANITIZE").unwrap()),
        ),
        output.parent().unwrap(),
        &format!("build-{mode}"),
        Duration::from_secs(180),
    ));
    let commands: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("compile-commands.json")).unwrap()).unwrap();
    let original = commands["production"].as_array().unwrap();
    let mut instrumented = commands["instrumented"].as_array().unwrap().clone();
    assert!(
        !original
            .iter()
            .any(|arg| arg.as_str().unwrap().starts_with("-fsanitize="))
    );
    let sanitizer = format!("-fsanitize={}", std::env::var("AUTOSAR_SANITIZE").unwrap());
    assert_eq!(
        instrumented.drain(1..5).collect::<Vec<_>>(),
        [
            sanitizer.as_str(),
            "-fno-sanitize-recover=all",
            "-fno-omit-frame-pointer",
            "-g"
        ]
        .map(serde_json::Value::from)
    );
    let destination = original.iter().position(|arg| arg == "-o").unwrap() + 1;
    assert_ne!(instrumented[destination], original[destination]);
    assert!(
        Path::new(original[destination].as_str().unwrap()).starts_with(output.join("production"))
    );
    assert!(Path::new(instrumented[destination].as_str().unwrap()).starts_with(output));
    instrumented[destination] = original[destination].clone();
    assert_eq!(
        &instrumented, original,
        "source/include/kernel/flags closure drift"
    );
    for define in ["-DECU_TARGET_TESTS", "-DOS_HOST_FAILURE_TESTS"] {
        assert_eq!(original.iter().any(|arg| arg == define), mode == "test");
    }
}

#[test]
fn sealed_probe_and_test_modes_preserve_closure_and_runtime_protocol() {
    if !enabled() {
        return;
    }
    let scratch = Scratch::new();
    let (project, _) = single_component_project(&scratch.0, tooling::native_target());
    let before = packages::package_tree(&project);
    for mode in ["probe", "test"] {
        let output = scratch.0.join(mode);
        instrument(&project, &output, mode, None);
        let binary = tooling::native_binary(&output, "ecu_probe");
        let run = tooling::run_public_command(
            &mut Command::new(&binary),
            &scratch.0,
            &format!("run-{mode}"),
            Duration::from_secs(30),
        );
        require_success(&run);
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(stdout.contains("ecu_probe completed=20"), "{stdout}");
        assert!(
            stdout.contains("lifecycle=Closed state=Ready reason=0"),
            "{stdout}"
        );
        assert!(
            run.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let rejected = tooling::run_public_command(
            Command::new(&binary).arg(if mode == "test" { "1" } else { "unexpected" }),
            &scratch.0,
            &format!("reject-{mode}"),
            Duration::from_secs(30),
        );
        assert!(!rejected.status.success(), "{rejected:?}");
        if mode == "probe" {
            assert_eq!(rejected.status.code(), Some(98));
        } else {
            let stdout = String::from_utf8_lossy(&rejected.stdout);
            assert!(stdout.contains("fail_stage=1"), "{stdout}");
            assert!(stdout.contains("ecu_shutdown reason="), "{stdout}");
        }
    }
    assert_eq!(packages::package_tree(&project), before);
    fs::write(project.join("README.md"), b"unsealed modification").unwrap();
    let output = scratch.0.join("refused");
    let rejected = tooling::run_public_command(
        &mut tooling::ecu_sanitizer_build_command(
            &project,
            &output,
            "probe",
            None,
            OsStr::new("address,undefined"),
        ),
        &scratch.0,
        "reject-unsealed",
        Duration::from_secs(180),
    );
    assert!(!rejected.status.success(), "{rejected:?}");
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("Source identity mismatch: README.md"),
        "{rejected:?}",
    );
    assert!(!output.join("ecu_probe").exists());
    assert!(!output.join("compile-commands.json").exists());
}

#[test]
fn sealed_control_reports_runtime_faults_and_accepts_clean_boundaries() {
    if !enabled() || std::env::var("AUTOSAR_SANITIZE").as_deref() != Ok("address,undefined") {
        return;
    }
    let scratch = Scratch::new();
    let (project, _) = single_component_project(&scratch.0, tooling::native_target());
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sanitizer_faults.c");
    let output = scratch.0.join("faults");
    instrument(&project, &output, "probe", Some(&fixture));
    let binary = tooling::native_binary(&output, "ecu_probe");
    for (fault, bad, clean, report) in [
        (
            "overflow",
            "16",
            "8",
            "ERROR: AddressSanitizer: global-buffer-overflow",
        ),
        ("undefined", "40", "1", "runtime error: shift exponent 40"),
    ] {
        let run = tooling::run_public_command(
            Command::new(&binary).args([fault, bad]),
            &scratch.0,
            fault,
            Duration::from_secs(30),
        );
        assert!(!run.status.success(), "{run:?}");
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert!(stderr.contains(report), "{fault}: {stderr}");
        let run = tooling::run_public_command(
            Command::new(&binary).args([fault, clean]),
            &scratch.0,
            &format!("clean-{fault}"),
            Duration::from_secs(30),
        );
        require_success(&run);
        assert!(run.stderr.is_empty(), "{run:?}");
    }
}

#[test]
fn shared_host_batch_separates_instrumentation_and_consumer_images() {
    if !enabled() {
        return;
    }
    let scratch = Scratch::new();
    let (project, _) = single_component_project(&scratch.0, tooling::native_target());
    let production =
        sealed_builds::host_batch_with_sanitizer(&project, &scratch.0.join("plain"), None);
    let first = sealed_builds::host_batch(&project, &scratch.0.join("first"));
    let bytes = fs::read(&first).unwrap();
    assert!(
        fs::read(production).unwrap() != bytes,
        "sanitizer reused production image"
    );
    fs::write(&first, b"mutated consumer").unwrap();
    let second = sealed_builds::host_batch(&project, &scratch.0.join("second"));
    assert!(
        fs::read(&second).unwrap() == bytes,
        "consumer modified cached image"
    );
    verify_production_owner_binary(&second, &scratch.0, None);
}

#[test]
fn sealed_guard_refuses_untrusted_standard_library_shadow_before_import() {
    if !enabled() {
        return;
    }
    let scratch = Scratch::new();
    let (project, _) = single_component_project(&scratch.0, tooling::native_target());
    let marker = scratch.0.join("untrusted-module-executed");
    fs::write(
        project.join("tools/hashlib.py"),
        format!(
            "from pathlib import Path\nPath({:?}).write_text('untrusted module executed')\n",
            marker
        ),
    )
    .unwrap();
    let output = scratch.0.join("refused-shadow");
    let rejected = tooling::run_public_command(
        &mut tooling::ecu_sanitizer_build_command(
            &project,
            &output,
            "probe",
            None,
            OsStr::new("address,undefined"),
        ),
        &scratch.0,
        "reject-shadow",
        Duration::from_secs(180),
    );
    assert!(!rejected.status.success(), "{rejected:?}");
    assert!(
        !marker.exists(),
        "unverified module executed before seal validation"
    );
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("Unlisted source file: tools/hashlib.py"),
        "{rejected:?}",
    );
    assert!(!output.exists(), "refused source created build output");
}

#[test]
fn sealed_os_registration_and_interrupt_identity_use_real_thread_owners() {
    if !enabled() {
        return;
    }
    let scratch = Scratch::new();
    let (project, _) = single_component_project(&scratch.0, tooling::native_target());
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/os_lifecycle_threads.c");
    let output = scratch.0.join("thread-owners");
    instrument(&project, &output, "probe", Some(&fixture));
    let run = tooling::run_public_command(
        &mut Command::new(tooling::native_binary(&output, "ecu_probe")),
        &scratch.0,
        "thread-owners",
        Duration::from_secs(30),
    );
    require_success(&run);
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "registration=published host_isr=inactive dispatcher_isr=restored"
    );
    assert!(run.stderr.is_empty(), "{run:?}");
}

#[test]
fn sealed_actor_signal_lifecycle_preserves_context_and_terminal_stop() {
    if !enabled() {
        return;
    }
    let scratch = Scratch::new();
    let (project, _) = single_component_project(&scratch.0, tooling::native_target());
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/os_signal_lifecycle.c");
    let output = scratch.0.join("signal-lifecycle");
    instrument(&project, &output, "probe", Some(&fixture));
    for (name, directory) in [
        ("production", output.join("production")),
        ("sanitized", output),
    ] {
        let run = tooling::run_public_command(
            &mut Command::new(tooling::native_binary(&directory, "ecu_probe")),
            &scratch.0,
            &format!("signal-lifecycle-{name}"),
            Duration::from_secs(30),
        );
        require_success(&run);
        assert_eq!(
            String::from_utf8_lossy(&run.stdout).trim(),
            "suspend=acknowledged resume=progress context=preserved terminal=irreversible running=stopped closed=detached"
        );
        assert!(run.stderr.is_empty(), "{run:?}");
    }
}
