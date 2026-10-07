"""Common development commands for contributors, agents and CI."""

from __future__ import annotations

import argparse
import json
import os
import sys

from autosar_tooling.config import ROOT, environment
from autosar_tooling.diagnostics import PROFILES, report
from autosar_tooling.verify import CLIPPY, commands, execute

SCOPES = ("ui", "tooling", "core", "desktop", "runtime", "all")
SUITES = ("fast", "core", "ui", "process", "integration", "native", "all")


def test_plan(
    suite: str, pattern: str | None = None
) -> list[tuple[str, list[str], int]]:
    cargo = ["cargo", "test", "--locked", "--manifest-path", "core/Cargo.toml"]
    extra = [pattern] if pattern else []
    modules = [
        "autosar_tooling." + path.stem
        for path in sorted((ROOT / "scripts/autosar_tooling").glob("test_*.py"))
        if path.stem != "test_process"
    ]
    python = [
        sys.executable,
        "-B",
        "-m",
        "unittest",
        *modules,
        *(["-k", pattern] if pattern else []),
    ]
    if suite == "fast":
        return [("python:unit", python, 300)]
    if suite == "ui":
        if pattern:
            raise ValueError(
                "--filter is supported by Python and Cargo suites; use npm's node --test options for UI"
            )
        return [("ui:test", ["npm", "run", "test", "--prefix", "ui"], 300)]
    if suite == "core":
        return [
            (
                "core:unit",
                [*cargo, "--lib", *extra, "--", "--skip", "execution::"],
                1800,
            ),
            ("core:builtin", [*cargo, "--test", "builtin", *extra], 1800),
        ]
    if suite == "process":
        return [
            (
                "python:process",
                [
                    sys.executable,
                    "-B",
                    "-m",
                    "unittest",
                    "autosar_tooling.test_process",
                    *(["-k", pattern] if pattern else []),
                ],
                300,
            )
        ]
    if suite == "integration":
        return [
            (
                "core:integration",
                [
                    *cargo,
                    "--features",
                    "official-oracles",
                    "--test",
                    "end_to_end",
                    "--test",
                    "builtin",
                    *extra,
                ],
                3600,
            )
        ]
    if suite == "native":
        return [
            *test_plan("process", pattern),
            ("core:process", [*cargo, "--lib", *(extra or ["execution::"])], 1800),
            (
                "core:native",
                [*cargo, "--features", "native-tests", "--test", "native", *extra],
                7200,
            ),
        ]
    if suite == "all":
        return [
            (
                "python:all",
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
                    *(["-k", pattern] if pattern else []),
                ],
                600,
            ),
            (
                "core:all",
                [*cargo, "--features", "official-oracles,native-tests", *extra],
                7200,
            ),
            *test_plan("ui"),
        ]
    raise ValueError(f"Unknown suite: {suite}")


