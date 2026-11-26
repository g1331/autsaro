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

from autosar_tooling import c_acceptance, c_comparison, c_fingerprint, c_rename
from autosar_tooling.config import ROOT

SAMPLES = c_comparison.MATRIX_SAMPLES
TARGETS = c_comparison.MATRIX_TARGETS
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


def _bswm_baseline_inputs(baseline: Path) -> dict[str, bytes]:
    """Keep the first parent's callouts when its validator predates ConfirmMode."""
    originals = {}
    for name in ("core/resources/templates/standard-ecu/ecuc.arxml",
                 "core/tests/fixtures/component-pipeline/arxml/ecuc.arxml"):
        current, previous = ROOT / name, baseline / name
        if not current.is_file() or not previous.is_file():
            return {}
        original = previous.read_bytes()
        updated = c_comparison.upgraded_bswm_callouts(original, 0)
        if (updated is None
                or updated.replace(b"\r\n", b"\n") != current.read_bytes().replace(b"\r\n", b"\n")):
            return {}
        originals[name] = original
    return originals


def stage(baseline: Path) -> None:
    """Use current sample inputs with the first parent's generator and runtime."""
    parent = git("rev-parse", "HEAD^1")
    actual = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=baseline, text=True,
    ).strip()
    if actual != parent:
        raise ValueError("Baseline checkout must be the tested commit's first parent")
    baseline_callouts = _bswm_baseline_inputs(baseline)
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
    for name, content in baseline_callouts.items():
        (baseline / name).write_bytes(content)


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


def new_dispositions(current: dict, previous: dict, acceptance: dict, project: Path,
                     baseline: Path, renames: dict[str, str], *,
                     headers: set[str] | None, previous_headers: set[str] | None) -> Counter:
    """Match exact dispositions for additions to the in-scope diagnostic inventory."""
    old_positions = Counter()
    for diagnostic in previous["diagnostics"]:
        if diagnostic["id"] == "checkersReport":
            continue
        for location in diagnostic["locations"]:
            if c_comparison.location_scope(previous["scope"], location,
                                           headers=previous_headers) != "in-scope":
                continue
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
        if c_comparison.location_scope(current["scope"], location, headers=headers) != "in-scope":
            continue
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


def in_scope_additions(current: dict, previous: dict, project: Path, baseline_project: Path,
                       renames: dict[str, str], *, headers: set[str] | None,
                       previous_headers: set[str] | None) -> Counter:
    inside = c_comparison.diagnostic_counts(current, project, in_scope_only=True, headers=headers)
    before = Counter()
    for (rule, name, source, program), count in c_comparison.diagnostic_counts(
            previous, baseline_project, in_scope_only=True, headers=previous_headers).items():
        before[rule, renames.get(name, name), source, program] += count
    return inside - before


def verdict(current: dict, output: Path, baseline: Path, renames: dict[str, str],
            *, baseline_matrix: dict | None = None, identifier_evidence: dict | None = None,
            identifier_evidence_error: str | None = None) -> dict:
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
    previous = read(baseline / "summary.json")
    current_matrix = acceptance.get("scopeMatrix")
    headers = c_comparison.matrix_headers(current, current_matrix) if current_matrix is not None else None
    previous_headers = (c_comparison.matrix_headers(previous, baseline_matrix)
                        if baseline_matrix is not None else None)
    additions = in_scope_additions(current, previous, output / "project", baseline / "project",
                                   renames, headers=headers, previous_headers=previous_headers)
    accepted = new_dispositions(current, previous, acceptance, output / "project", baseline,
                                renames, headers=headers, previous_headers=previous_headers)
    equivalent = Counter()
    if identifier_evidence_error is None:
        try:
            equivalent = c_rename.equivalent(previous, current, baseline, output, comparison,
                                             identifier_evidence, previous_headers, headers)
        except c_rename.EvidenceError as error:
            identifier_evidence_error = str(error)
    blocked, outside = [], []
    for row in comparison["new"]:
        key = row["rule"], row["file"], row["source"], row["program"]
        count = min(row["count"], additions[key])
        if row["count"] > count:
            outside.append({**row, "count": row["count"] - count})
    # A change in ownership can add an in-scope diagnostic with an empty raw delta.
    for key, count in sorted(additions.items()):
        # A disposition and a rename may describe the same occurrence.
        remaining = count - max(accepted[key], equivalent[key])
        if remaining > 0:
            rule, name, source, program = key
            blocked.append({"rule": rule, "file": name, "source": source,
                            "program": program, "count": remaining})
    return {"status": "blocked" if blocked else "passed", "comparison": comparison,
            "unacceptedNew": blocked, "outOfScopeNew": outside,
            "complianceScope": c_comparison.scope_statistics(current, headers=headers),
            "projectAcceptance": acceptance["status"],
            **({"identifierEvidenceError": identifier_evidence_error} if identifier_evidence_error else {})}


