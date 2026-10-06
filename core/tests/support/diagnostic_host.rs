use super::{Scratch, archive, tooling};
use autosar_config_core::{DiagnosticSettings, Direction, Workspace, generator};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(windows)]
pub(super) fn configured_diagnostic_ecu_roundtrips_arxml_and_exchanges_live_multiframe_did() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "LiveA".into(), 0, 32, 0)
        .unwrap();
    let view = project
        .add_signal(frame.clone(), "LiveB".into(), 32, 32, 0)
        .unwrap();
    let sources: Vec<_> = view
        .signals
        .iter()
        .filter(|signal| signal.frame_path == frame)
        .map(|signal| signal.path.clone())
        .collect();
    assert!(
        project
            .configure_diagnostic(DiagnosticSettings {
                request_id: 0x321,
                response_id: 0x708,
                s3_ms: 5000,
                n_as_ms: Some(200),
                n_bs_ms: 200,
                n_cr_ms: 200,
                did: 0x1234,
                signal_paths: sources.clone(),
                write_enabled: false,
                reset_routine_id: None,
                security_enabled: false
            })
            .is_err()
    );
    assert!(project.view().diagnostic.is_none());
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(75),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: sources,
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    let validation = project.validate().unwrap();
    assert!(validation.issues.is_empty(), "{:?}", validation.issues);
    project.save().unwrap();

    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&original).unwrap();
    let global: Vec<_> = doc
        .descendants()
        .filter(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text()
                            == Some("/AUTOSAR/EcucDefs/EcuC/EcucConfigSet/EcucPduCollection/Pdu")
                })
        })
        .collect();
    assert_eq!(
        global.len(),
        5,
        "Com, CanTp and Dcm system PDUs need distinct global handles"
    );
    assert!(
        global.iter().all(|pdu| !pdu
            .descendants()
            .any(|node| node.has_tag_name("DEFINITION-REF")
                && node
                    .text()
                    .is_some_and(|value| value.ends_with("/DynamicLength")))),
        "constr_3448 excludes DynamicLength for I-SIGNAL-I-PDU, N-PDU and DCM-I-PDU"
    );
    let n_pdus: Vec<_> = doc
        .descendants()
        .filter(|node| {
            node.has_tag_name("N-PDU")
                && node.children().any(|child| {
                    child.has_tag_name("SHORT-NAME")
                        && child
                            .text()
                            .is_some_and(|name| name.starts_with("NPdu_Diag"))
                })
        })
        .collect();
    assert_eq!(n_pdus.len(), 2);
    assert!(
        n_pdus.iter().all(|pdu| !pdu
            .children()
            .any(|child| child.has_tag_name("HAS-DYNAMIC-LENGTH"))),
        "System Template constr_3448 excludes hasDynamicLength on N-PDU"
    );
    assert!(doc.descendants().any(|node| {
        node.has_tag_name("ECUC-TEXTUAL-PARAM-VALUE")
            && node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|value| value.ends_with("/PduLengthTypeEnum"))
            })
            && node
                .children()
                .any(|child| child.has_tag_name("VALUE") && child.text() == Some("UINT16"))
    }));
    for (name, target) in [
        ("CanIfRxPduRef", "NPdu_DiagRequest"),
        ("CanIfTxPduRef", "NPdu_DiagResponse"),
        ("CanTpRxNPduRef", "NPdu_DiagRequest"),
        ("CanTpTxFcNPduRef", "NPdu_DiagResponse"),
        ("CanTpTxNPduRef", "NPdu_DiagResponse"),
        ("CanTpRxFcNPduRef", "NPdu_DiagRequest"),
        ("CanTpRxNSduRef", "DcmPdu_DiagRequest"),
        ("CanTpTxNSduRef", "DcmPdu_DiagResponse"),
        ("DcmDslProtocolRxPduRef", "DcmPdu_DiagRequest"),
        ("DcmDslProtocolTxPduRef", "DcmPdu_DiagResponse"),
    ] {
        let target = format!("/Diag/EcuCCfg/EcucConfigSet/Pdus/{target}");
        assert!(
            doc.descendants()
                .any(|node| node.has_tag_name("ECUC-REFERENCE-VALUE")
                    && node
                        .children()
                        .any(|child| child.has_tag_name("DEFINITION-REF")
                            && child
                                .text()
                                .is_some_and(|value| value.ends_with(&format!("/{name}"))))
                    && node.children().any(|child| child.has_tag_name("VALUE-REF")
                        && child.attribute("DEST") == Some("ECUC-CONTAINER-VALUE")
                        && child.text() == Some(target.as_str()))),
            "{name} must target {target}"
        );
    }
    assert!(original.contains("CanTpNas"));
    fs::write(&source, original.replacen("CanTpNas", "CanTpMissingNas", 1)).unwrap();
    let missing_n_as = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    assert!(
        missing_n_as
            .view()
            .issues
            .iter()
            .any(|issue| issue.code == "DIAG_UNSUPPORTED")
    );
    fs::write(
        &source,
        original.replace("</ELEMENTS>", "<!-- user annotation --></ELEMENTS>"),
    )
    .unwrap();
    let mut reopened = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    let diagnostic = reopened.view().diagnostic.unwrap();
    assert_eq!((diagnostic.n_as_ms, diagnostic.n_bs_ms), (75, 200));
    assert_eq!(
        (
            diagnostic.request_id,
            diagnostic.response_id,
            diagnostic.did
        ),
        (0x700, 0x708, 0x1234)
    );
    let output = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &output, tooling::native_target()).unwrap();
    let generated_config = fs::read_to_string(output.join("Ecu_Config.c")).unwrap();
    assert!(generated_config.contains("5000u, 75u, 200u, 200u"));
    let mut names: Vec<String> = fs::read_to_string(output.join("files.list"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    names.extend(["files.list".into(), "files.sha256".into()]);
    let original_files: Vec<_> = names
        .iter()
        .map(|name| fs::read(output.join(name)).unwrap())
        .collect();
    generator::generate(&mut reopened, &output, tooling::native_target()).unwrap();
    for (name, expected) in names.iter().zip(&original_files) {
        assert_eq!(
            &fs::read(output.join(name)).unwrap(),
            expected,
            "identical diagnostic ARXML changed {name}"
        );
    }
    let binary = tooling::build_host(&output).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"S 0 287454020\nS 1 1432778632\nR 1792 4 03221234\nR 1792 3 021003\nR 1792 4 03221234\nR 1792 3 300000\nT 5001\nR 1792 4 03221234\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let frames: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        frames,
        [
            "X 1800 4 037F2231",
            "X 1800 7 06500300320032",
            "X 1800 8 100B621234112233",
            "X 1800 6 214455667788",
            "X 1800 4 037F2231",
        ],
        "{log}"
    );
    let report = tooling::run_diagnostic(&temp.0.join("GeneratedDiag")).unwrap();
    assert!(
        report.passed,
        "independent host diagnostic tester failed: {}",
        report.log
    );
    assert!(report.events.iter().any(|event| event.contains("多帧")));
    reopened.clear_diagnostic().unwrap();
    reopened.save().unwrap();
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<!-- user annotation -->")
    );
    let mut signal_only =
        Workspace::open_legacy(vec![temp.0.join("Diag/Diag.arxml")], archive()).unwrap();
    assert!(signal_only.view().diagnostic.is_none());
    let signal_output = temp.0.join("GeneratedSignalsOnly");
    generator::generate(&mut signal_only, &signal_output, tooling::native_target()).unwrap();
    tooling::build_host(&signal_output).unwrap();
    assert!(tooling::run_diagnostic(&signal_output).is_err());
}

