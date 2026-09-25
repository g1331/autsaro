use std::fs;
use std::io::Write;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use autosar_config_core::{generator, host, schema, Direction, Workspace};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("autosar-config-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}
fn archive() -> PathBuf {
    schema::schema_archive(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}
fn create_pair(root: &Path) -> (Workspace, Workspace) {
    let zip = archive();
    let mut a = Workspace::create(&root.join("Alpha"), "Alpha", zip.clone()).unwrap();
    let tx = a.add_frame("Command".into(), 0x321, 2, Direction::Tx, Some(10), None).unwrap().frames[0].path.clone();
    a.add_signal(tx, "SendCount".into(), 3, 8, 5).unwrap();
    let rx = a.add_frame("Reply".into(), 0x456, 2, Direction::Rx, None, Some(50)).unwrap().frames.into_iter().find(|f| f.name == "Reply").unwrap().path;
    a.add_signal(rx, "RecvStatus".into(), 0, 8, 0).unwrap();
    assert!(a.validate().unwrap().issues.is_empty());
    a.save().unwrap();
    let mut b = Workspace::create(&root.join("Beta"), "Beta", zip).unwrap();
    let rx = b.add_frame("Command".into(), 0x321, 2, Direction::Rx, None, Some(40)).unwrap().frames[0].path.clone();
    b.add_signal(rx, "RecvCount".into(), 3, 8, 0).unwrap();
    let tx = b.add_frame("Reply".into(), 0x456, 2, Direction::Tx, Some(20), None).unwrap().frames.into_iter().find(|f| f.name == "Reply").unwrap().path;
    b.add_signal(tx, "SendStatus".into(), 0, 8, 7).unwrap();
    assert!(b.validate().unwrap().issues.is_empty());
    b.save().unwrap();
    (a, b)
}

#[cfg(windows)]
#[test]
fn generated_c99_ecus_exchange_golden_vectors_and_recover_from_faults() {
    let temp = Scratch::new();
    let (mut a, mut b) = create_pair(&temp.0);
    let out_a = temp.0.join("GeneratedAlpha");
    let out_b = temp.0.join("GeneratedBeta");
    fs::create_dir(&out_a).unwrap();
    fs::create_dir(&out_b).unwrap();
    generator::generate(&mut a, &out_a).unwrap();
    generator::generate(&mut b, &out_b).unwrap();
    let original = fs::read(out_a.join("Ecu_Config.c")).unwrap();
    generator::generate(&mut a, &out_a).unwrap();
    assert_eq!(original, fs::read(out_a.join("Ecu_Config.c")).unwrap(), "identical ARXML must generate stable C");
    let exe_a = generator::build(&out_a).unwrap().binary_path;
    let exe_b = generator::build(&out_b).unwrap().binary_path;
    let mut tx = Command::new(exe_a).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    tx.stdin.take().unwrap().write_all(b"S 0 54\nT 10\n").unwrap();
    let actual = String::from_utf8(tx.wait_with_output().unwrap().stdout).unwrap();
    assert!(actual.lines().any(|line| line.trim_end_matches('\r') == "X 801 2 B001"), "golden LSB0 bit packing: {actual}");
    let mut rx = Command::new(exe_b).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    rx.stdin.take().unwrap().write_all(b"T 10\nR 801 2 B001\nG 0\n").unwrap();
    let received = String::from_utf8(rx.wait_with_output().unwrap().stdout).unwrap();
    assert!(received.lines().any(|line| line.trim_end_matches('\r') == "V 0 54 1"), "independent received value: {received}");
    let result = host::run(&out_a, &out_b).unwrap();
    assert!(result.passed, "host bus failed: {}", result.log);
    assert!(result.events.iter().any(|e| e.contains("BUS_OFF")));
    assert!(result.events.iter().any(|e| e.contains("DLC")));
}

#[cfg(windows)]
#[test]
fn host_rejects_id_matched_wrong_dlc_even_when_other_frames_exchange() {
    let temp = Scratch::new();
    let (mut a, mut b) = create_pair(&temp.0);
    let tx = a.add_frame("Mismatch".into(), 0x600, 2, Direction::Tx, Some(10), None).unwrap()
        .frames.into_iter().find(|frame| frame.name == "Mismatch").unwrap().path;
    a.add_signal(tx, "ExtraTx".into(), 0, 8, 1).unwrap();
    let rx = b.add_frame("Mismatch".into(), 0x600, 1, Direction::Rx, None, Some(50)).unwrap()
        .frames.into_iter().find(|frame| frame.name == "Mismatch").unwrap().path;
    b.add_signal(rx, "ExtraRx".into(), 0, 8, 0).unwrap();
    a.save().unwrap();
    b.save().unwrap();
    let out_a = temp.0.join("GeneratedAlpha");
    let out_b = temp.0.join("GeneratedBeta");
    generator::generate(&mut a, &out_a).unwrap();
    generator::generate(&mut b, &out_b).unwrap();
    generator::build(&out_a).unwrap();
    generator::build(&out_b).unwrap();
    let result = host::run(&out_a, &out_b).unwrap();
    assert!(!result.passed && result.log.contains("FRAME_DLC"), "ID-matched DLC mismatch was ignored: {}", result.log);
}

#[test]
fn imported_unknown_content_survives_supported_edit_without_rewriting_other_file() {
    let temp = Scratch::new();
    let (a, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let retained = "<I-SIGNAL><SHORT-NAME>RetainedUnknown</SHORT-NAME><LENGTH>1</LENGTH></I-SIGNAL>";
    fs::write(&source, text.replacen("</ELEMENTS>", &format!("{retained}</ELEMENTS>"), 1)).unwrap();
    let other = temp.0.join("Unrelated.arxml");
    let unrelated = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Other</SHORT-NAME><ELEMENTS><I-SIGNAL><SHORT-NAME>Extra</SHORT-NAME><LENGTH>1</LENGTH></I-SIGNAL></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>";
    fs::write(&other, unrelated).unwrap();
    let mut imported = Workspace::open(vec![source.clone(), other.clone()], archive()).unwrap();
    assert_eq!(imported.view().files.len(), 2);
    let frame = imported.view().frames.into_iter().find(|f| f.name == "Command").unwrap();
    imported.update_frame(&frame.path, serde_json::json!({"id": 802})).unwrap();
    imported.save().unwrap();
    assert!(fs::read_to_string(&source).unwrap().contains(retained));
    assert_eq!(unrelated, fs::read_to_string(other).unwrap(), "unmodified ARXML must stay byte-identical");
    assert_eq!(a.view().frames.len(), 2);
}

#[test]
fn save_does_not_overwrite_external_changes_to_managed_arxml() {
    let temp = Scratch::new();
    let (mut project, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let frame = project.view().frames.into_iter().find(|frame| frame.name == "Command").unwrap();
    project.update_frame(&frame.path, serde_json::json!({"id": 802})).unwrap();
    let original = fs::read_to_string(&source).unwrap();
    let external = original.replacen("<VALUE>801</VALUE>", "<VALUE>803</VALUE>", 1);
    assert_ne!(original, external);
    fs::write(&source, &external).unwrap();
    assert!(project.save().unwrap_err().contains("外部修改"));
    assert_eq!(fs::read_to_string(&source).unwrap(), external);
    assert!(project.view().dirty);
    assert_eq!(fs::read_dir(source.parent().unwrap()).unwrap().count(), 1, "failed save left staging files");
}

#[test]
fn official_r24_sample_imports_as_one_split_package_without_rewriting_sources() {
    let temp = Scratch::new();
    let zip_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_EXP_ModelingShowCases.zip");
    let mut zip = zip::ZipArchive::new(fs::File::open(zip_path).unwrap()).unwrap();
    let prefix = "AUTOSAR_CP_EXP_ModelingShowCases/30_MeasurementCalibration/10_Introductory/model/";
    let mut paths = Vec::new();
    let mut originals = Vec::new();
    for index in 0..zip.len() {
        let mut member = zip.by_index(index).unwrap();
        if !member.name().starts_with(prefix) || !member.name().ends_with(".arxml") { continue; }
        let name = Path::new(member.name()).file_name().unwrap();
        let path = temp.0.join(name);
        let mut content = Vec::new();
        member.read_to_end(&mut content).unwrap();
        fs::write(&path, &content).unwrap();
        originals.push((path.clone(), content));
        paths.push(path);
    }
    assert_eq!(paths.len(), 15, "the official R24 example must have all model files");
    let mut project = Workspace::open(paths, archive()).unwrap();
    assert!(project.validate().unwrap().issues.is_empty(), "split AR-PACKAGE paths may merge across files");
    assert!(!project.view().dirty);
    project.save().unwrap();
    for (path, content) in originals { assert_eq!(content, fs::read(path).unwrap()); }
    assert!(generator::generate(&mut project, &temp.0.join("Generated")).is_err(), "unconfigured CAN profile must not produce a misleading ECU");
}

#[test]
fn unresolved_r24_variant_is_preserved_but_blocks_generation() {
    let temp = Scratch::new();
    let (a, _) = create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let anchor = "</START-POSITION></I-SIGNAL-TO-I-PDU-MAPPING>";
    let variant = "<VARIATION-POINT><SHORT-LABEL>UnboundVariant</SHORT-LABEL></VARIATION-POINT>";
    assert!(text.contains(anchor));
    fs::write(&source, text.replacen(anchor, &format!("</START-POSITION>{variant}</I-SIGNAL-TO-I-PDU-MAPPING>"), 1)).unwrap();
    let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(checked.issues.iter().any(|issue| issue.code == "VARIANT_DEPENDENCY"));
    imported.save().unwrap();
    assert!(fs::read_to_string(source).unwrap().contains(variant));
    let generated = temp.0.join("UnsafeOutput");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(!generated.exists(), "variant-dependent C99 output must not be materialized");
    assert_eq!(a.view().frames.len(), 2);
}

#[test]
fn package_variant_affecting_profile_blocks_generation() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let variant = "<VARIATION-POINT><SHORT-LABEL>UnboundPackage</SHORT-LABEL></VARIATION-POINT>";
    fs::write(&source, text.replacen("</AR-PACKAGE>", &format!("{variant}</AR-PACKAGE>"), 1)).unwrap();
    let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(checked.issues.iter().any(|issue| issue.code == "VARIANT_DEPENDENCY"), "{:?}", checked.issues);
    let generated = temp.0.join("UnsafePackageOutput");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(!generated.exists());
}

#[test]
fn conflicting_canif_entries_for_one_pdu_block_generation() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let start = text.find("<ECUC-CONTAINER-VALUE><SHORT-NAME>Can_Command</SHORT-NAME>").unwrap();
    let end = start + text[start..].find("</ECUC-CONTAINER-VALUE>").unwrap() + "</ECUC-CONTAINER-VALUE>".len();
    let duplicate = text[start..end].replace("<SHORT-NAME>Can_Command</SHORT-NAME>", "<SHORT-NAME>Can_Conflict</SHORT-NAME>")
        .replace("<VALUE>801</VALUE>", "<VALUE>1536</VALUE>");
    assert!(duplicate.contains("<VALUE>1536</VALUE>"));
    fs::write(&source, format!("{}{}{}", &text[..end], duplicate, &text[end..])).unwrap();
    let mut imported = Workspace::open(vec![source], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(checked.issues.iter().any(|issue| issue.code == "CANIF_PDU_DUPLICATE"), "{:?}", checked.issues);
    let generated = temp.0.join("UnsafeCanIfOutput");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(!generated.exists());
}

#[test]
fn ipdu_mapping_disagreement_with_com_blocks_generation() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    for (field, original, changed) in [
        ("byte order", "<PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-LAST</PACKING-BYTE-ORDER>", "<PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-FIRST</PACKING-BYTE-ORDER>"),
        ("start position", "<START-POSITION>3</START-POSITION>", "<START-POSITION>4</START-POSITION>"),
    ] {
        assert!(text.contains(original), "missing {field} fixture");
        fs::write(&source, text.replacen(original, changed, 1)).unwrap();
        let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
        let checked = imported.validate().unwrap();
        assert!(checked.issues.iter().any(|issue| issue.code == "PDU_MAPPING"), "{field}: {:?}", checked.issues);
        let generated = temp.0.join(format!("Unsafe{field}Output"));
        assert!(generator::generate(&mut imported, &generated).is_err(), "{field} generated incompatible C99");
        assert!(!generated.exists());
    }
}

#[test]
fn linked_can_frame_must_match_canif_and_pdu_layout() {
    let temp = Scratch::new();
    create_pair(&temp.0);
    let source = temp.0.join("Alpha/Alpha.arxml");
    let text = fs::read_to_string(&source).unwrap();
    let topology = r#"
<CAN-FRAME><SHORT-NAME>CommandFrame</SHORT-NAME><FRAME-LENGTH>2</FRAME-LENGTH>
  <PDU-TO-FRAME-MAPPINGS><PDU-TO-FRAME-MAPPING><SHORT-NAME>CommandMapping</SHORT-NAME>
    <PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-LAST</PACKING-BYTE-ORDER>
    <PDU-REF DEST="I-SIGNAL-I-PDU">/Alpha/Pdu_Command</PDU-REF><START-POSITION>0</START-POSITION>
  </PDU-TO-FRAME-MAPPING></PDU-TO-FRAME-MAPPINGS></CAN-FRAME>
<CAN-CLUSTER><SHORT-NAME>Network</SHORT-NAME><CAN-CLUSTER-VARIANTS><CAN-CLUSTER-CONDITIONAL>
  <PHYSICAL-CHANNELS><CAN-PHYSICAL-CHANNEL><SHORT-NAME>Bus</SHORT-NAME>
    <FRAME-TRIGGERINGS><CAN-FRAME-TRIGGERING><SHORT-NAME>CommandOnBus</SHORT-NAME>
      <FRAME-REF DEST="CAN-FRAME">/Alpha/CommandFrame</FRAME-REF><IDENTIFIER>801</IDENTIFIER>
    </CAN-FRAME-TRIGGERING></FRAME-TRIGGERINGS>
  </CAN-PHYSICAL-CHANNEL></PHYSICAL-CHANNELS>
</CAN-CLUSTER-CONDITIONAL></CAN-CLUSTER-VARIANTS></CAN-CLUSTER>"#;
    let compatible = text.replacen("</ELEMENTS>", &format!("{topology}</ELEMENTS>"), 1);
    fs::write(&source, &compatible).unwrap();
    let mut imported = Workspace::open(vec![source.clone()], archive()).unwrap();
    assert!(imported.validate().unwrap().issues.is_empty());
    generator::generate(&mut imported, &temp.0.join("CompatibleNetworkOutput")).unwrap();
    for (original, changed) in [
        ("<START-POSITION>0</START-POSITION>", "<START-POSITION>8</START-POSITION>"),
        ("<FRAME-LENGTH>2</FRAME-LENGTH>", "<FRAME-LENGTH>3</FRAME-LENGTH>"),
    ] {
        let altered_topology = topology.replacen(original, changed, 1);
        fs::write(&source, text.replacen("</ELEMENTS>", &format!("{altered_topology}</ELEMENTS>"), 1)).unwrap();
        let mut mismatched = Workspace::open(vec![source.clone()], archive()).unwrap();
        let checked = mismatched.validate().unwrap();
        assert!(checked.issues.iter().any(|issue| issue.code == "CAN_FRAME_MAPPING"), "{:?}", checked.issues);
        assert!(generator::generate(&mut mismatched, &temp.0.join("UnsafeFrameOutput")).is_err());
    }
    fs::write(&source, compatible.replacen("<IDENTIFIER>801</IDENTIFIER>", "<IDENTIFIER>802</IDENTIFIER>", 1)).unwrap();
    imported = Workspace::open(vec![source], archive()).unwrap();
    let checked = imported.validate().unwrap();
    assert!(checked.issues.iter().any(|issue| issue.code == "CAN_ID_MISMATCH"), "{:?}", checked.issues);
    let generated = temp.0.join("UnsafeNetworkOutput");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(!generated.exists());
}

#[cfg(windows)]
#[test]
fn configured_diagnostic_ecu_roundtrips_arxml_and_exchanges_live_multiframe_did() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project.add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None).unwrap().frames[0].path.clone();
    project.add_signal(frame.clone(), "LiveA".into(), 0, 32, 0).unwrap();
    let view = project.add_signal(frame.clone(), "LiveB".into(), 32, 32, 0).unwrap();
    let sources: Vec<_> = view.signals.iter().filter(|signal| signal.frame_path == frame).map(|signal| signal.path.clone()).collect();
    assert!(project.configure_diagnostic(0x321, 0x708, 5000, 200, 200, 0x1234, sources.clone()).is_err());
    assert!(project.view().diagnostic.is_none());
    project.configure_diagnostic(0x700, 0x708, 5000, 200, 200, 0x1234, sources).unwrap();
    let validation = project.validate().unwrap();
    assert!(validation.issues.is_empty(), "{:?}", validation.issues);
    project.save().unwrap();

    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    fs::write(&source, original.replace("</ELEMENTS>", "<!-- user annotation --></ELEMENTS>")).unwrap();
    let mut reopened = Workspace::open(vec![source.clone()], archive()).unwrap();
    let diagnostic = reopened.view().diagnostic.unwrap();
    assert_eq!((diagnostic.request_id, diagnostic.response_id, diagnostic.did), (0x700, 0x708, 0x1234));
    let output = temp.0.join("GeneratedDiag");
    generator::generate(&mut reopened, &output).unwrap();
    let binary = generator::build(&output).unwrap().binary_path;
    let mut ecu = Command::new(binary).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"S 0 287454020\nS 1 1432778632\nR 1792 4 03221234\nR 1792 3 021003\nR 1792 4 03221234\nR 1792 3 300000\nT 5001\nR 1792 4 03221234\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let frames: Vec<_> = log.lines().map(|line| line.trim_end_matches('\r')).filter(|line| line.starts_with("X 1800 ")).collect();
    assert_eq!(frames, [
        "X 1800 4 037F2231",
        "X 1800 7 06500300320032",
        "X 1800 8 100B621234112233",
        "X 1800 6 214455667788",
        "X 1800 4 037F2231",
    ], "{log}");
    let report = host::run_diagnostic(&temp.0.join("GeneratedDiag")).unwrap();
    assert!(report.passed, "independent host diagnostic tester failed: {}", report.log);
    assert!(report.events.iter().any(|event| event.contains("多帧")));
    reopened.clear_diagnostic().unwrap();
    reopened.save().unwrap();
    assert!(fs::read_to_string(&source).unwrap().contains("<!-- user annotation -->"));
    let mut signal_only = Workspace::open(vec![temp.0.join("Diag/Diag.arxml")], archive()).unwrap();
    assert!(signal_only.view().diagnostic.is_none());
    let signal_output = temp.0.join("GeneratedSignalsOnly");
    generator::generate(&mut signal_only, &signal_output).unwrap();
    generator::build(&signal_output).unwrap();
    assert!(host::run_diagnostic(&signal_output).is_err());
}

