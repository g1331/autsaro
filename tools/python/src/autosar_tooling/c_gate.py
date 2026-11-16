"""Fresh two-tree C source gate used by the required CI analysis shards."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import re
import shutil
import subprocess
from collections import Counter
from pathlib import Path

from autosar_tooling import c_acceptance, c_comparison
from autosar_tooling.config import ROOT

SAMPLES = ("standard-ecu", "user-application", "component-pipeline", "no-diagnostic")
TARGETS = ("linux-x64-controlled-v1", "windows-x64-controlled-v1")
FORMAT = "c-source-no-regression-v1"


def read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def baseline_source(source: dict, parent: str) -> dict:
    if source["tree"] != parent or source["kind"] not in {"same-run", "master-artifact"}:
        raise ValueError("Baseline analysis source identity differs")
    if source["kind"] == "master-artifact" and (
            any(type(source.get(key)) is not int or source[key] <= 0
                for key in ("run", "attempt", "job", "artifact"))
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", source.get("digest", ""))):
        raise ValueError("Baseline artifact provenance missing or invalid")
    return source


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT).decode("utf-8").strip()


def stage(baseline: Path) -> None:
    """Use current sample inputs with the first parent's generator and runtime."""
    parent = git("rev-parse", "HEAD^1")
    actual = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=baseline, text=True,
    ).strip()
    if actual != parent:
        raise ValueError("Baseline checkout must be the tested commit's first parent")
    # Tests and fixtures define the sample selection; production Rust/runtime stay old.
    inputs = ("core/tests", "core/resources", "Cargo.toml", "Cargo.lock",
              "core/Cargo.toml", "src-tauri/Cargo.toml", "rust-toolchain.toml", ".cargo")
    for name in inputs:
        source, destination = ROOT / name, baseline / name
        if destination.is_dir():
            shutil.rmtree(destination)
        elif destination.exists():
            destination.unlink()
        if source.is_dir():
            shutil.copytree(source, destination)
        elif source.exists():
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)


def prepare(baseline: Path, output: Path | None = None) -> None:
    stage(baseline)
    output = output or ROOT / "build/c-analysis-base-samples"
    environment = {**os.environ,
                   "AUTOSAR_C_ANALYSIS_SAMPLES": str(output.resolve()),
                   "CARGO_TARGET_DIR": str(baseline.resolve() / "target")}
    subprocess.run(["cargo", "test", "--locked", "-p", "autosar-config-core", "--test", "it",
                    "--", "c_analysis::"], cwd=baseline, env=environment, check=True)
    found = {path.name for path in output.iterdir()}
    if found != set(SAMPLES):
        raise ValueError(f"Generated baseline sample inventory differs: {found}")


def rename_paths(previous: Path, current: Path) -> dict[str, str]:
    """Translate Git origins into the corresponding sealed delivery paths."""
    fields = git("diff", "--name-status", "-z", "--find-renames", "HEAD^1", "HEAD").split("\0")
    mappings = {}
    index = 0
    while index < len(fields) and fields[index]:
        status = fields[index]
        index += 1
        if status.startswith("R"):
            mappings[fields[index]] = fields[index + 1]
            index += 2
        else:
            index += 1
    old = read(previous / "meta/integration.json").get("originalBswSources", {})
    new = read(current / "meta/integration.json").get("originalBswSources", {})
    for origin, delivered in old.items():
        renamed = mappings.get(origin, origin)
        if renamed not in new and origin.startswith("runtime/os/"):
            # A split source has one selected provider in each delivered target.
            candidates = [name for name in new if name.startswith("runtime/os/src/host/")
                          and Path(name).name == Path(origin).name]
            if len(candidates) == 1:
                renamed = candidates[0]
                mappings[origin] = renamed
        if renamed in new and new[renamed] != delivered:
            mappings[delivered] = new[renamed]
    return mappings