#[cfg(windows)]
pub(super) fn active_session_did_reports_session_transitions_and_rejects_invalid_reads() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "LiveValue".into(), 0, 32, 42)
        .unwrap()
        .signals[0]
        .path
        .clone();
    assert!(
        project
            .configure_diagnostic(DiagnosticSettings {
                request_id: 0x700,
                response_id: 0x708,
                s3_ms: 5000,
                n_as_ms: Some(200),
                n_bs_ms: 200,
                n_cr_ms: 200,
                did: 0xF186,
                signal_paths: vec![signal.clone()],
                write_enabled: false,
                reset_routine_id: None,
                security_enabled: false
            })
            .is_err()
    );
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let saved = fs::read(&source).unwrap();
    let mut reopened = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    assert!(reopened.validate().unwrap().issues.is_empty());
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
    assert_eq!(fs::read(&source).unwrap(), saved);
    let binary = tooling::build_host(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"R 1792 4 0322F186\nR 1792 4 0322F187\nR 1792 3 0222F1\nR 1792 3 021003\nR 1792 4 0322F186\nR 1792 3 021001\nR 1792 4 0322F186\nR 1792 3 021003\nT 5001\nR 1792 4 0322F186\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let log = String::from_utf8(output.stdout).unwrap();
    let diagnostic: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        diagnostic,
        [
            "X 1800 5 0462F18601",
            "X 1800 4 037F2231",
            "X 1800 4 037F2213",
            "X 1800 7 06500300320032",
            "X 1800 5 0462F18603",
            "X 1800 7 06500100320032",
            "X 1800 5 0462F18601",
            "X 1800 7 06500300320032",
            "X 1800 5 0462F18601",
        ],
        "{log}"
    );
    let report = tooling::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{report:?}");
    assert!(report.events.iter().any(|event| event.contains("0xF186")));

    let signal = reopened.view().diagnostic.unwrap().signal_paths[0].clone();
    reopened
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0xF187,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    reopened.save().unwrap();
    let generated = temp.0.join("GeneratedF187");
    generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
    tooling::build_host(&generated).unwrap();
    let report = tooling::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "0xF187 is a configurable DID: {report:?}");
}

#[cfg(windows)]
pub(super) fn multiple_dids_keep_request_order_and_skip_unavailable_values() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "LiveValue".into(), 0, 32, 42)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let saved = fs::read(&source).unwrap();
    let mut reopened = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    assert!(reopened.validate().unwrap().issues.is_empty());
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
    assert_eq!(fs::read(&source).unwrap(), saved);
    let binary = tooling::build_host(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut commands = b"R 1792 6 05221234F186\nR 1792 3 021003\nR 1792 6 05221234F186\nR 1792 3 300000\nR 1792 6 0522F1861234\nR 1792 3 300000\nR 1792 6 0522F187F186\nR 1792 6 0522F187F188\nR 1792 5 0422F18612\nR 1792 6 0522F186F186\nR 1792 4 03221234\nR 1792 3 300000\n".to_vec();
    let mut oversized = vec![0x22];
    for _ in 0..86 {
        oversized.extend_from_slice(&[0xF1, 0x86]);
    }
    assert_eq!(oversized.len(), 173);
    let mut first = vec![0x10, oversized.len() as u8];
    first.extend_from_slice(&oversized[..6]);
    let mut request_frames = vec![first];
    for (index, chunk) in oversized[6..].chunks(7).enumerate() {
        let mut frame = vec![0x20 | ((index as u8 + 1) & 0x0F)];
        frame.extend_from_slice(chunk);
        request_frames.push(frame);
    }
    for frame in request_frames {
        let hex = frame
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join("");
        commands.extend_from_slice(format!("R 1792 {} {hex}\n", frame.len()).as_bytes());
    }
    commands.extend_from_slice(b"R 1792 4 0322F186\n");
    ecu.stdin.take().unwrap().write_all(&commands).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let log = String::from_utf8(output.stdout).unwrap();
    let frames: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        frames,
        [
            "X 1800 5 0462F18601",
            "X 1800 7 06500300320032",
            "X 1800 8 100A621234000000",
            "X 1800 5 212AF18603",
            "X 1800 8 100A62F186031234",
            "X 1800 5 210000002A",
            "X 1800 5 0462F18603",
            "X 1800 4 037F2231",
            "X 1800 4 037F2213",
            "X 1800 8 0762F18603F18603",
            "X 1800 8 076212340000002A",
            "X 1800 3 300000",
            "X 1800 4 037F2214",
            "X 1800 5 0462F18603",
        ],
        "{log}"
    );
    let report = tooling::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{report:?}");
    assert!(report.events.iter().any(|event| event.contains("多 DID")));
}

