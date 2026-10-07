"""Cooperative process-group launcher; no target instruction runs before release."""

from __future__ import annotations

import os
import sys


def main() -> int:
    if os.name == "nt" or len(sys.argv) < 4 or sys.argv[2] != "--":
        raise SystemExit("launch requires a POSIX gate FD and an argv vector")
    gate = int(sys.argv[1])
    permit = os.read(gate, 1)
    os.close(gate)
    if permit != b"1":
        return 90  # Closed or failed supervisor: actual command never executes.
    argv = sys.argv[3:]
    if not argv or not os.path.isabs(argv[0]):
        return 91
    try:
        os.execve(argv[0], argv, os.environ)
    except OSError as error:
        print(f"Cannot execute registered command: {error}", file=sys.stderr)
        return 127


if __name__ == "__main__":
    raise SystemExit(main())
