//! Built-in schema and rule checks of authored ARXML inputs.
use super::*;
use crate::support::rule_inputs::*;
use autosar_config_core::model::Severity;
use autosar_config_core::project_model::ValidationStatus;
use autosar_config_core::{Workspace, rules};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

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
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/echo-application/positive");
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
fn native_rules_cover_the_standard_template_with_diagnostic_and_dtc_closure() {
    let scratch = Scratch::new();
    let directory = scratch.0.join("Diagnostic");
    let preview =
        Workspace::preview_project_creation(&directory, "Diagnostic", "standard-ecu").unwrap();
    let workspace = Workspace::create_project_previewed(&preview).unwrap();
    let files: Vec<_> = workspace
        .project_manifest()
        .unwrap()
        .inputs
        .iter()
        .map(|input| {
            let path = directory.join(&input.path);
            let text = fs::read_to_string(&path).unwrap();
            (path, text)
        })
        .collect();
    assert!(
        files
            .iter()
            .any(|(_, text)| text.contains("<SHORT-NAME>Dem</SHORT-NAME>"))
    );
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
        let message = serde_json::to_value(&issue.message).unwrap();
        let constraint = serde_json::to_value(&witness.constraint).unwrap();
        assert!(
            constraint["key"]
                .as_str()
                .unwrap()
                .starts_with("backend.rules.constraint_")
        );
        assert_eq!(message[0], constraint);
        assert_eq!(
            message[1],
            serde_json::to_value(&witness.counterexample).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&issue.remedy).unwrap()["key"],
            "backend.rules.remedy_schema"
        );
    }
}

#[test]
fn cardinality_diagnostics_preserve_actual_bounds_and_counterexamples() {
    for (elements, names, min, max, count) in [
        (
            "<I-SIGNAL><LENGTH>8</LENGTH></I-SIGNAL>",
            "SHORT-NAME",
            "1",
            "1",
            "0",
        ),
        (
            "<I-SIGNAL><SHORT-NAME>Signal</SHORT-NAME><LENGTH>8</LENGTH><LENGTH>9</LENGTH></I-SIGNAL>",
            "LENGTH",
            "0",
            "1",
            "2",
        ),
    ] {
        let result = check(&document(elements));
        let issue = result
            .diagnostics
            .iter()
            .find(|issue| {
                let constraint =
                    serde_json::to_value(&issue.witness.as_ref().unwrap().constraint).unwrap();
                constraint["key"] == "backend.rules.constraint_cardinality"
                    && constraint["params"]["names"] == names
            })
            .expect("cardinality failure for the affected child");
        let value = serde_json::to_value(&issue.message).unwrap();
        assert_eq!(value[0]["params"]["min"], min);
        assert_eq!(value[0]["params"]["max"], max);
        assert_eq!(value[1], count);
        let presented = issue.message.to_string();
        assert!(presented.contains(names));
        assert!(presented.contains(&format!("{min}..{max}")));
        assert!(presented.ends_with(count));
        assert!(!presented.contains("{{"));
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
    assert_eq!(
        serde_json::to_value(
            result
                .coverage
                .iter()
                .find(|row| !row.supported)
                .unwrap()
                .reason
                .as_ref()
                .unwrap()
        )
        .unwrap()["key"],
        "backend.rules.reason_uncovered"
    );
    let unknown_issue = result
        .diagnostics
        .iter()
        .find(|issue| issue.code == "NATIVE_UNSUPPORTED")
        .unwrap();
    let unknown_witness = unknown_issue.witness.as_ref().unwrap();
    assert_eq!(
        serde_json::to_value(&unknown_witness.counterexample).unwrap(),
        "COMPU-METHOD"
    );
    assert_eq!(
        serde_json::to_value(&unknown_issue.remedy).unwrap()["key"],
        "backend.rules.remedy_unsupported"
    );
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
    assert_eq!(
        serde_json::to_value(rules::validate_native(&[]).unwrap_err()).unwrap(),
        serde_json::json!({"key": "backend.rules.source_required", "params": {}})
    );
    assert!(
        rules::validate_native(&[
            (Path::new("same.arxml"), &valid),
            (Path::new("same.arxml"), &valid)
        ])
        .is_err()
    );
    let oversized = " ".repeat(50 * 1024 * 1024 + 1);
    assert!(rules::validate_native(&[(Path::new("large.arxml"), &oversized)]).is_err());
    let path_error = rules::validate_native(&[(Path::new("../escape.arxml"), &valid)]).unwrap_err();
    assert_eq!(
        serde_json::to_value(&path_error).unwrap(),
        serde_json::json!({"key": "backend.rules.source_path", "params": {"path": "../escape.arxml"}})
    );
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
    let bound: serde_json::Value = serde_json::from_slice(rules::inventory_bytes()).unwrap();
    let paths = bound["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["path"] == "core/src/xml_paths.rs")
        .expect("XML diagnostic path indexing must be bound to the native rule identity");
    assert_eq!(
        paths["sha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("../../src/xml_paths.rs"))
        )
    );

    let expected = rules::verify_inventory_bytes(rules::inventory_bytes()).unwrap();
    assert_eq!(expected.release, "R24-11");
    assert_eq!(expected.rules_version, "1.1.0");
    let messages = bound["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["path"] == "core/src/messages.json")
        .expect("localized catalog guidance must participate in the rule identity");
    assert_eq!(
        messages["sha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("../../src/messages.json"))
        )
    );
    let inventory_bytes = rules::inventory_bytes().to_vec();
    let unsupported = rules::coverage()
        .unwrap()
        .into_iter()
        .find(|row| row.rule_id == "native.unsupported")
        .unwrap();
    assert_eq!(
        serde_json::to_value(unsupported.reason.unwrap()).unwrap(),
        serde_json::json!({"key": "backend.rules.inventory_partial_coverage", "params": {}})
    );
    assert_eq!(rules::inventory_bytes(), inventory_bytes);
    assert_eq!(rules::rule_set_identity().unwrap(), expected);
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
