//! Independent protocol and OS oracle vectors bound to an ECUC configuration.
//!
//! The vectors in `fixtures/oracles/` are written from the standards,
//! not from generated output. [`validate`] refuses them when their settings
//! differ from the ECUC being tested or when a fixed literal has drifted.
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

const NAMESPACE: &str = "http://autosar.org/schema/r4.0";
const PREFIX: &str = "/AUTOSAR/EcucDefs/";

/// Protocol settings and the ECUC parameter (seconds or counts) each binds to,
/// with the factor that converts the ECUC value into the setting's unit.
const BINDINGS: [(&str, &str, i128); 18] = [
    (
        "p2_ms",
        "Dcm/DcmConfigSet/DcmDsp/DcmDspSession/DcmDspSessionRow/DcmDspSessionP2ServerMax",
        1000,
    ),
    (
        "p2_star_ms",
        "Dcm/DcmConfigSet/DcmDsp/DcmDspSession/DcmDspSessionRow/DcmDspSessionP2StarServerMax",
        1000,
    ),
    ("padding_byte", "CanTp/CanTpGeneral/CanTpPaddingByte", 1),
    (
        "max_dids_per_request",
        "Dcm/DcmConfigSet/DcmDsp/DcmDspMaxDidToRead",
        1,
    ),
    (
        "nbs_ms",
        "CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu/CanTpNbs",
        1000,
    ),
    (
        "ncr_ms",
        "CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpNcr",
        1000,
    ),
    (
        "nas_ms",
        "CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu/CanTpNas",
        1000,
    ),
    (
        "nar_ms",
        "CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpNar",
        1000,
    ),
    (
        "nbr_ms",
        "CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpNbr",
        1000,
    ),
    (
        "ncs_ms",
        "CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu/CanTpNcs",
        1000,
    ),
    (
        "bs",
        "CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpBs",
        1,
    ),
    (
        "stmin_ms",
        "CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpSTmin",
        1000,
    ),
    (
        "rx_sdu_buffer_bytes",
        "Dcm/DcmConfigSet/DcmDsl/DcmDslBuffer/DcmDslBufferSize",
        1,
    ),
    (
        "tx_sdu_buffer_bytes",
        "Dcm/DcmConfigSet/DcmDsl/DcmDslBuffer/DcmDslBufferSize",
        1,
    ),
    ("rx_deadline_ms", "Com/ComConfig/ComSignal/ComTimeout", 1000),
    (
        "first_rx_timeout_ms",
        "Com/ComConfig/ComSignal/ComFirstTimeout",
        1000,
    ),
    ("tick_ms", "Dcm/DcmGeneral/DcmTaskTime", 1000),
    (
        "tx_period_ms",
        "Com/ComConfig/ComIPdu/ComTxIPdu/ComTxModeTrue/ComTxMode/ComTxModeTimePeriod",
        1000,
    ),
];

/// Reads one oracle document from `fixtures/oracles`.
pub(crate) fn load(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/oracles")
        .join(name);
    serde_json::from_slice(&std::fs::read(&path).unwrap())
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The ECUC configuration the reference vectors were written for.
pub(crate) fn reference_ecuc() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/echo-application/positive/ecuc.arxml"),
    )
    .unwrap()
}

/// An exact decimal: `mantissa * 10^-scale`.
fn decimal(text: &str) -> Option<(i128, u32)> {
    let text = text.trim();
    let (number, exponent) = match text.split_once(['e', 'E']) {
        Some((number, exponent)) => (number, exponent.parse::<i32>().ok()?),
        None => (text, 0),
    };
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    let mantissa: i128 = format!("{whole}{fraction}").parse().ok()?;
    let scale = fraction.len() as i32 - exponent;
    if scale >= 0 {
        Some((mantissa, scale as u32))
    } else {
        Some((
            mantissa.checked_mul(10i128.checked_pow((-scale) as u32)?)?,
            0,
        ))
    }
}

/// `value * factor == expected`, compared exactly.
fn scaled_equals(value: &str, factor: i128, expected: &Value) -> bool {
    let (Some((mantissa, scale)), Some((target, target_scale))) =
        (decimal(value), decimal(&expected.to_string()))
    else {
        return false;
    };
    let left = mantissa * factor * 10i128.pow(target_scale);
    let right = target * 10i128.pow(scale);
    left == right
}

fn ecuc(name: &'static str) -> roxmltree::ExpandedName<'static, 'static> {
    (NAMESPACE, name).into()
}

fn child_text<'a>(node: roxmltree::Node<'a, '_>, name: &'static str) -> Option<&'a str> {
    node.children()
        .find(|child| child.has_tag_name(ecuc(name)))
        .and_then(|child| child.text())
}

/// Every numerical parameter value below `node`, keyed by definition reference.
fn parameters<'a>(node: roxmltree::Node<'a, '_>) -> BTreeMap<&'a str, Vec<&'a str>> {
    let mut values = BTreeMap::<_, Vec<_>>::new();
    for parameter in node
        .descendants()
        .filter(|node| node.has_tag_name(ecuc("ECUC-NUMERICAL-PARAM-VALUE")))
    {
        values
            .entry(child_text(parameter, "DEFINITION-REF").unwrap_or_default())
            .or_default()
            .push(child_text(parameter, "VALUE").unwrap_or_default());
    }
    values
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn by_id(cases: &Value) -> BTreeMap<&str, &Value> {
    cases
        .as_array()
        .unwrap()
        .iter()
        .map(|case| (case["id"].as_str().unwrap(), case))
        .collect()
}

