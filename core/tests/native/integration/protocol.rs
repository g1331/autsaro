//! Host batch protocol and timing of a generated single-component project.
//!
//! Independent CAN/UDS protocol, timing and HostBatch expectations replayed
//! on the mainline single-SWC project (`Ingress.c` reference application).
//!
//! The byte vectors come from `fixtures/epic4_oracles/protocol.json`, whose
//! settings are bound to the mainline ECUC before use. Two mainline facts
//! shape the replay and are not taken from generated output:
//! - CanTp and Dcm handle received frames in their 1 ms main functions. A
//!   HostBatch batch at a new epoch is injected before that epoch's tick; a
//!   batch at an epoch that already ticked (including StartOS epoch 0) does
//!   not rerun periodic work (docs/architecture.md, batch protocol). Every
//!   diagnostic input therefore gets its own tick.
//! - `Ingress_Periodic` commits the received value on `E_OK` and on the
//!   overlaid `RTE_E_MAX_AGE_EXCEEDED` (SWS_Rte_02703; `ComRxDataTimeoutAction`
//!   is NONE, so the last value is kept) and writes it to 0x321. Before the
//!   first reception (`RTE_E_NEVER_RECEIVED`, SWS_Rte_07643) nothing is
//!   committed and Com sends the init value 0. The DID returns the committed
//!   value big-endian. Unlike EchoApplication nothing expires after 30 ms.
use super::*;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const REQUEST: u64 = 1792;

const RESPONSE: u64 = 1800;

const RECEIVE: u64 = 800;

const TRANSMIT: u64 = 801;

const READ: &str = "0322123400000000";

struct Built {
    scratch: super::Scratch,
    project: PathBuf,
    binary: PathBuf,
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn require_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn build(mode: &str, control: Option<&Path>, built: &super::Scratch, project: &Path) -> PathBuf {
    let output = built.0.join(format!("{mode}-build"));
    require_success(&super::tooling::run_public_command(
        &mut super::tooling::ecu_build_command(project, &output, mode, control),
        &built.0,
        &format!("{mode}-build"),
        Duration::from_secs(180),
    ));
    output
}

fn host_batch_project() -> Built {
    let scratch = super::Scratch::new();
    let (project, _) = super::single_component_project(&scratch.0, super::tooling::native_target());
    let output = build("host-batch", None, &scratch, &project);
    let binary = super::tooling::native_binary(&output, "ecu_host_batch");
    Built {
        scratch,
        project,
        binary,
    }
}

fn run(binary: &Path, directory: &Path, script: &[u8]) -> String {
    fs::create_dir_all(directory).unwrap();
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(fs::File::create(directory.join("stdout")).unwrap())
        .stderr(fs::File::create(directory.join("stderr")).unwrap())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(script).unwrap();
    let started = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("HostBatch exceeded its watchdog: {}", directory.display());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let status = child.wait().unwrap();
    let text = fs::read_to_string(directory.join("stdout")).unwrap();
    let errors = fs::read_to_string(directory.join("stderr")).unwrap();
    assert!(status.success(), "{}\n{text}{errors}", directory.display());
    assert!(text.starts_with("READY HostBatchV1\n"), "{text}");
    assert!(
        text.contains("lifecycle=Closed state=Ready reason=0"),
        "{text}"
    );
    let mut previous = 0;
    for line in text
        .lines()
        .filter(|line| line.starts_with("OUT ") || line.starts_with("COMMIT_"))
    {
        let sequence = field(line, "sequence=").parse::<u64>().unwrap();
        assert!(sequence > previous, "nonmonotonic identity: {text}");
        previous = sequence;
    }
    text
}

fn field<'a>(line: &'a str, key: &str) -> &'a str {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(key))
        .unwrap_or_else(|| panic!("missing {key}: {line}"))
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

fn owned(frames: &[(u64, &str)]) -> Vec<(u64, String)> {
    frames
        .iter()
        .map(|(at, data)| (*at, (*data).to_owned()))
        .collect()
}

