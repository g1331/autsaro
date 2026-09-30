"""Check independent protocol and OS vectors against the reference ECU configuration."""

import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
ORACLES = ROOT / "core/tests/fixtures/epic4_oracles"
NS = {"a": "http://autosar.org/schema/r4.0"}

def load(path):
    return json.loads(path.read_text(encoding="utf-8"))

def require(condition, message):
    if not condition:
        raise ValueError(message)

def validate_protocol(protocol: dict, os_oracles: dict, xml: bytes) -> None:
    root = ET.fromstring(xml)
    parameters = {}
    for node in root.findall(".//a:ECUC-NUMERICAL-PARAM-VALUE", NS):
        key = node.findtext("a:DEFINITION-REF", namespaces=NS)
        parameters.setdefault(key, []).append(node.findtext("a:VALUE", namespaces=NS))
    prefix = "/AUTOSAR/EcucDefs/"
    bindings = {
        "p2_ms": (
            "Dcm/DcmConfigSet/DcmDsp/DcmDspSession/DcmDspSessionRow/DcmDspSessionP2ServerMax",
            1000,
        ),
        "p2_star_ms": (
            "Dcm/DcmConfigSet/DcmDsp/DcmDspSession/DcmDspSessionRow/DcmDspSessionP2StarServerMax",
            1000,
        ),
        "padding_byte": ("CanTp/CanTpGeneral/CanTpPaddingByte", 1),
        "max_dids_per_request": ("Dcm/DcmConfigSet/DcmDsp/DcmDspMaxDidToRead", 1),
        "nbs_ms": ("CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu/CanTpNbs", 1000),
        "ncr_ms": ("CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpNcr", 1000),
        "nas_ms": ("CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu/CanTpNas", 1000),
        "nar_ms": ("CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpNar", 1000),
        "nbr_ms": ("CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpNbr", 1000),
        "ncs_ms": ("CanTp/CanTpConfig/CanTpChannel/CanTpTxNSdu/CanTpNcs", 1000),
        "bs": ("CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpBs", 1),
        "stmin_ms": ("CanTp/CanTpConfig/CanTpChannel/CanTpRxNSdu/CanTpSTmin", 1000),
        "rx_sdu_buffer_bytes": (
            "Dcm/DcmConfigSet/DcmDsl/DcmDslBuffer/DcmDslBufferSize",
            1,
        ),
        "tx_sdu_buffer_bytes": (
            "Dcm/DcmConfigSet/DcmDsl/DcmDslBuffer/DcmDslBufferSize",
            1,
        ),
        "rx_deadline_ms": ("Com/ComConfig/ComSignal/ComTimeout", 1000),
        "first_rx_timeout_ms": ("Com/ComConfig/ComSignal/ComFirstTimeout", 1000),
        "tick_ms": ("Dcm/DcmGeneral/DcmTaskTime", 1000),
        "tx_period_ms": (
            "Com/ComConfig/ComIPdu/ComTxIPdu/ComTxModeTrue/ComTxMode/ComTxModeTimePeriod",
            1000,
        ),
    }
    from decimal import Decimal

    for setting, (definition, scale) in bindings.items():
        actual = parameters.get(prefix + definition, [])
        require(
            bool(actual)
            and all(
                Decimal(value) * scale == protocol["settings"][setting]
                for value in actual
            ),
            f"oracle/config mismatch: {setting}",
        )
    require(
        parameters.get(prefix + "CanTp/CanTpConfig/CanTpMainFunctionPeriod")
        and all(
            Decimal(value) * 1000 == protocol["settings"]["tick_ms"]
            for value in parameters[
                prefix + "CanTp/CanTpConfig/CanTpMainFunctionPeriod"
            ]
        ),
        "oracle/config mismatch: CanTp main function period",
    )
    counters = [
        node
        for node in root.findall(".//a:ECUC-CONTAINER-VALUE", NS)
        if node.findtext("a:SHORT-NAME", namespaces=NS) == "SystemCounter"
    ]
    require(len(counters) == 1, "reference SystemCounter must be unique")
    seconds_per_tick = [
        node.findtext("a:VALUE", namespaces=NS)
        for node in counters[0].findall(".//a:ECUC-NUMERICAL-PARAM-VALUE", NS)
        if node.findtext("a:DEFINITION-REF", namespaces=NS)
        == prefix + "Os/OsCounter/OsSecondsPerTick"
    ]
    require(
        len(seconds_per_tick) == 1
        and Decimal(seconds_per_tick[0]) * 1000 == protocol["settings"]["tick_ms"],
        "oracle/config mismatch: SystemCounter tick duration",
    )
    alarms = [
        node
        for node in root.findall(".//a:ECUC-CONTAINER-VALUE", NS)
        if node.findtext("a:SHORT-NAME", namespaces=NS) == "Alarm_App"
    ]
    require(len(alarms) == 1, "reference application alarm must be unique")
    cycles = [
        node.findtext("a:VALUE", namespaces=NS)
        for node in alarms[0].findall(".//a:ECUC-NUMERICAL-PARAM-VALUE", NS)
        if node.findtext("a:DEFINITION-REF", namespaces=NS)
        == prefix + "Os/OsAlarm/OsAlarmAutostart/OsAlarmCycleTime"
    ]
    require(
        len(cycles) == 1
        and Decimal(cycles[0]) * protocol["settings"]["tick_ms"]
        == protocol["settings"]["application_period_ms"],
        "oracle/config mismatch: application alarm cycle",
    )
    require(protocol["settings"]["s3_ms"] == 5000, "normative S3 fallback changed")
    require(
        not any(key.endswith("/DcmS3ServerTimeoutOverwrite") for key in parameters),
        "S3 fallback oracle used despite configured overwrite",
    )
    require(
        protocol["settings"]["input_before_deadline"] is True,
        "controlled input/deadline ordering changed",
    )
    cases = {case["id"]: case for case in protocol["cases"]}
    require(
        cases["initial-did"]["expected"][0]["data"] == "0762123400000000",
        "initial DID literal changed",
    )
    require(
        cases["sr-and-same-epoch-did"]["expected"]
        == [
            {"epoch": 10, "id": 801, "data": "78563412"},
            {"epoch": 10, "id": 1800, "data": "0762123412345678"},
        ],
        "independent CAN/UDS contract bytes changed",
    )
    for case in protocol["cases"]:
        require(
            bool(case["source"]), f"{case['id']}: missing expected-value provenance"
        )
        for frame in case["expected"]:
            require(
                0 <= frame["id"] <= 0x7FF
                and 1 <= len(bytes.fromhex(frame["data"])) <= 8,
                f"{case['id']}: invalid Classical CAN vector",
            )
    os_cases = {case["id"]: case for case in os_oracles["cases"]}
    require(
        os_cases["fifo-aabb"]["expected_entry_order"] == ["A#1", "A#2", "B#1", "B#2"],
        "AABB expected request sequence changed",
    )
    require(
        os_cases["fifo-abab"]["expected_entry_order"] == ["A#1", "B#1", "A#2", "B#2"],
        "ABAB expected request sequence changed",
    )
    require(
        os_cases["null-state"]["expected_status"] == "E_OS_ILLEGAL_ADDRESS",
        "R24-11 null-pointer status weakened",
    )
    require(
        cases["unsupported-did"]["expected"] == [{"epoch": 0, "id": 1800, "data": "037F223100000000"}],
        "independent oracle changed: unsupported-did",
    )
    timing = {case["id"]: case for case in protocol["timing_cases"]}
    require(timing["ncr-on-deadline"]["deadline"] == 210,
            "independent oracle changed: ncr-on-deadline")
    require(os_cases["activation-limit"]["expected_status"] == "E_OS_LIMIT",
            "independent oracle changed: activation-limit")


if __name__ == "__main__":
    validate_protocol(load(ORACLES / "protocol.json"), load(ORACLES / "os.json"),
                      (ROOT / "core/tests/fixtures/epic4/positive/ecuc.arxml").read_bytes())
    print("epic4_independent_oracle_contracts PASS")
