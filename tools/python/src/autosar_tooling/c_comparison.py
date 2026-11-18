"""Compare complete C analyses by diagnostic source text, retaining occurrences."""

from __future__ import annotations

import hashlib
import json
import re
from collections import Counter
from pathlib import Path

from ecu_tools.build import FILE_HASHES, digest, refuse_links, safe_relative

from autosar_tooling import c_sdk
from autosar_tooling.c_standard import MANUAL_ASSESSMENT_SOURCES


def _read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


MATRIX_TARGETS = ("linux-x64-controlled-v1", "windows-x64-controlled-v1")
MATRIX_SAMPLES = ("standard-ecu", "user-application", "component-pipeline", "no-diagnostic")


def raw_identity(summary: dict) -> str:
    """Bind raw analysis independently of the later matrix acceptance annotations."""
    raw = {key: value for key, value in summary.items()
           if key not in {"projectAcceptance", "complianceScope"}}
    raw["diagnostics"] = [{key: value for key, value in row.items() if key != "complianceScope"}
                          for row in summary["diagnostics"]]
    return hashlib.sha256(json.dumps(raw, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def scope_receipt(summary: dict, sample: str) -> dict:
    return {"format": "c-misra-scope-input-v1", "sample": sample,
            "analysisIdentity": raw_identity(summary),
            **{key: summary["scope"][key] for key in
               ("target", "translationUnits", "projectTranslationUnits")}}


def scope_matrix(receipts: list[dict]) -> dict:
    """Require exactly eight raw dependency receipts before applying rule 2."""
    expected = {(target, sample) for target in MATRIX_TARGETS for sample in MATRIX_SAMPLES}
    found = set()
    for row in receipts:
        if (set(row) != {"format", "target", "sample", "analysisIdentity",
                         "translationUnits", "projectTranslationUnits"}
                or row["format"] != "c-misra-scope-input-v1"
                or not re.fullmatch(r"[0-9a-f]{64}", row["analysisIdentity"])):
            raise ValueError("Invalid matrix scope receipt")
        pair = row["target"], row["sample"]
        if pair not in expected or pair in found:
            raise ValueError("Unexpected or duplicated matrix scope receipt")
        found.add(pair)
        units, dependencies = row["translationUnits"], row["projectTranslationUnits"]
        if (not isinstance(units, list) or not units or len(units) != len(set(units))
                or any(not safe_relative(unit) for unit in units)
                or not isinstance(dependencies, dict) or set(dependencies) != set(units)
                or any(not isinstance(paths, list) or paths != sorted(set(paths))
                       or unit not in paths or any(not safe_relative(name) for name in paths)
                       for unit, paths in dependencies.items())):
            raise ValueError("Matrix GCC dependency receipt incomplete")
    if found != expected:
        raise ValueError(f"Matrix scope incomplete: {sorted(expected - found)}")
    members = sorted(receipts, key=lambda row: (row["target"], row["sample"]))
    return {"format": "c-misra-scope-matrix-v1", "members": members,
            "targetHeaders": sorted(target_headers(members))}


def matrix_headers(summary: dict, matrix: dict) -> set[str]:
    if matrix != scope_matrix(matrix["members"]):
        raise ValueError("Matrix scope union differs from dependency receipts")
    if not any(row == scope_receipt(summary, row["sample"])
               for row in matrix["members"] if row["target"] == summary["scope"]["target"]):
        raise ValueError("Matrix scope does not bind this raw analysis")
    return set(matrix["targetHeaders"])


def annotate_scope(summary: dict, headers: set[str] | None = None) -> None:
    for diagnostic in summary["diagnostics"]:
        diagnostic["complianceScope"] = [location_scope(summary["scope"], location, headers=headers)
                                         for location in diagnostic["locations"]]
    summary["complianceScope"] = scope_statistics(summary, headers=headers)


def platform_location(name: str) -> bool:
    """ADR 0030 rule 1: platform selection owns the complete directory."""
    return name.startswith(("os/src/host/", "kernel/portable/MSVC-MingW/",
                            "kernel/portable/ThirdParty/GCC/Posix/"))


def target_headers(scopes: list[dict]) -> set[str]:
    """Union GCC -MD receipts from target TUs, across samples and programs."""
    headers = set()
    for scope in scopes:
        dependencies = scope.get("projectTranslationUnits", {})
        for unit in scope["translationUnits"]:
            if platform_location(unit) or unit.startswith("integration/"):
                continue
            headers.update(dependencies.get(unit, []))
    return headers


def location_scope(scope: dict, location: dict, *, headers: set[str] | None = None) -> str:
    """Classify a physical position, including mixed adopted/project diagnostics."""
    name = location["file"]
    if platform_location(name):
        return "host-platform"
    if name.startswith("integration/"):
        if name.endswith(".c"):
            return "host-integration"
        dependencies = scope.get("projectTranslationUnits")
        units = [unit for unit in scope["translationUnits"]
                 if not platform_location(unit) and not unit.startswith("integration/")]
        if not isinstance(dependencies, dict) or any(unit not in dependencies for unit in units):
            raise ValueError("MISRA scope requires complete GCC project dependency receipts")
        if name not in (target_headers([scope]) if headers is None else headers):
            return "host-integration"
    if (name.startswith("kernel/") and "patchedLines" in scope
            and int(location["line"]) not in scope["patchedLines"].get(name, [])):
        return "adopted-kernel"
    return "in-scope"


def scope_statistics(summary: dict, *, headers: set[str] | None = None) -> dict:
    """Count physical locations once; program multiplicity is kept by comparison."""
    categories = Counter(location_scope(summary["scope"], location, headers=headers)
                         for diagnostic in summary["diagnostics"]
                         if diagnostic["id"] != "checkersReport"
                         for location in diagnostic["locations"])
    inside = categories["in-scope"]
    total = sum(categories.values())
    raw = [diagnostic for diagnostic in summary["diagnostics"]
           if diagnostic["id"] != "checkersReport"]
    raw_outside = sum(bool(row["locations"]) and all(
        location_scope(summary["scope"], location, headers=headers) != "in-scope"
        for location in row["locations"]) for row in raw)
    return {"total": total, "inScope": inside, "outOfScope": total - inside,
            "categories": dict(sorted(categories.items())),
            "diagnostics": {"total": len(raw), "inScope": len(raw) - raw_outside,
                            "outOfScope": raw_outside,
                            "informational": len(summary["diagnostics"]) - len(raw)}}


def diagnostic_counts(summary: dict, root: Path, *, in_scope_only: bool = False,
                      headers: set[str] | None = None) -> Counter:
    counts = Counter()
    for diagnostic in summary["diagnostics"]:
        if diagnostic["id"] == "checkersReport":
            continue
        locations = diagnostic["locations"]
        if not locations:
            raise ValueError(f"Diagnostic has no source location: {diagnostic['id']}")
        for location in locations:
            if in_scope_only and location_scope(summary["scope"], location, headers=headers) != "in-scope":
                continue
            name = location["file"]
            if not safe_relative(name):
                raise ValueError(f"Diagnostic outside sample: {name}")
            lines = (root / name).read_text(encoding="utf-8").split("\n")
            line = int(location["line"])
            if not 1 <= line <= len(lines):
                raise ValueError(f"Diagnostic source line unavailable: {name}:{line}")
            for program in diagnostic["programs"]:
                counts[diagnostic["id"], name, lines[line - 1], program] += 1
    return counts


def _complete(summary: dict) -> None:
    if (summary["error"] is not None or not summary["programs"]
            or any(program["exitCode"] not in (0, 1) for program in summary["programs"])):
        raise ValueError("Analysis is incomplete")
    incomplete = {"misra-config", "syntaxError", "preprocessorErrorDirective", "internalError",
                  "cppcheckError", "addonError", "missingInclude", "missingIncludeSystem", "toomanyconfigs"}
    if any(row["id"] in incomplete for row in summary["diagnostics"]):
        raise ValueError("Analysis contains configuration/parsing failures")


def _semantic(summary: dict) -> dict:
    identity = summary.get("semanticIdentity")
    def require(condition):
        if not condition:
            raise ValueError("Semantic identity missing, unsupported or incomplete")
    def hashes(value, keys, nullable=False):
        require(isinstance(value, dict) and set(value) == set(keys))
        require(all((nullable and item is None) or
                    (isinstance(item, str) and re.fullmatch(r"[0-9a-f]{64}", item))
                    for item in value.values()))
    def strings(value, nonempty=False):
        return isinstance(value, list) and (not nonempty or bool(value)) and all(isinstance(x, str) and x for x in value)
    if isinstance(identity, dict) and identity.get("version") == 2:
        raise ValueError(
            "Semantic identity version 2 sealed the complete SDK provider environment "
            "and is superseded by version 3; regenerate the analysis with the current tool")
    require(isinstance(identity, dict) and set(identity) ==
            {"version", "compiler", "defines", "models", "libraries", "analysisCode", "addonRuntime", "sdk"})
    require(type(identity["version"]) is int and identity["version"] == 3)
    compiler = identity["compiler"]
    require(isinstance(compiler, dict) and set(compiler) ==
            {"machine", "release", "predefines", "builtins", "includeSearch"})
    require(isinstance(compiler["machine"], str) and bool(compiler["machine"]))
    require(isinstance(compiler["release"], str) and bool(re.fullmatch(r"[0-9]+(?:\.[0-9]+)*", compiler["release"])))
    require(strings(compiler["predefines"], True) and strings(compiler["builtins"], True))
    search = compiler["includeSearch"]
    require(isinstance(search, dict) and set(search) == {"quote", "system", "environment"}
            and strings(search["quote"]) and strings(search["system"], True)
            and isinstance(search["environment"], dict)
            and set(search["environment"]) == set(c_sdk.SEARCH_ENVIRONMENT)
            and all(value is None or isinstance(value, str)
                    for value in search["environment"].values()))
    defines = identity["defines"]
    require(isinstance(defines, dict) and set(defines) == set(summary["scope"]["translationUnits"]))
    require(all(strings(value) for value in defines.values()))
    hashes(identity["models"], ("platformModel", "platformLibrary"), True)
    hashes(identity["libraries"], ("std", "windows" if summary["scope"]["target"].startswith("windows") else "posix", "implicitStd"))
    hashes(identity["analysisCode"], ("c_check.py", "c_addon.py", "c_comparison.py", "c_sdk.py", "c_standard.py", "c_retention.py", "c_acceptance.py", "c_preprocessor.py", "build.py", "process.py", "owner.py", "launch.py", "windows_job.py"))
    hashes(identity["addonRuntime"], ("runaddon.py", "cppcheck.py", "cppcheckdata.py"))
    assessment = summary["coverage"]["manualAssessment"]
    require(assessment["format"] == "misra-manual-assessment-v1")
    manual = assessment["inputIdentity"]
    require(set(manual) == {"parserSha256", "selectionSha256", "documents"})
    hashes(manual["documents"], MANUAL_ASSESSMENT_SOURCES)
    require(isinstance(manual["parserSha256"], str) and bool(re.fullmatch("[0-9a-f]{64}", manual["parserSha256"])))
    require(manual["selectionSha256"] == hashlib.sha256("\n".join(MANUAL_ASSESSMENT_SOURCES).encode()).hexdigest())
    require(isinstance(identity["sdk"], str) and bool(re.fullmatch("[0-9a-f]{64}", identity["sdk"])))
    # Both comparison modes rest on the recorded inventory and opened headers.
    c_sdk.validate_inventory(summary["scope"])
    sdk = c_sdk.semantic_inputs(summary["scope"])
    if hashlib.sha256(json.dumps(sdk, sort_keys=True).encode()).hexdigest() != identity["sdk"]:
        raise ValueError("Semantic identity differs: sdk receipt")
    return identity


def _source_sdk(summary: dict, identity: dict) -> dict:
    """Separate verified SDK header evidence from source-dependent usage inventories."""
    sdk = c_sdk.semantic_inputs(summary["scope"])
    headers = {row["header"]: row for row in sdk["headers"]}
    header_names = {row["key"]: row["header"] for row in sdk["headers"]}
    macros = {}
    for uses in sdk["macroInputs"].values():
        for use in uses:
            definition = json.loads(use)
            key = header_names[definition["headerKey"]], definition["macro"]
            macros.setdefault(key, set()).add((definition["value"], definition["function"]))
    defines = {}
    for unit, values in identity["defines"].items():
        for value in values:
            match = re.fullmatch(r"-D([A-Za-z_]\w*)(.*)", value)
            if match is None:
                raise ValueError("Semantic identity differs: defines format")
            defines[unit, match[1]] = match[2]
    return {"headers": headers, "macros": macros, "defines": defines,
            "inventory": sdk["inventory"], "links": sdk["links"],
            "lineDirectives": sdk["lineDirectives"]}


def _compare_source_sdk(current: dict, previous: dict, renames: dict) -> None:
    now = _source_sdk(current, current["semanticIdentity"])
    before = _source_sdk(previous, previous["semanticIdentity"])
    before["defines"] = {(renames.get(unit, unit), name): value
                         for (unit, name), value in before["defines"].items()}
    if now["lineDirectives"] != before["lineDirectives"]:
        raise ValueError("Semantic identity differs: sdk line directives")
    # Every provider file the compiler can find must match, opened or not: a header that
    # only one tree opens, or a different file behind the same include, changes the result.
    files = now["inventory"].keys() | before["inventory"].keys()
    changed = sorted(name for name in files
                     if now["inventory"].get(name) != before["inventory"].get(name))
    if changed:
        raise ValueError(f"Semantic identity differs: sdk inventory: {changed[0]}")
    names = now["links"].keys() | before["links"].keys()
    changed = sorted(name for name in names if now["links"].get(name) != before["links"].get(name))
    if changed:
        raise ValueError(f"Semantic identity differs: sdk inventory: link {changed[0]}")
    for field in ("headers", "macros", "defines"):
        # Source changes may open more or fewer headers and use other macros. What both
        # trees consumed must agree byte for byte, as a second check on the inventory.
        for key in now[field].keys() & before[field].keys():
            if now[field][key] != before[field][key]:
                label = "defines" if field == "defines" else "sdk " + field
                raise ValueError(f"Semantic identity differs: {label}: {key}")


def _configuration(root: Path, *, source_tree: bool = False) -> dict:
    refuse_links(root / "meta/integration.json", root)
    metadata = _read(root / "meta/integration.json")
    plan = dict(metadata["plan"])
    if "runtimeSources" not in plan:
        if metadata.get("format") == "autosar-ecu-integration-v2":
            raise ValueError("Runtime source provenance missing")
        return plan
    sources, origins = plan["runtimeSources"], metadata.get("originalBswSources")
    if not isinstance(sources, dict) or not isinstance(origins, dict):
        raise TypeError("Runtime source provenance invalid")
    records = (root / FILE_HASHES).read_text(encoding="utf-8").splitlines()

    def require_source(name, identity):
        if not isinstance(name, str) or not safe_relative(name):
            raise ValueError("Runtime source path invalid")
        refuse_links(root / name, root)
        if records.count(f"{identity}  {name}") != 1 or digest(root / name) != identity:
            raise ValueError(f"Runtime source identity differs: {name}")

    require_source("meta/integration.json", digest(root / "meta/integration.json"))
    selected = dict(sources)
    for original, identity in sources.items():
        if not isinstance(original, str) or not safe_relative(original):
            raise ValueError("Runtime source origin invalid")
        if not isinstance(identity, str) or not re.fullmatch(r"[0-9a-f]{64}", identity):
            raise ValueError("Runtime source identity invalid")
        if original.startswith("runtime/") and Path(original).suffix in {".c", ".h"} and original in origins:
            copied = origins[original]
            require_source(copied, identity)
            # Delivered bytes may change; their selected source/path binding may not.
            selected[original] = copied
    if source_tree:
        # Producer bytes may differ in a freshly scanned source tree.
        # Keep selected origins and delivered paths as configuration identity.
        selected = {name: value for name, value in selected.items() if name in origins}
        # The native rule inventory includes the Rust generator itself, and
        # the definition fingerprint includes that inventory identity.
        if "ruleSetIdentity" in plan:
            plan["ruleSetIdentity"] = {key: value for key, value in
                                       plan["ruleSetIdentity"].items() if key != "sha256"}
        if "validationDependencies" in plan:
            plan["validationDependencies"] = {key: value for key, value in
                                              plan["validationDependencies"].items()
                                              if key not in {"rules.sha256", "definitions.sha256"}}
    plan["runtimeSources"] = selected
    return plan


def compare(
    current: dict, project: Path, output: Path, baseline: Path, *,
    source_tree: bool = False, renames: dict[str, str] | None = None,
) -> dict:
    """A baseline is evidence for comparison, never permission to skip analysis."""
    try:
        renames = renames or {}

        def mapped(value):
            if isinstance(value, str):
                return renames.get(value, value)
            if isinstance(value, list):
                return [mapped(item) for item in value]
            if isinstance(value, dict):
                return {renames.get(key, key): mapped(item) for key, item in value.items()}
            return value

        def normalized(value, field=""):
            if isinstance(value, dict):
                return {key: normalized(item, key) for key, item in value.items()
                        if not (source_tree and field == "ruleSetIdentity" and key == "sha256")}
            if isinstance(value, list):
                items = [normalized(item) for item in value]
                if source_tree and field == "includePaths":
                    # Selected host provider directories may move with source layout.
                    # Their actual dependencies remain bound by each complete scan.
                    items = [item for item in items if not item.startswith("os/src/host/")]
                # These are source sets, emitted in filename order. Renaming a
                # source can reorder them without changing the analysis closure.
                if (source_tree and field in {"sources", "kernelSources", "translationUnits",
                                              "supplemental"}
                        and all(isinstance(item, str) for item in items)):
                    return sorted(items)
                return items
            return value

        previous = _read(baseline / "summary.json")
        old_root = baseline / "project"
        _complete(current)
        _complete(previous)
        for field in ("version", "expectedSourceRevision", "addons"):
            if current["tool"][field] != previous["tool"][field]:
                raise ValueError(f"Tool identity differs: {field}")
        if current["coverage"] != previous["coverage"]:
            raise ValueError("Checker coverage differs")
        if digest(old_root / FILE_HASHES) != previous["sourceSealSha256"]:
            raise ValueError("Baseline source seal identity differs")
        if any(digest(root / FILE_HASHES) != current["sourceSealSha256"]
               for root in (project, output / "project")):
            raise ValueError("Current source seal identity differs")
        # Source bytes may change; the selected target/configuration/link closure may not.
        def target_configuration(path):
            value = _read(path / "target.json")
            if source_tree:
                # These digests include the producer's source bytes. The complete
                # selected plan and all remaining target configuration are compared below.
                value = {key: item for key, item in value.items()
                         if key not in {"definitionFingerprint", "inputSha256"}}
            return value
        if normalized(target_configuration(project)) != normalized(
                mapped(target_configuration(old_root))):
            raise ValueError("Target/sample configuration differs")
        for field in ("target", "profile", "entries", "compilerFlags", "includePaths",
                      "translationUnits", "supplemental", "patchedKernel"):
            if normalized(current["scope"][field], field) != normalized(
                    mapped(previous["scope"][field]), field):
                raise ValueError(f"Target/sample scope differs: {field}")
        for root in (project, old_root):
            if not (root / "meta/integration.json").is_file():
                raise ValueError("Sample generation identity unavailable")
        plan = _configuration(project, source_tree=source_tree)
        previous_plan = mapped(_configuration(old_root, source_tree=source_tree))
        output_plan = _configuration(output / "project", source_tree=source_tree)
        if (normalized(plan) != normalized(previous_plan)
                or normalized(plan) != normalized(output_plan)):
            raise ValueError("Sample generation configuration differs")
        def programs(summary, rename=False):
            rows = [(row["name"], renames.get(row["entry"], row["entry"])
                     if rename else row["entry"],
                     normalized(mapped(row["translationUnits"]) if rename
                                else row["translationUnits"], "translationUnits"))
                    for row in summary["programs"]]
            return sorted(rows) if source_tree else rows
        if programs(current) != programs(previous, rename=True):
            raise ValueError("Program identity differs")
        current_identity, previous_identity = _semantic(current), _semantic(previous)
        if source_tree:
            _compare_source_sdk(current, previous, renames)
        for field in ("compiler", "defines", "models", "libraries", "analysisCode", "addonRuntime", "sdk"):
            if source_tree and field in {"defines", "sdk"}:
                continue
            previous_value = previous_identity[field]
            if field == "defines":
                previous_value = mapped(previous_value)
            if current_identity[field] != previous_value:
                raise ValueError(f"Semantic identity differs: {field}")
        now = diagnostic_counts(current, output / "project")
        before = Counter()
        for (rule, name, source, program), count in diagnostic_counts(previous, old_root).items():
            before[rule, renames.get(name, name), source, program] += count
        def rows(counts):
            if source_tree:
                return [{"rule": rule, "file": name, "source": source,
                         "program": program, "count": count}
                        for (rule, name, source, program), count in sorted(counts.items())]
            locations = Counter()
            for (rule, name, source, _program), count in counts.items():
                locations[rule, name, source] += count
            return [{"rule": rule, "file": name, "source": source, "count": count}
                    for (rule, name, source), count in sorted(locations.items())]
        return {"new": rows(now - before), "removed": rows(before - now)}
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise ValueError(
            f"C baseline refused: {error}. Download the full c-analysis artifact with its project sources, "
            "or compute the matching baseline locally with c-check; local analysis is still required."
        ) from error