/// 0x321 at every 10 ms application period: the last 0x320 value received
/// at or before that tick, or the init value before any reception.
fn periodic(receives: &[(u64, String)], end: u64) -> Vec<(u64, String)> {
    (1..=end / 10)
        .map(|index| {
            let at = index * 10;
            let data = receives
                .iter()
                .rev()
                .find(|(received, _)| *received <= at)
                .map_or("00000000", |(_, data)| data.as_str());
            (at, data.to_owned())
        })
        .collect()
}

fn script(batches: &[(u64, &[(u64, &str)])]) -> String {
    let mut text = String::new();
    for (at, rx) in batches {
        text.push_str(&format!("BEGIN {at}\n"));
        for (id, data) in *rx {
            text.push_str(&format!("RX {id} {} {data}\n", data.len() / 2));
        }
        text.push_str("COMMIT\n");
    }
    text
}

fn bind_oracle_settings() {
    // The oracle settings must equal the ECUC actually built here.
    let ecuc = single_component_sources()
        .into_iter()
        .find(|source| source.logical_path() == "ecuc.arxml")
        .unwrap();
    protocol_oracles::validate(
        &protocol_oracles::load("protocol.json"),
        &protocol_oracles::load("os.json"),
        ecuc.bytes(),
    )
    .unwrap_or_else(|error| panic!("mainline ECUC: {error}"));
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn single_component_meets_independent_protocol_and_timing_vectors() {
    let oracle: Value = serde_json::from_slice(
        &fs::read(root().join("core/tests/fixtures/epic4_oracles/protocol.json")).unwrap(),
    )
    .unwrap();
    let built = host_batch_project();
    bind_oracle_settings();
    let binary = &built.binary;
    let directory = |name: &str| built.scratch.0.join("cases").join(name);

    // Oracle DID bytes that encode EchoApplication's 30 ms expiry. Ingress
    // still holds 0x12345678 at 30 ms (module comment).
    let ingress_did = [("rx-expires-at-30", "0762123412345678")];
    let cases = oracle["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 11);
    for case in cases {
        let name = case["id"].as_str().unwrap();
        let mut batches: Vec<(u64, Vec<(u64, String)>)> = Vec::new();
        if case.get("precondition").is_some() {
            batches.push((0, vec![(RECEIVE, "78563412".to_owned())]));
            batches.push((10, Vec::new()));
        }
        let first = batches.len();
        for input in case["input"].as_array().unwrap() {
            let rx = input["rx"]
                .as_array()
                .unwrap()
                .iter()
                .map(|frame| {
                    (
                        frame["id"].as_u64().unwrap(),
                        frame["data"].as_str().unwrap().to_owned(),
                    )
                })
                .collect();
            batches.push((input["epoch"].as_u64().unwrap(), rx));
        }
        // Give each diagnostic batch its own tick; StartOS epoch 0 has none.
        let mut ticked = 0;
        let mut diagnostic: Vec<(u64, u64)> = Vec::new();
        let mut text = String::new();
        let mut receives = Vec::new();
        for (index, (at, rx)) in batches.iter().enumerate() {
            let has_request = rx.iter().any(|(id, _)| *id == REQUEST);
            let replay = if has_request {
                (*at).max(ticked + 1)
            } else {
                (*at).max(ticked)
            };
            ticked = replay;
            if has_request && index >= first {
                diagnostic.push((*at, replay));
            }
            text.push_str(&format!("BEGIN {replay}\n"));
            for (id, data) in rx {
                text.push_str(&format!("RX {id} {} {data}\n", data.len() / 2));
                if *id == RECEIVE {
                    receives.push((replay, data.clone()));
                }
            }
            text.push_str("COMMIT\n");
        }
        let output = run(binary, &directory(name), text.as_bytes());
        assert!(
            !output.contains("REJECT ") && !output.contains("COMMIT_ERROR "),
            "{name}: {output}"
        );
        // The k-th expected response at an oracle epoch answers the k-th
        // diagnostic batch at that epoch.
        let mut used = std::collections::BTreeMap::<u64, usize>::new();
        let expected: Vec<_> = case["expected"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|frame| frame["id"] == RESPONSE)
            .map(|frame| {
                let at = frame["epoch"].as_u64().unwrap();
                let slot = used.entry(at).or_default();
                let replay = diagnostic
                    .iter()
                    .filter(|(oracle_at, _)| *oracle_at == at)
                    .nth(*slot)
                    .unwrap_or_else(|| panic!("{name}: no request for response at {at}"))
                    .1;
                *slot += 1;
                let data = ingress_did
                    .iter()
                    .find(|(case, _)| *case == name)
                    .map_or(frame["data"].as_str().unwrap(), |(_, data)| data);
                (replay, data.to_owned())
            })
            .collect();
        assert_eq!(frames(&output, RESPONSE), expected, "{name}: {output}");
        assert_eq!(
            frames(&output, TRANSMIT),
            periodic(&receives, ticked),
            "periodic {name}: {output}"
        );
    }

    // Timing cases: the oracle deadlines start at FF/FC confirmation (after
    // that tick's main functions) and an input at the deadline epoch is
    // handled before timeout processing.
    let seed: (u64, &[(u64, &str)]) = (0, &[(RECEIVE, "78563412")]);
    let tx: (u64, &[(u64, &str)]) = (10, &[(REQUEST, "0522123412340000")]);
    let rx: (u64, &[(u64, &str)]) = (10, &[(REQUEST, "1009221234123412")]);
    let read: &[(u64, &str)] = &[(REQUEST, READ)];
    let fc: &[(u64, &str)] = &[(REQUEST, "3000000000000000")];
    let cf: &[(u64, &str)] = &[(REQUEST, "2134123400000000")];
    let timing = oracle["timing_cases"].as_array().unwrap();
    assert_eq!(timing.len(), 6);
    for case in timing {
        let name = case["id"].as_str().unwrap();
        let (text, expected, timeout, seeded) = match name {
            "nbs-on-deadline" => (
                script(&[seed, tx, (210, fc)]),
                vec![(10, "100D621234123456"), (210, "2178123412345678")],
                false,
                true,
            ),
            // The late FC is ignored as unexpected; the recovery read uses
            // its own tick and returns Ingress's committed value.
            "nbs-late" => (
                script(&[seed, tx, (210, &[]), (211, fc), (212, read)]),
                vec![(10, "100D621234123456"), (212, "0762123412345678")],
                true,
                true,
            ),
            "ncr-on-deadline" => (
                script(&[rx, (210, cf)]),
                vec![(10, "3000000000000000"), (210, "037F221300000000")],
                false,
                false,
            ),
            "ncr-late" => (
                script(&[rx, (210, &[]), (211, cf), (212, read)]),
                vec![(10, "3000000000000000"), (212, "0762123400000000")],
                true,
                false,
            ),
            "s3-expired" | "s3-input-at-boundary" => {
                // The session response is confirmed at tick 1, so S3 ends at
                // 5001; the oracle timeline shifts by that one tick.
                let s3 = oracle["settings"]["s3_ms"].as_u64().unwrap();
                let mut text = script(&[(1, &[(REQUEST, "0210030000000000")])]);
                let mut last = 1;
                for at in case["input_epochs"].as_array().unwrap() {
                    last = at.as_u64().unwrap() + 1;
                    text.push_str(&format!("BEGIN {last}\nCOMMIT\n"));
                }
                let query = (1 + s3).max(last + 1);
                text.push_str(&script(&[(query, &[(REQUEST, "0322F18600000000")])]));
                let output = run(binary, &directory(name), text.as_bytes());
                assert_eq!(
                    frames(&output, RESPONSE),
                    vec![
                        (1, "065003003201F400".to_owned()),
                        (query, case["expected_can"].as_str().unwrap().to_owned())
                    ],
                    "{name}: {output}"
                );
                assert!(
                    !output.contains("REJECT ") && !output.contains("COMMIT_ERROR "),
                    "{name}: {output}"
                );
                assert_eq!(frames(&output, TRANSMIT), periodic(&[], query));
                continue;
            }
            _ => panic!("unimplemented immutable timing case: {name}"),
        };
        let output = run(binary, &directory(name), text.as_bytes());
        assert_eq!(
            frames(&output, RESPONSE),
            owned(&expected),
            "{name}: {output}"
        );
        assert!(!output.contains("REJECT "), "{name}: {output}");
        let errors: Vec<_> = output
            .lines()
            .filter(|line| line.starts_with("COMMIT_ERROR "))
            .collect();
        if timeout {
            // The CanTp timeout is reported on the batch reaching the deadline.
            assert_eq!(errors.len(), 1, "{name}: {output}");
            assert!(errors[0].contains(" epoch=210 "), "{name}: {output}");
            assert!(
                errors[0].contains("transport_status=10 transport_epoch=210 transport_count=1"),
                "{name}: {output}"
            );
        } else {
            assert!(errors.is_empty(), "{name}: {output}");
        }
        let end = text
            .lines()
            .filter_map(|line| line.strip_prefix("BEGIN "))
            .map(|at| at.parse::<u64>().unwrap())
            .max()
            .unwrap();
        let receives = if seeded {
            vec![(0, "78563412".to_owned())]
        } else {
            Vec::new()
        };
        // A CF sent by CanTp_MainFunction occupies the only Tx hardware object
        // (CanHwObjectCount 1, CanIfBufferSize 0) when Com transmits in the
        // same tick; CanIf refuses 0x321 and Com does not retry it
        // (ComRetryFailedTransmitRequests is not configured).
        let blocked = if name == "nbs-on-deadline" {
            Some(210)
        } else {
            None
        };
        let mut expected_periodic = periodic(&receives, end);
        expected_periodic.retain(|(at, _)| Some(*at) != blocked);
        assert_eq!(
            frames(&output, TRANSMIT),
            expected_periodic,
            "periodic {name}: {output}"
        );
    }

    // Services not selected in the configuration are refused with NRC 0x11
    // (SWS_Dcm_00197); malformed session/TesterPresent requests with 0x13/0x12.
    // Each refusal is followed by a real selected read.
    for (name, request, response) in [
        (
            "write-unselected",
            "072E123411223344",
            Some("037F2E1100000000"),
        ),
        (
            "routine-unselected",
            "0431011234000000",
            Some("037F311100000000"),
        ),
        (
            "security-unselected",
            "0227010000000000",
            Some("037F271100000000"),
        ),
        (
            "dtc-read-unselected",
            "031902FF00000000",
            Some("037F191100000000"),
        ),
        (
            "dtc-clear-unselected",
            "0414FFFFFF000000",
            Some("037F141100000000"),
        ),
        (
            "dtc-setting-unselected",
            "0285020000000000",
            Some("037F851100000000"),
        ),
        (
            "unknown-service",
            "0199000000000000",
            Some("037F991100000000"),
        ),
        (
            "session-bad-length",
            "0110000000000000",
            Some("037F101300000000"),
        ),
        (
            "tester-bad-length",
            "013E000000000000",
            Some("037F3E1300000000"),
        ),
        (
            "tester-bad-subfunction",
            "023E010000000000",
            Some("037F3E1200000000"),
        ),
        (
            "tester-present",
            "023E000000000000",
            Some("027E000000000000"),
        ),
        ("tester-suppress", "023E800000000000", None),
    ] {
        let output = run(
            binary,
            &directory(name),
            script(&[(1, &[(REQUEST, request)]), (2, read)]).as_bytes(),
        );
        let mut expected = Vec::new();
        if let Some(response) = response {
            expected.push((1, response));
        }
        expected.push((2, "0762123400000000"));
        assert_eq!(
            frames(&output, RESPONSE),
            owned(&expected),
            "{name}: {output}"
        );
        assert_eq!(output.matches("COMMIT_OK ").count(), 2, "{name}: {output}");
    }

    // FC(OVFLW) and a reserved FlowStatus abort the transmission; the next
    // single-frame request is served (ISO 15765-2 FlowStatus).
    for (name, bad) in [
        ("flow-overflow", "3200000000000000"),
        ("flow-invalid-status", "3300000000000000"),
    ] {
        let output = run(
            binary,
            &directory(name),
            script(&[seed, tx, (11, &[(REQUEST, bad)]), (12, read)]).as_bytes(),
        );
        assert_eq!(
            frames(&output, RESPONSE),
            owned(&[(10, "100D621234123456"), (12, "0762123412345678")]),
            "{name}: {output}"
        );
    }
    // FC(WAIT) restarts N_Bs (SWS_CanTp_00315): a CTS after the original
    // 210 deadline but inside the restarted one still completes the read.
    let output = run(
        binary,
        &directory("flow-wait"),
        script(&[seed, tx, (100, &[(REQUEST, "3100000000000000")]), (250, fc)]).as_bytes(),
    );
    assert_eq!(
        frames(&output, RESPONSE),
        owned(&[(10, "100D621234123456"), (250, "2178123412345678")]),
        "{output}"
    );
    assert!(!output.contains("COMMIT_ERROR "), "{output}");
    // A reserved STmin (0xF0) is handled as 0x7F (127 ms), not as an abort.
    let output = run(
        binary,
        &directory("flow-reserved-stmin"),
        script(&[seed, tx, (11, &[(REQUEST, "3000F00000000000")]), (200, &[])]).as_bytes(),
    );
    let responses = frames(&output, RESPONSE);
    assert_eq!(responses.len(), 2, "{output}");
    assert_eq!(responses[1].1, "2178123412345678", "{output}");
    assert!((12..=11 + 127).contains(&responses[1].0), "{output}");
    assert!(!output.contains("COMMIT_ERROR "), "{output}");
    // A wrong sequence number aborts the reception without a response.
    let output = run(
        binary,
        &directory("wrong-sequence"),
        script(&[rx, (11, &[(REQUEST, "2234123400000000")]), (12, read)]).as_bytes(),
    );
    assert_eq!(
        frames(&output, RESPONSE),
        owned(&[(10, "3000000000000000"), (12, "0762123400000000")]),
        "{output}"
    );
    // A batch crossing the N_Bs deadline reports that timeout, still serves
    // its own request, and the next batch is clean.
    let output = run(
        binary,
        &directory("timeout-prefix"),
        script(&[seed, tx, (250, read), (251, read)]).as_bytes(),
    );
    assert_eq!(
        frames(&output, RESPONSE),
        owned(&[
            (10, "100D621234123456"),
            (250, "0762123412345678"),
            (251, "0762123412345678")
        ]),
        "{output}"
    );
    assert!(
        output.contains("COMMIT_ERROR batch=3 epoch=250 "),
        "{output}"
    );
    assert!(
        output.contains("transport_status=10 transport_epoch=210 transport_count=1"),
        "{output}"
    );
    assert!(output.contains("COMMIT_OK batch=4 epoch=251 "), "{output}");
}

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
            if sender.send(line.unwrap()).is_err() {
                break;
            }
        }
    });
    let received =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
            let next = || {
                receiver
                    .recv_timeout(Duration::from_secs(7))
                    .map_err(|error| error.to_string())
            };
            assert_eq!(next()?, "READY HostBatchV1");
            input
                .write_all(b"BEGIN 10\nRX 800 4 78563412\nCOMMIT\n")
                .unwrap();
            let output = next()?;
            assert!(output.starts_with("OUT epoch=10 "), "{output}");
            assert!(output.ends_with("id=801 dlc=4 data=78563412"), "{output}");
            let receipt = next()?;
            assert!(
                receipt.starts_with("COMMIT_OK batch=1 epoch=10 "),
                "{receipt}"
            );
            assert!(child.try_wait().unwrap().is_none());
            // stdin is still open: neither output nor receipt can come from
            // process-exit flushing.
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
        if started.elapsed() > Duration::from_secs(7) {
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
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

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn single_component_host_batch_commits_refuses_and_closes_on_faults() {
    let built = host_batch_project();
    let binary = &built.binary;
    let directory = |name: &str| built.scratch.0.join("commit").join(name);
    live_output(binary);
    // The 801 frame is refused as an input: 0x321 is a transmitted PDU.
    let text = run(
        binary,
        &directory("basic"),
        b"BEGIN 0\nRX 0x320 4 78563412\nCOMMIT\nBEGIN 10\nRX 0x700 8 0322123400000000\nCOMMIT\nBEGIN 10\nCOMMIT\nBEGIN 11\nRX 801 4 00000000\nCOMMIT\nBEGIN 11\nCOMMIT\n",
    );
    assert_eq!(text.matches("COMMIT_OK ").count(), 4, "{text}");
    assert_eq!(text.matches("REJECT ").count(), 1, "{text}");
    assert_eq!(text.matches("OUT epoch=").count(), 2, "{text}");
    for expected in [
        "COMMIT_OK batch=1 epoch=0 sequence=2 input_first=1 inputs=1",
        "OUT epoch=10 sequence=13 ticket=1 pdu=1 id=801 dlc=4 data=78563412",
        "OUT epoch=10 sequence=14 ticket=2 pdu=3 id=1800 dlc=8 data=0762123412345678",
        "COMMIT_OK batch=2 epoch=10 sequence=16 input_first=12 inputs=1",
        "COMMIT_OK batch=3 epoch=10 sequence=17 input_first=0 inputs=0",
        "REJECT status=8 epoch=10 sequence=17",
        "COMMIT_OK batch=4 epoch=11 sequence=19 input_first=0 inputs=0",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    // An epoch going back is refused before any state change.
    let text = run(
        binary,
        &directory("epoch-back"),
        b"BEGIN 10\nCOMMIT\nBEGIN 9\nRX 800 4 78563412\nCOMMIT\nBEGIN 20\nCOMMIT\n",
    );
    assert!(text.contains("REJECT status=8 epoch=10 "), "{text}");
    assert_eq!(text.matches("COMMIT_OK ").count(), 2, "{text}");
    assert_eq!(
        frames(&text, TRANSMIT),
        owned(&[(10, "00000000"), (20, "00000000")])
    );
    // A malformed diagnostic frame discards the whole batch.
    let text = run(
        binary,
        &directory("diagnostic-dlc"),
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
    // 256 inputs fit one batch; the 257th refuses the batch.
    let mut bounded = String::from("BEGIN 0\n");
    for _ in 0..256 {
        bounded.push_str("RX 800 4 78563412\n");
    }
    bounded.push_str("COMMIT\nBEGIN 10\nCOMMIT\n");
    let text = run(binary, &directory("capacity256"), bounded.as_bytes());
    assert_eq!(text.matches("COMMIT_OK ").count(), 2, "{text}");
    assert!(text.contains("inputs=256 status=0"), "{text}");
    assert!(text.contains("data=78563412"), "{text}");
    let overflow = bounded.replacen("COMMIT\n", "RX 800 4 78563412\nCOMMIT\n", 1);
    let text = run(binary, &directory("capacity257"), overflow.as_bytes());
    assert_eq!(text.matches("REJECT ").count(), 2, "{text}");
    assert_eq!(text.matches("COMMIT_OK ").count(), 1, "{text}");
    assert!(
        text.contains("data=00000000") && !text.contains("data=78563412"),
        "{text}"
    );
    // One batch spans at most 1000 ms; the maximum span runs every period.
    let text = run(
        binary,
        &directory("span"),
        b"BEGIN 1001\nCOMMIT\nBEGIN 1000\nCOMMIT\n",
    );
    assert_eq!(text.matches("REJECT ").count(), 2, "{text}");
    assert!(text.contains("COMMIT_OK batch=1 epoch=1000 "), "{text}");
    assert_eq!(frames(&text, TRANSMIT), periodic(&[], 1000), "{text}");
    let dropped = text
        .lines()
        .find_map(|line| line.split("trace_dropped=").nth(1))
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(dropped > 0, "{text}");
    // N_Cr is 200 ms: a CF at the exact deadline is consumed first.
    let text = run(
        binary,
        &directory("receive-deadline"),
        b"BEGIN 10\nRX 1792 8 1008221234000000\nCOMMIT\nBEGIN 210\nRX 1792 8 2100000000000000\nCOMMIT\n",
    );
    assert!(!text.contains("COMMIT_ERROR "), "{text}");
    assert_eq!(
        frames(&text, RESPONSE),
        owned(&[(10, "3000000000000000"), (210, "037F221300000000")]),
        "{text}"
    );
    // A FF arriving while the Dcm still owns an unanswered request is refused
    // by StartOfReception: no FC is sent and the pending request is served.
    let text = run(
        binary,
        &directory("busy-connection"),
        b"BEGIN 1\nRX 1792 8 0322123400000000\nRX 1792 8 1009221234123412\nCOMMIT\nBEGIN 2\nRX 1792 8 0322123400000000\nCOMMIT\n",
    );
    assert!(!text.contains("COMMIT_ERROR "), "{text}");
    assert!(text.contains("COMMIT_OK batch=2 epoch=2 "), "{text}");
    assert_eq!(
        frames(&text, RESPONSE),
        owned(&[(1, "0762123400000000"), (2, "0762123400000000")]),
        "{text}"
    );
    for (name, bad) in [
        ("ctrl-z", b"COMMIT\x1asuffix".as_slice()),
        ("embedded-nul", b"COMMIT\0suffix".as_slice()),
        ("utf8", b"COMMIT\xffsuffix".as_slice()),
    ] {
        let mut script = b"BEGIN 0\nRX 800 4 78563412\n".to_vec();
        script.extend_from_slice(bad);
        script.extend_from_slice(b"\nCOMMIT\nBEGIN 10\nCOMMIT\n");
        let text = run(binary, &directory(name), &script);
        assert_eq!(text.matches("REJECT ").count(), 2, "{text}");
        assert!(
            text.contains("data=00000000") && !text.contains("data=78563412"),
            "{text}"
        );
    }

    // Compile the sealed production writer in the same control TU; the
    // fixture calls that exact function rather than simulating I/O refusal.
    let faults = built.scratch.0.join("host-batch-faults.c");
    fs::write(
        &faults,
        format!(
            "#define main production_main\n#define batch production_batch\n#define control production_control\n{}\n#undef main\n#undef batch\n#undef control\n{}",
            fs::read_to_string(built.project.join("src/ecu_host_batch.c")).unwrap(),
            fs::read_to_string(root().join("core/tests/fixtures/host_batch_faults.c")).unwrap()
        ),
    )
    .unwrap();
    let fault_build = build("probe", Some(&faults), &built.scratch, &built.project);
    // The single-chain `overflow` mode is not replayed: it injects at startup
    // stage 8, which the mainline bootstrap does not have, and the mainline
    // cannot queue more than its one Tx hardware object holds (output
    // reuse is gated by the host confirmation).
    for mode in ["fail", "block", "sequence", "write", "flush"] {
        let directory = built.scratch.0.join("faults").join(mode);
        fs::create_dir_all(&directory).unwrap();
        let started = std::time::Instant::now();
        let mut child = Command::new(super::tooling::native_binary(&fault_build, "ecu_probe"))
            .arg(mode)
            .stdout(fs::File::create(directory.join("stdout")).unwrap())
            .stderr(fs::File::create(directory.join("stderr")).unwrap())
            .spawn()
            .unwrap();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > Duration::from_secs(8) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("{mode} failed to close within its host watchdog");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let result = child.wait().unwrap();
        let mut text = fs::read_to_string(directory.join("stdout")).unwrap();
        text.push_str(&fs::read_to_string(directory.join("stderr")).unwrap());
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
        } else {
            assert_eq!(result.code(), Some(7), "{text}");
            assert_eq!(text.matches("fault_output ").count(), 1, "{text}");
            assert!(!text.contains("fault_commit "), "{text}");
            if matches!(mode, "write" | "flush") {
                assert!(
                    text.contains(&format!("physical_{mode} failed no_receipt ticket=")),
                    "{text}"
                );
                assert!(!text.contains("COMMIT_OK"), "{text}");
            }
            if mode == "block" {
                assert!(started.elapsed() >= Duration::from_millis(4500));
            }
        }
    }
}

#[cfg(all(feature = "native-tests", any(windows, target_os = "linux")))]
#[test]
fn single_component_native_batch_boundary_and_confirmations() {
    let scratch = super::Scratch::new();
    let (project, _) = super::single_component_project(&scratch.0, super::tooling::native_target());
    let output = build(
        "probe",
        Some(&root().join("core/tests/fixtures/host_batch_native.c")),
        &scratch,
        &project,
    );
    let result = super::tooling::run_public_command(
        &mut Command::new(super::tooling::native_binary(&output, "ecu_probe")),
        &scratch.0,
        "native-batch",
        Duration::from_secs(15),
    );
    require_success(&result);
    assert!(
        String::from_utf8(result.stdout).unwrap().contains(
            "native_batch capacity=256 refusal=257 same_epoch=0 future_requires_tick=pass confirmations=pass"
        )
    );
}
