"""Presence checks for test design; semantic adequacy needs review."""

from dataclasses import dataclass


@dataclass
class CheckResult:
    rating: str
    finding: str = ""


CHECKS = {
    "TC001": {"measure": "Nonempty unique test IDs", "category": "Design"},
    "TC002": {
        "measure": "Titles present (semantic clarity requires review)",
        "category": "Design",
    },
    "TC003": {"measure": "Preconditions present", "category": "Design"},
    "TC004": {
        "measure": "Inputs present (types/ranges require review)",
        "category": "Design",
    },
    "TC005": {
        "measure": "Expected outputs present (objectivity requires review)",
        "category": "Design",
    },
    "TC006": {
        "measure": "Pass criteria present (measurability requires review)",
        "category": "Design",
    },
    "TC007": {
        "measure": "Requirement references present (validity requires review)",
        "category": "Traceability",
    },
    "TC008": {
        "measure": "Bidirectional coverage needs source requirements",
        "category": "Traceability",
    },
    "TC009": {
        "measure": "Orphan requirements need source requirements",
        "category": "Traceability",
    },
    "TC010": {
        "measure": "Orphan test references need source requirements",
        "category": "Traceability",
    },
    "TC011": {"measure": "Test levels present", "category": "Levels"},
    "TC012": {"measure": "Component allocation requires review", "category": "Levels"},
    "TC013": {"measure": "Automation status present", "category": "Automation"},
    "TC014": {
        "measure": "Automation tool allocation requires review",
        "category": "Automation",
    },
    "TC015": {"measure": "Automation date requires evidence", "category": "Automation"},
    "TC016": {"measure": "Execution priority present", "category": "Priority"},
    "TC017": {"measure": "Execution sequence requires review", "category": "Priority"},
    "TC018": {"measure": "Priority rationale requires review", "category": "Priority"},
    "TC019": {"measure": "Environment present", "category": "Preconditions"},
    "TC020": {
        "measure": "Boundary coverage requires design review",
        "category": "Design",
    },
    "TC021": {
        "measure": "Error/recovery adequacy requires design review",
        "category": "Design",
    },
    "TC022": {"measure": "Acceptance margins require review", "category": "Design"},
    "TC023": {"measure": "Verification method requires review", "category": "Methods"},
    "TC024": {
        "measure": "Conflicting cases require semantic review",
        "category": "Quality",
    },
    "TC025": {
        "measure": "Test count matches scope requires source review",
        "category": "Metrics",
    },
    "TC026": {
        "measure": "Automation ratio target requires project decision",
        "category": "Metrics",
    },
    "TC027": {"measure": "Document control requires review", "category": "Control"},
    "TC028": {
        "measure": "Traceability report requires source review",
        "category": "Reporting",
    },
}


def evaluate_tc_catalog(probed):
    cases = probed.test_cases
    results = {
        key: {
            "rating": "NA",
            "finding": "DRAFT - not machine-assessed; source/design review required",
            "evidence": "DRAFT",
        }
        for key in CHECKS
    }
    if not cases:
        for value in results.values():
            value["finding"] = "Not assessed: no test cases detected"
        return results
    ids = [tc.id for tc in cases]
    unique = all(ids) and len(ids) == len(set(ids))
    results["TC001"] = {
        "rating": "FC" if unique else "NO",
        "finding": "CONTENT - IDs checked"
        if unique
        else "CONTENT - empty or duplicate test IDs",
        "evidence": "CONTENT",
    }
    fields = {
        "TC002": "title",
        "TC003": "precondition",
        "TC004": "inputs",
        "TC005": "expected_outputs",
        "TC006": "pass_criterion",
        "TC007": "req_id",
        "TC011": "test_level",
        "TC013": "automation",
        "TC016": "priority",
        "TC019": "environment",
    }
    for key, attribute in fields.items():
        missing = [
            tc.id or "(empty ID)" for tc in cases if not getattr(tc, attribute).strip()
        ]
        results[key] = {
            "rating": "NO" if missing else "LC",
            "finding": f"STRUCTURE - missing {attribute}: {', '.join(missing)}"
            if missing
            else f"STRUCTURE - {attribute} present; meaning not assessed",
            "evidence": "STRUCTURE",
        }
    return results