#[cfg(windows)]
pub(super) fn diagnostic_transport_discards_bad_or_timed_out_multiframe_requests_and_recovers() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "LiveValue".into(), 0, 32, 42)
        .unwrap();
    let view = project
        .add_signal(frame, "LiveOther".into(), 32, 32, 43)
        .unwrap();
    let sources = view
        .signals
        .iter()
        .map(|signal| signal.path.clone())
        .collect();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: sources,
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut project, &generated, tooling::native_target()).unwrap();
    let binary = tooling::build_host(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"R 1792 8 100A221234123412\nR 1792 4 22341234\nR 1792 8 100A221234123412\nR 1792 5 2134123412\nR 1792 8 100A221234123412\nT 201\nR 1792 3 023E00\nR 1792 3 021003\nR 1792 4 03221234\nT 402\nR 1792 3 023E00\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let flow_controls = lines
        .iter()
        .filter(|line| **line == "X 1800 3 300000")
        .count();
    assert_eq!(flow_controls, 3, "{log}");
    assert!(lines.contains(&"E TP_SEQUENCE"), "{log}");
    assert!(lines.contains(&"X 1800 4 037F2213"), "{log}");
    assert_eq!(
        lines.iter().filter(|line| **line == "E TP_TIMEOUT").count(),
        2,
        "{log}"
    );
    assert!(lines.contains(&"X 1800 3 027E00"), "{log}");
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("X 1800 8 100B621234")),
        "{log}"
    );
}

#[cfg(windows)]
pub(super) fn diagnostic_tester_present_keeps_session_and_fc_block_size_paces_response() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    for (name, id) in [("LiveA", 0x321), ("LiveB", 0x322)] {
        let frame = project
            .add_frame(name.into(), id, 8, Direction::Tx, Some(1000), None)
            .unwrap()
            .frames
            .into_iter()
            .find(|item| item.name == name)
            .unwrap()
            .path;
        project
            .add_signal(frame.clone(), format!("{name}Low"), 0, 32, 0)
            .unwrap();
        project
            .add_signal(frame, format!("{name}High"), 32, 32, 0)
            .unwrap();
    }
    let sources = project
        .view()
        .signals
        .iter()
        .map(|signal| signal.path.clone())
        .collect();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: sources,
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut project, &generated, tooling::native_target()).unwrap();
    let binary = tooling::build_host(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"S 0 16909060\nS 1 84281096\nS 2 151653132\nS 3 219025168\nR 1792 3 021003\nT 4000\nR 1792 3 023E80\nT 8000\nR 1792 4 03221234\nR 1792 3 300105\nG 0\nT 8005\nG 0\nR 1792 3 300105\nT 13006\nR 1792 4 03221234\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let diagnostic: Vec<_> = lines
        .iter()
        .filter(|line| line.starts_with("X 1800 "))
        .copied()
        .collect();
    assert_eq!(
        diagnostic,
        [
            "X 1800 7 06500300320032",
            "X 1800 8 1013621234010203",
            "X 1800 8 210405060708090A",
            "X 1800 7 220B0C0D0E0F10",
            "X 1800 4 037F2231",
        ],
        "{log}"
    );
    let fence_positions: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("V 0 "))
        .map(|(index, _)| index)
        .collect();
    let second_cf = lines
        .iter()
        .position(|line| *line == "X 1800 7 220B0C0D0E0F10")
        .unwrap();
    assert_eq!(fence_positions.len(), 2, "{log}");
    assert!(
        fence_positions[1] < second_cf,
        "FC block size was ignored: {log}"
    );
}

#[cfg(windows)]
pub(super) fn unsupported_imported_transport_padding_blocks_diagnostic_generation() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "LiveA".into(), 0, 32, 0)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    fs::write(&source, original.replacen(">CANTP_OFF<", ">CANTP_ON<", 1)).unwrap();
    let mut imported = Workspace::open_legacy(vec![source], archive()).unwrap();
    assert!(
        imported
            .view()
            .issues
            .iter()
            .any(|issue| issue.code == "DIAG_UNSUPPORTED")
    );
    let generated = temp.0.join("RejectedDiag");
    assert!(generator::generate(&mut imported, &generated, tooling::native_target()).is_err());
    assert!(!generated.exists());
}

