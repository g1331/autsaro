"""Standard-library-only commands delivered alongside immutable ECU sources."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


class Once(argparse.Action):
    def __call__(self, parser, namespace, values, option_string=None):
        marker = "_seen_" + self.dest
        if getattr(namespace, marker, False):
            parser.error(f"Duplicate argument: {option_string}")
        setattr(namespace, marker, True)
        setattr(namespace, self.dest, values)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("build", help="Build a pinned native ECU target")
    build.add_argument("--project", required=True, type=Path, action=Once)
    build.add_argument("--output", required=True, type=Path, action=Once)
    build.add_argument("--mode", choices=("host-batch", "probe", "test", "host"), required=True, action=Once)
    build.add_argument("--control-source", type=Path, action=Once)
    verify = commands.add_parser("verify", help="Run an independent real production protocol oracle")
    verify.add_argument("--project", required=True, type=Path, action=Once)
    verify.add_argument("--build-directory", required=True, type=Path, action=Once)
    verify.add_argument("--report-path", type=Path, action=Once)
    args = parser.parse_args()
    if sys.version_info < (3, 11):  # noqa: UP036 -- shipped without this project's Python metadata
        parser.error("Offline engineering tools require Python 3.11 or newer")
    try:
        if args.command == "build":
            from ecu_tools.build import build as run_build
            run_build(args.project, args.output, args.mode, args.control_source)
        else:
            target = json.loads((args.project / "target.json").read_text(encoding="utf-8"))
            if target.get("profile") == "host-reference":
                from ecu_tools.reference import verify as run_reference
                run_reference(args.project, args.build_directory, args.report_path)
            else:
                if args.report_path is not None:
                    raise ValueError("--report-path is only applicable to a fixed dual-host reference package")
                from ecu_tools.verify import verify as run_verify
                run_verify(args.project, args.build_directory)
    except (OSError, ValueError, RuntimeError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        return 1
    return 0