def load_scope_matrix(directory: Path, side: str) -> dict:
    receipts = [read(path) for path in directory.glob(f"c-analysis-scope-{side}-*/*/scope-input.json")]
    return c_comparison.scope_matrix(receipts)


def validate_analysis_identity(manifest: dict, summary: dict, tree: str, target: str, sample: str) -> None:
    from autosar_tooling.c_baseline import FORMAT as RAW_FORMAT

    identity = manifest["identity"]
    if (manifest["format"] != RAW_FORMAT
            or (identity["tree"], identity["target"], identity["sample"]) != (tree, target, sample)
            or identity["sourceSeal"] != summary["sourceSealSha256"]
            or identity["sdk"] != summary["semanticIdentity"]["sdk"]
            or identity["analyzer"] != {key: summary["tool"][key] for key in (
                "version", "expectedSourceRevision", "binarySha256", "addons",
            )}
            or set(identity["compiler"]) != {"driver", "backend"}
            or set(identity["runner"]) != {"ImageOS", "ImageVersion"}
            or any(not value for value in identity["runner"].values())
            or set(identity["configuration"]) != {"target", "sources"}
            or not identity["configuration"]["sources"] or not identity["generator"] or not identity["rules"]
            or not identity["python"]["implementation"] or not identity["python"]["version"]
            or any(not c_fingerprint.SHA256.fullmatch(value) for value in (
                identity["sourceSeal"], identity["inputs"], identity["analyzer"]["binarySha256"],
                *identity["compiler"].values(), identity["python"]["binary"],
                identity["configuration"]["target"], manifest["fingerprintSha256"],
                manifest["summarySha256"],
            ))):
        raise ValueError(f"Raw analysis identity incomplete or differs: {target}/{sample}/{tree}")


def validate_consumed_inputs(manifest: dict, project: Path, producer_digest: str) -> None:
    from autosar_tooling.c_baseline import (
        analysis_inputs_identity,
        sealed_project_identity,
    )

    expected = {**sealed_project_identity(project), "inputs": analysis_inputs_identity()}
    for field, actual in expected.items():
        if manifest["identity"][field] != actual:
            raise ValueError(f"Transferred analysis manifest differs from actual inputs: {field}")
    if c_fingerprint.sha(manifest) != producer_digest:
        raise ValueError("Transferred analysis manifest differs from producer receipt")


def producer_receipt(directory: Path, side: str, target: str, sample: str) -> str:
    receipts = list(directory.glob(f"c-analysis-scope-{side}-{target}-{sample}-*/{sample}/analysis-producer.sha256"))
    if len(receipts) != 1:
        raise ValueError(f"Missing or duplicated analysis producer receipt: {side}/{target}/{sample}")
    return receipts[0].read_text(encoding="ascii").strip()


