use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn run(binary: &Path, directory: &Path, script: &str) -> String {
    fs::create_dir_all(directory).unwrap();
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(fs::File::create(directory.join("stdout")).unwrap())
        .stderr(fs::File::create(directory.join("stderr")).unwrap())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let started = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > std::time::Duration::from_secs(30) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!(
                "protocol case exceeded its process deadline: {}",
                directory.display()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let status = child.wait().unwrap();
    let text = fs::read_to_string(directory.join("stdout")).unwrap();
    let errors = fs::read_to_string(directory.join("stderr")).unwrap();
    assert!(status.success(), "{script}\n{text}{errors}");
    let mut previous = 0;
    for line in text
        .lines()
        .filter(|line| line.starts_with("OUT ") || line.starts_with("COMMIT_"))
    {
        let sequence = field(line, "sequence=").parse::<u64>().unwrap();
        assert!(
            sequence > previous,
            "nonmonotonic published identity: {text}"
        );
        previous = sequence;
    }
    text
}

fn field<'a>(line: &'a str, key: &str) -> &'a str {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(key))
        .unwrap_or_else(|| panic!("missing {key}: {line}"))
}

fn digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

fn tree(path: &Path, base: &Path, result: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            tree(&path, base, result);
        } else {
            result.insert(
                path.strip_prefix(base)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/"),
                digest(&path),
            );
        }
    }
}

fn frames(text: &str, id: u64) -> Vec<(u64, String)> {
    text.lines()
        .filter(|line| line.starts_with("OUT "))
        .filter(|line| field(line, "id=").parse::<u64>().unwrap() == id)
        .map(|line| {
            (
                field(line, "epoch=").parse().unwrap(),
                field(line, "data=").to_ascii_uppercase(),
            )
        })
        .collect()
}

