"""Reuse only identity-matched raw master analysis; otherwise scan the parent."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import zipfile
from pathlib import Path

from ecu_tools.build import (
    FILE_HASHES,
    digest,
    refuse_links,
    safe_relative,
    sealed_sources,
)
from ecu_tools.process import ProcessResult, ProcessSpec, run_bounded

from autosar_tooling import c_acceptance, c_check, c_comparison, c_gate, c_rename
from autosar_tooling.config import ROOT

FORMAT = "c-analysis-raw-baseline-v1"
INPUTS = ("tools/python/src", "core/tests", "core/resources", "Cargo.toml", "Cargo.lock",
          "pyproject.toml", "uv.lock", ".python-version",
          "core/Cargo.toml", "src-tauri/Cargo.toml", "rust-toolchain.toml", ".cargo",
          "docs/standards", "docs/adr", "docs/development/c-coding-standard.md",
          ".github/actions/native-analysis-compiler", ".github/actions/core-build-dependencies",
          ".agents/skills/misra-c2012/references/baseline-index.json")


def read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def git(*arguments: str) -> str:
    return subprocess.check_output(["git", *arguments], cwd=ROOT, text=True).strip()


def compiler_identity(compiler: Path) -> dict:
    backend = subprocess.check_output([str(compiler), "-print-prog-name=cc1"],
                                      text=True, timeout=30).strip()
    backend_path = Path(backend)
    if not backend_path.is_absolute():
        found = shutil.which(backend)
        if found is None:
            raise ValueError("Actual compiler backend unavailable")
        backend_path = Path(found)
    return {"driver": digest(compiler), "backend": digest(backend_path)}


def identity(project: Path, tree: str, target: str, sample: str, tool: dict,
             compiler: dict, sdk: str) -> dict:
    """Bind the generated tree and every PR-controlled analysis/sample input."""
    plan = read(project / "meta/integration.json")["plan"]
    paths = git("ls-files", "-z", "--", *INPUTS).split("\0")
    inputs = {name: digest(ROOT / name) for name in sorted(paths) if name}
    encoded = json.dumps(inputs, sort_keys=True, separators=(",", ":")).encode()
    return {
        "tree": tree, "target": target, "sample": sample,
        "analyzer": {key: tool[key] for key in
                     ("version", "expectedSourceRevision", "binarySha256", "addons")},
        "rules": plan["ruleSetIdentity"],
        "generator": {name: value for name, value in plan["runtimeSources"].items()
                      if name.startswith("core/")},
        "configuration": {"target": digest(project / "target.json"), "sources": plan["sources"]},
        "inputs": hashlib.sha256(encoded).hexdigest(),
        "sourceSeal": digest(project / FILE_HASHES), "compiler": compiler, "sdk": sdk,
        "runner": {key: os.environ.get(key) for key in ("ImageOS", "ImageVersion")},
        "python": {"implementation": sys.implementation.name, "version": sys.version,
                   "binary": digest(Path(sys.executable))},
    }


def record(output: Path, tree: str, target: str, sample: str) -> None:
    summary = read(output / "summary.json")
    c_acceptance._complete(summary)
    if summary["projectAcceptance"]["status"] not in {"accepted", "blocked"}:
        raise ValueError("Raw analysis project acceptance execution failed")
    (output / "scope-input.json").write_text(
        json.dumps(c_comparison.scope_receipt(summary, sample), indent=2) + "\n", encoding="utf-8")
    value = identity(output / "project", tree, target, sample, summary["tool"],
                     compiler_identity(Path(summary["scope"]["diagnosticCompiler"]["path"])),
                     summary["semanticIdentity"]["sdk"])
    started = time.perf_counter()
    identifier_evidence = c_rename.record(output, summary)
    evidence_bytes = sum(path.stat().st_size for path in (output / c_rename.EVIDENCE).rglob("*.json.gz"))
    print(f"Identifier evidence: {len(identifier_evidence)} TUs, {evidence_bytes} bytes, "
          f"{time.perf_counter() - started:.3f} seconds to package and hash", flush=True)
    (output / "analysis-identity.json").write_text(json.dumps({
        "format": FORMAT, "identity": value,
        "summarySha256": digest(output / "summary.json"),
        "identifierEvidence": identifier_evidence,
        "run": int(os.environ["GITHUB_RUN_ID"]) if os.environ.get("GITHUB_RUN_ID") else None,
        "attempt": int(os.environ.get("GITHUB_RUN_ATTEMPT", "1")),
    }, indent=2) + "\n", encoding="utf-8")


def gh(arguments: list[str], directory: Path, stage: str, timeout: int) -> ProcessResult:
    executable = shutil.which("gh")
    if executable is None:
        raise ValueError("GitHub CLI unavailable for optional baseline reuse")
    return run_bounded(ProcessSpec.seconds(
        [str(Path(executable).resolve()), *arguments], directory, timeout, directory, stage),
        allowed_exit_codes=(0,))


def api(repository: str, path: str) -> dict:
    with tempfile.TemporaryDirectory(prefix="c-baseline-api-") as temporary:
        result = gh(["api", f"repos/{repository}/{path}"], Path(temporary), "api", 30)
        return read(result.stdout)


def pages(repository: str, path: str, key: str) -> list[dict]:
    found = []
    page = 1
    while True:
        rows = api(repository, f"{path}?per_page=100&page={page}")[key]
        found.extend(rows)
        if len(rows) < 100:
            return found
        page += 1


def extract_raw(archive: Path, expected: str, directory: Path, sample: str) -> None:
    if "sha256:" + digest(archive) != expected:
        raise ValueError("Downloaded baseline artifact digest differs")
    with zipfile.ZipFile(archive) as contents:
        rows = contents.infolist()
        names = [row.filename.rstrip("/") for row in rows]
        if (len(names) != len(set(names))
                or any(not safe_relative(name) or name.split("/")[0] != sample for name in names)
                or any(stat.S_ISLNK(row.external_attr >> 16) for row in rows)):
            raise ValueError("Unsafe or duplicated baseline artifact member")
        contents.extractall(directory)


def download(repository: str, tree: str, target: str, sample: str,
             directory: Path) -> dict | None:
    """Require a successful raw producer in a protected master push run."""
    runs = api(repository, "actions/workflows/checks.yml/runs?event=push&branch=master&"
               f"head_sha={tree}&per_page=100")["workflow_runs"]
    for run in runs:
        if (run["event"] != "push" or run["head_branch"] != "master"
                or run["head_sha"] != tree):
            continue
        artifacts = pages(repository, f"actions/runs/{run['id']}/artifacts", "artifacts")
        prefix = f"c-analysis-raw-current-{target}-{sample}-"
        found = [row for row in artifacts if row["name"].startswith(prefix)
                 and not row["expired"]]
        if len(found) != 1:
            continue
        artifact = found[0]
        if not re.fullmatch(r"sha256:[0-9a-f]{64}", artifact.get("digest") or ""):
            continue
        windows = target.startswith("windows")
        job = "c-analysis-samples" + ("-windows" if windows else "")
        system = "windows-2025-vs2026" if windows else "ubuntu-24.04"
        expected = f"{job} ({system}, {target}, {sample}, {sample})"
        jobs = pages(repository, f"actions/runs/{run['id']}/jobs", "jobs")
        producers = [row for row in jobs if row["name"] == expected or
                     row["name"].endswith("...") and expected.startswith(row["name"][:-3])]
        if len(producers) != 1 or producers[0]["conclusion"] != "success":
            continue
        result = gh(["api", f"repos/{repository}/actions/artifacts/{artifact['id']}/zip"],
                    directory, "download", 60)
        extract_raw(result.stdout, artifact["digest"], directory, sample)
        return {"kind": "master-artifact", "tree": tree, "run": run["id"],
                "attempt": run["run_attempt"], "job": producers[0]["id"],
                "artifact": artifact["id"], "digest": artifact["digest"]}
    return None


def restore(candidate: Path, project: Path, output: Path, tree: str,
            target: str, sample: str) -> None:
    """Validate on the target host and recompute acceptance; no verdict is reused."""
    manifest = read(candidate / "analysis-identity.json")
    summary = read(candidate / "summary.json")
    if (manifest["format"] != FORMAT
            or manifest["summarySha256"] != digest(candidate / "summary.json")):
        raise ValueError("Cached raw analysis receipt identity differs")
    c_rename.verify(candidate, manifest.get("identifierEvidence", {}))
    c_acceptance._complete(summary)
    sealed_sources(project)
    compiler = os.environ.get("AUTOSAR_CC") or shutil.which("gcc")
    if compiler is None:
        raise ValueError("Current analysis compiler unavailable")
    with tempfile.TemporaryDirectory(prefix="c-baseline-tool-") as temporary:
        tool = c_check.tool_identity(Path(temporary))
        expected = identity(project, tree, target, sample, tool[2],
                            compiler_identity(Path(compiler)),
                            summary["semanticIdentity"]["sdk"])
        if manifest["identity"] != expected:
            raise ValueError("Cached baseline input identity differs")
        if summary["tool"]["binarySha256"] != tool[2]["binarySha256"]:
            raise ValueError("Cached analyzer binary differs")
        if summary["coverage"] != c_check.coverage(tool[1]):
            raise ValueError("Cached checker/manual coverage differs")
        # Recorded model paths are provenance; retain their exact bytes at the new location.
        for field, name in (("platformModel", "platform-model.h"),
                            ("platformLibrary", "platform-library.cfg")):
            if summary["scope"].get(field):
                model = candidate / name
                if digest(model) != summary["scope"][field]["sha256"]:
                    raise ValueError("Cached platform model bytes differ")
                summary["scope"][field]["path"] = str(model.resolve())
        summary["scope"]["diagnosticCompiler"]["path"] = str(Path(compiler).resolve())
        c_check.recheck_semantic_inputs(summary, candidate / "project", tool)
        # The historical producer/assessment context is never today's acceptance receipt.
        summary["projectAcceptance"] = c_acceptance.evaluate(summary, candidate / "project", sdk_revision=tree)
        if summary["projectAcceptance"]["status"] not in {"accepted", "blocked"}:
            raise ValueError("Reused raw analysis could not be assessed with current inputs")
        shutil.copytree(candidate, output, dirs_exist_ok=True)
        for field, name in (("platformModel", "platform-model.h"),
                            ("platformLibrary", "platform-library.cfg")):
            if summary["scope"].get(field):
                summary["scope"][field]["path"] = str((output / name).resolve())
        (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")


def analyze(side: str, target: str, sample: str, project: Path, output: Path) -> int:
    refuse_links(project)
    refuse_links(output)
    project, output = project.resolve(strict=True), output.absolute().resolve()
    if (output == project or output.is_relative_to(project) or project.is_relative_to(output)
            or output.exists() and (not output.is_dir() or any(output.iterdir()))):
        raise ValueError("C analysis output must be new or empty and separate from its project")
    tree = git("rev-parse", "HEAD^1" if side == "baseline" else "HEAD")
    source = {"kind": "same-run", "tree": tree}
    if side == "baseline":
        repository = os.environ.get("GITHUB_REPOSITORY")
        reason = "repository_unavailable"
        if repository:
            with tempfile.TemporaryDirectory(prefix="c-baseline-raw-") as temporary:
                directory = Path(temporary)
                try:
                    cached = download(repository, tree, target, sample, directory)
                    if cached is not None:
                        candidate = directory / sample
                        manifest = read(candidate / "analysis-identity.json")
                        if (manifest["run"] != cached["run"]
                                or manifest["attempt"] != cached["attempt"]):
                            raise ValueError("Cached artifact belongs to a different run attempt")
                        restore(candidate, project, output, tree, target, sample)
                        source = cached
                    else:
                        reason = "artifact_missing"
                except (OSError, RuntimeError, ValueError, KeyError, TypeError,
                        subprocess.CalledProcessError, subprocess.TimeoutExpired,
                        zipfile.BadZipFile) as error:
                    reason = f"reuse_refused: {error}"
                    print(reason, flush=True)
                    if output.exists():
                        shutil.rmtree(output)
        if source["kind"] == "same-run":
            source["reason"] = reason
    if source["kind"] == "same-run":
        result = c_check.check(project, target, output,
                               **({"sdk_revision": tree} if side == "baseline" else {}))
    else:
        result = 0 if read(output / "summary.json")["passed"] else 1
    record(output, tree, target, sample)
    (output / "baseline-source.json").write_text(json.dumps(source, indent=2) + "\n",
                                                encoding="utf-8")
    print("Raw analysis source:", json.dumps(source), flush=True)
    print("Raw c-check exit status:", result, flush=True)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--side", choices=("current", "baseline"), required=True)
    parser.add_argument("--target", choices=c_check.TARGETS, required=True)
    parser.add_argument("--sample", choices=c_gate.SAMPLES, required=True)
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--output-directory", type=Path, required=True)
    args = parser.parse_args()
    analyze(args.side, args.target, args.sample, args.project, args.output_directory)
    # Source diagnostic status remains in the raw receipt, separately from orchestration errors.
    raise SystemExit(0)


if __name__ == "__main__":
    main()
