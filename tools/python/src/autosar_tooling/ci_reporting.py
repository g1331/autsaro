"""Display CI failures and scanner artifacts without changing their verdicts."""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import os
import re
import shutil
import subprocess
from collections import Counter
from pathlib import Path, PurePosixPath
from urllib.parse import quote

from autosar_tooling import c_fingerprint

# GitHub truncates displayed results above this limit even when upload succeeds.
SARIF_RESULTS_PER_RUN = 5000
SARIF_RUNS_PER_FILE = 20
CPPCHECK_RUNS_PER_PROGRAM = SARIF_RUNS_PER_FILE // 2


def summary(title: str, lines: list[str]) -> None:
    destination = os.environ.get("GITHUB_STEP_SUMMARY")
    if destination:
        with Path(destination).open("a", encoding="utf-8") as output:
            output.write(f"### {html.escape(title)}\n\n")
            output.write("\n".join(f"- {html.escape(line)}" for line in lines) + "\n\n")


def failure_names(text: str) -> list[str]:
    patterns = (
        r"^(?:FAIL|ERROR): (.+)$",  # unittest, including subtests
        r"^test (.+?) \.{3} FAILED$",  # Cargo
        r"^\s*(?:FAIL|❯)\s+(.+)$",  # Vitest FAIL and failed suites
    )
    names = []
    for line in text.splitlines():
        line = re.sub(r"\x1b\[[0-9;]*m", "", line)
        for pattern in patterns:
            match = re.match(pattern, line)
            if match and match[1] not in names:
                names.append(match[1])
    return names


def requirement_lines(text: str) -> list[str]:
    lines = []
    listing = False
    scopes = undisposed = stale = 0
    for line in text.splitlines():
        counts = re.search(
            r"official items;.*undisposed (\d+); (\d+) stale citations", line
        )
        if counts:
            scopes += 1
            undisposed += int(counts[1])
            stale += int(counts[2])
            if int(counts[1]) or int(counts[2]):
                lines.append(line)
            listing = False
        elif line.startswith(
            (
                "Undisposed items:",
                "Stale citations (",
                "Configuration conditions without a Rejects:",
            )
        ):
            lines.append(line)
            listing = True
        elif listing and line.startswith("  "):
            lines.append(line.strip())
        elif not line.strip():
            listing = False
        elif line.startswith("error:"):
            lines.append(line)
    if scopes:
        lines.insert(
            0,
            f"Requirements: {scopes} accounting scopes; undisposed {undisposed}; {stale} stale citations.",
        )
    return lines


def run(command: list[str]) -> int:
    """Stream the normal command and return its exact exit status."""
    executable = shutil.which(command[0]) or command[0]
    process = subprocess.Popen(
        [executable, *command[1:]],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    lines = []
    assert process.stdout is not None
    for line in process.stdout:
        print(line, end="", flush=True)
        lines.append(line)
    process.stdout.close()
    code = process.wait()
    text = "".join(lines)
    details = [f"Exit status: {code}"]
    details.extend(f"Failed test: {name}" for name in failure_names(text))
    details.extend(requirement_lines(text))
    if code and len(details) == 1:
        details.append(
            "Command failed before a test name was reported; see the step log."
        )
    summary("Command result: " + " ".join(command), details)
    return code


def relative(name: str) -> str:
    path = PurePosixPath(name.replace("\\", "/"))
    if (
        path.is_absolute()
        or ".." in path.parts
        or not path.parts
        or ":" in path.parts[0]
    ):
        raise ValueError(f"Scanner path is not repository relative: {name}")
    return path.as_posix()


def location(
    name: str,
    line: int,
    project: Path | None,
    repository: Path,
    origins: dict[str, str],
) -> tuple[str, int]:
    """Map delivered runtime copies to their real checkout source, never to an emitter."""
    name = relative(name)
    if line < 1:
        raise ValueError("Scanner line must be positive")
    if project is None:
        return name, line
    if name in origins:
        original = relative(origins[name])
        # Copies can carry a generated preamble. Only exact source text can relocate a line.
        source = (project / name).read_text(encoding="utf-8").splitlines()
        target = (repository / original).read_text(encoding="utf-8").splitlines()
        if line <= len(source):
            if line <= len(target) and source[line - 1] == target[line - 1]:
                return original, line
            matches = [
                index
                for index, text in enumerate(target, 1)
                if text == source[line - 1]
            ]
            if len(matches) == 1:
                return original, matches[0]
    return (project.relative_to(repository) / name).as_posix(), line


def sarif_run(rows: list[dict], tool: str, category: str | None) -> dict:
    results = []
    # SARIF 2.1.0 requires baselineState on either all results in a run or none.
    complete_comparison = all("baselineState" in row for row in rows)
    for row in rows:
        name = relative(row["file"])
        line = int(row["line"])
        if line < 1:
            raise ValueError("Scanner line must be positive")
        results.append(
            {
                "ruleId": row["rule"],
                "level": "warning",
                "message": {"text": row["message"]},
                **({"partialFingerprints": row["partialFingerprints"]} if "partialFingerprints" in row else {}),
                **({"baselineState": row["baselineState"]} if complete_comparison else {}),
                "locations": [
                    {
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": quote(name, safe="/"),
                                "uriBaseId": "%SRCROOT%",
                            },
                            "region": {"startLine": line},
                        }
                    }
                ],
            }
        )
    return {"tool": {"driver": {"name": tool}}, "results": results,
            **({"automationDetails": {"id": category + "/"}} if category else {})}