#[cfg(windows)]
pub(super) fn supported_dtcs_include_zero_status_and_follow_configured_lifecycle() {
    let temp = Scratch::new();
    for (name, code, encoded) in [
        ("SupportedA", 0x123456, "123456"),
        ("SupportedB", 0xABCDEF, "ABCDEF"),
    ] {
        let directory = temp.0.join(name);
        let mut project = Workspace::create_legacy(&directory, name, archive()).unwrap();
        let tx = project
            .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
            .unwrap()
            .frames[0]
            .path
            .clone();
        let signal = project
            .add_signal(tx, "LiveValue".into(), 0, 32, 7)
            .unwrap()
            .signals[0]
            .path
            .clone();
        let rx = project
            .add_frame("Heartbeat".into(), 0x456, 2, Direction::Rx, None, Some(50))
            .unwrap()
            .frames
            .into_iter()
            .find(|frame| frame.name == "Heartbeat")
            .unwrap()
            .path;
        project
            .add_signal(rx.clone(), "HeartbeatValue".into(), 0, 8, 0)
            .unwrap();
        project
            .configure_diagnostic(DiagnosticSettings {
                request_id: 0x700,
                response_id: 0x708,
                s3_ms: 5000,
                n_as_ms: Some(200),
                n_bs_ms: 200,
                n_cr_ms: 200,
                did: 0x1234,
                signal_paths: vec![signal],
                write_enabled: false,
                reset_routine_id: None,
                security_enabled: false,
            })
            .unwrap();
        project.configure_dtc(code, rx).unwrap();
        project.save().unwrap();
        let source = directory.join(format!("{name}.arxml"));
        let saved = fs::read(&source).unwrap();
        let mut reopened = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
        assert!(reopened.validate().unwrap().issues.is_empty());
        assert_eq!(reopened.view().diagnostic.unwrap().dtc.unwrap().code, code);
        let generated = directory.join("generated");
        generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
        assert_eq!(fs::read(&source).unwrap(), saved);
        let binary = tooling::build_host(&generated).unwrap().binary_path;
        let storage = directory.join("dtc.nvm");
        let run = |input: &str| {
            let mut ecu = Command::new(&binary)
                .arg("--nvm")
                .arg(&storage)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            ecu.stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            let output = ecu.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            let lines: Vec<String> = String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .map(|line| line.trim_end_matches('\r').to_owned())
                .collect();
            assert!(
                !lines.iter().any(|line| line.starts_with("E ")),
                "{lines:?}"
            );
            lines
                .into_iter()
                .filter(|line| line.starts_with("X 1800 "))
                .collect::<Vec<_>>()
        };
        let supported = |status| format!("X 1800 8 07590A7F{encoded}{status:02X}");
        // Initial reads, including rejected requests, must not mutate persistent state.
        assert!(run("").is_empty());
        let before = fs::read(&storage).unwrap();
        assert_eq!(
            run(
                "R 1792 3 02190A\nR 1792 4 03190A00\nR 1792 2 0119\nR 1792 3 021903\nR 1792 3 02198A\nR 1792 3 02190A\n"
            ),
            vec![
                supported(0x50),
                "X 1800 4 037F1913".into(),
                "X 1800 4 037F1913".into(),
                "X 1800 4 037F1912".into(),
                "X 1800 4 037F1912".into(),
                supported(0x50),
            ]
        );
        assert_eq!(fs::read(&storage).unwrap(), before);
        // Zero status is still supported; a zero status mask remains empty for 01/02.
        assert_eq!(
            run(
                "R 1110 2 0100\nR 1792 3 02190A\nR 1792 4 03190100\nR 1792 4 03190200\nR 1792 4 0319027F\nR 1792 3 021003\nR 1792 3 028502\nT 51\nR 1792 3 02190A\nR 1792 3 028501\nR 1110 2 0100\nT 102\nR 1792 3 02190A\n"
            ),
            vec![
                supported(0x00),
                "X 1800 7 0659017F010000".into(),
                "X 1800 4 0359027F".into(),
                "X 1800 4 0359027F".into(),
                "X 1800 7 06500300320032".into(),
                "X 1800 3 02C502".into(),
                supported(0x00),
                "X 1800 3 02C501".into(),
                supported(0x2F),
            ]
        );
        assert_eq!(
            run(
                "R 1792 3 02190A\nR 1110 2 0100\nR 1792 3 02190A\nR 1792 3 021003\nR 1792 5 0414FFFFFF\nR 1792 3 02190A\n"
            ),
            vec![
                supported(0x6D),
                supported(0x2C),
                "X 1800 7 06500300320032".into(),
                "X 1800 2 0154".into(),
                supported(0x50),
            ]
        );
        assert_eq!(run("R 1792 3 02190A\n"), vec![supported(0x50)]);
        let report = tooling::run_diagnostic(&generated).unwrap();
        assert!(report.passed, "{report:?}");
        assert!(
            report
                .events
                .iter()
                .any(|event| event.contains("0x19/0x0A"))
        );

        reopened.clear_dtc().unwrap();
        reopened.save().unwrap();
        let mut cleared = Workspace::open_legacy(vec![source], archive()).unwrap();
        assert!(cleared.view().diagnostic.unwrap().dtc.is_none());
        let without_dtc = directory.join("without-dtc");
        generator::generate(&mut cleared, &without_dtc, tooling::native_target()).unwrap();
        let binary = tooling::build_host(&without_dtc).unwrap().binary_path;
        let mut ecu = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        ecu.stdin
            .take()
            .unwrap()
            .write_all(b"R 1792 3 02190A\n")
            .unwrap();
        let output = ecu.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "X 1800 4 037F1911"
        );
    }
}

