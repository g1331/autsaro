//! The independent protocol and OS vectors are bound to the reference ECUC
//! configuration, and drift in their settings or fixed literals is refused.
use crate::support::protocol_oracles;
use serde_json::Value;

fn oracles() -> (Value, Value) {
    (
        protocol_oracles::load("protocol.json"),
        protocol_oracles::load("os.json"),
    )
}

fn child<'a>(node: roxmltree::Node<'a, '_>, tag: &str) -> Option<&'a str> {
    node.children()
        .find(|child| child.tag_name().name() == tag)
        .and_then(|child| child.text())
}

fn rejected(protocol: &Value, os: &Value, ecuc: &[u8]) -> String {
    protocol_oracles::validate(protocol, os, ecuc).expect_err("drifted oracle was accepted")
}

/// Sets the first numerical parameter ending in `suffix` (inside the
/// container `container`, when given) to `value`.
fn with_parameter(ecuc: &[u8], container: Option<&str>, suffix: &str, value: &str) -> Vec<u8> {
    let text = std::str::from_utf8(ecuc).unwrap();
    let document = roxmltree::Document::parse(text).unwrap();
    let named = |node: &roxmltree::Node, tag: &str| node.tag_name().name() == tag;
    let scope = match container {
        Some(name) => document
            .descendants()
            .find(|node| {
                named(node, "ECUC-CONTAINER-VALUE") && child(*node, "SHORT-NAME") == Some(name)
            })
            .unwrap(),
        None => document.root(),
    };
    let parameter = scope
        .descendants()
        .find(|node| {
            named(node, "ECUC-NUMERICAL-PARAM-VALUE")
                && child(*node, "DEFINITION-REF").is_some_and(|path| path.ends_with(suffix))
        })
        .unwrap_or_else(|| panic!("no parameter {suffix}"));
    let range = parameter
        .children()
        .find(|child| named(child, "VALUE"))
        .and_then(|node| node.first_child())
        .unwrap()
        .range();
    format!("{}{value}{}", &text[..range.start], &text[range.end..]).into_bytes()
}

#[test]
fn reference_configuration_matches_the_independent_oracles() {
    let (protocol, os) = oracles();
    protocol_oracles::validate(&protocol, &os, &protocol_oracles::reference_ecuc()).unwrap();
}

#[test]
fn changed_timing_protocol_and_os_literals_are_refused() {
    let ecuc = protocol_oracles::reference_ecuc();
    for (family, id, field, value) in [
        (
            "timing_cases",
            "ncr-on-deadline",
            "deadline",
            serde_json::json!(211),
        ),
        (
            "cases",
            "unsupported-did",
            "expected",
            serde_json::json!([]),
        ),
        (
            "os",
            "activation-limit",
            "expected_status",
            serde_json::json!("E_OK"),
        ),
        (
            "cases",
            "initial-did",
            "expected",
            serde_json::json!([{"epoch": 0, "id": 1800, "data": "0762123400000001"}]),
        ),
    ] {
        let (mut protocol, mut os) = oracles();
        let cases = if family == "os" {
            &mut os["cases"]
        } else {
            &mut protocol[family]
        };
        let case = cases
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|case| case["id"] == id)
            .unwrap();
        case[field] = value;
        let error = rejected(&protocol, &os, &ecuc);
        assert!(
            error.contains("independent oracle changed") || error.contains("initial DID literal"),
            "{id}: {error}"
        );
    }
}

#[test]
fn configured_processing_periods_cannot_diverge_from_the_oracles() {
    let (protocol, os) = oracles();
    let ecuc = protocol_oracles::reference_ecuc();
    for (container, suffix, value) in [
        (None, "/CanTpMainFunctionPeriod", "0.02"),
        (None, "/ComTxModeTimePeriod", "0.02"),
        (Some("Alarm_App"), "/OsAlarmCycleTime", "20"),
        (Some("SystemCounter"), "/OsSecondsPerTick", "20"),
        (None, "/DcmDspSessionP2ServerMax", "0.051"),
        (None, "/ComTimeout", "0.031"),
    ] {
        let changed = with_parameter(&ecuc, container, suffix, value);
        let error = rejected(&protocol, &os, &changed);
        assert!(
            error.starts_with("oracle/config mismatch"),
            "{suffix}: {error}"
        );
    }
}

#[test]
fn every_fixed_requirement_refuses_its_own_drift() {
    let ecuc = protocol_oracles::reference_ecuc();
    type Edit = fn(&mut Value, &mut Value);
    let edits: [(&str, Edit); 7] = [
        ("normative S3 fallback changed", |protocol, _| {
            protocol["settings"]["s3_ms"] = 4000.into()
        }),
        (
            "controlled input/deadline ordering changed",
            |protocol, _| protocol["settings"]["input_before_deadline"] = false.into(),
        ),
        (
            "independent CAN/UDS contract bytes changed",
            |protocol, _| {
                protocol["cases"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|case| case["id"] == "sr-and-same-epoch-did")
                    .unwrap()["expected"][0]["data"] = "78563413".into()
            },
        ),
        ("missing expected-value provenance", |protocol, _| {
            protocol["cases"][0]["source"] = serde_json::json!([])
        }),
        ("invalid Classical CAN vector", |protocol, _| {
            protocol["cases"][0]["expected"][0]["id"] = 0x800.into()
        }),
        ("AABB expected request sequence changed", |_, os| {
            os["cases"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|case| case["id"] == "fifo-aabb")
                .unwrap()["expected_entry_order"] = serde_json::json!(["A#1", "B#1", "A#2", "B#2"])
        }),
        ("R24-11 null-pointer status weakened", |_, os| {
            os["cases"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|case| case["id"] == "null-state")
                .unwrap()["expected_status"] = "E_OS_ID".into()
        }),
    ];
    for (message, edit) in edits {
        let (mut protocol, mut os) = oracles();
        edit(&mut protocol, &mut os);
        let error = rejected(&protocol, &os, &ecuc);
        assert!(error.contains(message), "{message}: {error}");
    }
}
