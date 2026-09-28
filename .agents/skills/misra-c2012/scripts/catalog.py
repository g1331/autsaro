"""Validate/query the guideline inventory and produce an unassessed work matrix."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOCKED_DIGEST = "2880302f294925feec62b25d53cd976ff24809710f77c64d993a767072654143"
REVISION_DIGEST = "918701df0498b8b03c4ee6cfc77e15e30d25e642c112b518dfc04a5d32450f2f"
RULE_COUNTS = (5, 8, 2, 2, 9, 3, 6, 17, 7, 8, 10, 6, 6, 4, 7, 7, 13, 10, 2, 14, 26, 20, 8)
DIR_COUNTS = {1: 1, 2: 1, 3: 1, 4: 15, 5: 3}
RESULTS = {"not_assessed", "pass", "violation", "deviation_pending", "deviation_approved", "not_applicable"}


def expected_ids() -> set[str]:
    return {
        *(f"R{chapter}.{number}" for chapter, count in enumerate(RULE_COUNTS, 1) for number in range(1, count + 1)),
        *(f"D{chapter}.{number}" for chapter, count in DIR_COUNTS.items() for number in range(1, count + 1)),
    }


def load_catalog(root: Path = ROOT) -> tuple[dict, list[dict]]:
    """Reject inventory drift, duplicates, mismatched categories and missing guidance."""
    index = json.loads((root / "references/baseline-index.json").read_text(encoding="utf-8"))
    metadata = index["guideline_metadata"]
    ids = [entry["id"] for entry in metadata]
    if len(ids) != 221 or len(set(ids)) != 221 or set(ids) != expected_ids():
        raise ValueError("Baseline identifiers differ from the 200-rule/21-directive inventory")
    digest = hashlib.sha256("\n".join(f"{r['id']}:{r['category']}" for r in metadata).encode()).hexdigest()
    if digest != LOCKED_DIGEST or index["id_category_sha256"] != LOCKED_DIGEST:
        raise ValueError("Official identifier/category fingerprint mismatch")
    if (index["guidelines"], index["rules"], index["directives"]) != (221, 200, 21):
        raise ValueError("Baseline count metadata mismatch")
    if Counter(entry["introduced"] for entry in metadata) != {"2012": 159, "AMD1": 14, "AMD2": 2, "AMD3": 24, "AMD4": 22}:
        raise ValueError("Introduction revision counts differ from the locked baseline")
    revision_digest = hashlib.sha256("\n".join(f"{r['id']}:{r['category']}:{r['introduced']}" for r in metadata).encode()).hexdigest()
    if revision_digest != REVISION_DIGEST:
        raise ValueError("Per-guideline introduction revision mismatch")
    rows = {}
    for path in sorted((root / "references").glob("*.md")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if not re.match(r"^\| \[[DR]\d+\.\d+\]", line):
                continue
            cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
            if len(cells) != 6 or any(not cell for cell in cells):
                raise ValueError(f"Malformed or empty guidance row: {path.name}")
            match = re.fullmatch(r"\[([DR]\d+\.\d+)\]\((https://[^)]+)\)", cells[0])
            if match is None:
                raise ValueError(f"Invalid guideline source: {path.name}")
            ident, url = match.groups()
            if ident in rows:
                raise ValueError(f"Duplicate guidance: {ident}")
            rows[ident] = dict(id=ident, category=cells[1], introduced=cells[2], condition=cells[3], action=cells[4], check=cells[5], reference_url=url, guidance_file=path.relative_to(root).as_posix())
    if set(rows) != expected_ids():
        raise ValueError(f"Guidance coverage mismatch: missing={sorted(expected_ids() - set(rows))}; extra={sorted(set(rows) - expected_ids())}")
    for entry in metadata:
        row = rows[entry["id"]]
        for field in ("category", "introduced", "reference_url", "guidance_file"):
            if row[field] != entry[field]:
                raise ValueError(f"Metadata/guidance mismatch: {entry['id']} {field}")
        if entry["introduced"] not in {"2012", "AMD1", "AMD2", "AMD3", "AMD4"}:
            raise ValueError(f"Invalid introduction revision: {entry['id']}")
    return index, [rows[ident] for ident in ids]


def compare_official(path: Path, rows: list[dict]) -> None:
    """Read the user's local, unmodified official file without redistributing it."""
    source = path.read_text(encoding="utf-8-sig")
    entries = re.findall(r"^(Dir|Rule)\s+(\d+\.\d+)\s+(Mandatory|Required|Advisory)\s*$", source, re.M)
    pairs = [(('D' if kind == 'Dir' else 'R') + number, category) for kind, number, category in entries]
    if len(pairs) != 221 or len(dict(pairs)) != 221:
        raise ValueError("Official file is not the expected complete 221-item edition")
    if dict(pairs) != {row["id"]: row["category"] for row in rows}:
        raise ValueError("Official identifier/category comparison failed")


