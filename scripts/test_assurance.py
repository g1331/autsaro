"""Keep AUTOSAR support claims separate from completed development stories."""

import copy
import json
import unittest

from assurance import LEDGER, ROOT, check_ledger


class AssuranceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ledger = json.loads(LEDGER.read_text(encoding="utf-8"))

    def test_migrated_claims_and_evidence_are_valid(self):
        self.assertEqual(check_ledger(self.ledger, ROOT), [])
        self.assertGreater(len(self.ledger["capabilities"]), 0)

    def test_claim_upgrade_needs_six_gates_and_independent_review(self):
        ledger = copy.deepcopy(self.ledger)
        ledger["capabilities"][0]["claim_level"] = "internal_supported"
        ledger["capabilities"][0]["review"] = {"status": "not_run"}
        for gate in ledger["capabilities"][0]["gates"].values():
            gate["status"] = "not_run"
        errors = check_ledger(ledger, ROOT)
        self.assertTrue(any("blocks internal_supported" in error for error in errors))
        self.assertTrue(any("needs independent review" in error for error in errors))

    def test_passed_gate_without_evidence_is_rejected(self):
        ledger = copy.deepcopy(self.ledger)
        ledger["capabilities"][0]["gates"]["spec_obligations"] = {"status": "passed"}
        self.assertTrue(
            any(
                "passed without evidence" in error
                for error in check_ledger(ledger, ROOT)
            )
        )

    def test_not_applicable_requires_specific_reason(self):
        ledger = copy.deepcopy(self.ledger)
        ledger["capabilities"][0]["gates"]["spec_obligations"] = {
            "status": "not_applicable",
            "reason": None,
        }
        self.assertTrue(
            any("needs a reason" in error for error in check_ledger(ledger, ROOT))
        )


if __name__ == "__main__":
    unittest.main()
