"""Run the build, tests and source checks selected by a BMad task."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def verify(scope: str, base: str | None = None) -> int:
    npm = "npm.cmd" if sys.platform == "win32" else "npm"
    clippy = [
        "-A",
        "clippy::all",
        "-D",
        "clippy::correctness",
        "-D",
        "clippy::suspicious",
    ]
    commands = {
        "core": [
            ["cargo", "test", "--manifest-path", "core/Cargo.toml"],
            [
                "cargo",
                "clippy",
                "--manifest-path",
                "core/Cargo.toml",
                "--all-targets",
                "--",
                *clippy,
            ],
        ],
        "ui": [
            [npm, "ci", "--prefix", "ui"],
            [npm, "run", "lint", "--prefix", "ui"],
            [npm, "run", "build", "--prefix", "ui"],
        ],
        "desktop": [
            ["cargo", "build", "--manifest-path", "src-tauri/Cargo.toml"],
            [
                "cargo",
                "clippy",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--all-targets",
                "--",
                *clippy,
            ],
        ],
        "all": [
            [
                sys.executable,
                "-B",
                "-m",
                "unittest",
                "discover",
                "-s",
                "scripts",
                "-p",
                "test_*.py",
            ],
            [npm, "ci", "--prefix", "ui"],
            [
                sys.executable,
                "-B",
                "scripts/quality.py",
                *(["--base", base] if base else []),
            ],
            [npm, "run", "lint", "--prefix", "ui"],
            [npm, "run", "build", "--prefix", "ui"],
            ["cargo", "test", "--manifest-path", "core/Cargo.toml"],
            [
                "cargo",
                "clippy",
                "--manifest-path",
                "core/Cargo.toml",
                "--all-targets",
                "--",
                *clippy,
            ],
            ["cargo", "build", "--manifest-path", "src-tauri/Cargo.toml"],
            [
                "cargo",
                "clippy",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--all-targets",
                "--",
                *clippy,
            ],
        ],
    }
    for command in [
        ["git", "diff", "--check"],
        ["git", "diff", "--cached", "--check"],
        *commands[scope],
    ]:
        display = [
            "python"
            if argument == sys.executable
            else argument
            for argument in command
        ]
        print(f"> {' '.join(display)}", flush=True)
        try:
            result = subprocess.run(command, cwd=ROOT, check=False)
        except OSError as error:
            print(f"Cannot run {command[0]}: {error}", file=sys.stderr)
            return 1
        if result.returncode:
            print(
                f"Verification stopped: exit code {result.returncode}", file=sys.stderr
            )
            return result.returncode
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--scope", choices=("core", "ui", "desktop", "all"), required=True
    )
    parser.add_argument(
        "--base", help="Story baseline commit for incremental formatting"
    )
    args = parser.parse_args()
    if args.base and args.scope != "all":
        parser.error("--base requires --scope all")
    return verify(args.scope, args.base)


if __name__ == "__main__":
    raise SystemExit(main())
