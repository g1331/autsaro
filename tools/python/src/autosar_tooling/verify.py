"""Run the existing project checks with normal terminal output."""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

from autosar_tooling.config import ROOT, environment

CLIPPY = ["-A", "clippy::all", "-D", "clippy::correctness", "-D", "clippy::suspicious"]

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
                "--manifest-path",
                "core/Cargo.toml",
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
                "--manifest-path",
                "core/Cargo.toml",
                "--all-targets",
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
            ["cargo", "test", "--locked", "--manifest-path", "src-tauri/Cargo.toml"],
            1800,
        ),
        (
            "desktop:build",
            ["cargo", "build", "--locked", "--manifest-path", "src-tauri/Cargo.toml"],
            1800,
        ),
        (
            "desktop:clippy",
            [
                "cargo",
                "clippy",
                "--locked",
                "--manifest-path",
                "src-tauri/Cargo.toml",
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
            ("python:lint", ["ruff", "check", "tools", "tests/python", "scripts"], 120),
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


def verify(scope: str, base: str | None = None) -> int:
    if scope in ("core", "all") and sys.platform not in ("win32", "linux"):
        print("Native verification requires Windows or Linux.", file=sys.stderr)
        return 1
    print("GUI/IPC and installed bundle acceptance are separate checks.", flush=True)
    env = environment()
    for name, command, timeout in commands(scope, base):
        argv = npm_command(*command[1:]) if command[0] == "npm" else [executable(command[0]), *command[1:]]
        print(f"> {name}: {subprocess.list2cmdline(argv)}", flush=True)
        try:
            result = subprocess.run(argv, cwd=ROOT, env=env, timeout=timeout, check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            print(f"{name}: {error}", file=sys.stderr)
            return 1
        if result.returncode:
            return result.returncode if result.returncode > 0 else 1
    return 0