pub fn independent_behavior() {
    use autosar_config_core::integration::{PlanDependencies, RuntimeCatalog, build_plan};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let oracle: Value = serde_json::from_slice(
        &fs::read(root.join("core/tests/fixtures/epic4_oracles/protocol.json")).unwrap(),
    )
    .unwrap();
    let sealed = Command::new("python")
        .args(["-X", "utf8"])
        .arg(root.join("scripts/epic4_obligations.py"))
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        sealed.status.success(),
        "{}{}",
        String::from_utf8_lossy(&sealed.stdout),
        String::from_utf8_lossy(&sealed.stderr)
    );
    let scratch = super::Scratch::new();
    let source = scratch.0.join("protocol-source");
    let dependencies = PlanDependencies::from_repository(root);
    let runtime = RuntimeCatalog::from_repository(root).unwrap();
    let plan = build_plan(&super::epic4_plan::inputs(), &dependencies, &runtime).unwrap();
    let project = plan.ecu_integration_files().unwrap();
    let preview = project.preview(&source).unwrap();
    project
        .generate_previewed(&source, &preview.revision)
        .unwrap();
    let build = scratch.0.join("protocol-build");
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
    let binary = build.join("ecu_host_batch.exe");
    let cases = oracle["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 11);
    for case in cases {
        let name = case["id"].as_str().unwrap();
        let mut script = String::new();
        let mut receives = Vec::new();
        if case.get("precondition").is_some() {
            script.push_str("BEGIN 0\nRX 800 4 78563412\nCOMMIT\nBEGIN 10\nCOMMIT\n");
            receives.push((0, "78563412".to_owned()));
        }
        let mut last_epoch = 0;
        for input in case["input"].as_array().unwrap() {
            last_epoch = input["epoch"].as_u64().unwrap();
            script.push_str(&format!("BEGIN {last_epoch}\n"));
            for frame in input["rx"].as_array().unwrap() {
                let id = frame["id"].as_u64().unwrap();
                let data = frame["data"].as_str().unwrap();
                script.push_str(&format!("RX {id} {} {data}\n", data.len() / 2));
                if id == 800 {
                    receives.push((last_epoch, data.to_owned()));
                }
            }
            script.push_str("COMMIT\n");
        }
        let text = run(&binary, &scratch.0.join(name), &script);
        println!("EPIC4_PROTOCOL_CASE {name}\n{text}");
        assert!(
            !text.contains("REJECT ") && !text.contains("COMMIT_ERROR "),
            "{name}: {text}"
        );
        let expected: Vec<_> = case["expected"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|frame| frame["id"] == 1800)
            .map(|frame| {
                (
                    frame["epoch"].as_u64().unwrap(),
                    frame["data"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        assert_eq!(
            frames(&text, 1800),
            expected,
            "immutable diagnostic oracle: {name}"
        );
        // Independently model only the fixed ten-ms periodic output and the
        // thirty-ms receive age, using input bytes rather than generated code.
        let expected_periodic: Vec<_> = (1..=last_epoch / 10)
            .map(|index| {
                let at = index * 10;
                let data = receives
                    .iter()
                    .rev()
                    .find(|(received, _)| *received <= at)
                    .filter(|(received, _)| at - received < 30)
                    .map_or("00000000", |(_, data)| data.as_str());
                (at, data.to_owned())
            })
            .collect();
        assert_eq!(
            frames(&text, 801),
            expected_periodic,
            "periodic oracle: {name}"
        );
    }
    let seed = "BEGIN 0\nRX 800 4 78563412\nCOMMIT\n";
    let tx = "BEGIN 10\nRX 1792 8 0522123412340000\nCOMMIT\n";
    let rx = "BEGIN 10\nRX 1792 8 1009221234123412\nCOMMIT\n";
    let cases = oracle["timing_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for case in cases {
        let name = case["id"].as_str().unwrap();
        let (script, expected, failures) = match name {
            "nbs-on-deadline" => (
                format!("{seed}{tx}BEGIN 210\nRX 1792 8 3000000000000000\nCOMMIT\n"),
                vec![(10, "100D621234123456"), (210, "2178123412345678")],
                0,
            ),
            "nbs-late" => (
                format!(
                    "{seed}{tx}BEGIN 210\nCOMMIT\nBEGIN 211\nRX 1792 8 3000000000000000\nCOMMIT\nBEGIN 211\nRX 1792 8 0322123400000000\nCOMMIT\n"
                ),
                vec![(10, "100D621234123456"), (211, "0762123400000000")],
                2,
            ),
            "ncr-on-deadline" => (
                format!("{rx}BEGIN 210\nRX 1792 8 2134123400000000\nCOMMIT\n"),
                vec![(10, "3000000000000000"), (210, "037F221300000000")],
                0,
            ),
            "ncr-late" => (
                format!(
                    "{rx}BEGIN 210\nCOMMIT\nBEGIN 211\nRX 1792 8 2134123400000000\nCOMMIT\nBEGIN 211\nRX 1792 8 0322123400000000\nCOMMIT\n"
                ),
                vec![(10, "3000000000000000"), (211, "0762123400000000")],
                2,
            ),
            "s3-expired" | "s3-input-at-boundary" => {
                let mut script = "BEGIN 0\nRX 1792 8 0210030000000000\nCOMMIT\n".to_owned();
                for at in case["input_epochs"].as_array().unwrap() {
                    script.push_str(&format!("BEGIN {}\nCOMMIT\n", at.as_u64().unwrap()));
                }
                script.push_str("BEGIN 5000\nRX 1792 8 0322F18600000000\nCOMMIT\n");
                (
                    script,
                    vec![
                        (0, "065003003201F400"),
                        (5000, case["expected_can"].as_str().unwrap()),
                    ],
                    0,
                )
            }
            _ => panic!("unimplemented immutable timing case: {name}"),
        };
        let text = run(&binary, &scratch.0.join(name), &script);
        assert_eq!(
            frames(&text, 1800),
            expected
                .iter()
                .map(|(at, data)| (*at, (*data).to_owned()))
                .collect::<Vec<_>>(),
            "{name}: {text}"
        );
        assert!(!text.contains("REJECT "), "{name}: {text}");
        assert_eq!(
            text.matches("COMMIT_ERROR ").count(),
            failures,
            "{name}: {text}"
        );
        if failures != 0 {
            assert_eq!(
                text.matches("transport_status=10 transport_epoch=210 transport_count=1")
                    .count(),
                1,
                "{name}: {text}"
            );
            let late_status = if name == "nbs-late" { 12 } else { 9 };
            assert!(
                text.contains(&format!("input_status={late_status} transport_status=0")),
                "{name}: {text}"
            );
            assert!(
                text.contains("COMMIT_OK batch=5 epoch=211")
                    || text.contains("COMMIT_OK batch=4 epoch=211"),
                "recovery: {text}"
            );
        }
        let end = if name.starts_with("s3-") {
            5000
        } else if name.ends_with("late") {
            211
        } else {
            210
        };
        let periodic: Vec<_> = (1..=end / 10)
            .map(|index| {
                let at = index * 10;
                let value = if name.starts_with("nbs-") && at < 30 {
                    "78563412"
                } else {
                    "00000000"
                };
                (at, value.to_owned())
            })
            .collect();
        assert_eq!(
            frames(&text, 801),
            periodic,
            "periodic timing oracle: {name}"
        );
        println!("EPIC4_PROTOCOL_TIMING {name} pass");
    }
    // Every unselected service is refused before its old host-v1 handler.
    // Each refusal is followed by a real selected read in the same process.
    for (name, request, response) in [
        ("write-unselected", "072E123411223344", "037F2E1100000000"),
        ("routine-unselected", "0431011234000000", "037F311100000000"),
        (
            "security-unselected",
            "0227010000000000",
            "037F271100000000",
        ),
        (
            "dtc-read-unselected",
            "031902FF00000000",
            "037F191100000000",
        ),
        (
            "dtc-clear-unselected",
            "0414FFFFFF000000",
            "037F141100000000",
        ),
        (
            "dtc-setting-unselected",
            "0285020000000000",
            "037F851100000000",
        ),
        ("unknown-service", "0199000000000000", "037F991100000000"),
        ("session-bad-length", "0110000000000000", "037F101300000000"),
        ("tester-bad-length", "013E000000000000", "037F3E1300000000"),
        (
            "tester-bad-subfunction",
            "023E010000000000",
            "037F3E1200000000",
        ),
        ("tester-present", "023E000000000000", "027E000000000000"),
    ] {
        let script = format!(
            "BEGIN 0\nRX 1792 8 {request}\nCOMMIT\nBEGIN 0\nRX 1792 8 0322123400000000\nCOMMIT\n"
        );
        let text = run(&binary, &scratch.0.join(name), &script);
        assert_eq!(
            frames(&text, 1800),
            vec![(0, response.to_owned()), (0, "0762123400000000".to_owned())],
            "{name}: {text}"
        );
        assert_eq!(text.matches("COMMIT_OK ").count(), 2, "{text}");
    }
    let text = run(
        &binary,
        &scratch.0.join("tester-suppress"),
        "BEGIN 0\nRX 1792 8 023E800000000000\nCOMMIT\nBEGIN 0\nRX 1792 8 0322123400000000\nCOMMIT\n",
    );
    assert_eq!(
        frames(&text, 1800),
        vec![(0, "0762123400000000".to_owned())]
    );
    assert_eq!(text.matches("COMMIT_OK ").count(), 2);
    for (name, bad) in [
        ("flow-wait", "3100000000000000"),
        ("flow-overflow", "3200000000000000"),
        ("flow-invalid-status", "3300000000000000"),
        ("flow-invalid-stmin", "3000F00000000000"),
    ] {
        let script = format!(
            "{seed}{tx}BEGIN 11\nRX 1792 8 {bad}\nCOMMIT\nBEGIN 11\nRX 1792 8 0322123400000000\nCOMMIT\n"
        );
        let text = run(&binary, &scratch.0.join(name), &script);
        assert_eq!(
            frames(&text, 1800),
            vec![
                (10, "100D621234123456".to_owned()),
                (11, "0762123412345678".to_owned())
            ],
            "{name}: {text}"
        );
        assert_eq!(text.matches("COMMIT_ERROR ").count(), 1, "{name}: {text}");
        assert!(
            text.contains("input_status=12 transport_status=0"),
            "{text}"
        );
        assert!(text.contains("COMMIT_OK batch=4 epoch=11"), "{text}");
    }
    let script = format!(
        "{rx}BEGIN 11\nRX 1792 8 2234123400000000\nCOMMIT\nBEGIN 11\nRX 1792 8 0322123400000000\nCOMMIT\n"
    );
    let text = run(&binary, &scratch.0.join("wrong-sequence"), &script);
    assert_eq!(
        frames(&text, 1800),
        vec![
            (10, "3000000000000000".to_owned()),
            (11, "0762123400000000".to_owned())
        ]
    );
    assert_eq!(text.matches("COMMIT_ERROR ").count(), 1);
    assert!(text.contains("input_status=9 transport_status=0"), "{text}");
    let script = format!(
        "{seed}{tx}BEGIN 250\nRX 1792 8 0322123400000000\nCOMMIT\nBEGIN 250\nRX 1792 8 0322123400000000\nCOMMIT\n"
    );
    let text = run(&binary, &scratch.0.join("timeout-prefix"), &script);
    assert_eq!(
        frames(&text, 1800),
        vec![
            (10, "100D621234123456".to_owned()),
            (250, "0762123400000000".to_owned()),
            (250, "0762123400000000".to_owned())
        ]
    );
    assert!(text.contains("COMMIT_ERROR batch=3 epoch=250"), "{text}");
    assert!(
        text.contains("input_status=0 transport_status=10 transport_epoch=210 transport_count=1"),
        "{text}"
    );
    assert!(text.contains("COMMIT_OK batch=4 epoch=250"), "{text}");
    // Independently generate/build the old target with its own old settings
    // and literal offline vectors. These are not evidence for the new target.
    super::diagnostic_transport_discards_bad_or_timed_out_multiframe_requests_and_recovers();
    super::diagnostic_tester_present_keeps_session_and_fc_block_size_paces_response();
    super::extended_session_write_did_changes_live_can_but_not_restart_state();
    super::security_access_roundtrips_and_gates_host_writes();
    super::start_routine_restores_written_did_signals_and_respects_session();
    println!("EPIC4_PROTOCOL_LEGACY five independent host-v1 configurations pass");
    let mut observations = BTreeMap::new();
    for entry in fs::read_dir(&scratch.0).unwrap() {
        let directory = entry.unwrap().path();
        if directory.join("stdout").is_file() {
            let text = fs::read_to_string(directory.join("stdout")).unwrap();
            let records: Vec<_> = text
                .lines()
                .filter(|line| line.starts_with("OUT ") || line.starts_with("COMMIT_"))
                .collect();
            assert!(
                text.contains("lifecycle=Closed state=Ready reason=0"),
                "{text}"
            );
            observations.insert(directory.file_name().unwrap().to_str().unwrap().to_owned(), serde_json::json!({"exit_code":0,"records":records,"raw_stdout_sha256":digest(&directory.join("stdout")),"raw_stderr_sha256":digest(&directory.join("stderr"))}));
        }
    }
    let mut generated = BTreeMap::new();
    tree(&source, &source, &mut generated);
    let repository: BTreeMap<_, _> = [
        "core/tests/end_to_end.rs",
        "core/tests/support/epic4_protocol.rs",
        "runtime/src/Dcm.c",
        "runtime/src/CanTp.c",
        "runtime/contracts/bsw-v1.json",
        "runtime/ecu/include/Ecu_HostBatch.h",
        "runtime/ecu/include/Ecu_Target.h",
        "runtime/ecu/src/Ecu_Target.c",
        "runtime/ecu/src/Ecu_HostBridge.c",
        "runtime/ecu/src/ecu_host_batch.c",
    ]
    .iter()
    .map(|name| ((*name).to_owned(), digest(&root.join(name))))
    .collect();
    println!(
        "EPIC4_PROTOCOL_EVIDENCE {}",
        serde_json::json!({"format":"epic4-independent-protocol-v1","status":"pass for story4.16 host protocol and legacy scope; full SC1/MISRA/handoff remain open","baseline_commit":"8431b5bf6834db903692d9b9081cf7b64a2c2d8f","oracle_sha256":digest(&root.join("core/tests/fixtures/epic4_oracles/protocol.json")),"repository_sources":repository,"generated_sources":generated,"binary_sha256":digest(&binary),"observations":observations,"legacy":"five separate host-v1 configurations built and run with existing literal offline vectors","limits":["Headless Win64 host evidence only.","Unselected services stay available only on the separate old target.","Raw stack/thread observations omitted; raw output digests retained.","Full 221-item normative assessment, Required approvals, adopted code, full SC1 and handoff exits remain open."]})
    );
}
