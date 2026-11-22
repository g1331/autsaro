"""Reuse only identity-matched raw master analysis; otherwise scan the parent."""

from __future__ import annotations

import argparse
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

from autosar_tooling import (
    c_acceptance,
    c_check,
    c_comparison,
    c_fingerprint,
    c_gate,
    c_rename,
    c_type_match,
)
from autosar_tooling.c_baseline_jobs import previous_wrapper, producer_matches
from autosar_tooling.config import ROOT

FORMAT = "c-analysis-raw-baseline-v1"
JOB_SELECTOR = "tools/python/src/autosar_tooling/c_baseline_jobs.py"
WRAPPER = "tools/python/src/autosar_tooling/c_baseline.py"
INPUTS = ("tools/python/src", "core/tests", "core/resources", "Cargo.toml", "Cargo.lock",
          "pyproject.toml", "uv.lock", ".python-version",
          "core/Cargo.toml", "src-tauri/Cargo.toml", "rust-toolchain.toml", ".cargo",
          "docs/standards", "docs/adr", "docs/development/c-coding-standard.md",
          ".github/actions/native-analysis-compiler", ".github/actions/core-build-dependencies",
          ".github/workflows/checks.yml", ".github/cmake", ".github/actions/retain-analysis-sample",
          ".agents/skills/misra-c2012/references/baseline-index.json")


def read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def git(*arguments: str) -> str:
    return subprocess.check_output(["git", *arguments], cwd=ROOT, encoding="utf-8").strip()


def rename_candidates(project: Path, side: str) -> set[str]:
    """Select copied runtime C units before either producer starts its scan."""
    try:
        changed = git("diff", "--name-only", "--diff-filter=M", "--no-renames", "-z",
                      "HEAD^1", "HEAD", "--", ":(glob)runtime/**/*.c").split("\0")
        if changed == [""]:
            return set()
        origins = read(project / "meta/integration.json").get("originalBswSources", {})
        candidates = set()
        for origin in changed:
            name = origins.get(origin)
            if not safe_relative(origin) or not isinstance(name, str) or not safe_relative(name):
                continue
            old, new = (git("show", revision + ":" + origin) for revision in ("HEAD^1", "HEAD"))
            delivered = (project / name).read_text(encoding="utf-8").strip()
            if delivered == (old if side == "baseline" else new) and c_rename._physical_changes(old, new):
                candidates.add(name)
        return candidates
    except (OSError, subprocess.CalledProcessError, UnicodeError):
        return set()


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


def analysis_inputs_identity(previous: str | None = None) -> str:
    paths = git("ls-files", "-z", "--", *INPUTS).split("\0")
    # Job labels select a protected artifact. The scan and validation driver
    # itself still participates in the byte-for-byte input identity.
    inputs = {name: digest(ROOT / name) for name in sorted(paths)
              if name and name != JOB_SELECTOR}
    if previous is not None:
        inputs[WRAPPER] = previous
    return c_fingerprint.sha(inputs)


def sealed_project_identity(project: Path) -> dict:
    plan = read(project / "meta/integration.json")["plan"]
    return {
        "rules": plan["ruleSetIdentity"],
        "generator": {name: value for name, value in plan["runtimeSources"].items()
                      if name.startswith("core/")},
        "configuration": {"target": digest(project / "target.json"), "sources": plan["sources"]},
        "sourceSeal": digest(project / FILE_HASHES),
    }


def identity(project: Path, tree: str, target: str, sample: str, tool: dict,
             compiler: dict, sdk: str, *, previous: str | None = None) -> dict:
    """Bind the generated tree and every PR-controlled analysis/sample input."""
    return {
        "tree": tree, "target": target, "sample": sample,
        "analyzer": {key: tool[key] for key in
                     ("version", "expectedSourceRevision", "binarySha256", "addons")},
        **sealed_project_identity(project),
        "inputs": analysis_inputs_identity(previous), "compiler": compiler, "sdk": sdk,
        "runner": {key: os.environ.get(key) for key in ("ImageOS", "ImageVersion")},
        "python": {"implementation": sys.implementation.name, "version": sys.version,
                   "binary": digest(Path(sys.executable))},
    }


def database_digests(output: Path, summary: dict) -> dict[str, str]:
    databases = {}
    for program in summary["programs"]:
        name = program["name"]
        directory = output / name if len(summary["programs"]) > 1 else output
        path = directory / "compile_commands.json"
        if path.is_file():
            databases[name] = digest(path)
    return databases


