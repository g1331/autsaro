"""Developer tooling entry points."""

from __future__ import annotations

import argparse
import os
from pathlib import Path

from autosar_tooling import doctor


def main() -> int:
    from autosar_tooling.config import environment

    os.environ.update(environment())
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("doctor", help="Check the workbench prerequisites")
    check.add_argument("--role", choices=("workbench", "native"), default="workbench")
    check.add_argument("--component", choices=("ui", "core", "desktop", "resources"), default="desktop")
    check.add_argument(
        "--target",
        choices=("windows-x64-controlled-v1", "linux-x64-controlled-v1"),
        help="Required for --role native; identifies the output target, not the workbench",
    )
    aggregate = commands.add_parser("verify", help="Run project checks sequentially")
    aggregate.add_argument(
        "--scope", choices=("core", "ui", "desktop", "all"), required=True
    )
    aggregate.add_argument(
        "--base", help="Incremental quality baseline for the all gate"
    )
    desktop = commands.add_parser(
        "desktop", help="Verify actual IPC in an isolated native desktop"
    )
    desktop.add_argument(
        "--platform", choices=("windows", "linux", "macos"), required=True
    )
    desktop.add_argument("--binary", type=Path, required=True)
    desktop.add_argument("--output-directory", type=Path)
    desktop.add_argument(
        "--installed",
        action="store_true",
        help="Verify extracted release app without Vite",
    )
    desktop.add_argument(
        "--source-checkout",
        type=Path,
        help="Original build checkout, required to be absent for installed verification",
    )
    quality = commands.add_parser(
        "quality", help="Check source hygiene and incremental formatting"
    )
    quality.add_argument("--base")
    quality.add_argument("--all-format", action="store_true")
    quality.add_argument(
        "--scope",
        choices=("ui", "tooling", "core", "desktop", "runtime", "all"),
        default="all",
    )
    c_check = commands.add_parser("c-check", help="Run pinned free C/MISRA analysis on sealed sources")
    c_check.add_argument("--target", choices=("windows-x64-controlled-v1", "linux-x64-controlled-v1"), required=True)
    c_check.add_argument("--project", type=Path, required=True)
    c_check.add_argument("--output-directory", type=Path, required=True)
    oracles = commands.add_parser(
        "protocol-oracles", help="Check independent protocol/OS reference vectors"
    )
    oracles.add_argument(
        "--ecuc", type=Path, help="ECUC file the oracle settings must match"
    )
    asset = commands.add_parser("assets", help="Check or update project-owned resource digests")
    asset.add_argument("action", choices=("check", "update"))
    args = parser.parse_args()
    if args.command == "c-check":
        from autosar_tooling.c_check import check
        return check(args.project, args.target, args.output_directory)
    if args.command == "assets":
        from autosar_tooling.assets import maintain
        return maintain(update=args.action == "update")
    if args.command == "doctor":
        if args.role == "native":
            if not args.target:
                parser.error("--role native requires --target")
            return doctor.native(args.target)
        if args.target:
            parser.error("--role workbench does not take --target")
        return doctor.report(args.component)
    if args.command == "verify":
        from autosar_tooling.verify import verify

        return verify(args.scope, args.base)
    if args.command == "desktop":
        from autosar_tooling.acceptance.desktop import run

        return run(
            args.platform,
            args.binary,
            args.installed,
            args.source_checkout,
            output_directory=args.output_directory,
        )
    if args.command == "quality":
        from autosar_tooling.quality import main as check_quality

        options = (["--base", args.base] if args.base else []) + (
            ["--all-format"] if args.all_format else []
        )
        return check_quality([*options, "--scope", args.scope])
    if args.command == "protocol-oracles":
        from autosar_tooling.protocol_oracles import main as check_protocol

        check_protocol(args.ecuc)
        return 0
    return 1