def new_dispositions(current: dict, acceptance: dict, project: Path,
                     baseline: Path, renames: dict[str, str]) -> Counter:
    """An existing position's approval cannot authorize an identical added position."""
    previous = read(baseline / "summary.json")
    old_positions = Counter()
    for diagnostic in previous["diagnostics"]:
        if diagnostic["id"] == "checkersReport":
            continue
        for location in diagnostic["locations"]:
            name = renames.get(location["file"], location["file"])
            for program in diagnostic["programs"]:
                old_positions[diagnostic["id"], name, int(location["line"]), program] += 1
    old_names = {new: old for old, new in renames.items()}
    line_maps = {}
    accepted = Counter()
    for occurrence in acceptance["occurrences"]:
        if occurrence["assessment"] != "accepted":
            continue
        diagnostic = current["diagnostics"][occurrence["diagnostic"]]
        location = diagnostic["locations"][occurrence["locationIndex"]]
        name, line = location["file"], int(location["line"])
        if name not in line_maps:
            old_path = baseline / "project" / old_names.get(name, name)
            old_lines = (old_path.read_text(encoding="utf-8").split("\n")
                         if old_path.is_file() else [])
            new_lines = (project / name).read_text(encoding="utf-8").split("\n")
            matcher = difflib.SequenceMatcher(None, old_lines, new_lines, autojunk=False)
            line_maps[name] = {new + offset + 1: old + offset + 1
                               for old, new, size in matcher.get_matching_blocks()
                               for offset in range(size)}
        programs = []
        for program in diagnostic["programs"]:
            position = diagnostic["id"], name, line_maps[name].get(line), program
            if old_positions[position]:
                old_positions[position] -= 1
            else:
                programs.append(program)
        row = {"diagnostics": [{**diagnostic, "locations": [location], "programs": programs}]}
        accepted.update(c_comparison.diagnostic_counts(row, project))
    return accepted


def verdict(current: dict, output: Path, baseline: Path, renames: dict[str, str]) -> dict:
    """Reuse comparison counts and the current, independently recomputed disposition."""
    binary = current["tool"].get("binarySha256")
    if not binary or binary != read(baseline / "summary.json")["tool"].get("binarySha256"):
        raise ValueError("Two-tree scans require the same analyzer binary")
    comparison = c_comparison.compare(
        current, output / "project", output, baseline, source_tree=True, renames=renames,
    )
    acceptance = c_acceptance.evaluate(current, output / "project")
    if acceptance["status"] not in {"accepted", "blocked"}:
        raise ValueError("Project acceptance execution failed")
    if acceptance != current["projectAcceptance"]:
        raise ValueError("Project acceptance receipt differs from current inputs")
    accepted = new_dispositions(current, acceptance, output / "project", baseline, renames)
    previous = read(baseline / "summary.json")
    inside = c_comparison.diagnostic_counts(current, output / "project", in_scope_only=True)
    before = Counter()
    for (rule, name, source, program), count in c_comparison.diagnostic_counts(
            previous, baseline / "project", in_scope_only=True).items():
        before[rule, renames.get(name, name), source, program] += count
    additions = inside - before
    blocked, outside = [], []
    for row in comparison["new"]:
        key = row["rule"], row["file"], row["source"], row["program"]
        count = min(row["count"], additions[key])
        if row["count"] > count:
            outside.append({**row, "count": row["count"] - count})
        remaining = count - accepted[key]
        if remaining > 0:
            blocked.append({**row, "count": remaining})
    return {"status": "blocked" if blocked else "passed", "comparison": comparison,
            "unacceptedNew": blocked, "outOfScopeNew": outside,
            "complianceScope": c_comparison.scope_statistics(current),
            "projectAcceptance": acceptance["status"]}