def sarif(rows: list[dict], tool: str, *, categories: tuple[str, ...] = ()) -> dict:
    groups: dict[str | None, list[dict]] = {category: [] for category in categories}
    for row in rows:
        groups.setdefault(row.get("category"), []).append(row)
    runs = []
    for category, group in groups.items():
        parts = max(1, (len(group) + SARIF_RESULTS_PER_RUN - 1) // SARIF_RESULTS_PER_RUN)
        if category in categories:
            # Publish every slot after a complete scan, even when its previous
            # findings moved or disappeared. Actions jobs have no shared state.
            if parts > CPPCHECK_RUNS_PER_PROGRAM:
                raise ValueError(f"SARIF exceeds {CPPCHECK_RUNS_PER_PROGRAM} runs for {category}")
            parts = CPPCHECK_RUNS_PER_PROGRAM
        for part in range(1, parts + 1):
            part_category = category
            if part > 1 or (category is None and parts > 1):
                part_category = f"{category or tool}-part-{part}"
            start = (part - 1) * SARIF_RESULTS_PER_RUN
            runs.append(sarif_run(group[start:start + SARIF_RESULTS_PER_RUN], tool, part_category))
    if len(runs) > SARIF_RUNS_PER_FILE:
        raise ValueError("SARIF exceeds GitHub's run limit; distribute samples across files")
    return {
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": runs or [sarif_run([], tool, None)],
    }


def fingerprint_comparison(path: Path, current_bytes: bytes, current: dict,
                           baseline: Path | None) -> tuple[dict, set, set]:
    gate = json.loads(path.with_name("gate.json").read_bytes())
    preview = gate["diagnosticFingerprints"]
    if preview["format"] != c_fingerprint.FORMAT:
        raise ValueError("Unsupported diagnostic fingerprint report")
    baseline_path = baseline if baseline is not None else path.with_name("baseline-summary.json")
    baseline_bytes = baseline_path.read_bytes()
    for side, content in (("current", current_bytes), ("baseline", baseline_bytes)):
        if gate["summarySha256"][side] != hashlib.sha256(content).hexdigest():
            raise ValueError(f"Gate {side} summary digest differs")
    c_fingerprint.validate_occurrences(current, preview["current"])
    c_fingerprint.validate_occurrences(json.loads(baseline_bytes), preview["baseline"])
    fingerprints = {(row["diagnostic"], row["locationIndex"], row["program"]): row["fingerprint"]
                    for row in preview["current"]}
    before = {row["fingerprint"] for row in preview["baseline"]}
    if preview.get("mode") == "pr":
        before.update(preview.get("replayed", {}))
    incomplete = {row["program"] for side in ("current", "baseline") for row in preview[side]
                  if row["fingerprint"] is None}
    return fingerprints, before, incomplete


def cppcheck_category(prefix: str, program: str) -> str:
    # HostBatch keeps the pre-split category so its next run updates those alerts.
    return prefix if program == "host-batch" else f"{prefix}-{program}"


def cppcheck(
    directory: Path, repository: Path, baseline: Path | None = None, *,
    categories: set[str] | None = None,
) -> list[dict]:
    repository = repository.absolute().resolve()
    directory = (repository / directory).resolve()
    rows = []
    details = []
    for path in sorted(directory.glob("*/summary.json")):
        current_bytes = path.read_bytes()
        data = json.loads(current_bytes)
        project = path.parent / "project"
        details.append(
            f"{path.parent.name}: source passed={data['passed']}; analysis error={data['error']}"
        )
        if classification := data.get("sourceClassification"):
            details.append(
                f"SDK source provenance: complete={classification['complete']}; "
                f"SDK diagnostic keys={classification['sdkDiagnosticKeys']}; "
                f"compiler predefine keys={classification.get('compilerPredefineDiagnosticKeys', 'unknown')}; "
                f"unclassified keys={classification['unclassifiedDiagnosticKeys']}; "
                f"unclassified headers={classification['unclassifiedHeaders']}; "
                f"unresolved macro diagnostics={classification['unresolvedMacroDiagnostics']}. "
                "Source classification does not assess rule satisfaction or provider conformance."
            )
        comparison = data.get("comparison")
        if comparison is None and baseline is not None:
            from autosar_tooling.c_comparison import compare

            try:
                comparison = compare(
                    data, project, path.parent, baseline / path.parent.name
                )
            except ValueError as error:
                details.append(str(error))
        if comparison is None:
            details.append(
                "New diagnostics: unknown (no accepted baseline comparison)."
            )
        else:
            details.append(
                f"New diagnostics: {sum(row['count'] for row in comparison['new'])} occurrences."
            )
        new = {
            (row["rule"], row["file"], row["source"])
            for row in (comparison or {}).get("new", [])
        }
        metadata = project / "meta/integration.json"
        origins = (
            {
                copy: source
                for source, copy in json.loads(metadata.read_text(encoding="utf-8"))[
                    "originalBswSources"
                ].items()
            }
            if metadata.is_file()
            else {}
        )
        fingerprint_rows = {}
        previous_fingerprints = set()
        incomplete = set()
        gate = path.with_name("gate.json")
        if gate.is_file():
            try:
                fingerprint_rows, previous_fingerprints, incomplete = fingerprint_comparison(
                    path, current_bytes, data, baseline / path.parent.name / "summary.json"
                    if baseline is not None else None,
                )
            except (OSError, ValueError, KeyError, TypeError) as error:
                details.append(f"Fingerprint comparison unavailable for {path.parent.name}: {error}")
        target = data.get("scope", {}).get("target", "unknown-target")
        prefix = f"cppcheck-{target}-{path.parent.name}"
        if categories is not None and data["error"] is None:
            categories.update(cppcheck_category(prefix, row["name"]) for row in data.get("programs", []))
        for index, diagnostic in enumerate(data.get("diagnostics", [])):
            if diagnostic["id"] == "checkersReport":
                continue
            for position, place in enumerate(diagnostic["locations"]):
                if not place.get("line") or int(place["line"]) < 1:
                    continue
                name, line = location(
                    place["file"], int(place["line"]), project, repository, origins
                )
                row = {
                    "rule": diagnostic["id"],
                    "file": name,
                    "line": line,
                    "message": diagnostic["message"],
                }
                for program in diagnostic.get("programs") or ("raw",):
                    item = {**row, "category": cppcheck_category(prefix, program)}
                    fingerprint = fingerprint_rows.get((index, position, program))
                    if fingerprint is not None:
                        item["partialFingerprints"] = {c_fingerprint.SARIF_KEY: fingerprint}
                        if program not in incomplete:
                            item["baselineState"] = (
                                "unchanged" if fingerprint in previous_fingerprints else "new"
                            )
                    rows.append(item)
                source = (
                    (project / relative(place["file"]))
                    .read_text(encoding="utf-8")
                    .splitlines()[int(place["line"]) - 1]
                )
                if (diagnostic["id"], place["file"], source) in new:
                    details.append(f"{diagnostic['id']} {name}:{line}")
    if not details:
        details.append("No analysis summaries produced; see the analysis step log.")
    summary("Cppcheck / MISRA diagnostics", details)
    return rows


def clang_tidy(path: Path, samples: Path, repository: Path, outcome: str) -> list[dict]:
    from autosar_tooling.quality import (
        CLANG_TIDY_FOREIGN,
        CLANG_TIDY_RUNTIME_ONLY,
        clang_tidy_comparison,
        read_clang_tidy_baseline,
    )

    repository = repository.absolute().resolve()
    samples = (repository / samples).resolve()
    raw = json.loads(path.read_text(encoding="utf-8"))
    rows = []
    per_sample = {}
    seen = set()
    positions = {}
    mappings = {}
    for item in raw:
        name = relative(item["file"])
        sample = relative(item["sample"])
        if sample not in mappings:
            metadata = samples / sample / "meta/integration.json"
            mappings[sample] = (
                json.loads(metadata.read_text(encoding="utf-8"))["originalBswSources"]
                if metadata.is_file()
                else {}
            )
        original = name in mappings[sample] or name.startswith("runtime/")
        delivered = mappings[sample].get(name, name)
        source = (repository / name if original else samples / sample / name).resolve()
        line = int(item["line"])
        text = source.read_text(encoding="utf-8").splitlines()[line - 1].strip()
        key = (item["rule"], name, text)
        position = (item["sample"], name, line, item.get("column", 1), item["rule"])
        eligible = not delivered.startswith(CLANG_TIDY_FOREIGN) and (
            item["rule"] not in CLANG_TIDY_RUNTIME_ONLY or original
        )
        if eligible and position not in seen:
            seen.add(position)
            per_sample.setdefault(item["sample"], Counter())[key] += 1
        mapped = name if original else source.relative_to(repository).as_posix()
        row = {**item, "file": mapped}
        if row not in rows:
            rows.append(row)
        if eligible:
            positions.setdefault(key, set()).add(f"{mapped}:{line}")
    current = Counter()
    for counts in per_sample.values():
        current |= counts
    # Derive display identities through the same normalization as the baseline verdict.
    normalized_positions = {}
    for key, places in positions.items():
        for message in clang_tidy_comparison(Counter({key: 1}), Counter()):
            normalized_positions.setdefault(message, set()).update(places)
    errors = clang_tidy_comparison(current, read_clang_tidy_baseline())
    details = [
        f"Analysis step: {outcome}; raw findings: {len(raw)} (includes existing baseline diagnostics)."
    ]
    if outcome != "success":
        details.append(
            "Analysis did not complete successfully; this report may be partial."
        )
    for error in errors:
        if outcome != "success" and ": fixed finding, remove:" in error:
            continue
        details.append(
            error
            + (
                "; " + ", ".join(sorted(normalized_positions[error]))
                if error in normalized_positions
                else ""
            )
        )
    if not errors:
        details.append("No new or fixed baseline findings.")
    summary("clang-tidy diagnostics", details)
    return rows


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    execution = commands.add_parser(
        "run", help="Stream a normal check, summarize it and preserve its exit status"
    )
    execution.add_argument("command", nargs=argparse.REMAINDER)
    report = commands.add_parser(
        "sarif", help="Convert existing scanner JSON artifacts"
    )
    report.add_argument("--cppcheck-directory", type=Path)
    report.add_argument("--baseline-directory", type=Path)
    report.add_argument(
        "--samples", type=Path, default=Path("build/c-analysis-samples")
    )
    report.add_argument("--clang-tidy", type=Path)
    report.add_argument("--analysis-outcome", default="unknown")
    report.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.action == "run":
        command = args.command[1:] if args.command[:1] == ["--"] else args.command
        if not command:
            parser.error("run requires a command after --")
        return run(command)
    if bool(args.cppcheck_directory) == bool(args.clang_tidy):
        parser.error("select exactly one scanner input")
    categories: set[str] = set()
    if args.cppcheck_directory:
        rows = cppcheck(
            args.cppcheck_directory.resolve(), Path.cwd(), args.baseline_directory,
            categories=categories,
        )
        tool = "Cppcheck / MISRA"
    else:
        rows = clang_tidy(
            args.clang_tidy, args.samples, Path.cwd(), args.analysis_outcome
        )
        tool = "clang-tidy"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(sarif(rows, tool, categories=tuple(sorted(categories))), ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