def fingerprint_comparison(summaries: dict, inventories: dict, matrices: dict, *,
                           mode: str | None = None) -> dict:
    """Compare per-program fingerprints within both complete scope matrices."""
    inside = {}
    for side, summary in summaries.items():
        headers = c_comparison.matrix_headers(summary, matrices[side])
        inside[side] = [
            row for row in inventories[side]
            if (not (locations := summary["diagnostics"][row["diagnostic"]]["locations"])
                or c_comparison.location_scope(summary["scope"], locations[row["locationIndex"]],
                                               headers=headers) == "in-scope")
        ]
    current = summaries["current"]
    if mode is not None:
        dispositions = current["projectAcceptance"]["fingerprintDispositions"]
        approved = set(dispositions["accepted"])
        comparison = c_fingerprint.verdict(inside["current"], inside["baseline"], approved)
        comparison["staleDispositions"] = dispositions["stale"]
        if "error" in dispositions:
            comparison.update(status="incomplete", dispositionError=dispositions["error"])
        elif comparison["status"] == "passed" and dispositions["stale"]:
            comparison["status"] = "blocked"
        return {"format": c_fingerprint.FORMAT, "mode": mode, **comparison,
                "current": inventories["current"], "baseline": inventories["baseline"]}
    accepted = {(row["diagnostic"], row["locationIndex"])
                for row in current["projectAcceptance"]["occurrences"]
                if row["assessment"] == "accepted"}
    mandatory = {"misra-c2012-" + row["id"][1:] for row in current["coverage"]["rules"]
                 if row["category"] == "Mandatory"}
    candidates = {
        row["fingerprint"] for row in inside["current"]
        if row["fingerprint"] is not None
        and (row["diagnostic"], row["locationIndex"]) in accepted
        and current["diagnostics"][row["diagnostic"]]["id"] not in mandatory
    }
    comparison = c_fingerprint.verdict(inside["current"], inside["baseline"], candidates)
    del comparison["staleDispositions"]
    return {"format": c_fingerprint.FORMAT, "mode": "migration-preview", **comparison,
            "current": inventories["current"], "baseline": inventories["baseline"],
            "candidateDispositions": sorted(candidates)}


