use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

fn live_output(binary: &Path) {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let stream = child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stream).lines() {
            let line = line.unwrap();
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let received =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
            let next = || {
                receiver
                    .recv_timeout(std::time::Duration::from_secs(7))
                    .map_err(|error| error.to_string())
            };
            assert_eq!(next()?, "READY HostBatchV1");
            input
                .write_all(b"BEGIN 10\nRX 800 4 78563412\nCOMMIT\n")
                .unwrap();
            let output = next()?;
            assert!(output.starts_with("OUT epoch=10 "), "{output}");
            assert!(output.ends_with("data=78563412"), "{output}");
            let receipt = next()?;
            assert!(
                receipt.starts_with("COMMIT_OK batch=1 epoch=10 "),
                "{receipt}"
            );
            assert!(child.try_wait().unwrap().is_none());
            // The process is still blocked on this open stdin. Neither startup,
            // output nor receipt visibility can come from process-exit flushing.
            input.write_all(b"BEGIN 10\nCOMMIT\n").unwrap();
            let receipt = next()?;
            assert!(
                receipt.starts_with("COMMIT_OK batch=2 epoch=10 "),
                "{receipt}"
            );
            assert!(child.try_wait().unwrap().is_none());
            Ok(())
        }));
    drop(input);
    let started = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > std::time::Duration::from_secs(7) {
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let status = child.wait().unwrap();
    drop(receiver);
    reader.join().unwrap();
    match received {
        Ok(result) => result.unwrap(),
        Err(panic) => std::panic::resume_unwind(panic),
    }
    assert!(status.success());
}

fn run_text(binary: &Path, directory: &Path, script: &[u8]) -> String {
    std::fs::create_dir_all(directory).unwrap();
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(std::fs::File::create(directory.join("host.stdout")).unwrap())
        .stderr(std::fs::File::create(directory.join("host.stderr")).unwrap())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(script).unwrap();
    let started = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > std::time::Duration::from_secs(15) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "HostBatch exceeded watchdog: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let result = child.wait_with_output().unwrap();
    let text = std::fs::read_to_string(directory.join("host.stdout")).unwrap();
    let errors = std::fs::read_to_string(directory.join("host.stderr")).unwrap();
    assert!(result.status.success(), "{text}{}", errors);
    text
}

