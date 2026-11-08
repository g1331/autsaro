"""Frozen project-source assessments, separate from raw analyzer results."""

from __future__ import annotations

import hashlib
import json
import re
import shutil
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

from autosar_tooling import c_retention, c_sdk, c_standard
from autosar_tooling.config import ROOT

BEGIN = "<!-- source-assessment-context-begin -->"
END = "<!-- source-assessment-context-end -->"
FORMAT = "project-source-acceptance-v1"
SHA256 = re.compile(r"[0-9a-f]{64}")
INCOMPLETE = {"misra-config", "syntaxError", "preprocessorErrorDirective", "internalError",
              "cppcheckError", "addonError", "missingInclude", "missingIncludeSystem", "toomanyconfigs"}
GROUPS = ("generated-retention", "manual-assessment", "checker-limitations")
GUIDELINE_CATEGORIES_SHA256 = "ef484316b09af067e673521476fd7a70f5757646e9c7e3918cdacef75a34bf66"
GRP = "docs/adr/0025-misra-layered-scope.md"
ADOPTION = "docs/standards/os/adoption.md"
SDK = "docs/standards/os/sdk.md"


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
                                              {"kind": "manual-assessment"},
                                              {"kind": "checker-limitations"},
                                              {"kind": "owned-source"},
                                              {"kind": "freertos-integration"},
                                              {"kind": "sdk-integration"})
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


def _owned_documents(root: Path) -> list[str]:
    return [path.relative_to(root).as_posix()
            for path in sorted((root / "docs/standards").rglob("*.md"))
            if re.search(r"MISRA C:2012 (?:Rule|Directive) \d+\.\d+",
                         path.read_text(encoding="utf-8"))
            or path.relative_to(root).as_posix() in {
                "docs/standards/memmap/README.md", "docs/standards/os/implementation.md",
                "docs/standards/os/native-facilities/addresses.md", ADOPTION}]


