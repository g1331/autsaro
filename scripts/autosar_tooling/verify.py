"""Run the selected real developer gates once, with owned command deadlines."""

from __future__ import annotations

import os
import platform
import shutil
import sys
import tempfile
from pathlib import Path

from ecu_tools.process import CompletionPolicy, OwnedProcess, ProcessSpec

ROOT = Path(__file__).resolve().parents[2]
CLIPPY = ["-A", "clippy::all", "-D", "clippy::correctness", "-D", "clippy::suspicious"]


def executable(name: str) -> str:
    found = shutil.which(name)
    if found is None:
        raise FileNotFoundError(f"Required developer executable is missing: {name}")
    # Preserve argv[0] for virtualenv and rustup symlink dispatch; the owner
    # still requires an absolute executable, without canonicalizing the link.
    return str(Path(found).absolute())


def npm_command(*arguments: str) -> list[str]:
    npm = Path(executable("npm"))
    if sys.platform == "win32":
        # CreateProcess cannot execute a batch wrapper. Use its real JS entry,
        # retaining argv and the same owned Node process without cmd.exe.
        entry = npm.parent / "node_modules/npm/bin/npm-cli.js"
        if not entry.is_file():
            raise FileNotFoundError(f"Installed npm JS entry is missing: {entry}")
        return [executable("node"), str(entry), *arguments]
    return [str(npm), *arguments]


def commands(scope: str, base: str | None) -> list[tuple[str, list[str], int]]:
    core = [
        ("core:test", ["cargo", "test", "--locked", "--manifest-path", "core/Cargo.toml"], 7200),
        (
            "core:clippy",
            ["cargo", "clippy", "--locked", "--manifest-path", "core/Cargo.toml", "--all-targets", "--", *CLIPPY],
            1800,
        ),
    ]
    ui = [
        ("ui:dependencies", ["npm", "ci", "--prefix", "ui"], 900),
        ("ui:lint", ["npm", "run", "lint", "--prefix", "ui"], 300),
        ("ui:build", ["npm", "run", "build", "--prefix", "ui"], 300),
    ] if scope in ("ui", "all") else []
    desktop = [
        ("desktop:build", ["cargo", "build", "--locked", "--manifest-path", "src-tauri/Cargo.toml"], 1800),
        (
            "desktop:clippy",
            ["cargo", "clippy", "--locked", "--manifest-path", "src-tauri/Cargo.toml", "--all-targets", "--", *CLIPPY],
            1800,
        ),
    ]
    if scope == "all":
        selected = [
            (
                "python:unit",
                [sys.executable, "-B", "-m", "unittest", "discover", "-s", "scripts", "-p", "test_*.py"],
                300,
            ),
            ui[0],
            (
                "source:quality",
                [sys.executable, "-B", "-m", "autosar_tooling", "quality", *(["--base", base] if base else [])],
                600,
            ),
            ("python:lint", ["ruff", "check", "scripts"], 120),
            *ui[1:],
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


def verify(scope: str, base: str | None = None) -> int:
    target = "windows-x64-controlled-v1" if sys.platform == "win32" else "linux-x64-controlled-v1"
    print(f"Verification platform={platform.platform()} target={target} scope={scope}", flush=True)
    print("Native GUI/IPC, installed bundles and macOS: not exercised by this build/test gate.", flush=True)
    if scope in ("core", "all"):
        print("OS verification plan: the 26 registered native suites run once through cargo test; no second OS CLI pass.", flush=True)
        if sys.platform != "win32":
            print("Windows BCrypt legacy security and Windows-only legacy consumers: not applicable; native Linux OS/ECU consumers remain required.", flush=True)
    if sys.platform not in ("win32", "linux"):
        print("This native verification gate requires Windows or Linux.", file=sys.stderr)
        return 1
    logs = Path(tempfile.mkdtemp(prefix="autosar-verify-"))
    if os.name != "nt":
        logs.chmod(0o700)
    env = dict(os.environ)
    env["AUTOSAR_PYTHON"] = str(Path(sys.executable).absolute())
    env["RUST_TEST_THREADS"] = "2"
    if os.name == "nt" and env.get("AUTOSAR_CC"):
        # The pinned MinGW distribution also supplies the maintained Cppcheck
        # consumer used by the Windows-only generated artifact regression.
        env["PATH"] = os.pathsep.join((str(Path(env["AUTOSAR_CC"]).parent), env.get("PATH", "")))
    stage = "source:plan"
    argv: list[str] = []
    try:
        for stage, argv, seconds in commands(scope, base):
            if argv[0] == "npm":
                argv = npm_command(*argv[1:])
            else:
                argv[0] = executable(argv[0])
            print(f"> stage={stage} argv={argv!r}", flush=True)
            completion = (
                CompletionPolicy.CLOSE_TREE_ON_EXIT
                if stage in ("core:test", "core:clippy", "desktop:build", "desktop:clippy")
                else CompletionPolicy.REQUIRE_TREE_EXIT
            )
            spec = ProcessSpec.seconds(
                argv, ROOT, seconds, logs, stage, env, completion=completion
            )
            result = OwnedProcess(spec).wait()
            stdout = result.stdout.read_text(encoding="utf-8", errors="replace")
            stderr = result.stderr.read_text(encoding="utf-8", errors="replace")
            if stdout:
                print(stdout, end="" if stdout.endswith("\n") else "\n", flush=True)
            if stderr:
                print(stderr, end="" if stderr.endswith("\n") else "\n", file=sys.stderr, flush=True)
            if result.descendants_reclaimed:
                print(
                    f"> stage={stage} exit={result.exit_code} "
                    f"cleanup=confirmed descendants=reclaimed owned_logs={logs}",
                    flush=True,
                )
            if not result.success:
                print(f"Verification failed stage={stage} status={result.status} exit={result.exit_code} owned_logs={logs}", file=sys.stderr)
                return result.exit_code if result.exit_code is not None and result.exit_code > 0 else 1
    except (OSError, RuntimeError) as error:
        print(f"Verification failed stage={stage} argv={argv!r} exit=not-observed error={error}; owned_logs={logs}", file=sys.stderr)
        return 1
    print(f"Verification PASS target={target} scope={scope} owned_logs={logs}", flush=True)
    return 0
