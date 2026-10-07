"""Read-only checks for the developer workbench and its external inputs."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import shutil
import tempfile
from pathlib import Path

from ecu_tools.process import OwnedProcess, ProcessSpec

from autosar_tooling.config import archive_path, archives

ROOT = Path(__file__).resolve().parents[2]
ARCHIVES = {name: entry for name, entry in archives().items() if entry[2]}


def _check(name: str, ok: bool, detail: str, *, mismatch: bool = False) -> bool:
    status = "ready" if ok else "version_mismatch" if mismatch else "missing"
    print(f"{name}: {status} — {detail}")
    return ok


def _archive(name: str, variable: str, relative: str, expected: str) -> bool:
    path = archive_path(variable, relative, ROOT)
    if not path.is_file():
        return _check(
            name,
            False,
            f"Obtain the licensed archive and set {variable} to its path: {path}",
        )
    try:
        with path.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
    except OSError as error:
        return _check(name, False, f"Cannot read {path}: {error}")
    return _check(
        name,
        digest == expected,
        f"{path}; SHA-256 {digest}; required {expected}",
        mismatch=True,
    )


def workbench() -> int:
    """Compatibility entry; new callers should select a task profile."""
    from autosar_tooling.diagnostics import report

    print("Legacy workbench check: use dev doctor --profile desktop or integration.")
    return report("desktop", strict=True) | report("integration", strict=True)


def _native_query(executable: Path, *args: str) -> tuple[str, str] | None:
    """Only explicit native checks launch a scoped, deadline-bound process."""
    try:
        with tempfile.TemporaryDirectory(prefix="autosar-doctor-") as temporary:
            spec = ProcessSpec.seconds(
                [str(executable), *args],
                ROOT,
                30,
                Path(temporary),
                f"native-identity-{executable.name}",
            )
            result = OwnedProcess(spec).wait()
            stdout = result.stdout.read_text(encoding="utf-8", errors="replace")
            stderr = result.stderr.read_text(encoding="utf-8", errors="replace")
            if not result.success:
                _check(
                    executable.name,
                    False,
                    f"status={result.status} exit={result.exit_code}: {stderr or stdout}",
                )
                return None
            return stdout, stderr
    except (OSError, RuntimeError) as error:
        _check(executable.name, False, f"Cannot query bounded identity: {error}")
        return None


def _native_path(name: str, variable: str | None = None) -> Path | None:
    selected = os.environ.get(variable) if variable else None
    found = shutil.which(selected or name)
    if not found:
        _check(
            name, False, f"Set {variable or 'PATH'} to an installed {name} executable"
        )
        return None
    path = Path(found).resolve()
    if not path.is_file():
        _check(name, False, f"Not a regular executable file: {path}")
        return None
    return path


def _native_digest(name: str, path: Path, expected: str) -> bool:
    try:
        with path.open("rb") as source:
            actual = hashlib.file_digest(source, "sha256").hexdigest()
    except OSError as error:
        return _check(name, False, f"Cannot read {path}: {error}")
    return _check(
        name,
        actual == expected,
        f"{path}; SHA-256 {actual}; required {expected}",
        mismatch=True,
    )


def native(
    target: str, *, include_archives: bool = True, require_explicit: bool = False
) -> int:
    locks = {
        "windows-x64-controlled-v1": ("Windows", "toolchain.json"),
        "linux-x64-controlled-v1": ("Linux", "toolchain-linux.json"),
    }
    system, lock_name = locks[target]
    pinned = json.loads((ROOT / "runtime/os" / lock_name).read_text(encoding="utf-8"))
    native_host = platform.system() == system and platform.machine().lower() in (
        "amd64",
        "x86_64",
    )
    checks = [
        _check(
            "native target",
            native_host,
            f"{platform.system()}/{platform.machine()}: "
            + (
                f"native execution of {target} is applicable"
                if native_host
                else f"cannot execute {target}; pure source rendering remains available"
            ),
        )
    ]
    if require_explicit:
        variables = ["AUTOSAR_CC", "AUTOSAR_OBJDUMP", "AUTOSAR_GIT", "AUTOSAR_PYTHON"]
        if system == "Linux":
            variables.append("AUTOSAR_NM")
        for variable in variables:
            value = os.environ.get(variable)
            checks.append(
                _check(
                    variable,
                    bool(value and Path(value).is_absolute() and Path(value).is_file()),
                    f"Set {variable} to the pinned executable's absolute path",
                )
            )
    if require_explicit and not all(checks):
        return 1
    git = _native_path("git", "AUTOSAR_GIT")
    if git:
        version = _native_query(git, "--version")
        checks.append(
            _check(
                "Git",
                bool(version and version[0].startswith("git version ")),
                f"{git}: {version[0].strip() if version else 'identity unavailable'}",
            )
        )
    else:
        checks.append(False)
    python = _native_path("python", "AUTOSAR_PYTHON")
    if python:
        version = _native_query(python, "--version")
        output = " ".join(version) if version else ""
        checks.append(
            _check(
                "CPython",
                output.strip() == "Python 3.12.9",
                f"{python}: {output.strip() or 'identity unavailable'}",
                mismatch=True,
            )
        )
    else:
        checks.append(False)
    compiler = _native_path("gcc", "AUTOSAR_CC")
    if compiler:
        version = _native_query(compiler, "--version")
        machine = _native_query(compiler, "-dumpmachine")
        observed = (
            version[0].splitlines()[0] if version and version[0].splitlines() else ""
        )
        expected = pinned["identity"]
        checks.append(
            _check(
                "compiler version",
                observed.partition(" ")[2] == expected.partition(" ")[2]
                and bool(machine and machine[0].strip() == pinned["target"]),
                f"{compiler}; {observed}; triple {machine[0].strip() if machine else 'unknown'}; required {expected} {pinned['target']}",
                mismatch=True,
            )
        )
        checks.append(
            _native_digest("compiler bytes", compiler, pinned["executable_sha256"])
        )
    else:
        checks.append(False)
    for name in ("objdump", "nm"):
        if name == "nm" and system == "Windows":
            continue
        variable = f"AUTOSAR_{name.upper()}"
        executable = _native_path(name, variable)
        if executable:
            version = _native_query(executable, "--version")
            checks.append(
                _check(
                    name,
                    bool(version and "GNU " in version[0]),
                    f"{executable}; {version[0].splitlines()[0] if version and version[0].splitlines() else 'identity unavailable'}",
                )
            )
            expected_hash = pinned.get(f"{name}_sha256")
            if expected_hash:
                checks.append(
                    _native_digest(f"{name} bytes", executable, expected_hash)
                )
        else:
            checks.append(False)
    if include_archives:
        for name, (variable, relative, expected) in ARCHIVES.items():
            checks.append(_archive(name, variable, relative, expected))
    return 0 if all(checks) else 1
