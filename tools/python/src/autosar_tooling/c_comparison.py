"""Compare complete C analyses by diagnostic source text, retaining occurrences."""

from __future__ import annotations

import json
from collections import Counter
from pathlib import Path

from ecu_tools.build import FILE_HASHES, digest, safe_relative


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
        if _read(project / "meta/integration.json")["plan"] != _read(old_root / "meta/integration.json")["plan"]:
            raise ValueError("Sample generation configuration differs")
        def programs(summary):
            return [(row["name"], row["entry"], row["translationUnits"]) for row in summary["programs"]]
        if programs(current) != programs(previous):
            raise ValueError("Program identity differs")
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