#[cfg(windows)]
pub(super) fn rx_timeout_dtc_is_reported_cleared_and_persists_across_ecu_restarts() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let tx = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let did_signal = project
        .add_signal(tx, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    let rx = project
        .add_frame("Heartbeat".into(), 0x456, 2, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Heartbeat")
        .unwrap()
        .path;
    project
        .add_signal(rx.clone(), "HeartbeatValue".into(), 0, 8, 0)
        .unwrap();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![did_signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.configure_dtc(0x123456, rx.clone()).unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let document = roxmltree::Document::parse(&original).unwrap();
    let dcm_service = document.descendants().find(|node| node.has_tag_name("ECUC-CONTAINER-VALUE") &&
        node.children().any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("ControlDTCSetting")) &&
        node.children().any(|child| child.has_tag_name("DEFINITION-REF") &&
            child.text() == Some("/AUTOSAR/EcucDefs/Dcm/DcmConfigSet/DcmDsd/DcmDsdServiceTable/DcmDsdService"))).unwrap();
    assert!(dcm_service.descendants().any(|node| {
        node.has_tag_name("ECUC-NUMERICAL-PARAM-VALUE")
            && node.children().any(|child| {
                child.has_tag_name("DEFINITION-REF")
                    && child
                        .text()
                        .is_some_and(|value| value.ends_with("/DcmDsdSidTabServiceId"))
            })
            && node
                .children()
                .any(|child| child.has_tag_name("VALUE") && child.text() == Some("133"))
    }));
    assert!(
        dcm_service
            .descendants()
            .any(|node| node.has_tag_name("VALUE-REF")
                && node.attribute("DEST") == Some("ECUC-CONTAINER-VALUE")
                && node.text() == Some("/Diag/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended"))
    );
    assert!(
        document
            .descendants()
            .any(|node| node.has_tag_name("ECUC-REFERENCE-VALUE")
                && node
                    .children()
                    .any(|child| child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|value| value.ends_with("/DcmDemClientRef")))
                && node.children().any(|child| child.has_tag_name("VALUE-REF")
                    && child.attribute("DEST") == Some("ECUC-CONTAINER-VALUE")
                    && child.text() == Some("/Diag/DemCfg/DemGeneral/DcmClient")))
    );
    let option = document
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-NUMERICAL-PARAM-VALUE")
                && node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|value| {
                            value.ends_with("/DcmSupportDTCSettingControlOptionRecord")
                        })
                })
        })
        .unwrap();
    let option_value = option
        .children()
        .find(|node| node.has_tag_name("VALUE"))
        .unwrap();
    assert_eq!(option_value.text(), Some("false"));
    let mutated = format!(
        "{}<VALUE>true</VALUE>{}",
        &original[..option_value.range().start],
        &original[option_value.range().end..]
    );
    let unsupported_path = temp.0.join("Unsupported.arxml");
    fs::write(&unsupported_path, &mutated).unwrap();
    let mut unsupported =
        Workspace::open_legacy(vec![unsupported_path.clone()], archive()).unwrap();
    assert!(
        unsupported
            .view()
            .issues
            .iter()
            .any(|issue| issue.code == "DIAG_UNSUPPORTED")
    );
    assert!(
        generator::generate(
            &mut unsupported,
            &temp.0.join("UnsupportedOutput"),
            tooling::native_target()
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(&unsupported_path).unwrap(), mutated);
    fs::write(
        &source,
        original.replace("</ELEMENTS>", "<!-- retained by owner --></ELEMENTS>"),
    )
    .unwrap();
    let mut reopened = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    let dtc = reopened.view().diagnostic.unwrap().dtc.unwrap();
    assert_eq!((dtc.code, dtc.monitor_frame_path), (0x123456, rx));
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
    let binary = tooling::build_host(&generated).unwrap().binary_path;
    let without_storage = Command::new(&binary).output().unwrap();
    assert!(!without_storage.status.success());
    assert!(String::from_utf8_lossy(&without_storage.stdout).contains("E CONFIG"));
    let storage = temp.0.join("dtc.nvm");
    let run = |storage: &Path, input: &[u8]| {
        let mut ecu = Command::new(&binary)
            .arg("--nvm")
            .arg(storage)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        ecu.stdin.take().unwrap().write_all(input).unwrap();
        let result = ecu.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
        String::from_utf8(result.stdout).unwrap()
    };
    let first = run(&storage, b"R 1110 2 0100\nR 1792 4 03190208\nR 1792 4 03190108\nR 1792 4 03190100\nR 1792 3 021901\nR 1792 4 03190308\nT 51\nR 1792 4 03190208\nR 1792 4 03190108\nR 1792 4 03190110\nR 1792 4 03190200\n");
    let first: Vec<_> = first
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        first,
        [
            "X 1800 4 0359027F",
            "X 1800 7 0659017F010000",
            "X 1800 7 0659017F010000",
            "X 1800 4 037F1913",
            "X 1800 4 037F1912",
            "X 1800 8 0759027F1234562F",
            "X 1800 7 0659017F010001",
            "X 1800 7 0659017F010000",
            "X 1800 4 0359027F",
        ],
        "{first:?}"
    );
    assert!(storage.is_file());

    let second = run(&storage, b"R 1792 4 03190208\nR 1792 4 03190108\nR 1792 4 03190110\nR 1110 2 0200\nR 1792 4 03190208\nR 1792 4 03190108\nR 1792 5 0414FFFFFF\nR 1792 3 021003\nR 1792 5 0414000001\nR 1792 5 0414FFFFFF\nR 1792 4 03190208\nR 1792 4 03190108\n");
    let second: Vec<_> = second
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        second,
        [
            "X 1800 8 0759027F1234566D",
            "X 1800 7 0659017F010001",
            "X 1800 7 0659017F010000",
            "X 1800 8 0759027F1234562C",
            "X 1800 7 0659017F010001",
            "X 1800 4 037F147F",
            "X 1800 7 06500300320032",
            "X 1800 4 037F1431",
            "X 1800 2 0154",
            "X 1800 4 0359027F",
            "X 1800 7 0659017F010000",
        ],
        "{second:?}"
    );

    let third = run(&storage, b"R 1792 4 03190208\nR 1792 4 03190108\n");
    assert_eq!(
        third
            .lines()
            .map(|line| line.trim_end_matches('\r'))
            .collect::<Vec<_>>(),
        ["X 1800 4 0359027F", "X 1800 7 0659017F010000"]
    );
    let control_storage = temp.0.join("dtc-control.nvm");
    let controlled = run(&control_storage, b"R 1792 3 028502\nR 1792 3 021003\nR 1792 2 0185\nR 1792 6 058502FFFFFF\nR 1792 3 028503\nR 1792 3 028582\nR 1792 3 028502\nR 1110 2 0100\nT 51\nR 1792 4 03190208\nR 1792 4 03190108\nR 1792 3 028501\nR 1110 2 0100\nT 102\nR 1792 4 03190208\nR 1792 3 028502\nR 1110 2 0100\nT 153\nR 1792 4 03190208\nR 1792 5 0414FFFFFF\nR 1792 4 03190208\nR 1792 3 028502\nR 1792 3 021001\nR 1792 3 028502\nR 1110 2 0100\nT 204\nR 1792 4 03190208\n");
    let controlled: Vec<_> = controlled
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        controlled,
        [
            "X 1800 4 037F857F",
            "X 1800 7 06500300320032",
            "X 1800 4 037F8513",
            "X 1800 4 037F8513",
            "X 1800 4 037F8512",
            "X 1800 4 037F8512",
            "X 1800 3 02C502",
            "X 1800 4 0359027F",
            "X 1800 7 0659017F010000",
            "X 1800 3 02C501",
            "X 1800 8 0759027F1234562F",
            "X 1800 3 02C502",
            "X 1800 8 0759027F1234562F",
            "X 1800 2 0154",
            "X 1800 4 0359027F",
            "X 1800 3 02C502",
            "X 1800 7 06500100320032",
            "X 1800 4 037F857F",
            "X 1800 8 0759027F1234562F",
        ],
        "{controlled:?}"
    );
    let s3_storage = temp.0.join("dtc-s3.nvm");
    let s3 = run(&s3_storage, b"R 1792 3 021003\nR 1792 3 028502\nR 1110 2 0100\nT 51\nT 5100\nR 1792 3 028502\nR 1110 2 0100\nT 5151\nR 1792 4 03190208\n");
    let s3: Vec<_> = s3
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        s3,
        [
            "X 1800 7 06500300320032",
            "X 1800 3 02C502",
            "X 1800 4 037F857F",
            "X 1800 8 0759027F1234562F",
        ],
        "{s3:?}"
    );
    let restart_storage = temp.0.join("dtc-restart.nvm");
    let disabled = run(
        &restart_storage,
        b"R 1792 3 021003\nR 1792 3 028502\nR 1110 2 0100\nT 51\nR 1792 4 03190208\n",
    );
    assert!(
        disabled
            .lines()
            .any(|line| line.trim_end_matches('\r') == "X 1800 4 0359027F")
    );
    let enabled_after_restart = run(
        &restart_storage,
        b"R 1110 2 0100\nT 51\nR 1792 4 03190208\n",
    );
    assert!(
        enabled_after_restart
            .lines()
            .any(|line| line.trim_end_matches('\r') == "X 1800 8 0759027F1234562F")
    );
    let mut slots = fs::read(&storage).unwrap();
    assert_eq!(slots.len(), 64);
    let first_sequence = u64::from_le_bytes(slots[4..12].try_into().unwrap());
    let second_sequence = u64::from_le_bytes(slots[36..44].try_into().unwrap());
    let newest = if first_sequence > second_sequence {
        0
    } else {
        1
    };
    slots[newest * 32 + 16] ^= 1;
    fs::write(&storage, slots).unwrap();
    let torn = Command::new(&binary)
        .arg("--nvm")
        .arg(&storage)
        .output()
        .unwrap();
    assert!(!torn.status.success());
    assert!(String::from_utf8_lossy(&torn.stdout).contains("E NVM"));
    fs::write(&storage, [0u8; 64]).unwrap();
    let failed = Command::new(&binary)
        .arg("--nvm")
        .arg(&storage)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stdout).contains("E NVM"));
    reopened.clear_dtc().unwrap();
    reopened.save().unwrap();
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<!-- retained by owner -->")
    );
    let saved_without_dtc = fs::read_to_string(&source).unwrap();
    let stripped = roxmltree::Document::parse(&saved_without_dtc).unwrap();
    assert!(!stripped.descendants().any(|node| {
        node.has_tag_name("ECUC-CONTAINER-VALUE")
            && node.children().any(|child| {
                child.has_tag_name("SHORT-NAME") && child.text() == Some("ControlDTCSetting")
            })
    }));
    assert!(
        !stripped
            .descendants()
            .any(|node| node.has_tag_name("ECUC-REFERENCE-VALUE")
                && node
                    .children()
                    .any(|child| child.has_tag_name("DEFINITION-REF")
                        && child
                            .text()
                            .is_some_and(|value| value.ends_with("/DcmDemClientRef"))))
    );
    let mut without_dtc = Workspace::open_legacy(vec![source], archive()).unwrap();
    let diagnostic = without_dtc.view().diagnostic.unwrap();
    assert!(diagnostic.dtc.is_none());
    assert_eq!(diagnostic.did, 0x1234);
    let simple = temp.0.join("GeneratedWithoutDtc");
    generator::generate(&mut without_dtc, &simple, tooling::native_target()).unwrap();
    let binary = tooling::build_host(&simple).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin
        .take()
        .unwrap()
        .write_all(b"R 1792 4 03190108\nR 1792 3 028502\n")
        .unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| line.trim_end_matches('\r'))
            .collect::<Vec<_>>(),
        ["X 1800 4 037F1911", "X 1800 4 037F8511"]
    );
    assert!(tooling::run_diagnostic(&simple).unwrap().passed);
}

