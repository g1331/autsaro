"""Guard the workflow against unsupported claims and ambiguous task ownership."""

import copy
import json
import unittest

from workflow import FEEDBACK, ROOT, STATE, check_feedback, check_state, feedback_review_reasons


class WorkflowStateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.current_state = json.loads(STATE.read_text(encoding="utf-8"))
        cls.current_feedback = json.loads(FEEDBACK.read_text(encoding="utf-8"))
        cls.baseline = copy.deepcopy(cls.current_state)
        cls.baseline["capabilities"] = cls.baseline["capabilities"][:2]
        cls.baseline["tasks"] = cls.baseline["tasks"][:2]
        cls.baseline["capabilities"][0]["claim_level"] = "documented_behavior"
        cls.baseline["capabilities"][0]["review"] = {"status": "not_run"}
        cls.baseline["capabilities"][0]["gates"] = {
            name: {"status": "not_run"} for name in cls.baseline["capabilities"][0]["gates"]
        }
        cls.baseline["tasks"][0].update(status="ready", evidence=[])
        cls.baseline["tasks"][0].pop("review", None)
        cls.baseline["tasks"][1]["status"] = "proposed"
        cls.feedback = {"schema_version": 1, "reviewed_done_task_ids": [], "items": []}

    def test_current_state_is_valid(self):
        self.assertEqual(check_state(self.current_state, ROOT), [])
        self.assertEqual(check_feedback(self.current_feedback, self.current_state, ROOT), [])

    def test_internal_support_requires_evidence_and_review(self):
        state = copy.deepcopy(self.baseline)
        state["capabilities"][0]["claim_level"] = "internal_supported"
        errors = check_state(state, ROOT)
        self.assertTrue(any("blocks the internal_supported claim" in error for error in errors))
        self.assertTrue(any("needs independent review" in error for error in errors))

    def test_two_active_tasks_are_rejected(self):
        state = copy.deepcopy(self.baseline)
        for task in state["tasks"]:
            task.update(status="active", branch="audit", base_commit="example")
        state["tasks"][1]["depends_on"] = []
        self.assertIn("at most one task may be active", check_state(state, ROOT))

    def test_done_audit_needs_evidence_and_independent_review(self):
        state = copy.deepcopy(self.baseline)
        state["tasks"][0]["status"] = "done"
        errors = check_state(state, ROOT)
        self.assertTrue(any("without evidence" in error for error in errors))
        self.assertTrue(any("without independent review" in error for error in errors))

    def test_malformed_status_is_reported_without_crashing(self):
        state = copy.deepcopy(self.baseline)
        state["tasks"][0]["status"] = ["active"]
        self.assertTrue(any("status is invalid" in error for error in check_state(state, ROOT)))

    def test_repeated_feedback_triggers_review_without_changing_task_status(self):
        feedback = copy.deepcopy(self.feedback)
        item = {
            "id": "WF-TEST-001",
            "status": "observed",
            "severity": "normal",
            "problem": "A handoff lost its input path",
            "impact": "The next agent could not reproduce the result",
            "observations": [{"task_id": "AUDIT-001", "detail": "First handoff lacked the input path"}],
        }
        feedback["items"].append(item)
        self.assertEqual(check_feedback(feedback, self.baseline, ROOT), [])
        self.assertEqual(feedback_review_reasons(feedback, self.baseline), [])
        item["observations"].append({"task_id": "AUDIT-001", "detail": "Second handoff repeated it"})
        self.assertTrue(any("WF-TEST-001" in reason for reason in feedback_review_reasons(feedback, self.baseline)))

    def test_trial_needs_two_later_tasks_and_an_observable_condition(self):
        state = copy.deepcopy(self.baseline)
        feedback = copy.deepcopy(self.feedback)
        feedback["items"].append({
            "id": "WF-002",
            "status": "trial",
            "severity": "normal",
            "problem": "Handoff lacks input paths",
            "impact": "Review cannot reproduce",
            "observations": [{"task_id": "AUDIT-001", "detail": "Missing input path"}],
            "change": "Require input paths in the handoff; revert that template edit if ineffective",
            "success_condition": "Two later handoffs include reproducible input paths",
            "change_files": ["docs/workflow/evidence/TEMPLATE.md"],
            "baseline_done_task_ids": [],
        })
        self.assertEqual(check_feedback(feedback, state, ROOT), [])
        state["tasks"][0]["status"] = "done"
        self.assertEqual(feedback_review_reasons(feedback, state), [])
        state["tasks"][1]["status"] = "done"
        self.assertTrue(any("WF-002" in reason for reason in feedback_review_reasons(feedback, state)))
        feedback["reviewed_done_task_ids"] = ["AUDIT-001", "AUDIT-002"]
        self.assertEqual(feedback_review_reasons(feedback, state), [])
        state["tasks"].append({"id": "BUILD-001", "status": "done"})
        self.assertEqual(feedback_review_reasons(feedback, state), [])
        state["tasks"].append({"id": "BUILD-002", "status": "done"})
        self.assertTrue(any("WF-002" in reason for reason in feedback_review_reasons(feedback, state)))
        del feedback["items"][-1]["success_condition"]
        self.assertTrue(any("success_condition" in error for error in check_feedback(feedback, state, ROOT)))

    def test_periodic_review_is_due_after_three_completed_tasks(self):
        state = copy.deepcopy(self.baseline)
        for task in state["tasks"]:
            task["status"] = "done"
        state["tasks"].append({"id": "AUDIT-003", "status": "done"})
        reasons = feedback_review_reasons(self.feedback, state)
        self.assertTrue(any("3 张任务卡" in reason for reason in reasons))


if __name__ == "__main__":
    unittest.main()
