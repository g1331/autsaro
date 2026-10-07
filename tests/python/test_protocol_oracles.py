"""Protect independent reference bytes, deadlines and processing periods."""

import unittest
import xml.etree.ElementTree as ET

from autosar_tooling import protocol_oracles as baseline


class OracleTests(unittest.TestCase):

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
