"""Match fixed generated-retention records, without granting a deviation."""

from __future__ import annotations

import collections
import hashlib
import json
import re
from pathlib import Path

from autosar_tooling.c_standard import code_words, is_comment, lex
from autosar_tooling.config import ROOT

PROFILES = {"S": "standard-ecu", "U": "user-application",
            "C": "component-pipeline", "N": "no-diagnostic"}
COHORT_SHA256 = "f300270c060c15fe7c9d449e1e38a6a7994c02b12280578acc8cd8956e500224"
TARGETS = {"linux-x64-controlled-v1", "windows-x64-controlled-v1"}
RECORD_DIRECTORY = "docs/standards/rte/generated-retention"
COHORT_FIELDS = ("rule", "name", "source", "delivered", "shape", "profiles", "producer")


def records() -> list[dict]:
    rows = []
    for path in sorted((ROOT / RECORD_DIRECTORY).glob("*.md")):
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            cells = line.split("|")
            if len(cells) < 6 or not (rule := re.search(r"MISRA C:2012 Rule (\d+\.\d+)", cells[1])):
                continue
            entities = re.findall(r"`([^`]+)`", cells[2])
            if len(entities) != 2:
                raise ValueError(f"Invalid retention source/entity: {path}:{number}")
            source, name = entities
            details = cells[3]
            fields = {}
            for field, pattern in (("delivered", r"delivered=`([^`]+)`"),
                                   ("shape", r"shape=`([^`]+)`"),
                                   ("profiles", r"profiles=([SUCN,]+)(?:;|$)"),
                                   ("producer", r"producer=(.+)$")):
                match = re.search(pattern, details.strip())
                if match is None:
                    raise ValueError(f"Invalid retention {field}: {path}:{number}")
                fields[field] = match[1].strip()
            rows.append({
                "rule": rule[1], "name": name, "source": source,
                "delivered": fields["delivered"], "shape": fields["shape"].replace("&#124;", "|"),
                "profiles": fields["profiles"], "producer": fields["producer"],
                "record": {"file": path.relative_to(ROOT).as_posix(), "line": number},
                "decisionSource": cells[4].strip(),
            })
    check_cohort(rows)
    return rows


def check_cohort(rows: list[dict]) -> None:
    ordered = sorted(({key: row[key] for key in COHORT_FIELDS} for row in rows),
                     key=lambda row: (row["rule"], row["delivered"], row["name"]))
    digest = hashlib.sha256(json.dumps(ordered, ensure_ascii=False, sort_keys=True,
                                      separators=(",", ":")).encode()).hexdigest()
    if (digest != COHORT_SHA256 or len(rows) != 160
            or collections.Counter(row["rule"] for row in rows) != {"2.5": 152, "2.3": 4, "8.7": 3, "11.4": 1}
            or [sum(p in row["profiles"].split(",") for row in rows) for p in PROFILES] != [154, 154, 157, 157]):
        raise ValueError("retention cohort identity differs from the specific decision")


def declaration_span(source: str, name: str, macro: bool, tokens=None) -> tuple[str, str, int, int]:
    """Return the complete declarator/replacement list, comments and text offsets."""
    all_tokens = lex(source) if tokens is None else tokens
    if macro:
        found = [t for t in all_tokens
                 if re.match(r"\s*#\s*define\s+" + re.escape(name) + r"(?!\w)", t.text)]
        if len(found) != 1:
            raise ValueError(name + ": declaration count")
        token = found[0]
        start, end, text = token.start, token.end, token.text
    else:
        ts = [t for t in all_tokens if not is_comment(t)]
        found = []
        for i, token in enumerate(ts):
            if token.text != name:
                continue
            a = i
            while a and ts[a - 1].text not in {";", "{", "}"} and not ts[a - 1].text.lstrip().startswith("#"):
                a -= 1
            b = i
            while b < len(ts) and ts[b].text not in {";", "{"}:
                b += 1
            if b == len(ts) or "=" in [t.text for t in ts[a:i]]:
                continue
            candidate = ts[a:b + 1]
            if candidate[0].text in {"const", "typedef", "RTE_CODE"}:
                found.append((candidate[0].start, candidate[-1].end, " ".join(t.text for t in candidate)))
        if len(found) != 1:
            raise ValueError(name + ": declaration count")
        start, end, text = found[0]
    comments = []
    for token in reversed([t for t in all_tokens if t.end <= start]):
        if not is_comment(token):
            break
        comments.append(token.text)
    return text, "\n".join(comments), start, end


def containing_function(tokens, offset: int) -> str | None:
    depth = 0
    owner = None
    preceding = []
    for token in tokens:
        if token.start >= offset:
            break
        if is_comment(token) or token.text.lstrip().startswith("#"):
            continue
        if token.text == "{":
            if depth == 0:
                match = re.search(r"\b(\w+)\s*\([^;{}]*\)\s*$", " ".join(preceding))
                owner = match[1] if match else None
            depth += 1
        elif token.text == "}":
            depth -= 1
            if depth == 0:
                owner = None
                preceding = []
        elif token.text == ";" and depth == 0:
            preceding = []
        else:
            preceding.append(token.text)
    return owner