#[cfg(windows)]
pub(super) fn extended_session_write_did_changes_live_can_but_not_restart_state() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "LiveA".into(), 0, 32, 1)
        .unwrap();
    let view = project
        .add_signal(frame.clone(), "LiveB".into(), 32, 32, 2)
        .unwrap();
    let signals = view
        .signals
        .iter()
        .filter(|signal| signal.frame_path == frame)
        .map(|signal| signal.path.clone())
        .collect();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: signals,
            write_enabled: true,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&original).unwrap();
    assert!(!doc.descendants().any(|node| {
        node.has_tag_name("SDG")
            && node
                .attribute("GID")
                .is_some_and(|gid| gid.starts_with("AutosarWorkbenchHostRestoreDid"))
    }));
    fs::write(
        &source,
        original.replace("</ELEMENTS>", "<!-- owner note --></ELEMENTS>"),
    )
    .unwrap();
    let mut reopened = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    assert!(reopened.view().diagnostic.unwrap().write_enabled);
    let generated = temp.0.join("GeneratedWrite");
    generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
    let binary = tooling::build_host(&generated).unwrap().binary_path;
    let run = |commands: &[u8]| {
        let mut ecu = Command::new(&binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        ecu.stdin.take().unwrap().write_all(commands).unwrap();
        let result = ecu.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
        String::from_utf8(result.stdout).unwrap()
    };
    let first = run(b"R 1792 4 032E1234\nR 1792 3 021003\nR 1792 5 043101F001\nR 1792 4 032E4321\nR 1792 4 032E1234\nR 1792 4 03221234\nR 1792 3 300000\nR 1792 8 100B2E1234112233\nR 1792 6 214455667788\nR 1792 4 03221234\nR 1792 3 300000\nT 100\nT 5101\nR 1792 4 032E1234\n");
    let responses: Vec<_> = first
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        responses,
        [
            "X 1800 4 037F2E31",
            "X 1800 7 06500300320032",
            "X 1800 4 037F3111",
            "X 1800 4 037F2E31",
            "X 1800 4 037F2E13",
            "X 1800 8 100B621234000000",
            "X 1800 6 210100000002",
            "X 1800 3 300000",
            "X 1800 4 036E1234",
            "X 1800 8 100B621234112233",
            "X 1800 6 214455667788",
            "X 1800 4 037F2E31",
        ],
        "{first}"
    );
    assert!(
        first
            .lines()
            .any(|line| line.trim_end_matches('\r') == "X 801 8 4433221188776655"),
        "{first}"
    );
    let after_restart = run(b"R 1792 3 021003\nR 1792 4 03221234\nR 1792 3 300000\n");
    assert!(
        after_restart.contains("X 1800 8 100B621234000000"),
        "{after_restart}"
    );
    assert!(
        after_restart.contains("X 1800 6 210100000002"),
        "{after_restart}"
    );
    let report = tooling::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{}", report.log);
    assert!(report.events.iter().any(|event| event.contains("写入")));
    reopened.clear_diagnostic().unwrap();
    reopened.save().unwrap();
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<!-- owner note -->")
    );
}

