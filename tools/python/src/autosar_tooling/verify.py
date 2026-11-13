"""Run the existing project checks with normal terminal output."""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from autosar_tooling.config import ROOT, environment

CLIPPY = ["-D", "warnings"]

def executable(name: str) -> str:
    found = shutil.which(name)
    if not found:
        raise FileNotFoundError(f"Required executable is missing: {name}")
    return str(Path(found).absolute())

def npm_command(*arguments: str) -> list[str]:
    npm = Path(executable("npm"))
    if os.name == "nt":
        entry = npm.parent / "node_modules/npm/bin/npm-cli.js"
        if not entry.is_file():
            raise FileNotFoundError(f"npm entry is missing: {entry}")
        return [executable("node"), str(entry), *arguments]
    return [str(npm), *arguments]

def commands(scope: str, base: str | None) -> list[tuple[str, list[str], int]]:
    core = [
        (
            "core:test",
            [
                "cargo",
                "test",
                "--locked",
                "-p",
                "autosar-config-core",
                "--features",
                "official-oracles,native-tests",
            ],
            7200,
        ),
        (
            "core:clippy",
            [
                "cargo",
                "clippy",
                "--locked",
                "-p",
                "autosar-config-core",
                "--all-targets",
                "--all-features",
                "--",
                *CLIPPY,
            ],
            1800,
        ),
    ]
    ui = (
        [
            ("ui:lint", ["npm", "run", "lint", "--prefix", "ui"], 300),
            ("ui:test", ["npm", "run", "test", "--prefix", "ui"], 300),
            ("ui:build", ["npm", "run", "build", "--prefix", "ui"], 300),
        ]
        if scope in ("ui", "all")
        else []
    )
    desktop = [
        (
            "desktop:test",
            ["cargo", "test", "--locked", "--profile", "dev", "-p", "autosar-config-desktop"],
            1800,
        ),
        (
            "desktop:build",
            ["cargo", "build", "--locked", "-p", "autosar-config-desktop"],
            1800,
        ),
        (
            "desktop:clippy",
            [
                "cargo",
                "clippy",
                "--locked",
                "-p",
                "autosar-config-desktop",
                "--all-targets",
                "--",
                *CLIPPY,
            ],
            1800,
        ),
    ]
    if scope == "all":
        selected = [
            (
                "python:unit",
                [
                    sys.executable,
                    "-B",
                    "-m",
                    "unittest",
                    "discover",
                    "-s",
                    "tests/python",
                    "-p",
                    "test_*.py",
                ],
                300,
            ),
            (
                "source:quality",
                [
                    sys.executable,
                    "-B",
                    "-m",
                    "autosar_tooling",
                    "quality",
                    *(["--base", base] if base else []),
                ],
                600,
            ),
            ("source:requirements", [sys.executable, "-B", "-m", "autosar_tooling", "requirements"], 120),
            ("python:lint", ["ruff", "check", "tools", "tests/python", "scripts"], 120),
            ("workflows:actionlint", ["uv", "run", "--locked", "--group", "workflows", "actionlint"], 300),
            ("workflows:zizmor", ["uv", "run", "--locked", "--group", "workflows", "zizmor", ".github"], 300),
            ("dependencies:cargo-deny", ["cargo", "deny", "--locked", "check"], 600),
            (
                "dependencies:lockfiles",
                ["osv-scanner", "scan", "source", "--lockfile", "uv.lock", "--lockfile", "ui/package-lock.json"],
                300,
            ),
            *ui,
            *core,
            *desktop,
        ]
    else:
        selected = {"core": core, "ui": ui, "desktop": desktop}[scope]
    return [
        ("source:diff", ["git", "diff", "--check"], 30),
        ("source:staged-diff", ["git", "diff", "--cached", "--check"], 30),
        *selected,
    ]


