use super::{Scratch, archive};
use autosar_config_core::model::Severity;
use autosar_config_core::project_model::ValidationStatus;
use autosar_config_core::{DiagnosticSettings, Direction, Workspace, rules, schema};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

fn document(elements: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <AUTOSAR xmlns=\"http://autosar.org/schema/r4.0\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd\">\
         <AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Original</SHORT-NAME>\
         <ELEMENTS>{elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"
    )
}

fn check(text: &str) -> autosar_config_core::project_model::ScopeValidation {
    rules::validate_native(&[(Path::new("native-input.arxml"), text)]).unwrap()
}

#[test]
fn native_schema_uses_complete_literals_split_by_xml_comments() {
    let input = document(
        "<I-SIGNAL><SHORT-NAME>A<!-- preserved -->B</SHORT-NAME><LENGTH>1<!-- preserved -->6</LENGTH></I-SIGNAL>",
    );
    assert_eq!(check(&input).status, ValidationStatus::Passed);
    let rejected = document(
        "<I-SIGNAL><SHORT-NAME>A<!-- preserved -->B</SHORT-NAME><LENGTH>1<!-- preserved -->x</LENGTH></I-SIGNAL>",
    );
    assert!(check(&rejected).diagnostics.iter().any(|diagnostic| {
        diagnostic.rule_id == "native.type.LENGTH"
            && matches!(diagnostic.severity, Severity::Error)
            && diagnostic.path.as_deref() == Some("/Original/AB")
    }));
}