#[cfg(windows)]
pub(super) fn security_access_roundtrips_and_gates_host_writes() {
    let temp = Scratch::new();
    let mut project =
        Workspace::create_legacy(&temp.0.join("Secure"), "Secure", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(frame, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: true,
            reset_routine_id: Some(0xf001),
            security_enabled: true,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Secure/Secure.arxml");
    let xml = fs::read_to_string(&source).unwrap();
    assert!(!xml.contains("5A5A5A5A"));
    let mut reopened = Workspace::open_legacy(vec![source], archive()).unwrap();
    assert!(reopened.view().diagnostic.unwrap().security_enabled);
    let generated = temp.0.join("GeneratedSecure");
    generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
    let configuration = fs::read_to_string(generated.join("Ecu_Config.c")).unwrap();
    assert!(!configuration.contains("5A5A5A5A"));
    let binary = tooling::build_host(&generated).unwrap().binary_path;
    let without_key = Command::new(&binary).output().unwrap();
    assert!(!without_key.status.success());
    assert!(String::from_utf8_lossy(&without_key.stdout).contains("E CONFIG"));
    let report = tooling::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{}", report.log);
}

#[cfg(windows)]
pub(super) fn security_access_gates_dtc_mutations_without_a_writable_did() {
    let temp = Scratch::new();
    let mut project =
        Workspace::create_legacy(&temp.0.join("SecureDtc"), "SecureDtc", archive()).unwrap();
    let tx = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = project
        .add_signal(tx, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    let rx = project
        .add_frame("Heartbeat".into(), 0x456, 2, Direction::Rx, None, Some(50))
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Heartbeat")
        .unwrap()
        .path;
    project
        .add_signal(rx.clone(), "HeartbeatValue".into(), 0, 8, 0)
        .unwrap();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.configure_dtc(0x123456, rx).unwrap();
    let current = project.view().diagnostic.unwrap();
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: current.signal_paths,
            write_enabled: false,
            reset_routine_id: None,
            security_enabled: true,
        })
        .unwrap();
    project.save().unwrap();
    let mut reopened =
        Workspace::open_legacy(vec![temp.0.join("SecureDtc/SecureDtc.arxml")], archive()).unwrap();
    assert!(reopened.view().diagnostic.unwrap().security_enabled);
    assert!(
        reopened.clear_dtc().is_err(),
        "sole protected operation cannot be removed silently"
    );
    let generated = temp.0.join("GeneratedSecureDtc");
    generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
    tooling::build_host(&generated).unwrap();
    let report = tooling::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{}", report.log);
}

#[cfg(windows)]
pub(super) fn start_routine_restores_written_did_signals_and_respects_session() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    project
        .add_signal(frame.clone(), "LiveA".into(), 0, 32, 1)
        .unwrap();
    let view = project
        .add_signal(frame.clone(), "LiveB".into(), 32, 32, 2)
        .unwrap();
    let signals: Vec<_> = view
        .signals
        .iter()
        .filter(|signal| signal.frame_path == frame)
        .map(|signal| signal.path.clone())
        .collect();
    assert!(
        project
            .configure_diagnostic(DiagnosticSettings {
                request_id: 0x700,
                response_id: 0x708,
                s3_ms: 5000,
                n_as_ms: Some(200),
                n_bs_ms: 200,
                n_cr_ms: 200,
                did: 0x1234,
                signal_paths: signals.clone(),
                write_enabled: false,
                reset_routine_id: Some(0xF001),
                security_enabled: false
            })
            .is_err()
    );
    assert!(project.view().diagnostic.is_none());
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: signals,
            write_enabled: true,
            reset_routine_id: Some(0xF001),
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    let doc = roxmltree::Document::parse(&original).unwrap();
    let did = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node
                    .children()
                    .any(|child| child.has_tag_name("SHORT-NAME") && child.text() == Some("Did"))
        })
        .unwrap();
    let routine: Vec<_> = did
        .descendants()
        .filter(|node| {
            node.has_tag_name("SDG")
                && node.attribute("GID") == Some("AutosarWorkbenchHostRestoreDidV1")
        })
        .collect();
    assert_eq!(routine.len(), 1, "host routine metadata must be unique");
    let field = |name: &str| {
        routine[0]
            .children()
            .find(|node| node.has_tag_name("SD") && node.attribute("GID") == Some(name))
            .and_then(|node| node.text())
    };
    assert_eq!(field("Rid"), Some("61441"));
    assert_eq!(
        field("SessionRef"),
        Some("/Diag/DcmCfg/DcmConfigSet/DcmDsp/Sessions/Extended")
    );
    assert!(!doc.descendants().any(|node| {
        node.has_tag_name("DEFINITION-REF")
            && node.text().is_some_and(|path| {
                path.ends_with("/DcmDspRoutine")
                    || path.ends_with("/DcmDspCommonAuthorization")
                    || path.ends_with("/DcmDspStartRoutine")
            })
    }));
    assert!(
        !doc.descendants()
            .any(|node| node.has_tag_name("ECUC-CONTAINER-VALUE")
                && node.children().any(|child| child.has_tag_name("SHORT-NAME")
                    && child.text() == Some("RoutineControl")))
    );
    fs::write(
        &source,
        original.replace("</ELEMENTS>", "<!-- retained annotation --></ELEMENTS>"),
    )
    .unwrap();
    let mut reopened = Workspace::open_legacy(vec![source.clone()], archive()).unwrap();
    assert_eq!(
        reopened.view().diagnostic.unwrap().reset_routine_id,
        Some(0xF001)
    );
    let generated = temp.0.join("GeneratedRoutine");
    generator::generate(&mut reopened, &generated, tooling::native_target()).unwrap();
    let binary = tooling::build_host(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"R 1792 5 043101F001\nR 1792 3 021003\nR 1792 5 043101F002\nR 1792 5 043102F001\nR 1792 6 053101F00100\nR 1792 8 100B2E1234112233\nR 1792 6 214455667788\nR 1792 4 03221234\nR 1792 3 300000\nR 1792 5 043101F001\nR 1792 4 03221234\nR 1792 3 300000\nT 100\nT 5101\nR 1792 5 043101F001\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let responses: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with("X 1800 "))
        .collect();
    assert_eq!(
        responses,
        [
            "X 1800 4 037F3131",
            "X 1800 7 06500300320032",
            "X 1800 4 037F3131",
            "X 1800 4 037F3112",
            "X 1800 4 037F3113",
            "X 1800 3 300000",
            "X 1800 4 036E1234",
            "X 1800 8 100B621234112233",
            "X 1800 6 214455667788",
            "X 1800 5 047101F001",
            "X 1800 8 100B621234000000",
            "X 1800 6 210100000002",
            "X 1800 4 037F3131",
        ],
        "{log}"
    );
    assert!(
        log.lines()
            .any(|line| line.trim_end_matches('\r') == "X 801 8 0100000002000000"),
        "{log}"
    );
    let report = tooling::run_diagnostic(&generated).unwrap();
    assert!(report.passed, "{}", report.log);
    assert!(report.events.iter().any(|event| event.contains("例程")));
    reopened.clear_diagnostic().unwrap();
    reopened.save().unwrap();
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("<!-- retained annotation -->")
    );
}