pub fn commit() {
    use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog, build_plan};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let plan = build_plan(&super::epic4_plan::inputs(), &dependencies, &runtime).unwrap();
    let project = plan.ecu_integration_files().unwrap();
    let scratch = super::Scratch::new();
    let source = scratch.0.join("host-batch-source");
    let preview = project.preview(&source).unwrap();
    project
        .generate_previewed(&source, &preview.revision)
        .unwrap();
    let build = scratch.0.join("host-batch-build");
    let output = Command::new("powershell.exe")
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
        .arg("-HostBatch")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let script = b"BEGIN 0\nRX 0x320 4 78563412\nCOMMIT\nBEGIN 10\nRX 0x700 8 0322123400000000\nCOMMIT\nBEGIN 10\nCOMMIT\nBEGIN 11\nRX 801 4 00000000\nCOMMIT\nBEGIN 11\nCOMMIT\n";
    let binary = build.join("ecu_host_batch.exe");
    live_output(&binary);
    let text = run_text(&binary, &scratch.0.join("basic"), script);
    assert!(text.contains("READY HostBatchV1"), "{text}");
    assert_eq!(text.matches("COMMIT_OK ").count(), 4, "{text}");
    assert_eq!(text.matches("REJECT ").count(), 1, "{text}");
    assert_eq!(text.matches("OUT epoch=").count(), 2, "{text}");
    assert!(text.contains("pdu=1 id=801 dlc=4 data=78563412"), "{text}");
    assert!(
        text.contains("pdu=3 id=1800 dlc=8 data=0762123412345678"),
        "{text}"
    );
    assert!(text.contains("COMMIT_OK batch=4 epoch=11"), "{text}");
    for expected in [
        "COMMIT_OK batch=1 epoch=0 sequence=2 input_first=1 inputs=1",
        "OUT epoch=10 sequence=13 ticket=1",
        "OUT epoch=10 sequence=14 ticket=2",
        "COMMIT_OK batch=2 epoch=10 sequence=16 input_first=12 inputs=1",
        "COMMIT_OK batch=3 epoch=10 sequence=17 input_first=0 inputs=0",
        "COMMIT_OK batch=4 epoch=11 sequence=19 input_first=0 inputs=0",
    ] {
        assert!(text.contains(expected), "{text}");
    }
    let text = run_text(
        &binary,
        &scratch.0.join("diagnostic_dlc"),
        b"BEGIN 10\nRX 800 4 78563412\nRX 1792 3 022212\nCOMMIT\nBEGIN 10\nCOMMIT\n",
    );
    assert_eq!(text.matches("REJECT ").count(), 1, "{text}");
    assert!(
        text.contains("REJECT status=8 epoch=0 sequence=0"),
        "{text}"
    );
    assert!(
        text.contains("data=00000000") && !text.contains("data=78563412"),
        "{text}"
    );
    let mut bounded = String::from("BEGIN 0\n");
    for _ in 0..256 {
        bounded.push_str("RX 800 4 78563412\n");
    }
    bounded.push_str("COMMIT\nBEGIN 10\nCOMMIT\n");
    let text = run_text(&binary, &scratch.0.join("capacity256"), bounded.as_bytes());
    assert_eq!(text.matches("COMMIT_OK ").count(), 2, "{text}");
    assert!(text.contains("inputs=256 status=0"), "{text}");
    assert!(text.contains("data=78563412"), "{text}");
    let overflow = bounded.replacen("COMMIT\n", "RX 800 4 78563412\nCOMMIT\n", 1);
    let text = run_text(&binary, &scratch.0.join("capacity257"), overflow.as_bytes());
    assert_eq!(text.matches("REJECT ").count(), 2, "{text}");
    assert_eq!(text.matches("COMMIT_OK ").count(), 1, "{text}");
    assert!(
        text.contains("data=00000000") && !text.contains("data=78563412"),
        "{text}"
    );
    let text = run_text(
        &binary,
        &scratch.0.join("span"),
        b"BEGIN 1001\nCOMMIT\nBEGIN 1000\nCOMMIT\n",
    );
    assert_eq!(text.matches("REJECT ").count(), 2, "{text}");
    assert_eq!(text.matches("OUT epoch=").count(), 100, "{text}");
    assert!(text.contains("COMMIT_OK batch=1 epoch=1000"), "{text}");
    assert!(text.contains("state=Ready reason=0"), "{text}");
    let dropped = text
        .lines()
        .find_map(|line| line.split("trace_dropped=").nth(1))
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(dropped > 0, "{text}");
    let epochs: Vec<_> = text
        .lines()
        .filter_map(|line| line.strip_prefix("OUT epoch="))
        .map(|line| line.split(' ').next().unwrap().parse::<u64>().unwrap())
        .collect();
    assert_eq!(
        epochs,
        (1..=100).map(|index| index * 10).collect::<Vec<_>>()
    );
    // N_Cr is 200 ms in the independent input fixture. The CF at the exact
    // deadline must be consumed before timeout processing for that epoch.
    let text = run_text(
        &binary,
        &scratch.0.join("receive_deadline"),
        b"BEGIN 0\nRX 1792 8 1008221234000000\nCOMMIT\nBEGIN 200\nRX 1792 8 2100000000000000\nCOMMIT\n",
    );
    assert_eq!(text.matches("COMMIT_OK ").count(), 2, "{text}");
    assert!(text.contains("COMMIT_OK batch=2 epoch=200"), "{text}");
    assert!(
        text.contains("id=1800 dlc=8 data=3000000000000000\n"),
        "{text}"
    );
    assert!(text.contains("data=037f221300000000"), "{text}");
    let text = run_text(
        &binary,
        &scratch.0.join("executed_protocol_refusal"),
        b"BEGIN 0\nRX 1792 8 0322123400000000\nRX 1792 8 1009221234123412\nCOMMIT\nBEGIN 1\nRX 1792 8 0322123400000000\nCOMMIT\n",
    );
    assert!(text.contains("COMMIT_ERROR batch=1 epoch=0"), "{text}");
    assert!(text.contains("COMMIT_OK batch=2 epoch=1"), "{text}");
    assert_eq!(text.matches("OUT epoch=").count(), 2, "{text}");
    assert_eq!(text.matches("data=0762123400000000").count(), 2, "{text}");
    for (name, bad) in [
        ("ctrl_z", b"COMMIT\x1asuffix".as_slice()),
        ("embedded_nul", b"COMMIT\0suffix".as_slice()),
        ("utf8", b"COMMIT\xffsuffix".as_slice()),
    ] {
        let mut script = b"BEGIN 0\nRX 800 4 78563412\n".to_vec();
        script.extend_from_slice(bad);
        script.extend_from_slice(b"\nCOMMIT\nBEGIN 10\nCOMMIT\n");
        let text = run_text(&binary, &scratch.0.join(name), &script);
        assert_eq!(text.matches("REJECT ").count(), 2, "{text}");
        assert!(
            text.contains("data=00000000") && !text.contains("data=78563412"),
            "{text}"
        );
    }
    let fault_build = scratch.0.join("fault-build");
    super::epic4_ecu::compile(
        &source,
        &fault_build,
        Some(&root.join("core/tests/fixtures/host_batch_faults.c")),
    );
    let overflow_build = scratch.0.join("overflow-build");
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(source.join("build.ps1"))
        .arg("-OutputDirectory")
        .arg(&overflow_build)
        .arg("-ControlSource")
        .arg(root.join("core/tests/fixtures/host_batch_faults.c"))
        .arg("-TestMode")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for mode in ["fail", "block", "sequence", "overflow"] {
        let directory = scratch.0.join(mode);
        std::fs::create_dir_all(&directory).unwrap();
        let started = std::time::Instant::now();
        let build = if mode == "overflow" {
            &overflow_build
        } else {
            &fault_build
        };
        let mut child = Command::new(build.join("ecu_probe.exe"))
            .arg(mode)
            .stdout(std::fs::File::create(directory.join("stdout")).unwrap())
            .stderr(std::fs::File::create(directory.join("stderr")).unwrap())
            .spawn()
            .unwrap();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > std::time::Duration::from_secs(8) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("{mode} failed to close within its host watchdog");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let result = child.wait().unwrap();
        let text = std::fs::read_to_string(directory.join("stdout")).unwrap();
        if mode == "sequence" {
            assert!(result.success(), "{text}");
            assert!(
                text.contains("before_state_change=pass final_identity=pass"),
                "{text}"
            );
            assert!(
                text.contains("fault_commit epoch=0 sequence=18446744073709551615"),
                "{text}"
            );
        } else if mode == "overflow" {
            assert_eq!(result.code(), Some(7), "{text}");
            assert!(
                text.contains("output_capacity accepted=256 next=257"),
                "{text}"
            );
            assert!(text.contains("output_close queued=256 reason=7"), "{text}");
        } else {
            assert_eq!(result.code(), Some(7), "{text}");
            assert_eq!(text.matches("fault_output ").count(), 1, "{text}");
            assert!(!text.contains("fault_commit "), "{text}");
            if mode == "block" {
                assert!(started.elapsed() >= std::time::Duration::from_millis(4500));
            }
        }
    }
}