def run_commands(plan: list[tuple[str, list[str], int]], env: dict[str, str]) -> int:
    for name, command, timeout in plan:
        argv = (npm_command(*command[1:]) if command[0] == "npm"
                else [executable(command[0]), *command[1:]])
        print(f"> {name}: {subprocess.list2cmdline(argv)}", flush=True)
        try:
            result = subprocess.run(argv, cwd=ROOT, env=env, timeout=timeout, check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            print(f"{name}: {error}", file=sys.stderr)
            return 1
        if result.returncode:
            print(f"{name}: argv={argv!r} exit_code={result.returncode}", file=sys.stderr)
            return result.returncode if result.returncode > 0 else 1
    return 0


def changed_rows(base: str) -> tuple[str, list[dict]]:
    """Read committed, staged, unstaged and untracked inputs, including rename sources."""
    baseline = subprocess.check_output(
        ["git", "merge-base", base, "HEAD"], cwd=ROOT, text=True,
    ).strip()
    fields = subprocess.check_output(
        ["git", "diff", "--find-renames", "--name-status", "-z", baseline, "--"], cwd=ROOT,
    ).decode("utf-8").split("\0")[:-1]
    statuses = {"M": "modified", "A": "added", "D": "removed"}
    records = iter(fields)
    rows = []
    for status in records:
        name = next(records)
        if status.startswith("R"):
            rows.append({"filename": next(records), "status": "renamed", "previous_filename": name})
        else:
            rows.append({"filename": name, "status": statuses.get(status, status)})
    untracked = subprocess.check_output(
        ["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=ROOT,
    ).decode("utf-8").split("\0")
    # An unstaged move can appear as a deletion plus an untracked destination.
    # Keep that ambiguous list conservative, just like an API rename record.
    status = "unverified" if any(row["status"] == "removed" for row in rows) else "added"
    rows.extend({"filename": name, "status": status} for name in untracked if name)
    return baseline, rows


def changed_commands(
    rows: list[dict], base: str, samples: Path,
) -> list[tuple[str, list[str], int]]:
    from autosar_tooling.ci_selection import select_changes

    selected = select_changes(rows)
    paths = {path for row in rows
             for path in (row["filename"], row.get("previous_filename")) if path}
    workflow = any(path.startswith(".github/") for path in paths)
    dependencies = bool(paths & {"Cargo.toml", "Cargo.lock", "pyproject.toml", "uv.lock",
                                 "ui/package.json", "ui/package-lock.json"})
    result = []
    for name, command, timeout in commands("all", base):
        area = name.split(":", 1)[0]
        enabled = (
            area == "source" or area == "python" and selected["code"]
            or area == "core" and selected["native"]
            or area in {"ui", "desktop"} and selected["application"]
            or area == "workflows" and workflow or area == "dependencies" and dependencies
        )
        if not enabled:
            continue
        if name == "core:test":
            # native-tests only adds test helpers/cases. One feature set runs lib, it,
            # native and doctests, retaining Cargo's complete target enumeration.
            command = ["cargo", "test", "--locked", "-p", "autosar-config-core",
                       "--features", "native-tests", "--no-fail-fast"]
        if name == "desktop:test":
            result.append(("desktop:ipc", ["npm", "run", "ipc:check", "--prefix", "ui"], 1800))
        result.append((name, command, timeout))
    if selected["analysis"]:
        if not selected["native"]:
            result.append(("analysis:samples", [
                "cargo", "test", "--locked", "-p", "autosar-config-core", "--features",
                "native-tests", "--test", "it", "--", "c_analysis::",
            ], 7200))
        result.append(("analysis:quality", [
            sys.executable, "-B", "-m", "autosar_tooling", "quality", "--scope", "runtime",
            "--c-samples", str(samples),
        ], 600))
    return result


def verify_changed(base: str, *, list_checks: bool = False) -> int:
    from autosar_tooling.ci_selection import select_changes

    baseline, rows = changed_rows(base)
    selected = select_changes(rows)
    print("Changed-input selection: " + json.dumps(selected), flush=True)
    if list_checks:
        for name, command, _ in changed_commands(rows, baseline, Path("<new-run>/samples")):
            print(f"> {name}: {subprocess.list2cmdline(command)}", flush=True)
        return 0
    if selected["analysis"] and sys.platform not in ("win32", "linux"):
        print("Native verification requires Windows or Linux.", file=sys.stderr)
        return 1
    if not selected["analysis"]:
        return run_commands(changed_commands(rows, baseline, Path("unused")), environment())
    env = environment()
    build = ROOT / "build"
    build.mkdir(exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="local-verification-", dir=build))
    samples = output / "samples"
    # it generates the four samples before native consumes them in the normal Cargo run.
    # Every invocation starts fresh; successful test/analysis results are never cached.
    env["AUTOSAR_C_ANALYSIS_SAMPLES"] = str(samples)
    result = 1
    try:
        result = run_commands(changed_commands(rows, baseline, samples), env)
        return result
    finally:
        if result and any(output.iterdir()):
            print(f"Verification inputs retained at {output}", flush=True)
        else:
            shutil.rmtree(output)


def verify(scope: str, base: str | None = None, *, list_checks: bool = False) -> int:
    if scope == "changed":
        return verify_changed(base or "origin/master", list_checks=list_checks)
    if scope in ("core", "all") and sys.platform not in ("win32", "linux"):
        print("Native verification requires Windows or Linux.", file=sys.stderr)
        return 1
    print("GUI/IPC and installed bundle acceptance are separate checks.", flush=True)
    return run_commands(commands(scope, base), environment())