def check_plan(
    scope: str, base: str | None, full: bool = False
) -> list[tuple[str, list[str], int]]:
    quality = (
        "source:quality",
        [
            sys.executable,
            "-B",
            "-m",
            "autosar_tooling",
            "quality",
            "--scope",
            scope,
            *(["--base", base] if base else []),
        ],
        600,
    )
    plan = [
        ("source:diff", ["git", "diff", "--check"], 30),
        ("source:staged-diff", ["git", "diff", "--cached", "--check"], 30),
        quality,
    ]
    if scope in ("tooling", "all"):
        plan += [("python:lint", ["ruff", "check", "scripts"], 120), *test_plan("fast")]
    if scope in ("ui", "all"):
        plan += [
            ("ui:lint", ["npm", "run", "lint", "--prefix", "ui"], 300),
            *test_plan("ui"),
            ("ui:build", ["npm", "run", "build", "--prefix", "ui"], 300),
        ]
    if scope in ("core", "all"):
        plan += test_plan("all")[:2] if full else test_plan("core")
        plan += [
            (
                "core:clippy",
                [
                    "cargo",
                    "clippy",
                    "--locked",
                    "--manifest-path",
                    "core/Cargo.toml",
                    "--all-targets",
                    "--all-features",
                    "--",
                    *CLIPPY,
                ],
                1800,
            )
        ]
    if scope in ("desktop", "all"):
        plan += [
            item for item in commands("desktop", base) if item[0].startswith("desktop:")
        ]
    if scope in ("core", "runtime", "all"):
        plan += [
            (
                "assets:check",
                [sys.executable, "-B", "-m", "autosar_tooling.dev", "assets", "check"],
                120,
            )
        ]
    return plan


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    setup = sub.add_parser(
        "setup", help="Install locked project dependencies; never installs OS packages"
    )
    setup.add_argument(
        "--profile", choices=("ui", "tooling", "core", "desktop", "all"), default="ui"
    )
    doctor = sub.add_parser("doctor", help="Check prerequisites for one task")
    doctor.add_argument("--profile", choices=PROFILES, default="ui")
    doctor.add_argument(
        "--strict",
        action="store_true",
        help="Require recommended patch versions as well",
    )
    doctor.add_argument(
        "--target", choices=("windows-x64-controlled-v1", "linux-x64-controlled-v1")
    )
    doctor.add_argument("--json", action="store_true")
    start = sub.add_parser(
        "start", help="Interactive development in your own desktop session"
    )
    start.add_argument("application", choices=("ui", "desktop"))
    check = sub.add_parser(
        "check", help="Check an area; default core checks do not need archives"
    )
    check.add_argument("--scope", choices=SCOPES, default="tooling")
    check.add_argument(
        "--base", help="Explicit Git baseline; default HEAD (pending changes)"
    )
    check.add_argument(
        "--full", action="store_true", help="Include strict resource and native tests"
    )
    test = sub.add_parser("test", help="Run a named test layer")
    test.add_argument("--suite", choices=SUITES, default="fast")
    test.add_argument("--filter", help="Python/Cargo test name substring")
    fmt = sub.add_parser(
        "fmt", help="Format changed files; review file diffs before committing"
    )
    fmt.add_argument("--scope", choices=SCOPES, default="all")
    fmt.add_argument("--base", default="HEAD")
    assets = sub.add_parser(
        "assets", help="Check or explicitly update project-owned inventories"
    )
    assets.add_argument("action", choices=("check", "update"))
    for command in (setup, start, check, test):
        command.add_argument(
            "--plan", action="store_true", help="Print commands without executing"
        )
        command.add_argument(
            "--json",
            action="store_true",
            help="Machine-readable plan/result; live logs go to stderr",
        )
    args = parser.parse_args(argv)
    try:
        env = environment()
        # Only this CLI process and its children receive local settings.
        os.environ.update(env)
        if args.command == "doctor":
            if args.target and args.profile not in ("native", "all"):
                parser.error("--target applies only to native/all profiles")
            return report(
                args.profile,
                strict=args.strict,
                target=args.target,
                json_output=args.json,
            )
        if args.command == "assets":
            from autosar_tooling.assets import maintain

            return maintain(update=args.action == "update")
        if args.command == "fmt":
            from autosar_tooling.quality import fix

            return fix(args.base, args.scope)
        if args.command == "setup":
            stages = [("python:dependencies", ["uv", "sync", "--locked"], 900)]
            if args.profile in ("ui", "desktop", "all"):
                stages.append(("ui:dependencies", ["npm", "ci", "--prefix", "ui"], 900))
        elif args.command == "start":
            stages = [
                (
                    f"start:{args.application}",
                    ["npm", "run", "dev", "--prefix", "ui"]
                    if args.application == "ui"
                    else ["npm", "run", "tauri", "--prefix", "ui", "--", "dev"],
                    86400,
                )
            ]
        elif args.command == "check":
            if args.full and args.scope not in ("core", "all"):
                parser.error("--full applies only to core/all checks")
            stages = check_plan(args.scope, args.base, args.full)
        else:
            if args.suite in ("native", "all") and sys.platform not in (
                "win32",
                "linux",
            ):
                parser.error(
                    "Native tests require Windows or Linux; select core/ui/fast instead"
                )
            stages = test_plan(args.suite, args.filter)
        return execute(stages, plan=args.plan, json_output=args.json)
    except (OSError, ValueError, RuntimeError) as error:
        if getattr(args, "json", False):
            print(json.dumps({"passed": False, "error": str(error)}))
        print(f"Development command failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