def _assessment_inputs(summary: dict, root: Path) -> dict:
    rules = _catalog(summary, root)
    supplementary = c_standard.supplementary_assessment(rules, summary["coverage"]["knownCheckerLimitations"], root)
    retention_documents = [path.relative_to(root).as_posix()
                           for path in sorted((root / c_retention.RECORD_DIRECTORY).glob("*.md"))]
    if not retention_documents:
        raise ValueError("Assessment retention records missing")
    retention_documents.append(GRP)
    return {"manual": _manual_identity(root),
            "ownedSource": {"parserSha256": digest(Path(c_standard.__file__)),
                            "documents": _document_identity(root, _owned_documents(root))},
            "retention": {"parserSha256": digest(Path(c_retention.__file__)),
                          "documents": _document_identity(root, retention_documents)},
            "sdkIntegration": {"documents": _document_identity(root, [SDK])
                               if (root / SDK).is_file() else {}},
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


def _grp_positions(
    summary: dict, project: Path, metadata: dict, root: Path, kernel_lines: dict
) -> set[tuple[int, int]]:
    """Locate owned return tokens covered by ADR 0025, independently of report labels."""
    positions = [(index, position, diagnostic, location)
                 for index, position, diagnostic, location in _positions(summary["diagnostics"])
                 if diagnostic["id"] == "misra-c2012-15.5" and location is not None]
    ledger = project / "meta/workbench-ownership.json"
    if not positions or not ledger.is_file() or summary["scope"].get("sdkLineDirectives"):
        return set()
    owners = {row["path"]: row for row in json.loads(ledger.read_text(encoding="utf-8"))["files"]}
    origins = {
        delivered: original for original, delivered in metadata["originalBswSources"].items()
    }
    owned = {}
    for name in {location["file"] for _, _, _, location in positions}:
        row = owners.get(name)
        if row is None or not safe_relative(name) or not (project / name).is_file():
            continue
        original = origins.get(name, "")
        if name.startswith("kernel/"):
            if (original.startswith("third_party/freertos/") and safe_relative(original)
                    and (root / original).is_file() and digest(root / original) == row["sha256"]
                    and name in kernel_lines):
                owned[name] = kernel_lines[name]
            continue
        if row["sha256"] != digest(project / name):
            continue
        if ((row["owner"], row["producerId"]) in {
                ("generated", "autosar-generator"),
                ("configuration", "autosar-workbench-config"),
                ("build", "autosar-workbench-build")}
                or (original.startswith("runtime/") and safe_relative(original)
                    and (root / original).is_file()
                    and digest(root / original) == row["sha256"])):
            owned[name] = None
    # Kernel ownership comes from the same verified ordered replay as integration
    # acceptance, rather than report labels or an original-vs-final text diff.
    returns = {}
    for name, lines in owned.items():
        source = (project / name).read_text(encoding="utf-8")
        returns[name] = set()
        for token in c_standard.lex(source):
            if token.text == "return":
                line = source.count("\n", 0, token.start) + 1
                column = token.start - source.rfind("\n", 0, token.start)
                if lines is None or line in lines:
                    returns[name].add((str(line), str(column)))
    accepted = set()
    for index, position, diagnostic, location in positions:
        point = str(location.get("line")), str(location.get("column"))
        if point not in returns.get(location["file"], ()):
            continue
        candidate = {**diagnostic, "locations": [location], "adopted": False,
                     "patched": location["file"].startswith("kernel/")}
        c_sdk.classify([candidate], summary["scope"], [], project)
        if candidate["sourceCategory"] in {"project", "project-patch"}:
            accepted.add((index, position))
    return accepted


def _owned_positions(summary: dict, project: Path, metadata: dict, root: Path) -> dict:
    """Join current source positions to the existing bidirectional MR12 records.

    Record association and formal acceptance remain separate. In particular a
    Required risk choice does not complete the formal deviation obligations.
    """
    ledger = project / "meta/workbench-ownership.json"
    if not ledger.is_file() or summary["scope"].get("sdkLineDirectives"):
        return {}
    owners = {row["path"]: row for row in json.loads(ledger.read_text(encoding="utf-8"))["files"]}
    origins = {delivered: original
               for original, delivered in metadata["originalBswSources"].items()}
    records = []
    errors = c_standard.deviation_findings(root, matched=records)
    # The shared MR12 parser also sees templates belonging to the fixed 159-key
    # group. Their own context and consumer remain the authority for those rows.
    records = [row for row in records
               if not row["record"]["file"].startswith(c_retention.RECORD_DIRECTORY + "/")]
    categories = {"misra-c2012-" + row["id"][1:]: row["category"]
                  for row in _catalog(summary, root) if row["id"].startswith("R")}
    positions = list(_positions(summary["diagnostics"]))
    candidates = [{**diagnostic, "locations": [location], "adopted": False, "patched": False}
                  for _, _, diagnostic, location in positions if location is not None]
    c_sdk.classify(candidates, summary["scope"], [], project)
    classified = iter(candidates)
    sources = {}
    mapping_validity = {}
    result = {}
    for index, position, diagnostic, location in positions:
        if location is None:
            continue
        candidate = next(classified)
        name = location["file"]
        if (name.startswith("kernel/")
                or not set(diagnostic["programs"]) <= {"host-batch", "probe"}):
            continue
        if name not in sources:
            original = origins.get(name, "")
            row = owners.get(name)
            path = project / name
            generated = row is not None and (row["owner"], row["producerId"]) in {
                ("generated", "autosar-generator"), ("configuration", "autosar-workbench-config")}
            runtime = (original.startswith("runtime/") and safe_relative(original)
                       and (root / original).is_file() and row is not None
                       and row["sha256"] == digest(root / original))
            if (not safe_relative(name) or row is None or not path.is_file()
                    or row["sha256"] != digest(path) or not (runtime or generated)):
                sources[name] = None
            else:
                text = path.read_text(encoding="utf-8")
                sources[name] = (original, text, c_standard.lex(text))
        if sources[name] is None:
            continue
        original, source, tokens = sources[name]
        lines = source.splitlines(keepends=True)
        line = int(location["line"])
        if not 1 <= line <= len(lines):
            continue
        column = int(location.get("column", 0))
        if column < 0 or column > len(lines[line - 1].rstrip("\r\n")):
            continue
        start = sum(map(len, lines[:line - 1]))
        offset = start + max(0, column - 1)
        if candidate["sourceCategory"] != "project":
            continue
        matches = [row for row in records if row["source"] == original and row["kind"] == "RULE"
                   and diagnostic["id"] == "misra-c2012-" + row["rule"]
                   and (row["start"] <= offset < row["end"]
                        or (int(location.get("column", 0)) == 0
                            and start <= row["start"] < start + len(lines[line - 1])))]
        if len(matches) == 1:
            record = matches[0]
            category = categories[diagnostic["id"]]
            scope = re.search(r"范围 `([^/]+)/([^/]+)/([^`]+)`", record["record"]["text"])
            target = "Linux" if summary["scope"]["target"].startswith("linux-") else "Windows"
            if scope and (scope[2] not in {"Both", target}
                          or (scope[3] != "Both" and set(diagnostic["programs"]) != {scope[3]})):
                continue
            result[index, position] = {"entity": record["entity"], "record": record["record"],
                                       "case": ("no-diagnostic"
                                                if scope and scope[1] == "ND" else None),
                                       "reason": None if category == "Advisory"
                                       else "mandatory_deviation_forbidden"
                                       if category == "Mandatory"
                                       else "formal_deviation_pending"}
        elif (diagnostic["id"] == "misra-c2012-17.3"
              and [flag for flag in summary["scope"]["compilerFlags"]
                   if flag.startswith("-std=")][-1:] == ["-std=c99"]):
            result[index, position] = {"entity": c_retention.containing_function(tokens, offset),
                "record": {"file": "docs/standards/os/implementation.md",
                           "text": "## C90隐式声明规则与当前编译"}, "reason": None}
        elif diagnostic["id"] == "misra-c2012-20.1":
            directive = next((t for t in tokens
                              if t.start <= offset < t.end), None)
            if directive is None:
                continue
            valid = c_standard.include_scope(directive.text)
            if not valid or not re.search(r'_MemMap\.h[">]$', valid):
                continue
            if name not in mapping_validity:
                try:
                    c_standard.verify_memory_mapping_pattern(source)
                    mapping_validity[name] = True
                except AssertionError:
                    mapping_validity[name] = False
            if not mapping_validity[name]:
                continue
            result[index, position] = {"entity": directive.text.strip(),
                "record": {"file": "docs/standards/memmap/README.md", "text": "## MemMap 类别偏离"},
                "reason": None}
    return {key: {**row, "reason": "owned_source_record_invalid" if errors else row["reason"]}
            for key, row in result.items()
            if (root / row["record"]["file"]).is_file()
            and row["record"]["text"] in (root / row["record"]["file"]).read_text(encoding="utf-8")}


def freertos_identity(summary: dict, identity: str, root: Path = ROOT) -> str:
    """Bind integration evidence to the exact reports as well as the analyzed input.

    Supplier responsibility does not authorize new diagnostics. Include every
    mixed diagnostic in full so adding a non-kernel location also invalidates it.
    """
    return _hash({"inputIdentity": identity,
                  "manifests": {name: digest(root / "third_party/freertos" / name)
                                for name in ("source-manifest.json", "posix-source-manifest.json")},
                  "diagnostics": sorted(_hash(row) for row in summary["diagnostics"]
                                        if any(location["file"].startswith("kernel/")
                                               for location in row["locations"]))})


def sdk_identity(summary: dict, identity: str) -> str:
    """Freeze every report and active definition, including unreported obligations."""
    scope = summary["scope"]
    return _hash({"inputIdentity": identity,
                  "headers": scope.get("sdkHeaders"),
                  "dependencies": scope.get("sdkTranslationUnits"),
                  "macros": scope.get("sdkMacroInputs"),
                  "diagnostics": sorted(_hash(row) for row in summary["diagnostics"])})


def _sdk_positions(summary: dict, project: Path, metadata: dict, root: Path,
                   kernel: tuple | None) -> tuple[dict, list[dict]]:
    """Check the exact SDK integration records, independently of diagnostic labels."""
    path = root / SDK
    if not path.is_file():
        raise ValueError("SDK integration evidence missing")
    text = path.read_text(encoding="utf-8")
    blocks = re.findall(r"<!-- sdk-integration-records-begin -->\s*(.*?)"
                        r"\s*<!-- sdk-integration-records-end -->", text, re.DOTALL)
    if len(blocks) != 1:
        raise ValueError("SDK integration records missing")
    record = json.loads(blocks[0], object_pairs_hook=_pairs)
    if set(record) != {"definitions", "sites"}:
        raise ValueError("Invalid SDK integration records")
    scope = summary["scope"]
    c_sdk.validate_sources(summary)
    headers = {row["path"]: row for row in scope["sdkHeaders"]}
    origins = {delivered: original
               for original, delivered in metadata["originalBswSources"].items()}
    owners = {row["path"]: row for row in json.loads(
        (project / "meta/workbench-ownership.json").read_text(encoding="utf-8"))["files"]}
    sources = dict(kernel[0]) if kernel is not None else {}
    selected = {}
    obligations = []
    for row in record["sites"]:
        if (set(row) != {"target", "rule", "file", "line", "symbol", "text"}
                or row["rule"] not in {"5.5", "5.7", "7.2", "7.3", "10.1"}
                or row["target"] not in c_retention.TARGETS
                or not safe_relative(row["file"]) or type(row["line"]) is not int
                or row["line"] < 1):
            raise ValueError("Invalid SDK integration site")
        if row["target"] != scope["target"]:
            continue
        key = row["rule"], row["file"], row["line"]
        if key in selected:
            raise ValueError("Duplicate SDK integration site")
        selected[key] = row
        name = row["file"]
        if name not in sources:
            original = origins.get(name, "")
            owner = owners.get(name, {})
            if (not original.startswith("runtime/") or not safe_relative(original)
                    or not (root / original).is_file()
                    or owner.get("sha256") != digest(root / original)
                    or digest(project / name) != digest(root / original)):
                raise ValueError(f"SDK integration source provenance differs: {name}")
            sources[name] = (project / name).read_text(encoding="utf-8")
        lines = c_sdk.source_code(sources[name]).splitlines()
        if row["line"] > len(lines) or lines[row["line"] - 1].strip() != row["text"]:
            raise ValueError(f"SDK integration site changed: {name}:{row['line']}")
        expected = record["definitions"].get(row["symbol"])
        if expected is None:
            raise ValueError("SDK integration symbol outside record")
        location = name + ":" + str(row["line"])
        # A shared source must have the same reviewed definition in every actual TU.
        uses = [(unit, values[location]) for unit, values in scope["sdkMacroInputs"].items()
                if location in values]
        if row["rule"] == "5.7":
            # sigaction is a public tag/function pair rather than a macro.
            if (row["symbol"] != "sigaction" or not re.search(r"\bsigaction\b", row["text"])
                    or name not in scope["sdkTranslationUnits"]
                    or not any(all(headers[header].get(field) == expected[field]
                                   for field in ("header", "provider", "version", "sha256"))
                               for header in scope["sdkTranslationUnits"][name])):
                raise ValueError("SDK sigaction interface provenance differs")
        else:
            if not uses:
                raise ValueError(f"SDK integration active definition missing: {location}")
            for unit, values in uses:
                for symbol, definition in expected.items():
                    actual = values.get(symbol, {})
                    header = headers.get(actual.get("file"), {})
                    if (actual.get("use", "").strip() != row["text"]
                            or actual.get("reference", False)
                            or actual.get("file") not in scope["sdkTranslationUnits"][unit]
                            or {field: actual.get(field) for field in ("line", "value", "function")}
                            != {field: definition[field] for field in ("line", "value", "function")}
                            or any(header.get(field) != definition[field]
                                   for field in ("header", "provider", "version", "sha256"))):
                        raise ValueError(f"SDK integration definition differs: {location}:{symbol}")
        obligations.append({"rule": row["rule"], "location": {"file": name, "line": row["line"]},
                            "entity": row["symbol"], "record": SDK})
    if not selected:
        raise ValueError("SDK integration target records missing")
    # Extra uses of a reviewed SDK macro require their own record, even with no warning.
    symbols = {row["symbol"] for row in selected.values() if row["rule"] != "5.7"}
    locations = {(row["file"] + ":" + str(row["line"]), row["symbol"])
                 for row in selected.values()}
    for values in scope["sdkMacroInputs"].values():
        for location, definitions in values.items():
            for symbol in symbols & definitions.keys():
                definition = definitions[symbol]
                if (definition["file"] in headers and not definition.get("reference", False)
                        and (location, symbol) not in locations):
                    raise ValueError(f"SDK integration unrecorded use: {location}:{symbol}")
    result = {}
    for index, position, diagnostic, location in _positions(summary["diagnostics"]):
        if location is None or not set(diagnostic["programs"]) <= {"host-batch", "probe"}:
            continue
        if not diagnostic["id"].startswith("misra-c2012-"):
            continue
        rule = diagnostic["id"].removeprefix("misra-c2012-")
        row = selected.get((rule, location["file"], int(location["line"])))
        if row is None:
            continue
        column = int(location.get("column", 0))
        source_line = c_sdk.source_code(sources[row["file"]]).splitlines()[row["line"] - 1]
        if not 0 <= column <= len(source_line):
            continue
        # R7.2 has actual unsigned operand evidence; a macro on the same line is insufficient.
        definition = record["definitions"][row["symbol"]].get(row["symbol"], {})
        if rule == "7.2" and not any(
                origin.get("kind") == "unsigned-literal" and origin.get("macro") == row["symbol"]
                and origin.get("category") == "sdk"
                and origin.get("value") == definition["value"]
                and origin.get("definitionLine") == definition["line"]
                and origin.get("headerKey") in {
                    header["key"] for header in headers.values()
                    if header["header"] == definition["header"]}
                for origin in diagnostic.get("sourceOrigins", [])):
            continue
        result[index, position] = {"entity": row["symbol"],
                                   "record": {"file": SDK, "text": row["text"]}}
    return result, obligations


def _freertos_sources(project: Path, metadata: dict, root: Path) -> tuple[dict, dict]:
    """Replay the ordered project patches against verified supplier originals."""
    from ecu_tools.build import (
        kernel_patch_environment,
        kernel_patch_targets,
        require_kernel_patch_applied,
    )

    from autosar_tooling import c_check

    target = json.loads((project / "target.json").read_text(encoding="utf-8"))
    manifests = [json.loads((root / "third_party/freertos" / name).read_text(encoding="utf-8"))
                 for name in ("source-manifest.json", "posix-source-manifest.json")]
    original_hashes = {name: value for manifest in manifests
                       for name, value in manifest["files"].items()}
    owners = {row["path"]: row for row in json.loads(
        (project / "meta/workbench-ownership.json").read_text(encoding="utf-8"))["files"]}
    sources = {}
    with tempfile.TemporaryDirectory(prefix="freertos-acceptance-") as temporary:
        replay = Path(temporary)
        for original, delivered in metadata["originalBswSources"].items():
            if not delivered.startswith("kernel/"):
                continue
            if not (safe_relative(original) and safe_relative(delivered)
                    and original.startswith("third_party/freertos/")):
                raise ValueError("FreeRTOS original provenance invalid")
            relative = original.removeprefix("third_party/freertos/")
            expected = original_hashes.get(relative)
            if relative in {"source-manifest.json", "posix-source-manifest.json"}:
                expected = digest(root / original)
            if (expected is None or digest(root / original) != expected
                    or owners.get(delivered, {}).get("sha256") != expected):
                raise ValueError(f"FreeRTOS original identity differs: {original}")
            destination = replay / delivered
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(root / original, destination)
        if not target.get("kernelPatches"):
            raise ValueError("FreeRTOS ordered patch selection missing")
        patched = {}
        provenance_error = None
        for index, patch in enumerate(target["kernelPatches"]):
            original = patch["path"]
            if not safe_relative(original) or not original.startswith("runtime/os/patches/"):
                raise ValueError("FreeRTOS patch provenance invalid")
            delivered = original.removeprefix("runtime/")
            if digest(root / original) != digest(project / delivered):
                raise ValueError(f"FreeRTOS patch identity differs: {original}")
            before = kernel_patch_targets(replay / "kernel", project / delivered, patch["includes"])
            git = shutil.which("git")
            if git is None:
                raise ValueError("Git is required to verify FreeRTOS integration")
            argv = [str(Path(git).resolve()), "-c", "core.autocrlf=false",
                    "-c", "core.quotePath=false", "apply", "--verbose", "--ignore-space-change",
                    *("--include=" + name for name in patch["includes"]), str(project / delivered)]
            _, _, report = c_check.run(
                argv, replay, f"patch-{index}", 30, cwd=replay / "kernel",
                env={**kernel_patch_environment(replay / "kernel"), "LC_ALL": "C"})
            require_kernel_patch_applied(project / delivered, before)
            if provenance_error is None:
                try:
                    patched = c_check.kernel_patched_lines(
                        before, replay, patched, project / delivered, report)
                except ValueError as error:
                    provenance_error = error
        require_kernel_identity(replay / "kernel", target["kernelSha256"])
        require_kernel_identity(project / "kernel", target["kernelSha256"])
        if provenance_error is not None:
            raise provenance_error
        for name in target["kernelSha256"]:
            if name.endswith((".c", ".h")):
                delivered = "kernel/" + name
                sources[delivered] = (replay / delivered).read_text(encoding="utf-8")
    return sources, patched


def _freertos_positions(
    summary: dict, project: Path, metadata: dict, root: Path, kernel: tuple | None = None
) -> dict:
    """Consume supplier integration and project MR12 records by actual position."""
    target = json.loads((project / "target.json").read_text(encoding="utf-8"))
    if not target.get("kernelPatches") and not any(
            location and location["file"].startswith("kernel/")
            for _, _, _, location in _positions(summary["diagnostics"])):
        return {}
    if (not (root / ADOPTION).is_file()
            or "## 当前集成义务与消费" not in (root / ADOPTION).read_text(encoding="utf-8")):
        return {}
    if not all((root / "third_party/freertos" / name).is_file()
               for name in ("source-manifest.json", "posix-source-manifest.json")):
        return {}
    sources, patched = kernel if kernel is not None else _freertos_sources(project, metadata, root)
    records = []
    errors = c_standard.deviation_findings(root, matched=records, kernel_sources=sources)
    records = [row for row in records if row["source"].startswith("kernel/")]
    result = {}
    for index, position, diagnostic, location in _positions(summary["diagnostics"]):
        if location is None or location["file"] not in sources:
            continue
        name = location["file"]
        source = sources[name]
        lines = source.splitlines(keepends=True)
        line, column = int(location["line"]), int(location.get("column", 0))
        if (not 1 <= line <= len(lines) or column < 0
                or column > len(lines[line - 1].rstrip("\r\n"))
                or not set(diagnostic["programs"]) <= {"host-batch", "probe"}):
            continue
        candidate = {**diagnostic, "locations": [location],
                     "patched": line in patched.get(name, ()), "adopted": False}
        c_sdk.classify([candidate], summary["scope"], [], project)
        if candidate["sourceCategory"] == "sdk":
            continue
        start = sum(map(len, lines[:line - 1]))
        offset = start + max(0, column - 1)
        if line not in patched.get(name, ()):
            # Only the exact frozen report set can consume this responsibility
            # classification. Project and SDK positions take their own paths.
            result[index, position] = {"entity": None, "reason": None,
                "record": {"file": ADOPTION, "text": "## 当前集成义务与消费"}}
            continue
        matches = [row for row in records if row["source"] == name and row["kind"] == "RULE"
                   and diagnostic["id"] == "misra-c2012-" + row["rule"]
                   and (row["start"] <= offset < row["end"]
                        or column == 0 and start <= row["start"] < start + len(lines[line - 1]))]
        if len(matches) == 1:
            row = matches[0]
            # Only the already bounded Advisory table belongs to this consumer.
            # Required and Mandatory records need their own formal disposition.
            if (row["record"]["file"] != ADOPTION or row["rule"] not in {"2.5", "8.7"}):
                continue
            result[index, position] = {"entity": row["entity"], "record": row["record"],
                "reason": "freertos_record_invalid" if errors else None}
        elif (diagnostic["id"] == "misra-c2012-17.3"
              and [flag for flag in summary["scope"]["compilerFlags"]
                   if flag.startswith("-std=")][-1:] == ["-std=c99"]):
            result[index, position] = {"entity": c_retention.containing_function(
                c_standard.lex(source), offset), "reason": None,
                "record": {"file": ADOPTION, "text": "## 补丁诊断的具体处置"}}
        elif (diagnostic["id"] == "misra-c2012-20.7"
              and name == "kernel/portable/ThirdParty/GCC/Posix/portmacro.h"
              and lines[line - 1].strip() in {
                  ("#define portTASK_FUNCTION_PROTO(function, parameters) "
                   "void function(void *parameters)"),
                  ("#define portTASK_FUNCTION(function, parameters) "
                   "void function(void *parameters)")}):
            result[index, position] = {"entity": lines[line - 1].split("(")[0].split()[-1],
                "reason": None, "record": {"file": ADOPTION, "text": "## 补丁诊断的具体处置"}}
        elif (diagnostic["id"] == "misra-c2012-5.5" and name == "kernel/include/task.h"
              and lines[line - 1].strip() == "extern TaskHandle_t volatile pxCurrentTCB;"):
            result[index, position] = {"entity": "pxCurrentTCB", "reason": None,
                "record": {"file": ADOPTION, "text": "## 补丁诊断的具体处置"}}
    return result


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
                    if (context["records"]["kind"] not in {
                            "freertos-integration", "sdk-integration"}
                        or context["source"] == {
                            "freertos-integration": ADOPTION,
                            "sdk-integration": SDK}[context["records"]["kind"]])
                    if case["target"] == result["target"] and case["inputIdentity"] == (
                        freertos_identity(summary, identity, root)
                        if context["records"]["kind"] == "freertos-integration"
                        else sdk_identity(summary, identity)
                        if context["records"]["kind"] == "sdk-integration" else identity)}
        case_ids = {case["caseId"] for _, case in selected.values()}
        if len(case_ids) == 1:
            result["caseId"] = next(iter(case_ids))
        group_reasons = {}
        if "owned-source" in selected:
            result["contexts"].append(_reference(selected["owned-source"][0]))
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
        target = json.loads((project / "target.json").read_text(encoding="utf-8"))
        kernel = None
        if target.get("kernelPatches") and all(
                (root / "third_party/freertos" / name).is_file()
                for name in ("source-manifest.json", "posix-source-manifest.json")):
            kernel = _freertos_sources(project, metadata, root)
        grp_positions = _grp_positions(
            summary, project, metadata, root, kernel[1] if kernel is not None else {})
        owned_positions = _owned_positions(summary, project, metadata, root)
        freertos_positions = _freertos_positions(summary, project, metadata, root, kernel)
        if "freertos-integration" in selected:
            result["contexts"].append(_reference(selected["freertos-integration"][0]))
        target = json.loads((project / "target.json").read_text(encoding="utf-8"))
        if target.get("kernelPatches"):
            context = selected.get("freertos-integration")
            result["freertosInputIdentity"] = freertos_identity(summary, identity, root)
            if context is None:
                add("context", "freertos-integration",
                    "freertos_integration_context_unbound_or_stale")
            elif context[1]["caseId"] not in c_retention.PROFILES.values():
                add("context", "freertos-integration", "freertos_case_outside_record")
            elif context[0]["formalStatus"] != "approved":
                add("context", "freertos-integration", "freertos_integration_assessment_pending")
            elif (not (root / ADOPTION).is_file()
                  or "## 当前集成义务与消费" not in (root / ADOPTION).read_text(encoding="utf-8")):
                add("context", "freertos-integration", "freertos_integration_evidence_missing")
        sdk_positions = {}
        sdk_reason = "sdk_integration_context_unbound_or_stale"
        if (root / SDK).is_file() or summary["scope"].get("sdkTranslationUnits"):
            sdk_positions, obligations = _sdk_positions(summary, project, metadata, root, kernel)
            result["sdkObligations"] = obligations
            result["sdkInputIdentity"] = sdk_identity(summary, identity)
            context = selected.get("sdk-integration")
            if context is not None:
                result["contexts"].append(_reference(context[0]))
                sdk_reason = ("assessment_case_mismatch" if len(case_ids) != 1
                              else "sdk_case_outside_record"
                              if context[1]["caseId"] not in c_retention.PROFILES.values()
                              else "sdk_integration_assessment_pending"
                              if context[0]["formalStatus"] != "approved" else None)
            if sdk_reason:
                add("context", "sdk-integration", sdk_reason)
        occurrences = [row for row in retention["occurrences"] if row["rule"] != "checkersReport"]
        positions = list(_positions(summary["diagnostics"]))
        if len(occurrences) != len(positions):
            raise ValueError("Retention occurrence coverage incomplete")
        profile = next((key for key, name in c_retention.PROFILES.items() if name == result["caseId"]), None)
        for occurrence, (index, position, diagnostic, location) in zip(occurrences, positions):
            if occurrence["diagnostic"] != index or occurrence["location"] != location:
                raise ValueError("Retention occurrence position differs")
            grp = (index, position) in grp_positions
            if grp:
                reason = None
                occurrence.update(status="grp_disapplied", record=GRP + "#grp指南重分类计划")
            elif (index, position) in sdk_positions:
                ruling = sdk_positions[index, position]
                occurrence.update(status="sdk_record_matched", entity=ruling["entity"],
                                  record=ruling["record"])
                reason = sdk_reason
            elif (index, position) in freertos_positions:
                ruling = freertos_positions[index, position]
                occurrence.update(status="freertos_record_matched", entity=ruling["entity"],
                                  record=ruling["record"])
                context = selected.get("freertos-integration")
                reason = ("freertos_integration_context_unbound_or_stale" if context is None
                          else "assessment_case_mismatch" if len(case_ids) != 1
                          else "freertos_case_outside_record"
                          if context[1]["caseId"] not in c_retention.PROFILES.values()
                          else "freertos_integration_assessment_pending"
                          if context[0]["formalStatus"] != "approved" else ruling["reason"])
            elif (index, position) in owned_positions:
                ruling = owned_positions[index, position]
                occurrence.update(status="owned_record_matched", entity=ruling["entity"],
                                  record=ruling["record"])
                context = selected.get("owned-source")
                reason = ("owned_source_context_unbound_or_stale" if context is None
                          else "assessment_case_mismatch" if len(case_ids) != 1
                          else "owned_source_case_outside_record"
                          if context[1]["caseId"] not in c_retention.PROFILES.values()
                          or (ruling.get("case") and ruling["case"] != context[1]["caseId"])
                          else "formal_deviation_pending"
                          if context[0]["formalStatus"] != "approved"
                          else ruling.get("reason"))
            elif occurrence["status"] != "record_matched":
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
                              contextId=(selected["sdk-integration"][0]["id"]
                                         if (index, position) in sdk_positions
                                         and "sdk-integration" in selected
                                         else selected["freertos-integration"][0]["id"]
                                         if (index, position) in freertos_positions
                                         and "freertos-integration" in selected
                                         else selected["owned-source"][0]["id"]
                                         if (index, position) in owned_positions
                                         and "owned-source" in selected
                                         else selected["generated-retention"][0]["id"]
                                         if "generated-retention" in selected else None),
                              evidenceClosure="not_established" if reason else "frozen_context",
                              assessment="blocked" if reason else "accepted",
                              reason=reason or ("grp_disapplied" if grp
                                                else "sdk_integration_verified"
                                                if (index, position) in sdk_positions
                                                else "freertos_integration_verified"
                                                if (index, position) in freertos_positions
                                                else "owned_source_approved"
                                                if (index, position) in owned_positions
                                                else "retention_approved"))
            result["occurrences"].append(occurrence)
            if reason:
                add("diagnostic", index, reason, rule=occurrence["rule"], location=location,
                    entity=occurrence["entity"], programs=occurrence["programs"], record=occurrence["record"])
        result["status"] = "blocked" if result["objects"] else "accepted"
    except (OSError, ValueError, KeyError, TypeError, AttributeError, RuntimeError) as error:
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