def scan(target: str, sample: str) -> None:
    outputs = {}
    parent = git("rev-parse", "HEAD^1")
    # Same-run c-check producers already validate actual target semantics before
    # and after scanning. Rebind transferred source bytes to those fresh inputs.
    for side, results in (("baseline", "c-analysis-baseline"),
                          ("current", "c-analysis-results")):
        output = ROOT / "build" / results / sample
        summary = read(output / "summary.json")
        c_acceptance._complete(summary)
        if (summary["scope"]["target"] != target
                or summary["projectAcceptance"]["status"] not in {"accepted", "blocked"}):
            raise ValueError(f"Incomplete {side} analysis: {sample}")
        identity = c_acceptance.input_identity(
            summary, output / "project", producer_revision=parent if side == "baseline" else None)
        if identity != summary["projectAcceptance"]["inputIdentity"]:
            raise ValueError(f"Transferred {side} analysis inputs changed: {sample}")
        outputs[side] = output
    current, baseline = outputs["current"], outputs["baseline"]
    source = baseline_source(read(baseline / "baseline-source.json"), parent)
    shutil.copy2(baseline / "summary.json", current / "baseline-summary.json")
    result = verdict(read(current / "summary.json"), current, baseline,
                     rename_paths(baseline / "project", current / "project"))
    result.update(format=FORMAT, target=target, sample=sample,
                  baselineSource=source,
                  commit=git("rev-parse", "HEAD"), baseline=git("rev-parse", "HEAD^1"),
                  summarySha256={side: sha(path / "summary.json")
                                 for side, path in outputs.items()},
                  inputIdentity={side: read(path / "summary.json")["projectAcceptance"][
                      "inputIdentity"] for side, path in outputs.items()})
    (current / "gate.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2))
    if result["status"] != "passed":
        raise ValueError(f"New source diagnostics without current precise disposition: {sample}")


def aggregate(directory: Path) -> None:
    expected = {(target, sample) for target in TARGETS for sample in SAMPLES}
    found, identities = set(), set()
    matrix = []
    for path in directory.glob("*/*/gate.json"):
        gate = read(path)
        pair = gate["target"], gate["sample"]
        if (gate["format"] != FORMAT or gate["status"] != "passed"
                or pair not in expected or pair in found
                or gate["commit"] != git("rev-parse", "HEAD")
                or gate["baseline"] != git("rev-parse", "HEAD^1")):
            raise ValueError(f"Source gate identity missing, duplicated or refused: {path}")
        baseline_source(gate["baselineSource"], gate["baseline"])
        for side, summary_path in (("current", path.with_name("summary.json")),
                                   ("baseline", path.with_name("baseline-summary.json"))):
            summary = read(summary_path)
            c_acceptance._complete(summary)
            identity = summary["projectAcceptance"]["inputIdentity"]
            if (not isinstance(identity, str) or len(identity) != 64
                    or (side, gate["target"], identity) in identities
                    or gate["inputIdentity"][side] != identity
                    or gate["summarySha256"][side] != sha(summary_path)
                    or summary["scope"]["target"] != gate["target"]):
                raise ValueError(f"Source gate analysis identity invalid: {summary_path}")
            identities.add((side, gate["target"], identity))
        matrix.append((gate, read(path.with_name("summary.json"))))
        found.add(pair)
    if found != expected or len(list(directory.glob("*/*/summary.json"))) != len(expected):
        raise ValueError(f"Source gate matrix incomplete: {sorted(expected - found)}")
    headers = c_comparison.target_headers([summary["scope"] for _gate, summary in matrix])
    statistics = []
    for gate, summary in matrix:
        # A header used by any selected target TU is in scope for every shard.
        promoted = [row for row in gate["outOfScopeNew"]
                    if row["file"].startswith("integration/")
                    and not row["file"].endswith(".c") and row["file"] in headers]
        if promoted:
            raise ValueError(f"New target-header diagnostics in matrix closure: {promoted}")
        statistics.append({"target": gate["target"], "sample": gate["sample"],
                           **c_comparison.scope_statistics(summary, headers=headers)})
    (directory / "compliance-scope.json").write_text(
        json.dumps({"targetHeaders": sorted(headers), "analyses": statistics}, indent=2) + "\n",
        encoding="utf-8")



def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    preparation = commands.add_parser("prepare")
    preparation.add_argument("baseline", type=Path)
    preparation.add_argument("--output-directory", type=Path)
    staging = commands.add_parser("stage")
    staging.add_argument("baseline", type=Path)
    scanning = commands.add_parser("scan")
    scanning.add_argument("--target", choices=TARGETS, required=True)
    scanning.add_argument("--sample", choices=SAMPLES, required=True)
    aggregation = commands.add_parser("aggregate")
    aggregation.add_argument("directory", type=Path)
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.baseline, args.output_directory)
    elif args.command == "stage":
        stage(args.baseline)
    elif args.command == "scan":
        scan(args.target, args.sample)
    else:
        aggregate(args.directory)


if __name__ == "__main__":
    main()
