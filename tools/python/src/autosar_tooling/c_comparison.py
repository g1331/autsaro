"""Compare complete C analyses by diagnostic source text, retaining occurrences."""

from __future__ import annotations

import json
import re
from collections import Counter
from pathlib import Path

from ecu_tools.build import FILE_HASHES, digest, refuse_links, safe_relative


def _read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _counts(summary: dict, root: Path) -> Counter:
    counts = Counter()
    for diagnostic in summary["diagnostics"]:
        if diagnostic["id"] == "checkersReport":
            continue
        locations = diagnostic["locations"]
        if not locations:
            raise ValueError(f"Diagnostic has no source location: {diagnostic['id']}")
        for location in locations:
            name = location["file"]
            if not safe_relative(name):
                raise ValueError(f"Diagnostic outside sample: {name}")
            lines = (root / name).read_text(encoding="utf-8").split("\n")
            line = int(location["line"])
            if not 1 <= line <= len(lines):
                raise ValueError(f"Diagnostic source line unavailable: {name}:{line}")
            counts[diagnostic["id"], name, lines[line - 1]] += len(diagnostic["programs"])
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
    require(isinstance(identity, dict) and set(identity) ==
            {"version", "compiler", "defines", "models", "libraries", "analysisCode", "addonRuntime"})
    require(type(identity["version"]) is int and identity["version"] == 1)
    compiler = identity["compiler"]
    require(isinstance(compiler, dict) and set(compiler) == {"machine", "release", "predefines", "builtins"})
    require(isinstance(compiler["machine"], str) and bool(compiler["machine"]))
    require(isinstance(compiler["release"], str) and bool(re.fullmatch(r"[0-9]+(?:\.[0-9]+)*", compiler["release"])))
    require(strings(compiler["predefines"], True) and strings(compiler["builtins"], True))
    defines = identity["defines"]
    require(isinstance(defines, dict) and set(defines) == set(summary["scope"]["translationUnits"]))
    require(all(strings(value) for value in defines.values()))
    hashes(identity["models"], ("platformModel", "platformLibrary"), True)
    hashes(identity["libraries"], ("std", "windows" if summary["scope"]["target"].startswith("windows") else "posix", "implicitStd"))
    hashes(identity["analysisCode"], ("c_check.py", "c_addon.py", "c_comparison.py", "build.py", "process.py", "owner.py", "launch.py", "windows_job.py"))
    hashes(identity["addonRuntime"], ("runaddon.py", "cppcheck.py", "cppcheckdata.py"))
    return identity


def _configuration(root: Path) -> dict:
    refuse_links(root / "meta/integration.json", root)
    metadata = _read(root / "meta/integration.json")
    plan = dict(metadata["plan"])
    if "runtimeSources" not in plan:
        if metadata.get("format") == "autosar-ecu-integration-v2":
            raise ValueError("Runtime source provenance missing")
        return plan
    sources, origins = plan["runtimeSources"], metadata.get("originalBswSources")
    if not isinstance(sources, dict) or not isinstance(origins, dict):
        raise ValueError("Runtime source provenance invalid")
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
    plan["runtimeSources"] = selected
    return plan


def compare(current: dict, project: Path, output: Path, baseline: Path) -> dict:
    """A baseline is evidence for comparison, never permission to skip analysis."""
    try:
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
        if _read(project / "target.json") != _read(old_root / "target.json"):
            raise ValueError("Target/sample configuration differs")
        for field in ("target", "profile", "entries", "compilerFlags", "includePaths",
                      "translationUnits", "supplemental", "patchedKernel"):
            if current["scope"][field] != previous["scope"][field]:
                raise ValueError(f"Target/sample scope differs: {field}")
        for root in (project, old_root):
            if not (root / "meta/integration.json").is_file():
                raise ValueError("Sample generation identity unavailable")
        plan = _configuration(project)
        if plan != _configuration(old_root) or plan != _configuration(output / "project"):
            raise ValueError("Sample generation configuration differs")
        def programs(summary):
            return [(row["name"], row["entry"], row["translationUnits"]) for row in summary["programs"]]
        if programs(current) != programs(previous):
            raise ValueError("Program identity differs")
        current_identity, previous_identity = _semantic(current), _semantic(previous)
        for field in ("compiler", "defines", "models", "libraries", "analysisCode", "addonRuntime"):
            if current_identity[field] != previous_identity[field]:
                raise ValueError(f"Semantic identity differs: {field}")
        now = _counts(current, output / "project")
        before = _counts(previous, old_root)
        def rows(counts):
            return [{"rule": rule, "file": name, "source": source, "count": count}
                    for (rule, name, source), count in sorted(counts.items())]
        return {"new": rows(now - before), "removed": rows(before - now)}
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise ValueError(
            f"C baseline refused: {error}. Download the full c-analysis artifact with its project sources, "
            "or compute the matching baseline locally with c-check; local analysis is still required."
        ) from error
