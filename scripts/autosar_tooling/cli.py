"""Developer tooling entry points."""

from __future__ import annotations

import argparse
from pathlib import Path

from autosar_tooling import doctor


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("doctor", help="Check the workbench prerequisites")
    check.add_argument("--role", choices=("workbench",), required=True)
    probe = commands.add_parser("probe", help="Run a named real-process probe")
    probe_kind = probe.add_subparsers(dest="probe", required=True)
    descendant = probe_kind.add_parser("descendant")
    descendant.add_argument("--pid-file", type=Path, required=True)
    descendant.add_argument(
        "--kind",
        choices=(
            "normal",
            "hang",
            "parent-first",
            "fail",
            "escape",
            "nested",
            "child",
            "normal-child",
            "leaf",
            "normal-leaf",
            "escape-child",
            "guardian",
        ),
        required=True,
    )
    args = parser.parse_args()
    if args.command == "doctor":
        return doctor.workbench()
    if args.command == "probe":
        from autosar_tooling.probe import descendant as run_descendant

        return run_descendant(args.pid_file, args.kind)
    return 1
