"""Protect the normative baseline against waived duties and edited oracles."""

import copy
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

import epic4_obligations as baseline


class ObligationTests(unittest.TestCase):
    def setUp(self):
        self.matrix = baseline.load(baseline.ASSURANCE / "obligations.json")
        self.index = baseline.load(baseline.ASSURANCE / "os-source-index.json")
        self.tests = {r["story"]: r["test"] for r in self.matrix["rows"]}

    def test_current_requirement_cannot_be_omitted_or_waived(self):
        removed = copy.deepcopy(self.matrix)
        removed["rows"] = [r for r in removed["rows"] if r["key"] != "SWS_Os_00069"]
        with self.assertRaisesRegex(ValueError, "omitted"):
            baseline.validate_matrix(removed, self.index, self.tests)
        waived = copy.deepcopy(self.matrix)
        next(r for r in waived["rows"] if r["key"] == "SWS_Os_00069")[
            "applicability"
        ] = "conditional_na"
        with self.assertRaisesRegex(ValueError, "incorrectly waived"):
            baseline.validate_matrix(waived, self.index, self.tests)

    def test_minimum_capacity_cannot_be_replaced_by_reference_count(self):
        matrix = copy.deepcopy(self.matrix)
        next(r for r in matrix["rows"] if r["key"] == "SC1_CAP_SOFTWARE_COUNTERS")[
            "minimum"
        ] = 1
        with self.assertRaisesRegex(ValueError, "capacity lowered"):
            baseline.validate_matrix(matrix, self.index, self.tests)

    def test_selected_integration_and_osek_services_cannot_disappear(self):
        for key in ["SWS_Rte_08104", "OSEK_SERVICE_WaitEvent"]:
            with self.subTest(key=key):
                matrix = copy.deepcopy(self.matrix)
                matrix["rows"] = [r for r in matrix["rows"] if r["key"] != key]
                with self.assertRaisesRegex(ValueError, "obligation inventory"):
                    baseline.validate_matrix(matrix, self.index, self.tests)

    def test_arti_and_sc1_status_mode_duties_cannot_be_waived(self):
        for key in ["SWS_Os_00838", "SWS_Os_00763", "SWS_Os_00107"]:
            with self.subTest(key=key):
                matrix = copy.deepcopy(self.matrix)
                next(r for r in matrix["rows"] if r["key"] == key)["applicability"] = (
                    "conditional_na"
                )
                with self.assertRaisesRegex(ValueError, "waived|applicability"):
                    baseline.validate_matrix(matrix, self.index, self.tests)

    def test_timing_other_protocol_and_os_oracles_cannot_drift(self):
        xml = (
            baseline.ROOT / "core/tests/fixtures/epic4/positive/ecuc.arxml"
        ).read_bytes()
        for family, case_id, field, value in [
            ("timing_cases", "ncr-on-deadline", "deadline", 211),
            ("cases", "unsupported-did", "expected", []),
            ("os", "activation-limit", "expected_status", "E_OK"),
        ]:
            with self.subTest(case_id=case_id):
                protocol = baseline.load(baseline.ORACLES / "protocol.json")
                os = baseline.load(baseline.ORACLES / "os.json")
                cases = os["cases"] if family == "os" else protocol[family]
                next(c for c in cases if c["id"] == case_id)[field] = value
                with self.assertRaisesRegex(ValueError, "independent oracle changed"):
                    baseline.validate_protocol(protocol, os, xml)

    def test_configured_processing_periods_cannot_diverge_from_oracles(self):
        protocol = baseline.load(baseline.ORACLES / "protocol.json")
        os = baseline.load(baseline.ORACLES / "os.json")
        original = (
            baseline.ROOT / "core/tests/fixtures/epic4/positive/ecuc.arxml"
        ).read_bytes()
        for suffix, container in [
            ("/CanTpMainFunctionPeriod", None),
            ("/ComTxModeTimePeriod", None),
            ("/OsAlarmCycleTime", "Alarm_App"),
            ("/OsSecondsPerTick", "SystemCounter"),
        ]:
            with self.subTest(parameter=suffix):
                root = ET.fromstring(original)
                parent = root
                if container:
                    parent = next(
                        n
                        for n in root.findall(".//a:ECUC-CONTAINER-VALUE", baseline.NS)
                        if n.findtext("a:SHORT-NAME", namespaces=baseline.NS)
                        == container
                    )
                parameter = next(
                    n
                    for n in parent.findall(
                        ".//a:ECUC-NUMERICAL-PARAM-VALUE", baseline.NS
                    )
                    if n.findtext("a:DEFINITION-REF", namespaces=baseline.NS).endswith(
                        suffix
                    )
                )
                parameter.find("a:VALUE", baseline.NS).text = (
                    "0.02" if not container else "20"
                )
                with self.assertRaisesRegex(ValueError, "oracle/config mismatch"):
                    baseline.validate_protocol(protocol, os, ET.tostring(root))

    def test_missing_external_source_is_not_a_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, "not_run"):
                baseline.digest(Path(directory) / "missing.pdf")

    def test_edited_golden_byte_cannot_validate_against_original_contract(self):
        protocol = baseline.load(baseline.ORACLES / "protocol.json")
        case = next(c for c in protocol["cases"] if c["id"] == "initial-did")
        case["expected"][0]["data"] = "0762123400000001"
        os = baseline.load(baseline.ORACLES / "os.json")
        xml = (
            baseline.ROOT / "core/tests/fixtures/epic4/positive/ecuc.arxml"
        ).read_bytes()
        with self.assertRaisesRegex(ValueError, "initial DID literal"):
            baseline.validate_protocol(protocol, os, xml)


if __name__ == "__main__":
    unittest.main()
