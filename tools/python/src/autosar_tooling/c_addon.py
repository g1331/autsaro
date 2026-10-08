"""Small project checks over Cppcheck's parsed C, not a complete MISRA checker."""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ALLOCATORS = {"malloc", "calloc", "realloc", "free"}
THREAD_CREATORS = {"CreateThread", "pthread_create", "_beginthread", "_beginthreadex"}


def findings(configuration, root: Path):
    """Check direct calls in owned source; declarations and kernel headers are not calls."""
    for token in configuration.tokenlist:
        if not token.next or token.next.str != "(" or not token.next.astOperand1:
            continue
        # Call expressions have a name operand and an executable function scope.
        if token.next.astOperand1 is not token or not token.scope or not token.scope.isExecutable:
            continue
        path = Path(token.file).resolve()
        if not path.is_relative_to(root):
            continue
        relative = path.relative_to(root).as_posix()
        if relative.startswith("kernel/"):
            continue
        if token.str in ALLOCATORS:
            yield token, "dynamicAllocation", "Direct dynamic allocation call; review MISRA Dir 4.12."
        if token.str in THREAD_CREATORS and (
            relative.startswith("src/") and not relative.startswith("src/host/")
            and relative != "src/Application.c"
        ):
            yield token, "hostThreadInBsw", "Host thread creation belongs in the OS/host adapter."


def main() -> int:
    # Cppcheck's addon runtime supplies this module; never vendor its implementation.
    directory = next((arg.split("=", 1)[1] for arg in sys.argv if arg.startswith("--data-directory=")), None)
    if directory is None:
        raise ValueError("Cppcheck addon data directory is required")
    sys.path.insert(0, directory)
    import cppcheckdata

    parser = cppcheckdata.ArgumentParser()
    parser.add_argument("--data-directory", required=True)
    parser.add_argument("--source-root", required=True, type=Path)
    parser.add_argument("--receipt-directory", required=True, type=Path)
    args = parser.parse_args()
    root = args.source_root.resolve(strict=True)
    if not args.dumpfile:
        raise ValueError("Project addon received no dump files")
    for dump in args.dumpfile:
        data = cppcheckdata.CppcheckData(dump)
        configurations = 0
        units = set()
        for configuration in data.iterconfigurations():
            configurations += 1
            units.update(str(Path(token.file).resolve()) for token in configuration.tokenlist
                         if token.file.endswith(".c") and Path(token.file).resolve().is_relative_to(root))
            for token, identifier, message in findings(configuration, root):
                cppcheckdata.reportError(token, "warning", message, "autsaro", identifier)
        if configurations == 0:
            raise ValueError("Project addon received no parsed configurations")
        args.receipt_directory.mkdir(exist_ok=True)
        receipt = args.receipt_directory / (hashlib.sha256(str(dump).encode()).hexdigest() + ".json")
        receipt.write_text(json.dumps({"units": sorted(units), "configurations": configurations}), encoding="utf-8")
    return cppcheckdata.EXIT_CODE


if __name__ == "__main__":
    sys.exit(main())