#[cfg(windows)]
pub(super) fn generated_dcm_callbacks_link_for_independent_consumer_and_update_live_signals() {
    let temp = Scratch::new();
    let mut project = Workspace::create_legacy(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project
        .add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(100), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let first = project
        .add_signal(frame.clone(), "First".into(), 0, 32, 0x01020304)
        .unwrap();
    let second = project
        .add_signal(frame.clone(), "Second".into(), 32, 32, 0xaabbccdd)
        .unwrap();
    let paths = [
        first.signals.last().unwrap().path.clone(),
        second.signals.last().unwrap().path.clone(),
    ];
    project
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(200),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: paths.to_vec(),
            write_enabled: true,
            reset_routine_id: None,
            security_enabled: false,
        })
        .unwrap();
    project.save().unwrap();
    let source_path = temp.0.join("Diag/Diag.arxml");
    let mut reopened = Workspace::open_legacy(vec![source_path.clone()], archive()).unwrap();
    reopened
        .update_frame(&frame, serde_json::json!({"periodMs": 110}))
        .unwrap();
    reopened.save().unwrap();
    let source = fs::read_to_string(&source_path).unwrap();
    let doc = roxmltree::Document::parse(&source).unwrap();
    let callbacks = |parameter: &str| -> Vec<String> {
        doc.descendants()
            .filter(|node| node.has_tag_name("ECUC-TEXTUAL-PARAM-VALUE"))
            .filter(|node| {
                node.children().any(|child| {
                    child.has_tag_name("DEFINITION-REF")
                        && child.text().is_some_and(|text| text.ends_with(parameter))
                })
            })
            .filter_map(|node| {
                node.children()
                    .find(|child| child.has_tag_name("VALUE"))
                    .and_then(|value| value.text())
                    .map(str::to_owned)
            })
            .collect()
    };
    let reads = callbacks("/DcmDspDataReadFnc");
    let writes = callbacks("/DcmDspDataWriteFnc");
    assert_eq!((reads.len(), writes.len()), (2, 2));
    let output = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &output, tooling::native_target()).unwrap();
    tooling::build_host(&output).unwrap();
    let report = tooling::run_diagnostic(&output).unwrap();
    assert!(
        report.passed,
        "generated ECU diagnostic smoke: {}",
        report.log
    );

    let consumer = temp.0.join("consumer.c");
    fs::write(&consumer, format!(r#"
#include <stdint.h>
#include <string.h>
#include "Dcm_Externals.h"
#include "Ecu_Runtime.h"
#include "Rte.h"

static EcuStatus sink(uint32_t id, uint8_t dlc, const uint8_t data[8])
{{
    (void)id;
    (void)dlc;
    (void)data;
    return ECU_OK;
}}

int main(void)
{{
    uint8_t data[4];
    uint8_t valid;
    uint32_t value;
    Dcm_NegativeResponseCodeType error_code = 0u;
    const uint8_t first_initial[4] = {{ 0x01u, 0x02u, 0x03u, 0x04u }};
    const uint8_t second_initial[4] = {{ 0xaau, 0xbbu, 0xccu, 0xddu }};
    const uint8_t written[4] = {{ 0x11u, 0x22u, 0x33u, 0x44u }};
    if (Ecu_Init(&Ecu_Config, sink, NULL, NULL, NULL) != ECU_OK) return 1;
    if ({read_first}(data) != E_OK || memcmp(data, first_initial, 4u) != 0) return 2;
    if ({read_second}(data) != E_OK || memcmp(data, second_initial, 4u) != 0) return 3;
    if ({write_first}(written, &error_code) != E_OK) return 4;
    if ({read_first}(data) != E_OK || memcmp(data, written, 4u) != 0) return 5;
    if (Rte_ReadSignal(0u, &value, &valid) != ECU_OK || value != 0x11223344u || valid != 1u) return 6;
    if (Rte_ReadSignal(1u, &value, &valid) != ECU_OK || value != 0xaabbccddu || valid != 1u) return 7;
    if ({write_second}(first_initial, &error_code) != E_OK) return 8;
    if (Rte_ReadSignal(1u, &value, &valid) != ECU_OK || value != 0x01020304u || valid != 1u) return 9;
    if ({write_first}(NULL, &error_code) != E_NOT_OK || error_code != 0x72u) return 10;
    return 0;
}}
"#, read_first = reads[0], read_second = reads[1], write_first = writes[0], write_second = writes[1])).unwrap();
    let cc = std::env::var_os("AUTOSAR_CC")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gcc"));
    let mut sources: Vec<_> = fs::read_dir(output.join("src"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "c")
                && path.file_name().unwrap() != "ecu_host_main.c"
        })
        .collect();
    sources.sort();
    sources.push(output.join("Ecu_Config.c"));
    sources.push(consumer);
    let executable = temp.0.join("consumer.exe");
    let built = Command::new(cc)
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(output.join("include"))
        .arg("-I")
        .arg(&output)
        .args(sources)
        .arg("-o")
        .arg(&executable)
        .arg("-lbcrypt")
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "independent consumer link: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let observed = Command::new(executable).output().unwrap();
    assert!(
        observed.status.success(),
        "independent consumer failed at check {:?}: {}",
        observed.status.code(),
        String::from_utf8_lossy(&observed.stderr)
    );
}
