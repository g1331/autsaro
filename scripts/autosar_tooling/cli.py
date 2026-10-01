"""Developer tooling entry points."""

from __future__ import annotations

import argparse

from autosar_tooling import doctor


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("doctor", help="Check the workbench prerequisites")
    check.add_argument("--role", choices=("workbench",), required=True)
    args = parser.parse_args()
    if args.command == "doctor":
        return doctor.workbench()
    return 1