def create_matrix(rows: list[dict]) -> dict:
    return {
        "baseline": "MISRA C:2012 + AMD1-AMD4 + TC1-TC2",
        "scope": "",
        "target_and_configuration": "",
        "entries": [dict(id=row["id"], category=row["category"], effective_category=row["category"], applicability="undetermined", result="not_assessed", reason="", normative_basis="", evidence=[], approval="") for row in rows],
    }


def check_matrix(matrix: dict, rows: list[dict]) -> list[str]:
    """Check bookkeeping, never infer technical compliance from filled-in strings."""
    entries = matrix.get("entries", [])
    ids = [entry.get("id") for entry in entries]
    if len(ids) != 221 or len(set(ids)) != 221 or set(ids) != expected_ids():
        raise ValueError("Matrix must account for every guideline exactly once")
    if matrix.get("baseline") != "MISRA C:2012 + AMD1-AMD4 + TC1-TC2":
        raise ValueError("Matrix baseline mismatch")
    categories = {row["id"]: row["category"] for row in rows}
    issues = []
    if not matrix.get("scope") or not matrix.get("target_and_configuration"):
        issues.append("Scope or target/configuration is missing")
    allowed = {"Mandatory": {"Mandatory"}, "Required": {"Required", "Mandatory"}, "Advisory": {"Advisory", "Required", "Mandatory", "Disapplied"}}
    for entry in entries:
        ident = entry["id"]
        category = categories[ident]
        effective = entry.get("effective_category")
        result = entry.get("result")
        applicability = entry.get("applicability")
        if entry.get("category") != category or effective not in allowed[category]:
            raise ValueError(f"Invalid classification/re-categorization: {ident}")
        if effective != category and not entry.get("approval"):
            issues.append(f"{ident}: re-categorization approval missing")
        if result not in RESULTS or applicability not in {"undetermined", "applicable", "not_applicable"}:
            raise ValueError(f"Invalid matrix status: {ident}")
        evidence = entry.get("evidence")
        if not isinstance(evidence, list) or any(not isinstance(item, str) or not item.strip() for item in evidence):
            raise ValueError(f"Invalid evidence list: {ident}")
        if result in {"deviation_pending", "deviation_approved"} and effective == "Mandatory":
            raise ValueError(f"Mandatory guideline cannot be deviated: {ident}")
        if applicability == "undetermined" or result == "not_assessed":
            issues.append(f"{ident}: not assessed")
            continue
        if (applicability == "not_applicable") != (result == "not_applicable"):
            raise ValueError(f"Applicability/result conflict: {ident}")
        if not entry.get("normative_basis") or not evidence:
            issues.append(f"{ident}: normative basis or evidence missing")
        if result != "pass" and not entry.get("reason"):
            issues.append(f"{ident}: reason missing")
        if result == "deviation_pending":
            issues.append(f"{ident}: deviation not approved")
        if result == "deviation_approved" and not entry.get("approval"):
            issues.append(f"{ident}: approval evidence missing")
        if result == "violation" and effective in {"Mandatory", "Required"}:
            issues.append(f"{ident}: unresolved {effective} violation")
        if effective == "Disapplied" and not entry.get("approval"):
            issues.append(f"{ident}: disapplication approval missing")
    return issues


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate = sub.add_parser("validate")
    validate.add_argument("--official-file", type=Path)
    query = sub.add_parser("query")
    query.add_argument("--id", action="append", default=[])
    query.add_argument("--group", help="Guidance filename stem, e.g. libraries")
    matrix = sub.add_parser("matrix")
    matrix.add_argument("--output", type=Path, required=True)
    check = sub.add_parser("check-matrix")
    check.add_argument("path", type=Path)
    args = parser.parse_args()
    try:
        index, rows = load_catalog()
        if args.command == "validate":
            if args.official_file:
                compare_official(args.official_file, rows)
            print(f"PASS: {index['rules']} Rules + {index['directives']} Directives; identifiers, categories and guidance accounted for")
        elif args.command == "query":
            if args.id and set(args.id) - expected_ids():
                raise ValueError(f"Unknown guideline identifiers: {sorted(set(args.id) - expected_ids())}")
            selected = [row for row in rows if (not args.id or row["id"] in args.id) and (not args.group or Path(row["guidance_file"]).stem == args.group)]
            if not selected:
                raise ValueError("No guidelines match the requested filter")
            print(json.dumps(selected, ensure_ascii=False, indent=2))
        elif args.command == "matrix":
            # Exclusive creation prevents overwriting a task's existing evidence.
            with args.output.open("x", encoding="utf-8", newline="\n") as output:
                json.dump(create_matrix(rows), output, ensure_ascii=False, indent=2)
                output.write("\n")
            print(f"Created {len(rows)} unassessed entries: {args.output}")
        else:
            issues = check_matrix(json.loads(args.path.read_text(encoding="utf-8")), rows)
            for issue in issues:
                print(issue)
            print("This checks matrix bookkeeping only; evidence still needs technical review.")
            return 1 if issues else 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
