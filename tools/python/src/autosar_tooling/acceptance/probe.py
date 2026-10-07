"""Named real-process probes used by owner tests and consumer diagnostics."""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

from ecu_tools.process import ProcessSpec, run_bounded


def _record(path: Path, role: str) -> None:
    group = os.getpgid(0) if os.name != "nt" else 0
    with os.fdopen(
        os.open(path, os.O_CREAT | os.O_WRONLY | os.O_APPEND, 0o600), "w"
    ) as output:
        output.write(f"{role} {os.getpid()} {group}\n")


def _spawn(path: Path, kind: str, *, detached: bool = False) -> subprocess.Popen[bytes]:
    options: dict[str, object] = {
        "stdin": subprocess.DEVNULL,
        "stdout": subprocess.DEVNULL,
        "stderr": subprocess.DEVNULL,
    }
    if os.name == "nt":
        options["creationflags"] = subprocess.CREATE_NO_WINDOW
    elif detached:
        options["start_new_session"] = True
    return subprocess.Popen(
        [
            sys.executable,
            "-m",
            "autosar_tooling",
            "probe",
            "descendant",
            "--pid-file",
            str(path),
            "--kind",
            kind,
        ],
        **options,
    )


def _wait_for_records(path: Path, count: int) -> None:
    end = time.monotonic() + 5
    while time.monotonic() < end:
        if (
            path.exists()
            and len(path.read_text(encoding="ascii").splitlines()) >= count
        ):
            return
        time.sleep(0.01)
    raise RuntimeError(f"descendant registration did not reach {count} records")


def descendant(pid_file: Path, kind: str) -> int:
    if kind == "leaf":
        _record(pid_file, "leaf")
        time.sleep(30)
        return 0
    if kind == "normal-leaf":
        _record(pid_file, "leaf")
        return 0
    if kind == "escape-child":
        _record(pid_file, "escape")
        time.sleep(30)
        return 0
    if kind in ("child", "normal-child"):
        _record(pid_file, "child")
        leaf = _spawn(pid_file, "normal-leaf" if kind == "normal-child" else "leaf")
        if kind == "normal-child":
            return leaf.wait(timeout=5)
        time.sleep(30)
        return 0
    if kind == "guardian":
        from ecu_tools.owner import Owner
        from ecu_tools.process import OwnedProcess

        spec = ProcessSpec.seconds(
            [
                sys.executable,
                "-m",
                "autosar_tooling",
                "probe",
                "descendant",
                "--pid-file",
                str(pid_file),
                "--kind",
                "hang",
            ],
            Path.cwd(),
            15,
            pid_file.parent,
            "guardian-probe",
        )
        with Owner.start() as owner:
            OwnedProcess(spec, owner=owner)
            _wait_for_records(pid_file, 3)
            _record(pid_file, "guardian")
            time.sleep(30)
        return 0
    _record(pid_file, "parent")
    if kind == "fail":
        print("probe failure on stderr", file=sys.stderr, flush=True)
        return 7
    if kind == "nested-parent-first":
        from ecu_tools.process import OwnedProcess

        spec = ProcessSpec.seconds(
            [
                sys.executable, "-m", "autosar_tooling", "probe", "descendant",
                "--pid-file", str(pid_file), "--kind", "hang",
            ],
            Path.cwd(), 10, pid_file.parent, "unfinished-nested-probe",
        )
        OwnedProcess(spec)
        _wait_for_records(pid_file, 4)
        return 0
    if kind == "nested":
        spec = ProcessSpec.seconds(
            [
                sys.executable,
                "-m",
                "autosar_tooling",
                "probe",
                "descendant",
                "--pid-file",
                str(pid_file),
                "--kind",
                "normal-leaf",
            ],
            Path.cwd(),
            10,
            pid_file.parent,
            "nested-probe",
        )
        run_bounded(spec)
        return 0
    if kind == "escape":
        _spawn(pid_file, "escape-child", detached=True)
        _wait_for_records(pid_file, 2)
        return 0
    if kind == "normal":
        child = _spawn(pid_file, "normal-child")
        return child.wait(timeout=5)
    if kind in ("hang", "parent-first", "parent-first-fail"):
        _spawn(pid_file, "child")
        _wait_for_records(pid_file, 3)
        if kind in ("parent-first", "parent-first-fail"):
            if kind == "parent-first-fail":
                print("probe failure with live descendants", file=sys.stderr, flush=True)
                return 7
            return 0
        time.sleep(30)
        return 0
    raise ValueError(f"Unknown descendant probe kind: {kind}")


def stdin(kind: str) -> int:
    """Exercise real interactive bytes/EOF or deliberate blocked input."""
    print(os.getpid(), flush=True)
    if kind == "hang":
        time.sleep(30)
        return 0
    if kind == "hash":
        import hashlib

        digest = hashlib.sha256()
        while data := sys.stdin.buffer.read(8192):
            digest.update(data)
        print(digest.hexdigest(), flush=True)
        return 0
    raise ValueError(f"Unknown stdin probe kind: {kind}")