def check_project(project: Path, rows: list[dict], overrides: dict[str, str] | None = None) -> list[tuple]:
    sources = {p.relative_to(project).as_posix(): p.read_text(encoding="utf-8")
               for p in (project / "generated").rglob("*") if p.suffix in {".c", ".h"}}
    sources.update(overrides or {})
    tokenized = {}
    failures = []
    for row in rows:
        name = row["delivered"]
        if name not in sources:
            failures.append((name, row["name"], "declaration"))
            continue
        source = sources[name]
        if name not in tokenized:
            tokenized[name] = lex(source)
        try:
            actual, comments, _, _ = declaration_span(source, row["name"], row["rule"] == "2.5", tokenized[name])
            if code_words(actual) != code_words(row["shape"]):
                failures.append((name, row["name"], "shape"))
            if f"MR12 RULE {row['rule']} VIOLATION:" not in comments:
                failures.append((name, row["name"], "marker"))
        except ValueError:
            failures.append((name, row["name"], "declaration"))
    allowed = {(r["rule"], r["delivered"], tuple(code_words(r["shape"]))) for r in records()}
    pattern = r"MR12 RULE (2\.3|2\.5|8\.7|11\.4) VIOLATION:\s*(?:retain|observe)\b"
    for name, source in sources.items():
        if re.search(pattern, source) is None:
            continue
        if name not in tokenized:
            tokenized[name] = lex(source)
        tokens = tokenized[name]
        for index, token in enumerate(tokens):
            retained = re.search(pattern, token.text) if is_comment(token) else None
            if retained is None:
                continue
            owner = []
            for following in tokens[index + 1:]:
                if is_comment(following):
                    continue
                owner.append(following.text)
                if following.text in {";", "{"} or following.text.lstrip().startswith("#"):
                    break
            shape = " ".join(owner)
            if (retained[1], name, tuple(code_words(shape))) not in allowed:
                macro = re.match(r"\s*#\s*define\s+(\w+)", shape)
                failures.append((name, macro[1] if macro else "declaration", "cohort"))
    return failures


def analyze(diagnostics: list[dict], project: Path, target: str) -> dict:
    """Consume every occurrence without mutating raw diagnostics or claiming approval."""
    rows = records()
    candidates = collections.defaultdict(list)
    for row in rows:
        candidates[("misra-c2012-" + row["rule"], row["delivered"])].append(row)
    sources = {}
    occurrences = []
    for index, diagnostic in enumerate(diagnostics):
        programs = diagnostic.get("programs", [])
        for location in diagnostic.get("locations", []) or [None]:
            occurrence = {"diagnostic": index, "rule": diagnostic.get("id"), "location": location,
                          "programs": list(programs), "status": "unmatched", "entity": None,
                          "record": None, "reason": "no_record", "evidenceClosure": "not_established"}
            occurrences.append(occurrence)
            if target not in TARGETS:
                occurrence["reason"] = "target_outside_record"
                continue
            if not programs or not set(programs) <= {"host-batch", "probe"}:
                occurrence["reason"] = "program_outside_record"
                continue
            if location is None:
                occurrence["reason"] = "location_missing"
                continue
            available = candidates.get((diagnostic.get("id"), location["file"]), [])
            if not available:
                continue
            name = location["file"]
            if name not in sources:
                path = project / name
                if path.is_file():
                    text = path.read_text(encoding="utf-8")
                    sources[name] = (text, lex(text))
                else:
                    sources[name] = None
            if sources[name] is None:
                occurrence["reason"] = "source_missing"
                continue
            source, tokens = sources[name]
            line = location.get("line")
            if line is None or not str(line).isdigit() or int(line) < 1:
                occurrence["reason"] = "location_invalid"
                continue
            occurrence["reason"] = "entity_location_unmatched"
            for row in available:
                try:
                    actual, comments, start, end = declaration_span(source, row["name"], row["rule"] == "2.5", tokens)
                except ValueError:
                    continue
                if not source.count("\n", 0, start) + 1 <= int(line) <= source.count("\n", 0, end - 1) + 1:
                    continue
                column = location.get("column")
                if column not in (None, "0", 0):
                    if not str(column).isdigit():
                        continue
                    offset = sum(len(part) for part in source.splitlines(keepends=True)[:int(line) - 1]) + int(column) - 1
                    if not start <= offset < end:
                        continue
                occurrence.update(entity=row["name"], record=row["record"], source=row["source"],
                                  producer=row["producer"], recordedProfiles=row["profiles"],
                                  decisionSource=row["decisionSource"])
                if code_words(actual) != code_words(row["shape"]):
                    occurrence["reason"] = "shape_mismatch"
                elif row["rule"] == "11.4" and containing_function(tokens, start) != "Os_ArtiAlarmBaseReturn":
                    occurrence["reason"] = "function_mismatch"
                elif f"MR12 RULE {row['rule']} VIOLATION:" not in comments:
                    occurrence["reason"] = "marker_mismatch"
                else:
                    occurrence.update(status="record_matched", reason="shape_and_marker_match_only")
                break
    return {"status": "partial", "evidenceClosure": "not_established", "cohortSha256": COHORT_SHA256,
            "profileVerification": "not_established",
            "unclosedEvidence": ["recorded_profile", "producer_and_source_closure", "formal_deviation"],
            "occurrences": occurrences}