pub fn native_boundary() {
    use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog, build_plan};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let plan = build_plan(&super::epic4_plan::inputs(), &dependencies, &runtime).unwrap();
    let project = plan.ecu_integration_files().unwrap();
    let scratch = super::Scratch::new();
    let source = scratch.0.join("batch-source");
    let preview = project.preview(&source).unwrap();
    project
        .generate_previewed(&source, &preview.revision)
        .unwrap();
    let build = scratch.0.join("batch-build");
    super::epic4_ecu::compile(
        &source,
        &build,
        Some(&root.join("core/tests/fixtures/host_batch_native.c")),
    );
    let result = super::epic4_ecu::run_probe(&build.join("ecu_probe.exe"), None);
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8(result.stdout).unwrap().contains("native_batch capacity=256 refusal=257 same_epoch=0 future_requires_tick=pass confirmations=pass"));
}

pub fn codec() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = super::Scratch::new();
    let binary = scratch.0.join("host-batch-codec.exe");
    let compiler = std::env::var_os("AUTOSAR_CC").unwrap_or_else(|| "gcc".into());
    let output = Command::new(compiler)
        .args(["-std=c99", "-O1", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(root.join("runtime/ecu/include"))
        .arg("-I")
        .arg(root.join("runtime/os/include"))
        .arg("-I")
        .arg(root.join("runtime/include"))
        .arg(root.join("runtime/ecu/src/Ecu_HostBatch.c"))
        .arg(root.join("core/tests/fixtures/host_batch_codec.c"))
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let result = Command::new(binary).output().unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8(result.stdout).unwrap().contains(
            "host_batch_codec boundaries=256/257,1000/1001 numeric/sequence/recovery=pass"
        )
    );
}
