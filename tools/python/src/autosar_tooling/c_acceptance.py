"""Frozen project-source assessments, separate from raw analyzer results."""

from __future__ import annotations

import hashlib
import json
import re
import tempfile
from pathlib import Path

from ecu_tools.build import (
    FILE_HASHES,
    FILE_LIST,
    digest,
    refuse_links,
    require_kernel_identity,
    safe_relative,
)

from autosar_tooling import c_retention, c_standard
from autosar_tooling.config import ROOT

BEGIN = "<!-- source-assessment-context-begin -->"
END = "<!-- source-assessment-context-end -->"
FORMAT = "project-source-acceptance-v1"
SHA256 = re.compile(r"[0-9a-f]{64}")
INCOMPLETE = {"misra-config", "syntaxError", "preprocessorErrorDirective", "internalError",
              "cppcheckError", "addonError", "missingInclude", "missingIncludeSystem", "toomanyconfigs"}
GROUPS = ("generated-retention", "manual-assessment", "checker-limitations")
GUIDELINE_CATEGORIES_SHA256 = "ef484316b09af067e673521476fd7a70f5757646e9c7e3918cdacef75a34bf66"


def _hash(value: object) -> str:
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True,
                                     separators=(",", ":")).encode()).hexdigest()


def _pairs(pairs: list[tuple]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate assessment JSON member: {key}")
        result[key] = value
    return result


def _blocks(text: str) -> list[str]:
    blocks = []
    active = None
    for line in text.splitlines(keepends=True):
        if line.strip() == BEGIN:
            if active is not None:
                raise ValueError("Nested source assessment context")
            active = []
        elif line.strip() == END:
            if active is None:
                raise ValueError("Unopened source assessment context")
            blocks.append("".join(active))
            active = None
        elif active is not None:
            active.append(line)
    if active is not None:
        raise ValueError("Unclosed source assessment context")
    return blocks


def contexts(root: Path = ROOT) -> list[dict]:
    """Only repository standards documents can supply assessment bindings."""
    result = []
    identifiers = set()
    cases = set()
    for path in sorted((root / "docs/standards").rglob("*.md")):
        refuse_links(path, root)
        for block in _blocks(path.read_text(encoding="utf-8")):
            row = json.loads(block, object_pairs_hook=_pairs)
            if (not isinstance(row, dict)
                    or set(row) != {"id", "evidence", "cases", "records", "formalStatus"}
                    or not isinstance(row["id"], str) or not row["id"] or row["id"] in identifiers
                    or not isinstance(row["evidence"], dict)
                    or set(row["evidence"]) != {"head", "receipt"}
                    or not isinstance(row["evidence"]["head"], str)
                    or not re.fullmatch(r"[0-9a-f]{40}", row["evidence"]["head"])
                    or not isinstance(row["evidence"]["receipt"], str) or not row["evidence"]["receipt"]
                    or row["formalStatus"] not in ("pending", "approved")
                    or row["records"] not in ({"kind": "generated-retention", "cohortSha256": c_retention.COHORT_SHA256},
                                              {"kind": "manual-assessment"}, {"kind": "checker-limitations"})
                    or not isinstance(row["cases"], list) or not row["cases"]):
                raise ValueError(f"Invalid source assessment context: {path}")
            identifiers.add(row["id"])
            for case in row["cases"]:
                if (not isinstance(case, dict) or set(case) != {"caseId", "target", "inputIdentity"}
                        or not isinstance(case["caseId"], str) or not case["caseId"]
                        or case["target"] not in tuple(c_retention.TARGETS)
                        or not isinstance(case["inputIdentity"], str)
                        or not SHA256.fullmatch(case["inputIdentity"])):
                    raise ValueError(f"Invalid assessment case: {path}")
                key = row["records"]["kind"], case["target"], case["inputIdentity"]
                if key in cases:
                    raise ValueError(f"Duplicate assessment input binding: {path}")
                cases.add(key)
            result.append({**row, "source": path.relative_to(root).as_posix()})
    return result


def _document_identity(root: Path, names: list[str] | tuple[str, ...]) -> dict:
    documents = {}
    for name in names:
        data = (root / name).read_bytes()
        text = data.decode("utf-8")
        _blocks(text)
        # Binding a document to its own full hash would require a hash fixed point.
        # Exclude only the binding blocks; the assessment text remains byte-bound.
        data = re.sub(rb"(?m)^[ \t]*<!-- source-assessment-context-begin -->[ \t]*\r?\n.*?^[ \t]*<!-- source-assessment-context-end -->[ \t]*(?:\r?\n|$)",
                      b"", data, flags=re.DOTALL)
        documents[name] = hashlib.sha256(data).hexdigest()
    return documents


def _manual_identity(root: Path) -> dict:
    return {"parserSha256": digest(Path(c_standard.__file__)),
            "selectionSha256": hashlib.sha256("\n".join(c_standard.MANUAL_ASSESSMENT_SOURCES).encode()).hexdigest(),
            "documents": _document_identity(root, c_standard.MANUAL_ASSESSMENT_SOURCES)}


def _assessment_inputs(summary: dict, root: Path) -> dict:
    rules = _catalog(summary, root)
    supplementary = c_standard.supplementary_assessment(rules, summary["coverage"]["knownCheckerLimitations"], root)
    retention_documents = [path.relative_to(root).as_posix()
                           for path in sorted((root / c_retention.RECORD_DIRECTORY).glob("*.md"))]
    if not retention_documents:
        raise ValueError("Assessment retention records missing")
    return {"manual": _manual_identity(root),
            "retention": {"parserSha256": digest(Path(c_retention.__file__)),
                          "documents": _document_identity(root, retention_documents)},
            "checkerLimitations": {"parserSha256": digest(Path(c_standard.__file__)),
                                   "documents": _document_identity(root, list(supplementary["inputIdentity"]["documents"]))}}


def input_identity(summary: dict, project: Path, *, root: Path = ROOT) -> str:
    """Hash the analyzed copy, including final patched kernel and application bytes.

    The caller verifies the original seal and semantic inputs before and after analysis.
    This copy has patched kernel bytes, so its original seal cannot be revalidated as-is.
    """
    refuse_links(project)
    names = (project / FILE_LIST).read_text(encoding="utf-8").splitlines()
    if not names or names != sorted(set(names)) or any(not safe_relative(name) for name in names):
        raise ValueError("Invalid assessment source inventory")
    if digest(project / FILE_HASHES) != summary["sourceSealSha256"]:
        raise ValueError("Assessment source seal differs")
    records = {}
    for line in (project / FILE_HASHES).read_text(encoding="utf-8").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", line)
        if match is None or match[2] in records:
            raise ValueError("Invalid assessment source hash record")
        records[match[2]] = match[1]
    if set(records) != set(names) | {FILE_LIST} or digest(project / FILE_LIST) != records[FILE_LIST]:
        raise ValueError("Assessment source hash closure differs")
    actual = {}
    for name in names:
        refuse_links(project / name, project)
        actual[name] = digest(project / name)
        if not name.startswith("kernel/") and actual[name] != records[name]:
            raise ValueError(f"Assessment source bytes differ: {name}")
    present = {path.relative_to(project).as_posix() for path in project.rglob("*") if path.is_file()}
    if present != set(names) | {FILE_LIST, FILE_HASHES}:
        raise ValueError("Assessment analyzed file closure differs")
    target = json.loads((project / "target.json").read_text(encoding="utf-8"))
    if target["target"] != summary["scope"]["target"]:
        raise ValueError("Assessment target differs")
    require_kernel_identity(project / "kernel", target["kernelSha256"])
    metadata = json.loads((project / "meta/integration.json").read_text(encoding="utf-8"))
    plan = metadata["plan"]
    if not plan.get("sources") or not plan.get("runtimeSources"):
        raise ValueError("Assessment plan source/producer provenance missing")
    producers = {}
    for name in plan["runtimeSources"]:
        if not safe_relative(name):
            raise ValueError("Assessment producer path invalid")
        refuse_links(root / name, root)
        producers[name] = digest(root / name) if (root / name).is_file() else None
    semantic = summary["semanticIdentity"]
    if semantic.get("version") != 2 or any(not semantic.get(key) for key in
            ("sdk", "compiler", "defines", "libraries", "analysisCode", "addonRuntime")):
        raise ValueError("Assessment semantic inputs incomplete")
    return _hash({"version": 2, "plan": plan, "target": target,
                  "originalBswSources": metadata["originalBswSources"],
                  "producerInputs": producers,
                  "files": actual, "semanticIdentity": semantic,
                  "tool": {key: summary["tool"][key] for key in ("version", "expectedSourceRevision", "addons")},
                  "assessmentInputs": _assessment_inputs(summary, root),
                  "programs": [{key: program[key] for key in ("name", "entry", "translationUnits")}
                               for program in summary["programs"]],
                  "scope": {key: summary["scope"][key] for key in
                            ("entries", "compilerFlags", "includePaths", "translationUnits", "supplemental", "patchedKernel")}})


def _complete(summary: dict) -> None:
    if summary.get("error") is not None or "error" not in summary:
        raise ValueError(f"Raw analysis failed: {summary.get('error', 'missing result')}")
    if type(summary.get("passed")) is not bool:
        raise ValueError("Raw analysis result missing")
    if not isinstance(summary.get("diagnostics"), list):
        raise TypeError("Raw diagnostic inventory missing or malformed")
    programs = summary["programs"]
    if (not isinstance(programs, list) or not programs
            or len({row["name"] for row in programs}) != len(programs)
            or any(type(row["exitCode"]) is not int or row["exitCode"] not in (0, 1)
                   or type(row["passed"]) is not bool or not row["translationUnits"] for row in programs)):
        raise ValueError("Analysis program results incomplete")
    if any(row["id"] in INCOMPLETE for row in summary["diagnostics"]):
        raise ValueError("Analyzer configuration or parsing incomplete")
    if summary["scope"]["target"] not in c_retention.TARGETS:
        raise ValueError("Analysis target unsupported")
    if summary["passed"] != all(row["passed"] for row in programs):
        raise ValueError("Raw program and aggregate results disagree")
    entries = summary["scope"]["entries"]
    units = summary["scope"]["translationUnits"]
    if (not isinstance(entries, dict) or not entries
            or {row["name"] for row in programs} != set(entries)
            or not isinstance(units, list) or len(units) != len(set(units))
            or any(not safe_relative(name) for name in units)):
        raise ValueError("Analysis program scope incomplete")
    for row in programs:
        expected_units = {name for name in units if name not in entries.values() or name == row["entry"]}
        if (row["entry"] != entries[row["name"]] or set(row["translationUnits"]) != expected_units
                or len(row["translationUnits"]) != len(expected_units)):
            raise ValueError("Analysis program translation-unit closure differs")
    for diagnostic in summary["diagnostics"]:
        if not diagnostic["programs"] or not set(diagnostic["programs"]) <= set(entries):
            raise ValueError("Diagnostic program membership incomplete")


def _catalog(summary: dict, root: Path) -> list[dict]:
    from autosar_tooling import c_check

    catalog = json.loads((root / ".agents/skills/misra-c2012/references/baseline-index.json").read_text(encoding="utf-8"))["guideline_metadata"]
    if len(catalog) != 221 or _hash({row["id"]: row["category"] for row in catalog}) != GUIDELINE_CATEGORIES_SHA256:
        raise ValueError("Unsupported fixed guideline categories")
    rules = summary["coverage"]["rules"]
    if (len(rules) != 221 or len({row["id"] for row in rules}) != 221
            or {(row["id"], row["category"]) for row in rules} != {(row["id"], row["category"]) for row in catalog}
            or any(type(row["automaticChecker"]) is not bool for row in rules)):
        raise ValueError("Assessment requires the complete fixed 221-guideline catalog")
    if ({row["id"] for row in rules if not row["automaticChecker"]} != c_standard.CHECKER_UNCOVERED
            or summary["tool"]["version"] != c_check.CPPCHECK_VERSION
            or summary["tool"]["expectedSourceRevision"] != c_check.CPPCHECK_REVISION
            or summary["tool"]["addons"] != c_check.ADDON_HASHES):
        raise ValueError("Assessment checker mapping differs from pinned 2.21.0 and its 66 uncovered guidelines")
    return rules


def _reference(context: dict) -> dict:
    return {key: context[key] for key in ("id", "source", "evidence", "records")}


def _positions(diagnostics: list[dict]):
    for index, diagnostic in enumerate(diagnostics):
        if diagnostic["id"] == "checkersReport":
            continue
        for position, location in enumerate(diagnostic["locations"] or [None]):
            yield index, position, diagnostic, location


def evaluate(summary: dict, project: Path, *, root: Path = ROOT) -> dict:
    """Assess three independent frozen record groups, preserving raw analysis."""
    result = {"format": FORMAT, "status": "blocked", "target": summary.get("scope", {}).get("target"),
              "caseId": None, "inputIdentity": None, "contexts": [], "assessmentInputs": None,
              "objects": [], "occurrences": []}

    def add(kind, identifier, reason, **details):
        result["objects"].append({"kind": kind, "id": identifier, "reason": reason, **details})

    try:
        _complete(summary)
        identity = input_identity(summary, project, root=root)
        result["inputIdentity"] = identity
        result["assessmentInputs"] = _assessment_inputs(summary, root)
        selected = {context["records"]["kind"]: (context, case)
                    for context in contexts(root) for case in context["cases"]
                    if case["target"] == result["target"] and case["inputIdentity"] == identity}
        case_ids = {case["caseId"] for _, case in selected.values()}
        if len(case_ids) == 1:
            result["caseId"] = next(iter(case_ids))
        group_reasons = {}
        for group in GROUPS:
            if group not in selected:
                group_reasons[group] = "input_context_unbound_or_stale"
            else:
                context, _case = selected[group]
                result["contexts"].append(_reference(context))
                if len(case_ids) != 1:
                    group_reasons[group] = "assessment_case_mismatch"
                elif context["formalStatus"] != "approved":
                    group_reasons[group] = "formal_deviation_pending" if group == "generated-retention" else "formal_assessment_pending"
            if group in group_reasons:
                add("context", group, group_reasons[group])
        metadata = json.loads((project / "meta/integration.json").read_text(encoding="utf-8"))
        for name, expected in metadata["plan"]["runtimeSources"].items():
            if not (root / name).is_file() or digest(root / name) != expected:
                add("producer", name, "producer_provenance_unverified")
        rules = _catalog(summary, root)
        manual = c_standard.manual_assessment(rules, root)
        if summary["coverage"]["manualAssessment"] != manual:
            add("manual", "inputIdentity", "manual_records_changed_since_analysis")
        supplementary = c_standard.supplementary_assessment(rules, summary["coverage"]["knownCheckerLimitations"], root)
        for group, kind, rows in (("manual-assessment", "manual", manual["checkerUncovered"]["records"]),
                                  ("checker-limitations", "checker-limitation", supplementary["records"])):
            for row in rows:
                reason = kind + "_" + row["status"] if row["status"] not in {"satisfied", "not_applicable", "retained"} else group_reasons.get(group)
                if reason:
                    add(kind, row["id"], reason, source=row.get("source"))
        if summary.get("sourceClassification", {}).get("complete") is not True:
            add("source-classification", "sdk", "sdk_source_or_macro_provenance_incomplete")
        retention = c_retention.analyze(summary["diagnostics"], project, result["target"])
        occurrences = [row for row in retention["occurrences"] if row["rule"] != "checkersReport"]
        positions = list(_positions(summary["diagnostics"]))
        if len(occurrences) != len(positions):
            raise ValueError("Retention occurrence coverage incomplete")
        profile = next((key for key, name in c_retention.PROFILES.items() if name == result["caseId"]), None)
        for occurrence, (index, position, diagnostic, location) in zip(occurrences, positions):
            if occurrence["diagnostic"] != index or occurrence["location"] != location:
                raise ValueError("Retention occurrence position differs")
            if occurrence["status"] != "record_matched":
                reason = occurrence["reason"]
                if diagnostic.get("sourceCategory") == "sdk":
                    reason = "sdk_integration_assessment_missing"
                elif diagnostic.get("adopted"):
                    reason = "adopted_source_assessment_missing"
            elif "generated-retention" in group_reasons:
                reason = group_reasons["generated-retention"]
            elif profile not in occurrence["recordedProfiles"].split(","):
                reason = "retention_case_outside_record"
            else:
                reason = None
            occurrence.update(locationIndex=position, diagnosticIdentity=_hash(diagnostic), profile=profile,
                              contextId=selected["generated-retention"][0]["id"] if "generated-retention" in selected else None,
                              evidenceClosure="not_established" if reason else "frozen_context",
                              assessment="blocked" if reason else "accepted", reason=reason or "retention_approved")
            result["occurrences"].append(occurrence)
            if reason:
                add("diagnostic", index, reason, rule=occurrence["rule"], location=location,
                    entity=occurrence["entity"], programs=occurrence["programs"], record=occurrence["record"])
        result["status"] = "blocked" if result["objects"] else "accepted"
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        result["status"] = "error"
        add("execution", "assessment", str(error))
    return result


def require_accepted(analyses: list[tuple[dict, Path]], expected: list[tuple[str, str]], upstream: dict[str, str], *, root: Path = ROOT) -> None:
    """Require the exact target/case matrix and successful prepare/analysis jobs.

    Raw passed=false and diagnostic exit code 1 are independent of project acceptance.
    Missing, skipped and cancelled upstream jobs are never successful analysis.
    """
    if not isinstance(upstream, dict) or not {"prepare", "analysis"} <= upstream.keys() or any(value != "success" for value in upstream.values()):
        raise ValueError("Project acceptance requires successful prepare and analysis upstream results")
    if not isinstance(expected, list) or any(not isinstance(row, tuple) or len(row) != 2
                                           or any(not isinstance(value, str) or not value for value in row) for row in expected):
        raise ValueError("Expected project acceptance matrix malformed")
    wanted = set(expected)
    if not wanted or len(wanted) != len(expected):
        raise ValueError("Expected project acceptance matrix is empty or duplicated")
    if not isinstance(analyses, list) or not analyses:
        raise ValueError("Complete analysis projects are required")
    from autosar_tooling import c_check

    found = set()
    with tempfile.TemporaryDirectory(prefix="c-acceptance-") as temporary:
        tool = c_check.tool_identity(Path(temporary))
        for analysis in analyses:
            try:
                if not isinstance(analysis, tuple) or len(analysis) != 2 or not isinstance(analysis[1], Path):
                    raise ValueError("Each analysis must contain its summary and complete patched project path")
                summary, project = analysis
                c_check.recheck_semantic_inputs(summary, project, tool)
                current = evaluate(summary, project, root=root)
                if current["status"] != "accepted":
                    reasons = "; ".join(dict.fromkeys(row["reason"] for row in current["objects"]))
                    raise ValueError(f"Project acceptance refused for actual analysis project and inputs: {reasons}")
                if _hash(current) != _hash(summary["projectAcceptance"]):
                    raise ValueError("Project acceptance receipt no longer matches the actual analysis project and inputs")
                key = current["target"], current["caseId"]
                if key not in wanted or key in found:
                    raise ValueError(f"Project acceptance identity unexpected or duplicated: {key}")
                found.add(key)
            except (OSError, KeyError, TypeError, AttributeError) as error:
                raise ValueError("Project acceptance summary missing or malformed") from error
    if found != wanted:
        raise ValueError(f"Project acceptance matrix incomplete: {sorted(wanted - found)}")