/// Checks the protocol and OS oracles against `ecuc` and their fixed literals.
pub(crate) fn validate(protocol: &Value, os: &Value, ecuc_bytes: &[u8]) -> Result<(), String> {
    let text = std::str::from_utf8(ecuc_bytes).map_err(|error| error.to_string())?;
    let document = roxmltree::Document::parse(text).map_err(|error| error.to_string())?;
    let all = parameters(document.root());
    let settings = &protocol["settings"];
    for (setting, definition, factor) in BINDINGS {
        let actual = all
            .get(format!("{PREFIX}{definition}").as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        require(
            !actual.is_empty()
                && actual
                    .iter()
                    .all(|value| scaled_equals(value, factor, &settings[setting])),
            &format!("oracle/config mismatch: {setting}"),
        )?;
    }
    let main_period = all
        .get(format!("{PREFIX}CanTp/CanTpConfig/CanTpMainFunctionPeriod").as_str())
        .map(Vec::as_slice)
        .unwrap_or_default();
    require(
        !main_period.is_empty()
            && main_period
                .iter()
                .all(|value| scaled_equals(value, 1000, &settings["tick_ms"])),
        "oracle/config mismatch: CanTp main function period",
    )?;
    let containers = |name: &'static str| {
        document
            .descendants()
            .filter(|node| {
                node.has_tag_name(ecuc("ECUC-CONTAINER-VALUE"))
                    && child_text(*node, "SHORT-NAME") == Some(name)
            })
            .collect::<Vec<_>>()
    };
    let counters = containers("SystemCounter");
    require(
        counters.len() == 1,
        "reference SystemCounter must be unique",
    )?;
    let tick = parameters(counters[0])
        .remove(format!("{PREFIX}Os/OsCounter/OsSecondsPerTick").as_str())
        .unwrap_or_default();
    require(
        tick.len() == 1 && scaled_equals(tick[0], 1000, &settings["tick_ms"]),
        "oracle/config mismatch: SystemCounter tick duration",
    )?;
    let alarms = containers("Alarm_App");
    require(
        alarms.len() == 1,
        "reference application alarm must be unique",
    )?;
    let cycles = parameters(alarms[0])
        .remove(format!("{PREFIX}Os/OsAlarm/OsAlarmAutostart/OsAlarmCycleTime").as_str())
        .unwrap_or_default();
    let tick_ms = settings["tick_ms"].as_i64().map(i128::from);
    require(
        cycles.len() == 1
            && tick_ms.is_some_and(|tick_ms| {
                scaled_equals(cycles[0], tick_ms, &settings["application_period_ms"])
            }),
        "oracle/config mismatch: application alarm cycle",
    )?;
    require(settings["s3_ms"] == 5000, "normative S3 fallback changed")?;
    require(
        !all.keys()
            .any(|key| key.ends_with("/DcmS3ServerTimeoutOverwrite")),
        "S3 fallback oracle used despite configured overwrite",
    )?;
    require(
        settings["input_before_deadline"] == true,
        "controlled input/deadline ordering changed",
    )?;
    let cases = by_id(&protocol["cases"]);
    require(
        cases["initial-did"]["expected"][0]["data"] == "0762123400000000",
        "initial DID literal changed",
    )?;
    require(
        cases["sr-and-same-epoch-did"]["expected"]
            == serde_json::json!([
                {"epoch": 10, "id": 801, "data": "78563412"},
                {"epoch": 10, "id": 1800, "data": "0762123412345678"},
            ]),
        "independent CAN/UDS contract bytes changed",
    )?;
    for case in protocol["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        require(
            match &case["source"] {
                Value::Array(sources) => !sources.is_empty(),
                Value::String(source) => !source.is_empty(),
                _ => false,
            },
            &format!("{id}: missing expected-value provenance"),
        )?;
        for frame in case["expected"].as_array().unwrap() {
            let bytes = frame["data"].as_str().unwrap_or_default();
            require(
                frame["id"].as_u64().is_some_and(|id| id <= 0x7ff)
                    && bytes.len() % 2 == 0
                    && (2..=16).contains(&bytes.len())
                    && bytes.chars().all(|digit| digit.is_ascii_hexdigit()),
                &format!("{id}: invalid Classical CAN vector"),
            )?;
        }
    }
    let os_cases = by_id(&os["cases"]);
    require(
        os_cases["fifo-aabb"]["expected_entry_order"]
            == serde_json::json!(["A#1", "A#2", "B#1", "B#2"]),
        "AABB expected request sequence changed",
    )?;
    require(
        os_cases["fifo-abab"]["expected_entry_order"]
            == serde_json::json!(["A#1", "B#1", "A#2", "B#2"]),
        "ABAB expected request sequence changed",
    )?;
    require(
        os_cases["null-state"]["expected_status"] == "E_OS_ILLEGAL_ADDRESS",
        "R24-11 null-pointer status weakened",
    )?;
    require(
        cases["unsupported-did"]["expected"]
            == serde_json::json!([{"epoch": 0, "id": 1800, "data": "037F223100000000"}]),
        "independent oracle changed: unsupported-did",
    )?;
    let timing = by_id(&protocol["timing_cases"]);
    require(
        timing["ncr-on-deadline"]["deadline"] == 210,
        "independent oracle changed: ncr-on-deadline",
    )?;
    require(
        os_cases["activation-limit"]["expected_status"] == "E_OS_LIMIT",
        "independent oracle changed: activation-limit",
    )
}
