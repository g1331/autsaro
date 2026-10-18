"""Display CI failures and scanner artifacts without changing their verdicts."""

from __future__ import annotations

import argparse
import html
import json
import os
import re
import shutil
import subprocess
from collections import Counter
from pathlib import Path, PurePosixPath
from urllib.parse import quote


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


def sarif(rows: list[dict], tool: str) -> dict:
    results = []
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
    return {
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{"tool": {"driver": {"name": tool}}, "results": results}],
    }


def cppcheck(
    directory: Path, repository: Path, baseline: Path | None = None
) -> list[dict]:
    rows = []
    details = []
    for path in sorted(directory.glob("*/summary.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        project = path.parent / "project"
        details.append(
            f"{path.parent.name}: source passed={data['passed']}; analysis error={data['error']}"
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
        for diagnostic in data.get("diagnostics", []):
            if diagnostic["id"] == "checkersReport":
                continue
            for place in diagnostic["locations"]:
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
                if row not in rows:
                    rows.append(row)
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

    samples, repository = samples.resolve(), repository.resolve()
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
        source = repository / name if original else samples / sample / name
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
        positions.setdefault(
            f"{name}: new clang-tidy finding [{item['rule']}]: {text}", set()
        ).add(f"{mapped}:{line}")
    current = Counter()
    for counts in per_sample.values():
        current |= counts
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
            + ("; " + ", ".join(sorted(positions[error])) if error in positions else "")
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
    if args.cppcheck_directory:
        rows = cppcheck(
            args.cppcheck_directory.resolve(), Path.cwd(), args.baseline_directory
        )
        tool = "Cppcheck / MISRA"
    else:
        rows = clang_tidy(
            args.clang_tidy, args.samples, Path.cwd(), args.analysis_outcome
        )
        tool = "clang-tidy"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(sarif(rows, tool), ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
