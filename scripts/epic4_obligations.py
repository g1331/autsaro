"""Verify fixed sources, normative trace inventory and independent Epic 4 oracles.

Passing this baseline never marks future OS/ECU behavior as measured. Official
documents remain external inputs and are never emitted into reference packages.
"""

from __future__ import annotations

import argparse
import bisect
import hashlib
import json
import re
import subprocess
import shutil
from pathlib import Path
import xml.etree.ElementTree as ET

import epic4_os

ROOT = Path(__file__).resolve().parents[1]
ASSURANCE = ROOT / "docs/assurance/epic4"
ORACLES = ROOT / "core/tests/fixtures/epic4_oracles"
NS = {"a": "http://autosar.org/schema/r4.0"}
REQUIREMENT = re.compile(
    r"\[(SWS_Os_(?:CONSTR_)?\d+)\]((?:(?!\[SWS_Os_(?:CONSTR_)?\d+\]).)*?)⌈(.*?)⌋",
    re.S,
)


def digest(path: Path) -> str:
    if not path.is_file():
        raise ValueError(f"not_run: required source is missing: {path}")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def canonical_digest(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def validate_reviewed_cases(cases: list[dict], fingerprints: dict, label: str) -> None:
    identifiers = [case["id"] for case in cases]
    require(
        len(identifiers) == len(set(identifiers))
        and set(identifiers) == set(fingerprints),
        f"{label}: reviewed case inventory changed",
    )
    for case in cases:
        require(bool(case.get("source")), f"{case['id']}: missing oracle provenance")
        require(
            canonical_digest(case) == fingerprints[case["id"]],
            f"{label}: reviewed independent oracle changed: {case['id']}",
        )


def current_os_index(pages: list[str]) -> dict:
    starts = []
    offset = 0
    for page in pages:
        starts.append(offset)
        offset += len(page) + 1
    found = {}
    for match in REQUIREMENT.finditer("\n".join(pages)):
        require(match[1] not in found, f"duplicate normative definition {match[1]}")
        found[match[1]] = {
            "pdf_page": bisect.bisect_right(starts, match.start()),
            "paragraph_sha256": hashlib.sha256(match[3].strip().encode()).hexdigest(),
        }
    return found


def validate_matrix(matrix: dict, index: dict, story_tests: dict) -> None:
    rows = matrix["rows"]
    keys = [row["key"] for row in rows]
    require(len(keys) == len(set(keys)), "duplicate obligation key")
    current = {row["id"] for row in index["current_definitions"]}
    require(current <= set(keys), "current OS requirement omitted from trace matrix")
    require(
        {key for key in keys if key.startswith("SWS_Os_")} == current,
        "deleted or unknown OS requirement introduced",
    )
    for row in rows:
        for field in [
            "source",
            "section",
            "pdf_page",
            "condition_and_reason",
            "configuration",
            "producer",
            "story",
            "test",
            "future_evidence",
        ]:
            require(bool(row.get(field)), f"{row['key']}: missing trace field {field}")
        require(
            row["applicability"] in {"applicable", "conditional", "conditional_na"},
            f"{row['key']}: unknown applicability",
        )
        require(
            row["status"] == "not_run",
            f"{row['key']}: unmeasured behavior marked passed",
        )
        require(
            story_tests.get(row["story"]) == row["test"],
            f"{row['key']}: future test does not match responsible story",
        )
    mandatory = {
        "SWS_Os_00001",
        "SWS_Os_00763",
        "SWS_Os_00107",
        "SWS_Os_00241",
        "SWS_Os_00566",
        "SWS_Os_00067",
        "SWS_Os_00068",
        "SWS_Os_00424",
        "SWS_Os_00425",
        "SWS_Os_00052",
        "SWS_Os_00069",
        "SWS_Os_00070",
        "SWS_Os_00239",
        "SWS_Os_00368",
        "SWS_Os_00369",
        "SWS_Os_00071",
        "SWS_Os_00092",
        "SWS_Os_00093",
        "SWS_Os_00858",
        "SWS_Os_00829",
        "SWS_Os_00836",
        "SWS_Os_00837",
        "SC1_CAP_SOFTWARE_COUNTERS",
        "SC1_CAP_SCHEDULE_TABLES",
    }
    by_key = {row["key"]: row for row in rows}
    for declaration in index["current_definitions"]:
        row = by_key[declaration["id"]]
        require(
            row["pdf_page"] == declaration["pdf_page"]
            and row["section"] == declaration["section"],
            f"{row['key']}: normative source location changed",
        )
    require(
        all(by_key[key]["applicability"] == "applicable" for key in mandatory),
        "core SC1 duty incorrectly waived",
    )
    require(
        by_key["SC1_CAP_SOFTWARE_COUNTERS"]["minimum"] >= 8,
        "software Counter capacity lowered",
    )
    require(
        by_key["SC1_CAP_SCHEDULE_TABLES"]["minimum"] >= 2,
        "ScheduleTable capacity lowered",
    )
    for cls, tasks, priorities in [
        ("BCC1", 8, 8),
        ("BCC2", 8, 8),
        ("ECC1", 16, 16),
        ("ECC2", 16, 16),
    ]:
        for label, minimum in [
            ("NON_SUSPENDED_TASKS", tasks),
            ("TASK_PRIORITIES", priorities),
            ("INTERNAL_RESOURCES", 2),
            ("RESOURCES", 1 if cls == "BCC1" else 8),
            ("ALARMS", 1),
            ("APPLICATION_MODES", 1),
        ]:
            row = by_key[f"OSEK_CAP_{cls}_{label}"]
            require(
                row["applicability"] == "applicable" and row["minimum"] >= minimum,
                f"{cls} minimum {label} lowered",
            )
        if cls.startswith("ECC"):
            row = by_key[f"OSEK_CAP_{cls}_EVENTS_PER_TASK"]
            require(
                row["applicability"] == "applicable" and row["minimum"] >= 8,
                f"{cls} event capacity lowered",
            )
    reviewed = load(ASSURANCE / "reviewed-baseline.json")
    require(
        {key for key in keys if not key.startswith("SWS_Os_")}
        == set(reviewed["non_os_keys"]),
        "reviewed OSEK or integration obligation inventory changed",
    )
    require(
        {r["key"] for r in rows if r["applicability"] == "applicable"}
        == set(reviewed["applicable_keys"]),
        "reviewed SC1/integration applicability changed",
    )
    trace = [
        {k: v for k, v in row.items() if k not in {"status", "applicability_review"}}
        for row in rows
    ]
    require(
        canonical_digest(trace) == reviewed["trace_sha256"],
        "reviewed normative trace changed; independent review required",
    )


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
    reviewed = load(ASSURANCE / "reviewed-baseline.json")
    require(
        canonical_digest(protocol["settings"]) == reviewed["protocol_settings_sha256"],
        "reviewed protocol settings changed",
    )
    validate_reviewed_cases(protocol["cases"], reviewed["protocol_cases"], "protocol")
    validate_reviewed_cases(
        protocol["timing_cases"], reviewed["timing_cases"], "timing"
    )
    validate_reviewed_cases(os_oracles["cases"], reviewed["os_cases"], "OS")


def verify(args: argparse.Namespace) -> dict:
    try:
        import pymupdf
    except ImportError as error:
        raise ValueError(
            "not_run: required external PDF reader PyMuPDF is missing"
        ) from error
    sources = load(ASSURANCE / "sources.json")
    documents = {}
    verified_sources = []
    for source in sources["sources"]:
        path = args.osek_pdf if source["id"] == "OSEK_OS" else ROOT / source["path"]
        require(
            digest(path) == source["sha256"],
            f"source identity mismatch: {source['id']}",
        )
        require(source["redistribute"] is False, "official source must remain external")
        documents[source["id"]] = [page.get_text() for page in pymupdf.open(path)]
        verified_sources.append(source["id"])
    index = load(ASSURANCE / "os-source-index.json")
    actual = current_os_index(documents["AUTOSAR_CP_SWS_OS"])
    declared = {
        r["id"]: {k: r[k] for k in ["pdf_page", "paragraph_sha256"]}
        for r in index["current_definitions"]
    }
    require(
        actual == declared and len(actual) == index["current_definition_count"],
        "current normative index differs from pinned source PDF",
    )
    matrix = load(ASSURANCE / "obligations.json")
    reviewed = load(ASSURANCE / "reviewed-baseline.json")
    for path, identity in reviewed["reference_inputs"].items():
        require(
            digest(ROOT / path) == identity, f"reviewed reference input changed: {path}"
        )
    planning = (ROOT / "_bmad-output/planning-artifacts/epics.md").read_text(
        encoding="utf-8"
    )
    tests = {}
    for number in range(1, 23):
        block = planning.split(f"### Story 4.{number}:", 1)[1].split("### Story ", 1)[0]
        tests[f"4.{number}"] = re.search(r"执行入口：\*\*`([^`]+)`", block)[1]
    validate_matrix(matrix, index, tests)
    protocol = load(ORACLES / "protocol.json")
    os_oracles = load(ORACLES / "os.json")
    validate_protocol(
        protocol,
        os_oracles,
        (ROOT / "core/tests/fixtures/epic4/positive/ecuc.arxml").read_bytes(),
    )
    kernel = epic4_os.verify_sources()
    require(
        kernel == sources["kernel"],
        "kernel provenance record differs from real originals",
    )
    require(
        digest(args.kernel_archive) == sources["archive_identity"]["sha256"],
        "fixed kernel archive identity mismatch",
    )
    for patch in sources["production_patches"] + sources["research_material"]:
        require(
            digest(ROOT / patch["path"]) == patch["sha256"],
            "patch/research identity mismatch",
        )
    version = subprocess.run(
        [args.compiler, "--version"], capture_output=True, text=True, check=True
    ).stdout.splitlines()[0]
    target = subprocess.run(
        [args.compiler, "-dumpmachine"], capture_output=True, text=True, check=True
    ).stdout.strip()
    require(
        epic4_os.compiler_description(version)
        == epic4_os.compiler_description(sources["compiler"]["identity"])
        and target == sources["compiler"]["target"]
        and digest(Path(args.compiler)) == sources["compiler"]["executable_sha256"],
        "fixed compiler identity mismatch",
    )
    return {
        "baseline_status": "pass",
        "future_behavior_status": "not_run",
        "independent_baseline_review": reviewed["status"],
        "source_count": len(verified_sources),
        "current_os_requirements": len(actual),
        "trace_rows": len(matrix["rows"]),
        "protocol_cases": len(protocol["cases"]),
        "timing_cases": len(protocol["timing_cases"]),
        "os_cases": len(os_oracles["cases"]),
        "pdf_reader": pymupdf.VersionBind,
        "compiler": version,
        "scope": "source/trace/oracle baseline only; target behavior and final applicability exit remain separate gates",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--osek-pdf", type=Path, default=ROOT / "docs/official/OSEK/os223.pdf"
    )
    parser.add_argument(
        "--kernel-archive",
        type=Path,
        default=ROOT / "docs/official/FreeRTOS/kernel.tar.gz",
    )
    parser.add_argument("--compiler", default=shutil.which("gcc") or "gcc")
    parser.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    result = verify(args)
    if args.evidence:
        result["inputs"] = {
            p.relative_to(ROOT).as_posix(): digest(p)
            for folder in [ASSURANCE, ORACLES]
            for p in sorted(folder.glob("*.json"))
        }
        args.evidence.parent.mkdir(parents=True, exist_ok=True)
        args.evidence.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print("epic4_obligation_and_oracle_baseline PASS " + json.dumps(result))


if __name__ == "__main__":
    main()