def scan(target: str, sample: str, scope_directory: Path | None = None, *,
         fingerprints: bool = False, mode: str | None = None) -> None:
    fingerprints = fingerprints or mode is not None
    if fingerprints and scope_directory is None:
        raise ValueError("Fingerprint comparison requires the complete scope matrix")
    outputs = {}
    evidence = {}
    evidence_errors = []
    inventories, manifests, producer_digests = {}, {}, {}
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
            summary, output / "project", producer_revision=parent if side == "baseline" else None,
            **({"sdk_revision": parent} if side == "baseline" else {}))
        if identity != summary["projectAcceptance"]["inputIdentity"]:
            raise ValueError(f"Transferred {side} analysis inputs changed: {sample}")
        if fingerprints:
            manifest = read(output / "analysis-identity.json")
            inventories[side] = c_fingerprint.read_receipt(output, summary, manifest["fingerprintSha256"])
            if mode is not None:
                if manifest["summarySha256"] != sha(output / "summary.json"):
                    raise ValueError(f"Raw analysis manifest differs from {side} scan: {sample}")
                validate_analysis_identity(manifest, summary, parent if side == "baseline"
                                           else git("rev-parse", "HEAD"), target, sample)
                producer_digests[side] = producer_receipt(scope_directory, side, target, sample)
                validate_consumed_inputs(manifest, output / "project", producer_digests[side])
                manifests[side] = manifest
        outputs[side] = output
        if mode is None:
            try:
                evidence[side] = c_rename.evidence(output, sha(output / "summary.json"))
            except c_rename.EvidenceError as error:
                evidence_errors.append(f"{side}: {error}")
    current, baseline = outputs["current"], outputs["baseline"]
    source = baseline_source(read(baseline / "baseline-source.json"), parent)
    matrices = {}
    if scope_directory is not None:
        matrices = {side: load_scope_matrix(scope_directory, side) for side in ("current", "baseline")}
        for side, output in outputs.items():
            c_comparison.matrix_headers(read(output / "summary.json"), matrices[side])
        summary = read(current / "summary.json")
        c_comparison.annotate_scope(summary, set(matrices["current"]["targetHeaders"]))
        summary["projectAcceptance"] = c_acceptance.evaluate(
            summary, current / "project", matrix_scope=matrices["current"],
            **({"fingerprints": inventories["current"], "sample": sample,
                "report_disposition_errors": mode == "pr"} if mode is not None else {}))
        (current / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    shutil.copy2(baseline / "summary.json", current / "baseline-summary.json")
    summaries = {side: read(path / "summary.json") for side, path in outputs.items()}
    renames = rename_paths(baseline / "project", current / "project")
    if mode is None:
        result = verdict(summaries["current"], current, baseline, renames, identifier_evidence=evidence,
                         identifier_evidence_error=" / ".join(evidence_errors) or None,
                         **({"baseline_matrix": matrices["baseline"]} if matrices else {}))
    else:
        summary = summaries["current"]
        if summary["projectAcceptance"]["status"] not in {"accepted", "blocked"}:
            raise ValueError(f"Project acceptance execution failed: {summary['projectAcceptance']['objects']}")
        if any(manifests["baseline"]["identity"][key] != manifests["current"]["identity"][key]
               for key in ("analyzer", "compiler")):
            raise ValueError("Two-tree scans require the same analyzer and compiler identities")
        comparison = c_comparison.compare(
            summary, current / "project", current, baseline, source_tree=True, renames=renames,
        )
        result = {"comparison": comparison, "complianceScope": summary["complianceScope"],
                  "projectAcceptance": summary["projectAcceptance"]["status"]}
        if mode == "pr":
            additions = in_scope_additions(
                summary, summaries["baseline"], current / "project", baseline / "project", renames,
                headers=set(matrices["current"]["targetHeaders"]),
                previous_headers=set(matrices["baseline"]["targetHeaders"]),
            )
            result["inScopeNew"] = [
                {"rule": rule, "file": name, "source": source, "program": program, "count": count}
                for (rule, name, source, program), count in sorted(additions.items())
            ]
            result["status"] = "blocked" if additions else "passed"
        else:
            result["analysisInputs"] = manifests
            result["analysisProducerSha256"] = producer_digests
    if matrices:
        result["baselineScopeMatrix"] = matrices["baseline"]
    if fingerprints:
        result["diagnosticFingerprints"] = fingerprint_comparison(
            summaries, inventories, matrices, mode=mode,
        )
        if mode == "strict":
            result["status"] = result["diagnosticFingerprints"]["status"]
    result.update(format=FORMAT, target=target, sample=sample,
                  baselineSource=source,
                  commit=git("rev-parse", "HEAD"), baseline=git("rev-parse", "HEAD^1"),
                  summarySha256={side: sha(path / "summary.json")
                                 for side, path in outputs.items()},
                  inputIdentity={side: read(path / "summary.json")["projectAcceptance"][
                      "inputIdentity"] for side, path in outputs.items()})
    (current / "gate.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: value for key, value in result.items() if key != "diagnosticFingerprints"},
                     indent=2))
    if fingerprints:
        comparison = result["diagnosticFingerprints"]
        print(f"Fingerprint comparison: {comparison['status']}; {len(comparison['new'])} new; "
              f"{len(comparison['removed'])} removed; {comparison['unavailable']} unavailable; "
              f"{len(comparison.get('staleDispositions', []))} stale")
    if result["status"] != "passed":
        if mode == "pr":
            raise ValueError(f"New in-scope source diagnostics: {sample}")
        raise ValueError(f"New source diagnostics without current precise disposition: {sample}")


