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
    quality = commands.add_parser(
        "quality", help="Check source hygiene and incremental formatting"
    )
    baseline = quality.add_mutually_exclusive_group()
    baseline.add_argument("--base")
    baseline.add_argument("--merge-base")
    quality.add_argument("--all-format", action="store_true")
    quality.add_argument(
        "--scope",
        choices=("ui", "tooling", "core", "desktop", "runtime", "all"),
        default="all",
    )
    quality.add_argument("--c-samples", type=Path)
    c_check = commands.add_parser("c-check", help="Run pinned free C/MISRA analysis on sealed sources")
    c_check.add_argument("--target", choices=("windows-x64-controlled-v1", "linux-x64-controlled-v1"), required=True)
    c_check.add_argument("--project", type=Path, required=True)
    c_check.add_argument("--output-directory", type=Path, required=True)
    c_check.add_argument("--baseline", type=Path, help="Full matching c-analysis artifact sample directory")
    asset = commands.add_parser("assets", help="Check assets or update the seal-guard digest")
    asset.add_argument("action", choices=("check", "update"))
    commands.add_parser(
        "fetch-official", help="Download and verify the pinned official development archives"
    )
    requirement = commands.add_parser(
        "requirements",
        help="List a specification's undisposed official requirement IDs and stale ID citations",
    )
    requirement.add_argument(
        "document",
        nargs="?",
        help="Specification document name, such as SWS_CANInterface; without it, check every committed catalog offline",
    )
    mode = requirement.add_mutually_exclusive_group()
    mode.add_argument(
        "--stale-only", action="store_true", help="Fail only on stale citations, for a module whose rework is still open"
    )
    mode.add_argument(
        "--update", action="store_true", help="Write the document's catalog from the archive, registering it for the required check"
    )
    requirement.add_argument(
        "--module",
        help="Module abbreviation, such as CanIf, to account SWS_BSWGeneral for; with --update, register the module for the required check",
    )
    coverage = commands.add_parser(
        "coverage", help="Draw the architecture diagrams with each delivered module's requirement accounting"
    )
    coverage.add_argument("--output", type=Path, required=True, help="Directory for architecture.svg and architecture-full.svg")
    args = parser.parse_args()
    if args.command == "coverage":
        from autosar_tooling.coverage import generate
        return generate(args.output)
    if args.command == "c-check":
        from autosar_tooling.c_check import check
        return check(args.project, args.target, args.output_directory, args.baseline)
    if args.command == "assets":
        from autosar_tooling.assets import maintain
        return maintain(update=args.action == "update")
    if args.command == "fetch-official":
        from autosar_tooling.official import fetch
        return fetch()
    if args.command == "requirements":
        from autosar_tooling import requirements

        if args.document:
            return requirements.check(args.document, stale_only=args.stale_only, update=args.update, module=args.module)
        if args.stale_only or args.update or args.module:
            parser.error("--stale-only, --update and --module require a document")
        return requirements.check_catalogs()
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
    if args.command == "quality":
        from autosar_tooling.quality import main as check_quality

        options = (["--base", args.base] if args.base else []) + (
            ["--merge-base", args.merge_base] if args.merge_base else []
        ) + (["--all-format"] if args.all_format else [])
        if args.c_samples:
            options += ["--c-samples", str(args.c_samples)]
        return check_quality([*options, "--scope", args.scope])
    return 1