#[test]
fn native_rules_support_original_can_and_standard_inputs_without_oracle() {
    for (name, bit_length) in [("Alpha", 8), ("Beta", 32)] {
        let input = document(&format!(
            "<I-SIGNAL><SHORT-NAME>{name}</SHORT-NAME><LENGTH>{bit_length}</LENGTH></I-SIGNAL>\
             <I-SIGNAL-I-PDU><SHORT-NAME>Pdu_{name}</SHORT-NAME><LENGTH>4</LENGTH>\
             <I-SIGNAL-TO-PDU-MAPPINGS><I-SIGNAL-TO-I-PDU-MAPPING><SHORT-NAME>Map_{name}</SHORT-NAME>\
             <I-SIGNAL-REF DEST=\"I-SIGNAL\">/Original/{name}</I-SIGNAL-REF>\
             <PACKING-BYTE-ORDER>MOST-SIGNIFICANT-BYTE-LAST</PACKING-BYTE-ORDER>\
             <START-POSITION>0</START-POSITION></I-SIGNAL-TO-I-PDU-MAPPING>\
             </I-SIGNAL-TO-PDU-MAPPINGS></I-SIGNAL-I-PDU>"
        ));
        let result = check(&input);
        assert_eq!(
            result.status,
            ValidationStatus::Passed,
            "{:?}",
            result.diagnostics
        );
    }
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/epic4/positive");
    let files: Vec<_> = [
        "application.arxml",
        "bsw.arxml",
        "ecuc.arxml",
        "extract.arxml",
        "services.arxml",
        "types.arxml",
        "unrelated.arxml",
    ]
    .into_iter()
    .map(|name| {
        let path = directory.join(name);
        let text = fs::read_to_string(&path).unwrap();
        (path, text)
    })
    .collect();
    let borrowed: Vec<_> = files
        .iter()
        .map(|(path, text)| (path.as_path(), text.as_str()))
        .collect();
    let result = rules::validate_native(&borrowed).unwrap();
    assert_eq!(
        result.status,
        ValidationStatus::Passed,
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn native_rules_cover_rendered_diagnostic_and_dtc_closure() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("Diagnostic");
    let mut workspace = Workspace::create(&directory, "Diagnostic").unwrap();
    let tx = workspace
        .add_frame("Live".into(), 0x321, 4, Direction::Tx, Some(10), None)
        .unwrap()
        .frames[0]
        .path
        .clone();
    let signal = workspace
        .add_signal(tx, "LiveValue".into(), 0, 32, 7)
        .unwrap()
        .signals[0]
        .path
        .clone();
    let rx = workspace
        .add_frame("Monitor".into(), 0x456, 1, Direction::Rx, None, Some(30))
        .unwrap()
        .frames
        .into_iter()
        .find(|frame| frame.name == "Monitor")
        .unwrap()
        .path;
    workspace
        .add_signal(rx.clone(), "Status".into(), 0, 8, 0)
        .unwrap();
    workspace
        .configure_diagnostic(DiagnosticSettings {
            request_id: 0x700,
            response_id: 0x708,
            s3_ms: 5000,
            n_as_ms: Some(100),
            n_bs_ms: 200,
            n_cr_ms: 200,
            did: 0x1234,
            signal_paths: vec![signal],
            write_enabled: true,
            reset_routine_id: Some(0x1234),
            security_enabled: false,
        })
        .unwrap();
    workspace.configure_dtc(0x123456, rx).unwrap();
    workspace.save().unwrap();
    let path = directory.join("Diagnostic.arxml");
    let text = fs::read_to_string(&path).unwrap();
    let result = rules::validate_native(&[(path.as_path(), &text)]).unwrap();
    assert_eq!(
        result.status,
        ValidationStatus::Passed,
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn native_schema_rejects_structure_order_cardinality_and_lexical_types() {
    let cases = [
        (
            "<I-SIGNAL><LENGTH>8</LENGTH></I-SIGNAL>",
            "native.structure.I-SIGNAL",
        ),
        (
            "<I-SIGNAL><LENGTH>8</LENGTH><SHORT-NAME>Signal</SHORT-NAME></I-SIGNAL>",
            "native.structure.I-SIGNAL",
        ),
        (
            "<I-SIGNAL><SHORT-NAME>Signal</SHORT-NAME><LENGTH>8</LENGTH><LENGTH>9</LENGTH></I-SIGNAL>",
            "native.structure.I-SIGNAL",
        ),
        (
            "<I-SIGNAL><SHORT-NAME>Signal</SHORT-NAME><LENGTH>eight</LENGTH></I-SIGNAL>",
            "native.type.LENGTH",
        ),
        (
            "<I-SIGNAL><SHORT-NAME>_Invalid</SHORT-NAME><LENGTH>8</LENGTH></I-SIGNAL>",
            "native.type.SHORT-NAME",
        ),
        (
            "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>Com</SHORT-NAME><POST-BUILD-VARIANT-USED>yes</POST-BUILD-VARIANT-USED></ECUC-MODULE-CONFIGURATION-VALUES>",
            "native.type.POST-BUILD-VARIANT-USED",
        ),
        (
            "<I-SIGNAL><SHORT-NAME>Signal</SHORT-NAME><LENGTH><SHORT-NAME>Nested</SHORT-NAME></LENGTH></I-SIGNAL>",
            "native.type.LENGTH",
        ),
        (
            "<I-SIGNAL><SHORT-NAME>Signal</SHORT-NAME><FRAME-LENGTH>8</FRAME-LENGTH></I-SIGNAL>",
            "native.structure.I-SIGNAL",
        ),
        (
            "<I-SIGNAL-I-PDU><SHORT-NAME>Pdu</SHORT-NAME><I-SIGNAL-TO-PDU-MAPPINGS><I-SIGNAL-TO-I-PDU-MAPPING><SHORT-NAME>Map</SHORT-NAME><I-SIGNAL-REF DEST=\"CAN-FRAME\">/Original/Signal</I-SIGNAL-REF></I-SIGNAL-TO-I-PDU-MAPPING></I-SIGNAL-TO-PDU-MAPPINGS></I-SIGNAL-I-PDU>",
            "native.type.I-SIGNAL-REF",
        ),
    ];
    for (elements, rule_id) in cases {
        let result = check(&document(elements));
        assert_eq!(result.status, ValidationStatus::Failed, "{elements}");
        let issue = result
            .diagnostics
            .iter()
            .find(|issue| issue.rule_id == rule_id && matches!(issue.severity, Severity::Error))
            .expect("consumer-visible rule diagnostic");
        assert_eq!(issue.file.as_deref(), Some("native-input.arxml"));
        let witness = issue.witness.as_ref().unwrap();
        assert_eq!(witness.rule_id, rule_id);
        assert_eq!(witness.subjects, [issue.path.clone().unwrap()]);
    }
}

#[test]
fn native_schema_unknown_content_and_numeric_expressions_are_unsupported() {
    let unknown = document(
        "<COMPU-METHOD><SHORT-NAME>PreservedScale</SHORT-NAME><CATEGORY>LINEAR</CATEGORY></COMPU-METHOD>\
         <I-SIGNAL><SHORT-NAME>Supported</SHORT-NAME><LENGTH>8</LENGTH></I-SIGNAL>",
    );
    let result = check(&unknown);
    assert_eq!(result.status, ValidationStatus::Unsupported);
    assert!(result.coverage.iter().any(|row| {
        !row.supported
            && row
                .subjects
                .iter()
                .any(|subject| subject.ends_with("COMPU-METHOD"))
    }));
    let expression = document(
        "<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>Com</SHORT-NAME><CONTAINERS>\
         <ECUC-CONTAINER-VALUE><SHORT-NAME>General</SHORT-NAME><PARAMETER-VALUES>\
         <ECUC-NUMERICAL-PARAM-VALUE><VALUE>SystemConstant + 1</VALUE></ECUC-NUMERICAL-PARAM-VALUE>\
         </PARAMETER-VALUES></ECUC-CONTAINER-VALUE></CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>",
    );
    assert_eq!(check(&expression).status, ValidationStatus::Unsupported);
    let unsupported_choice = document(
        "<I-SIGNAL><SHORT-NAME>ArraySignal</SHORT-NAME><INIT-VALUE>\
         <ARRAY-VALUE-SPECIFICATION/></INIT-VALUE><LENGTH>8</LENGTH></I-SIGNAL>",
    );
    assert_eq!(
        check(&unsupported_choice).status,
        ValidationStatus::Unsupported,
        "a wider legal XML choice cannot be mistaken for a missing supported alternative"
    );
    let duplicate_choice = unsupported_choice.replace("<ARRAY-VALUE-SPECIFICATION/>",
        "<ARRAY-VALUE-SPECIFICATION/><NUMERICAL-VALUE-SPECIFICATION><VALUE>0</VALUE></NUMERICAL-VALUE-SPECIFICATION>");
    assert_eq!(
        check(&duplicate_choice).status,
        ValidationStatus::Failed,
        "unsupported alternatives still count toward an XML choice's maximum cardinality"
    );
    let broken_supported = unknown.replace("<LENGTH>8</LENGTH>", "<LENGTH>bad</LENGTH>");
    assert_eq!(
        check(&broken_supported).status,
        ValidationStatus::Failed,
        "unknown preservation cannot suppress known schema errors"
    );
}

#[test]
fn native_schema_rejects_unsafe_xml_paths_release_and_size() {
    let valid = document("<I-SIGNAL><SHORT-NAME>Signal</SHORT-NAME><LENGTH>8</LENGTH></I-SIGNAL>");
    for text in [
        valid.replacen(
            "<AUTOSAR",
            "<!DOCTYPE AUTOSAR [<!ENTITY x SYSTEM 'file:///secret'>]><AUTOSAR",
            1,
        ),
        valid.replace("</I-SIGNAL>", "</BROKEN>"),
        valid.replace("http://autosar.org/schema/r4.0", "urn:untrusted:autosar"),
        valid.replace("AUTOSAR_00053.xsd", "AUTOSAR_00052.xsd"),
        valid.replace("UTF-8", "ISO-8859-1"),
    ] {
        assert!(rules::validate_native(&[(Path::new("safe.arxml"), &text)]).is_err());
    }
    assert!(rules::validate_native(&[(Path::new("../escape.arxml"), &valid)]).is_err());
    assert!(rules::validate_native(&[(Path::new("input.txt"), &valid)]).is_err());
    assert!(rules::validate_native(&[]).is_err());
    assert!(
        rules::validate_native(&[
            (Path::new("same.arxml"), &valid),
            (Path::new("same.arxml"), &valid)
        ])
        .is_err()
    );
    let oversized = " ".repeat(50 * 1024 * 1024 + 1);
    assert!(rules::validate_native(&[(Path::new("large.arxml"), &oversized)]).is_err());
}

#[cfg(unix)]
#[test]
fn native_schema_rejects_linked_source_ancestors() {
    let scratch = Scratch::new();
    let actual = scratch.0.join("actual");
    fs::create_dir(&actual).unwrap();
    let link = scratch.0.join("linked");
    std::os::unix::fs::symlink(&actual, &link).unwrap();
    let input = document("");
    assert!(rules::validate_native(&[(link.join("new.arxml").as_path(), &input)]).is_err());
}

#[test]
fn native_inventory_rejects_tampering_even_with_self_recomputed_metadata_hash() {
    let expected = rules::verify_inventory_bytes(rules::inventory_bytes()).unwrap();
    assert_eq!(expected.release, "R24-11");
    assert_eq!(expected.rules_version, "1.0.0");
    let mut inventory: serde_json::Value =
        serde_json::from_slice(rules::inventory_bytes()).unwrap();
    inventory["sources"][0]["sha256"] = serde_json::Value::String(format!(
        "{:x}",
        Sha256::digest(b"replacement implementation")
    ));
    let tampered = serde_json::to_vec(&inventory).unwrap();
    assert!(
        rules::verify_inventory_bytes(&tampered).is_err(),
        "an inventory cannot authorize itself by recomputing a mutable source hash"
    );
    let mut changed_coverage: serde_json::Value =
        serde_json::from_slice(rules::inventory_bytes()).unwrap();
    changed_coverage["coverage"][0]["supported"] = serde_json::Value::Bool(false);
    assert!(
        rules::verify_inventory_bytes(&serde_json::to_vec(&changed_coverage).unwrap()).is_err()
    );
    assert!(rules::verify_inventory_bytes(&[]).is_err());
}

#[test]
fn native_schema_oracle_agrees_on_independent_positive_and_negative_examples() {
    let legal = document("<I-SIGNAL><SHORT-NAME>Signal</SHORT-NAME><LENGTH>8</LENGTH></I-SIGNAL>");
    let file = Path::new("oracle-comparison.arxml");
    assert!(
        schema::validate_files(&archive(), &[(file, &legal)])
            .unwrap()
            .is_empty()
    );
    assert_eq!(check(&legal).status, ValidationStatus::Passed);
    let negatives = [
        legal.replace("<SHORT-NAME>Signal</SHORT-NAME>", ""),
        legal.replace(
            "<SHORT-NAME>Signal</SHORT-NAME><LENGTH>8</LENGTH>",
            "<LENGTH>8</LENGTH><SHORT-NAME>Signal</SHORT-NAME>",
        ),
        legal.replace("<LENGTH>8</LENGTH>", "<LENGTH>8</LENGTH><LENGTH>9</LENGTH>"),
        legal.replace("<LENGTH>8</LENGTH>", "<LENGTH>invalid</LENGTH>"),
    ];
    for text in negatives {
        assert!(
            !schema::validate_files(&archive(), &[(file, &text)])
                .unwrap()
                .is_empty()
        );
        assert_eq!(check(&text).status, ValidationStatus::Failed);
    }
    let legal_unknown = document(
        "<COMPU-METHOD><SHORT-NAME>OriginalScale</SHORT-NAME><CATEGORY>LINEAR</CATEGORY></COMPU-METHOD>",
    );
    assert!(
        schema::validate_files(&archive(), &[(file, &legal_unknown)])
            .unwrap()
            .is_empty()
    );
    assert_eq!(check(&legal_unknown).status, ValidationStatus::Unsupported);
}
