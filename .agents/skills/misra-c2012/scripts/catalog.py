"""Validate and query the MISRA coding guideline inventory."""

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






def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate = sub.add_parser("validate")
    validate.add_argument("--official-file", type=Path)
    query = sub.add_parser("query")
    query.add_argument("--id", action="append", default=[])
    query.add_argument("--group", help="Guidance filename stem, e.g. libraries")
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
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