#[cfg(windows)]
#[test]
fn diagnostic_transport_discards_bad_or_timed_out_multiframe_requests_and_recovers() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project.add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None).unwrap().frames[0].path.clone();
    project.add_signal(frame.clone(), "LiveValue".into(), 0, 32, 42).unwrap();
    let view = project.add_signal(frame, "LiveOther".into(), 32, 32, 43).unwrap();
    let sources = view.signals.iter().map(|signal| signal.path.clone()).collect();
    project.configure_diagnostic(0x700, 0x708, 5000, 200, 200, 0x1234, sources).unwrap();
    project.save().unwrap();
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut project, &generated).unwrap();
    let binary = generator::build(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"R 1792 8 1009221234123412\nR 1792 4 22341234\nR 1792 8 1009221234123412\nR 1792 4 21341234\nR 1792 8 1009221234123412\nT 201\nR 1792 3 023E00\nR 1792 3 021003\nR 1792 4 03221234\nT 402\nR 1792 3 023E00\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = log.lines().map(|line| line.trim_end_matches('\r')).collect();
    let flow_controls = lines.iter().filter(|line| **line == "X 1800 3 300000").count();
    assert_eq!(flow_controls, 3, "{log}");
    assert!(lines.contains(&"E TP_SEQUENCE"), "{log}");
    assert!(lines.contains(&"X 1800 4 037F2213"), "{log}");
    assert_eq!(lines.iter().filter(|line| **line == "E TP_TIMEOUT").count(), 2, "{log}");
    assert!(lines.contains(&"X 1800 3 027E00"), "{log}");
    assert!(lines.iter().any(|line| line.starts_with("X 1800 8 100B621234")), "{log}");
}

