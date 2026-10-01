"""Developer tooling entry points."""

from __future__ import annotations

import argparse
from pathlib import Path

from autosar_tooling import doctor, os_suites


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("doctor", help="Check the workbench prerequisites")
    check.add_argument("--role", choices=("workbench", "native"), required=True)
    check.add_argument(
        "--target",
        choices=("windows-x64-controlled-v1", "linux-x64-controlled-v1"),
        help="Required for --role native; identifies the output target, not the workbench",
    )
    native_os = commands.add_parser("os", help="Run independent native OS C99 suites")
    native_os.add_argument(
        "--target",
        choices=("windows-x64-controlled-v1", "linux-x64-controlled-v1"),
        required=True,
    )
    native_os.add_argument("--suite", choices=(*os_suites.SUITES, "all"), required=True)
    native_arti = commands.add_parser("os-arti-native", help="Verify generated ARTI bindings")
    native_arti.add_argument("--directory", type=Path, required=True)
    native_counter = commands.add_parser(
        "os-counter-service", help="Verify generated OS counter service"
    )
    native_counter.add_argument("--directory", type=Path, required=True)
    native_counter.add_argument("--project", type=Path, required=True)
    native_counter.add_argument("--counter", required=True)
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
        if args.role == "native":
            if not args.target:
                parser.error("--role native requires --target")
            return doctor.native(args.target)
        if args.target:
            parser.error("--role workbench does not take --target")
        return doctor.workbench()
    if args.command == "os":
        return os_suites.run(args.target, args.suite)
    if args.command == "os-arti-native":
        os_suites.arti_native(args.directory)
        return 0
    if args.command == "os-counter-service":
        os_suites.counter_service(args.directory, args.project, args.counter)
        return 0
    if args.command == "probe":
        from autosar_tooling.probe import descendant as run_descendant

        return run_descendant(args.pid_file, args.kind)
    return 1
