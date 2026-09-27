"""Validate capability claims independently of BMad delivery status."""

from __future__ import annotations

import json
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LEDGER = ROOT / "docs/assurance/capabilities.json"
GATES = (
    "input_roundtrip",
    "artifact_closure",
    "build_static",
    "independent_behavior",
    "user_workflow",
    "spec_obligations",
)
GATE_STATUSES = {"not_run", "failed", "pending_review", "passed", "not_applicable"}


def repository_file(root: Path, value: object) -> bool:
    if not isinstance(value, str) or not value or "\\" in value:
        return False
    path = Path(value)
    return (
        not (path.is_absolute() or path.drive or ".." in path.parts)
        and (root / path).is_file()
    )


def evidence_file(root: Path, value: object) -> bool:
    return repository_file(root, value) and str(value).startswith(
        "docs/workflow/evidence/"
    )


def check_ledger(ledger: object, root: Path) -> list[str]:
    errors: list[str] = []
    if not isinstance(ledger, dict) or ledger.get("schema_version") != 1:
        return ["capability ledger requires schema_version 1"]
    capabilities = ledger.get("capabilities")
    if not isinstance(capabilities, list):
        return ["capabilities must be an array"]
    seen: set[str] = set()
    for index, capability in enumerate(capabilities):
        label = f"capabilities[{index}]"
        if not isinstance(capability, dict):
            errors.append(f"{label} must be an object")
            continue
        identifier = capability.get("id")
        if not isinstance(identifier, str) or not identifier or identifier in seen:
            errors.append(f"{label}.id is missing or duplicated")
        else:
            seen.add(identifier)
            label = identifier
        for key in ("name", "release", "configuration", "target", "next_gap"):
            if not isinstance(capability.get(key), str) or not capability[key].strip():
                errors.append(f"{label}.{key} is required")
        if not repository_file(root, capability.get("documented_in")):
            errors.append(f"{label}.documented_in must point to a file")
        claim = capability.get("claim_level")
        if claim not in {"documented_behavior", "internal_supported"}:
            errors.append(f"{label}.claim_level is invalid")
        review = capability.get("review")
        if not isinstance(review, dict) or review.get("status") not in {
            "not_run",
            "pending_review",
            "failed",
            "passed",
        }:
            errors.append(f"{label}.review.status is invalid")
            review = {}
        if review.get("status") == "passed":
            if not evidence_file(root, review.get("evidence")):
                errors.append(f"{label}.review requires evidence")
            if (
                not isinstance(review.get("reviewer"), str)
                or not review["reviewer"].strip()
            ):
                errors.append(f"{label}.review requires reviewer")
            try:
                date.fromisoformat(review["date"])
            except (KeyError, TypeError, ValueError):
                errors.append(f"{label}.review requires ISO date")
        gates = capability.get("gates")
        if not isinstance(gates, dict) or set(gates) != set(GATES):
            errors.append(f"{label}.gates must contain exactly six gates")
            continue
        for gate_name, gate in gates.items():
            gate_label = f"{label}.{gate_name}"
            if not isinstance(gate, dict) or gate.get("status") not in GATE_STATUSES:
                errors.append(f"{gate_label}.status is invalid")
                continue
            if gate["status"] == "passed" and not evidence_file(
                root, gate.get("evidence")
            ):
                errors.append(f"{gate_label} passed without evidence")
            if gate["status"] == "not_applicable" and (
                not isinstance(gate.get("reason"), str) or not gate["reason"].strip()
            ):
                errors.append(f"{gate_label} needs a reason")
            if claim == "internal_supported" and gate["status"] not in {
                "passed",
                "not_applicable",
            }:
                errors.append(f"{gate_label} blocks internal_supported")
        if claim == "internal_supported" and review.get("status") != "passed":
            errors.append(f"{label} needs independent review before internal_supported")
    return errors


def main() -> int:
    if sys.platform == "win32":
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    try:
        ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"Cannot read capability ledger: {error}", file=sys.stderr)
        return 1
    errors = check_ledger(ledger, ROOT)
    for error in errors:
        print(f"Assurance error: {error}", file=sys.stderr)
    if errors:
        return 1
    print(
        f"Validated {len(ledger['capabilities'])} capability records; claim gates unchanged."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