#[cfg(windows)]
#[test]
fn diagnostic_tester_present_keeps_session_and_fc_block_size_paces_response() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    for (name, id) in [("LiveA", 0x321), ("LiveB", 0x322)] {
        let frame = project.add_frame(name.into(), id, 8, Direction::Tx, Some(1000), None).unwrap()
            .frames.into_iter().find(|item| item.name == name).unwrap().path;
        project.add_signal(frame.clone(), format!("{name}Low"), 0, 32, 0).unwrap();
        project.add_signal(frame, format!("{name}High"), 32, 32, 0).unwrap();
    }
    let sources = project.view().signals.iter().map(|signal| signal.path.clone()).collect();
    project.configure_diagnostic(0x700, 0x708, 5000, 200, 200, 0x1234, sources).unwrap();
    project.save().unwrap();
    let generated = temp.0.join("GeneratedDiag");
    generator::generate(&mut project, &generated).unwrap();
    let binary = generator::build(&generated).unwrap().binary_path;
    let mut ecu = Command::new(binary).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    ecu.stdin.take().unwrap().write_all(
        b"S 0 16909060\nS 1 84281096\nS 2 151653132\nS 3 219025168\nR 1792 3 021003\nT 4000\nR 1792 3 023E80\nT 8000\nR 1792 4 03221234\nR 1792 3 300105\nG 0\nT 8005\nG 0\nR 1792 3 300105\nT 13006\nR 1792 4 03221234\n"
    ).unwrap();
    let output = ecu.wait_with_output().unwrap();
    assert!(output.status.success());
    let log = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = log.lines().map(|line| line.trim_end_matches('\r')).collect();
    let diagnostic: Vec<_> = lines.iter().filter(|line| line.starts_with("X 1800 ")).copied().collect();
    assert_eq!(diagnostic, [
        "X 1800 7 06500300320032",
        "X 1800 8 1013621234010203",
        "X 1800 8 210405060708090A",
        "X 1800 7 220B0C0D0E0F10",
        "X 1800 4 037F2231",
    ], "{log}");
    let fence_positions: Vec<_> = lines.iter().enumerate().filter(|(_, line)| line.starts_with("V 0 ")).map(|(index, _)| index).collect();
    let second_cf = lines.iter().position(|line| *line == "X 1800 7 220B0C0D0E0F10").unwrap();
    assert_eq!(fence_positions.len(), 2, "{log}");
    assert!(fence_positions[1] < second_cf, "FC block size was ignored: {log}");
}

#[cfg(windows)]
#[test]
fn unsupported_imported_transport_padding_blocks_diagnostic_generation() {
    let temp = Scratch::new();
    let mut project = Workspace::create(&temp.0.join("Diag"), "Diag", archive()).unwrap();
    let frame = project.add_frame("Live".into(), 0x321, 8, Direction::Tx, Some(1000), None).unwrap().frames[0].path.clone();
    let signal = project.add_signal(frame, "LiveA".into(), 0, 32, 0).unwrap().signals[0].path.clone();
    project.configure_diagnostic(0x700, 0x708, 5000, 200, 200, 0x1234, vec![signal]).unwrap();
    project.save().unwrap();
    let source = temp.0.join("Diag/Diag.arxml");
    let original = fs::read_to_string(&source).unwrap();
    fs::write(&source, original.replacen(">CANTP_OFF<", ">CANTP_ON<", 1)).unwrap();
    let mut imported = Workspace::open(vec![source], archive()).unwrap();
    assert!(imported.view().issues.iter().any(|issue| issue.code == "DIAG_UNSUPPORTED"));
    let generated = temp.0.join("RejectedDiag");
    assert!(generator::generate(&mut imported, &generated).is_err());
    assert!(!generated.exists());
}