def aggregate(directory: Path, mode: str | None = None) -> None:
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
        if mode is not None:
            fingerprints = gate["diagnosticFingerprints"]
            if fingerprints["mode"] != mode or (mode == "strict" and fingerprints["status"] != "passed"):
                raise ValueError(f"Fingerprint gate mode or decision differs: {path}")
            if mode == "strict" and set(gate["analysisInputs"]) != {"baseline", "current"}:
                raise ValueError(f"Strict analysis input identity missing: {path}")
        summaries = {}
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
            if mode == "strict":
                manifest = gate["analysisInputs"][side]
                if c_fingerprint.sha(manifest) != gate["analysisProducerSha256"][side]:
                    raise ValueError(f"Strict analysis producer receipt differs: {path}")
                validate_analysis_identity(manifest, summary,
                                           gate["commit" if side == "current" else "baseline"],
                                           gate["target"], gate["sample"])
            identities.add((side, gate["target"], identity))
            summaries[side] = summary
        matrix.append((gate, summaries))
        found.add(pair)
    if found != expected or len(list(directory.glob("*/*/summary.json"))) != len(expected):
        raise ValueError(f"Source gate matrix incomplete: {sorted(expected - found)}")
    current_matrix = c_comparison.scope_matrix([
        c_comparison.scope_receipt(summaries["current"], gate["sample"]) for gate, summaries in matrix])
    baseline_matrix = c_comparison.scope_matrix([
        c_comparison.scope_receipt(summaries["baseline"], gate["sample"]) for gate, summaries in matrix])
    statistics = []
    for gate, summaries in matrix:
        summary = summaries["current"]
        acceptance = summary["projectAcceptance"]
        if (acceptance.get("scopeMatrix") != current_matrix
                or gate.get("baselineScopeMatrix") != baseline_matrix
                or acceptance.get("complianceScope") != summary.get("complianceScope")):
            raise ValueError("Project acceptance matrix scope receipt differs")
        c_comparison.matrix_headers(summary, current_matrix)
        expected_scope = c_comparison.scope_statistics(summary, headers=set(current_matrix["targetHeaders"]))
        if summary["complianceScope"] != expected_scope:
            raise ValueError("Project acceptance scope statistics differ")
        positions = {(index, position): location for index, position, _diagnostic, location
                     in c_acceptance._positions(summary["diagnostics"])}
        occurrences = acceptance["occurrences"]
        if (len(occurrences) != len(positions)
                or {(row["diagnostic"], row["locationIndex"]) for row in occurrences} != set(positions)):
            raise ValueError("Project acceptance matrix occurrence coverage differs")
        for occurrence in occurrences:
            location = positions[occurrence["diagnostic"], occurrence["locationIndex"]]
            outside = location is not None and c_comparison.location_scope(
                summary["scope"], location, headers=set(current_matrix["targetHeaders"])) != "in-scope"
            if outside != (occurrence["assessment"] == "out_of_scope"):
                raise ValueError("Project acceptance occurrence differs from matrix scope")

        if mode is not None:
            comparison = gate["diagnosticFingerprints"]
            inventories = {side: comparison[side] for side in summaries}
            for side, value in summaries.items():
                c_fingerprint.validate_occurrences(value, inventories[side])
            if comparison != fingerprint_comparison(
                    summaries, inventories, {"current": current_matrix, "baseline": baseline_matrix}, mode=mode):
                raise ValueError("Fingerprint gate decision differs from the analyzed positions")
        statistics.append({"target": gate["target"], "sample": gate["sample"], **expected_scope})
    (directory / "compliance-scope.json").write_text(
        json.dumps({"targetHeaders": current_matrix["targetHeaders"], "analyses": statistics}, indent=2) + "\n",
        encoding="utf-8")



def release_ready() -> None:
    from autosar_tooling import c_disposition

    for target in TARGETS:
        binaries = c_disposition.WINDOWS_DISPOSITION_BINARIES if target == c_disposition.WINDOWS else [None]
        for binary in binaries:
            for sample in SAMPLES:
                c_disposition.load(ROOT, target, sample, binary)


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
    scanning.add_argument("--scope-directory", type=Path, default=ROOT / "build/c-analysis-scope")
    scanning.add_argument("--fingerprints", action="store_true")
    scanning.add_argument("--mode", choices=("pr", "strict"))
    aggregation = commands.add_parser("aggregate")
    aggregation.add_argument("directory", type=Path)
    aggregation.add_argument("--mode", choices=("pr", "strict"))
    commands.add_parser("release-ready")
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.baseline, args.output_directory)
    elif args.command == "stage":
        stage(args.baseline)
    elif args.command == "scan":
        scan(args.target, args.sample, args.scope_directory, fingerprints=args.fingerprints, mode=args.mode)
    elif args.command == "aggregate":
        aggregate(args.directory, args.mode)
    else:
        release_ready()


if __name__ == "__main__":
    main()