def record(output: Path, tree: str, target: str, sample: str, candidates: set[str] | None = None, *,
           fingerprints: bool = False, cached_fingerprints: list[dict] | None = None) -> None:
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
    identifier_evidence = c_rename.record(output, summary, candidates or set())
    evidence_bytes = sum(c_rename._proof_path(output, name).stat().st_size for name in identifier_evidence)
    print(f"Identifier evidence: {len(identifier_evidence)} TUs, {evidence_bytes} bytes, "
          f"{time.perf_counter() - started:.3f} seconds to package and hash", flush=True)
    fingerprint_fields = {}
    if fingerprints:
        rows = (c_fingerprint.inventory(summary, output, value["compiler"])
                if cached_fingerprints is None else cached_fingerprints)
        fingerprint_fields["fingerprintSha256"] = c_fingerprint.write_receipt(output, rows)
        if cached_fingerprints is None:
            databases = database_digests(output, summary)
            views = c_type_match.producer_views(output, summary)
        else:
            cached_manifest = output / "analysis-identity.json"
            original = read(cached_manifest) if cached_manifest.is_file() else {}
            databases = original.get("compileCommandsSha256", {})
            views = original.get("compilerViewSha256", {})
        if views:
            fingerprint_fields["compilerViewSha256"] = views
        if databases:
            fingerprint_fields["compileCommandsSha256"] = databases
    manifest = {
        "format": FORMAT, "identity": value,
        "summarySha256": digest(output / "summary.json"),
        **({"identifierEvidence": identifier_evidence} if identifier_evidence else {}),
        **fingerprint_fields,
        "run": int(os.environ["GITHUB_RUN_ID"]) if os.environ.get("GITHUB_RUN_ID") else None,
        "attempt": int(os.environ.get("GITHUB_RUN_ATTEMPT", "1")),
    }
    (output / "analysis-identity.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (output / "analysis-producer.sha256").write_text(c_fingerprint.sha(manifest) + "\n", encoding="ascii")


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
        jobs = pages(repository, f"actions/runs/{run['id']}/jobs", "jobs")
        producers = [row for row in jobs if producer_matches(row["name"], target, sample)]
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
    original_databases = manifest.get("compileCommandsSha256")
    if original_databases and original_databases != database_digests(candidate, summary):
        raise ValueError("Restored compilation databases differ from their producer")
    sealed_sources(project)
    compiler = os.environ.get("AUTOSAR_CC") or shutil.which("gcc")
    if compiler is None:
        raise ValueError("Current analysis compiler unavailable")
    with tempfile.TemporaryDirectory(prefix="c-baseline-tool-") as temporary:
        tool = c_check.tool_identity(Path(temporary))
        expected = identity(project, tree, target, sample, tool[2],
                            compiler_identity(Path(compiler)),
                            summary["semanticIdentity"]["sdk"])
        recorded = manifest["identity"]
        if not isinstance(recorded, dict):
            raise TypeError("Cached baseline input identity is not an object")
        fields = sorted(key for key in recorded.keys() | expected.keys()
                        if key not in recorded or key not in expected or recorded[key] != expected[key])
        if fields == ["inputs"]:
            previous = previous_wrapper(digest(ROOT / WRAPPER))
            if previous is not None:
                legacy = identity(project, tree, target, sample, tool[2], expected["compiler"],
                                  expected["sdk"], previous=previous)
                if recorded == legacy:
                    fields.clear()
        if fields:
            raise ValueError("Cached baseline input identity differs: " + ", ".join(fields))
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
        # Restore is used only when this comparison has no rename candidates.
        # The historical proof was verified above, but belongs to its own pair.
        shutil.copytree(candidate, output, dirs_exist_ok=True, ignore=lambda directory, _names:
                        {c_rename.EVIDENCE} if Path(directory) == candidate else set())
        for field, name in (("platformModel", "platform-model.h"),
                            ("platformLibrary", "platform-library.cfg")):
            if summary["scope"].get(field):
                summary["scope"][field]["path"] = str((output / name).resolve())
        (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")


def analyze(side: str, target: str, sample: str, project: Path, output: Path, *,
            fingerprints: bool = False) -> int:
    refuse_links(project)
    refuse_links(output)
    project, output = project.resolve(strict=True), output.absolute().resolve()
    if (output == project or output.is_relative_to(project) or project.is_relative_to(output)
            or output.exists() and (not output.is_dir() or any(output.iterdir()))):
        raise ValueError("C analysis output must be new or empty and separate from its project")
    tree = git("rev-parse", "HEAD^1" if side == "baseline" else "HEAD")
    started = time.perf_counter()
    candidates = rename_candidates(project, side)
    print(f"Identifier candidates: {len(candidates)} TUs in {time.perf_counter() - started:.3f} seconds",
          flush=True)
    source = {"kind": "same-run", "tree": tree}
    cached_fingerprints = None
    if side == "baseline":
        repository = os.environ.get("GITHUB_REPOSITORY")
        reason = "repository_unavailable"
        if candidates:
            # A master artifact collected for a different pair need not contain
            # these bindings. Collect them during the normal baseline scan.
            reason = "identifier_candidates_require_evidence"
        elif repository:
            output.parent.mkdir(parents=True, exist_ok=True)
            # MinGW resolves rooted include paths on the working directory's drive.
            with tempfile.TemporaryDirectory(prefix="c-baseline-raw-", dir=output.parent) as temporary:
                directory = Path(temporary)
                try:
                    cached = download(repository, tree, target, sample, directory)
                    if cached is not None:
                        candidate = directory / sample
                        manifest = read(candidate / "analysis-identity.json")
                        if (manifest["run"] != cached["run"]
                                or manifest["attempt"] != cached["attempt"]):
                            raise ValueError("Cached artifact belongs to a different run attempt")
                        rows = (c_fingerprint.read_receipt(candidate, read(candidate / "summary.json"),
                                                           manifest["fingerprintSha256"])
                                if fingerprints else None)
                        restore(candidate, project, output, tree, target, sample)
                        cached_fingerprints = rows
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
                               **({"rename_units": candidates} if candidates else {}),
                               **({"sdk_revision": tree} if side == "baseline" else {}))
    else:
        result = 0 if read(output / "summary.json")["passed"] else 1
    if fingerprints:
        record(output, tree, target, sample, candidates, fingerprints=True,
               cached_fingerprints=cached_fingerprints)
    else:
        record(output, tree, target, sample, candidates)
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
    parser.add_argument("--fingerprints", action="store_true")
    args = parser.parse_args()
    analyze(args.side, args.target, args.sample, args.project, args.output_directory,
            **({"fingerprints": True} if args.fingerprints else {}))
    # Source diagnostic status remains in the raw receipt, separately from orchestration errors.
    raise SystemExit(0)


if __name__ == "__main__":
    main()
