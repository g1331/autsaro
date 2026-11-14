"""Fresh two-tree C source gate used by the required CI analysis shards."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

from autosar_tooling import c_acceptance, c_check, c_comparison
from autosar_tooling.config import ROOT

SAMPLES = ("standard-ecu", "user-application", "component-pipeline", "no-diagnostic")
TARGETS = ("linux-x64-controlled-v1", "windows-x64-controlled-v1")
FORMAT = "c-source-no-regression-v1"


def read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT).decode("utf-8").strip()


def prepare(baseline: Path) -> None:
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
    environment = {**os.environ,
                   "AUTOSAR_C_ANALYSIS_SAMPLES": str(ROOT / "build/c-analysis-base-samples"),
                   "CARGO_TARGET_DIR": str(ROOT / "target")}
    subprocess.run(["cargo", "test", "--locked", "-p", "autosar-config-core", "--test", "it",
                    "--", "c_analysis::"], cwd=baseline, env=environment, check=True)
    for name in ("c-analysis-samples", "c-analysis-base-samples"):
        found = {path.name for path in (ROOT / "build" / name).iterdir()}
        if found != set(SAMPLES):
            raise ValueError(f"Generated sample inventory differs: {name}: {found}")


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
        if renamed in new and new[renamed] != delivered:
            mappings[delivered] = new[renamed]
    return mappings


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
    accepted = Counter()
    for occurrence in acceptance["occurrences"]:
        if occurrence["assessment"] == "accepted":
            diagnostic = current["diagnostics"][occurrence["diagnostic"]]
            location = diagnostic["locations"][occurrence["locationIndex"]]
            # The shared counter supplies exactly the same text and program semantics.
            row = {"diagnostics": [{**diagnostic, "locations": [location]}]}
            accepted.update(c_comparison.diagnostic_counts(row, output / "project"))
    blocked = []
    for row in comparison["new"]:
        key = row["rule"], row["file"], row["source"], row["program"]
        remaining = row["count"] - accepted[key]
        if remaining > 0:
            blocked.append({**row, "count": remaining})
    return {"status": "blocked" if blocked else "passed", "comparison": comparison,
            "unacceptedNew": blocked, "projectAcceptance": acceptance["status"]}


def scan(target: str, sample: str) -> None:
    outputs = {}
    with tempfile.TemporaryDirectory(prefix="c-gate-tool-") as temporary:
        tool = c_check.tool_identity(Path(temporary))
        for side, inputs, results in (
            ("baseline", "c-analysis-base-samples", "c-analysis-baseline"),
            ("current", "c-analysis-samples", "c-analysis-results"),
        ):
            output = ROOT / "build" / results / sample
            command = [sys.executable, "-m", "autosar_tooling", "c-check", "--target", target,
                       "--project", str(ROOT / "build" / inputs / sample),
                       "--output-directory", str(output)]
            result = subprocess.run(command, cwd=ROOT, check=False) if side == "baseline" else None
            summary = read(output / "summary.json")
            c_acceptance._complete(summary)
            if (result is not None and result.returncode != (0 if summary["passed"] else 1)
                    or summary["scope"]["target"] != target
                    or summary["projectAcceptance"]["status"] not in {"accepted", "blocked"}):
                raise ValueError(f"Incomplete {side} analysis: {sample}")
            c_check.recheck_semantic_inputs(summary, output / "project", tool)
            outputs[side] = output
    current, baseline = outputs["current"], outputs["baseline"]
    shutil.copy2(baseline / "summary.json", current / "baseline-summary.json")
    result = verdict(read(current / "summary.json"), current, baseline,
                     rename_paths(baseline / "project", current / "project"))
    result.update(format=FORMAT, target=target, sample=sample,
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
    for path in directory.glob("*/*/gate.json"):
        gate = read(path)
        pair = gate["target"], gate["sample"]
        if (gate["format"] != FORMAT or gate["status"] != "passed"
                or pair not in expected or pair in found
                or gate["commit"] != git("rev-parse", "HEAD")
                or gate["baseline"] != git("rev-parse", "HEAD^1")):
            raise ValueError(f"Source gate identity missing, duplicated or refused: {path}")
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
        found.add(pair)
    if found != expected or len(list(directory.glob("*/*/summary.json"))) != len(expected):
        raise ValueError(f"Source gate matrix incomplete: {sorted(expected - found)}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    preparation = commands.add_parser("prepare")
    preparation.add_argument("baseline", type=Path)
    scanning = commands.add_parser("scan")
    scanning.add_argument("--target", choices=TARGETS, required=True)
    scanning.add_argument("--sample", choices=SAMPLES, required=True)
    aggregation = commands.add_parser("aggregate")
    aggregation.add_argument("directory", type=Path)
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.baseline)
    elif args.command == "scan":
        scan(args.target, args.sample)
    else:
        aggregate(args.directory)


if __name__ == "__main__":
    main()
