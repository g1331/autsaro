"""Read-only dependency checks; no installation or execution plan."""
from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import tempfile
from contextvars import ContextVar
from dataclasses import dataclass
from pathlib import Path

from ecu_tools.process import OwnedProcess, ProcessSpec

from autosar_tooling.config import ROOT, archive_path, archives


@dataclass
class Check:
    name: str
    status: str
    detail: str
    fix: str = ""

_native_results: ContextVar[list[Check] | None] = ContextVar("native_results", default=None)

def version_check(name: str, args: list[str], *, major: int | None = None, minimum: tuple[int, ...] | None = None) -> Check:
    path = shutil.which(args[0])
    if not path:
        return Check(name, "blocked", "Executable not found", f"Install {args[0]}")
    try:
        from autosar_tooling.verify import npm_command
        command = npm_command(*args[1:]) if args[0] == "npm" else [path, *args[1:]]
        result = subprocess.run(command, capture_output=True, text=True, timeout=30, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        return Check(name, "error", str(error))
    output = (result.stdout or result.stderr).strip()
    match = re.search(r"(?<!\d)(\d+)\.(\d+)\.(\d+)\b", output)
    if result.returncode or not match:
        return Check(name, "error", output)
    version = tuple(map(int, match.groups()))
    compatible = (major is None or version[0] == major) and (minimum is None or version >= minimum)
    return Check(name, "ready" if compatible else "blocked", output)

def inspect(component: str, target: str | None = None) -> list[Check]:
    checks = [version_check("Git", ["git", "--version"])]
    if component in ("ui", "desktop"):
        checks += [version_check("Node", ["node", "--version"], major=24), version_check("npm", ["npm", "--version"], major=11)]
        checks.append(Check("UI dependencies", "ready" if (ROOT / "ui/node_modules/typescript/bin/tsc").is_file() else "blocked", "ui/node_modules", "npm ci --prefix ui"))
    if component in ("core", "desktop"):
        checks += [version_check("Rust", ["rustc", "--version"]), version_check("Cargo", ["cargo", "--version"])]
        _platform_dependencies(checks, component == "desktop")
    if component == "resources":
        token = _native_results.set(checks)
        try:
            for name, (variable, relative, expected) in ((name, entry) for name, entry in archives().items() if entry[2]):
                _archive(name, variable, relative, expected)
        finally:
            _native_results.reset(token)
    if component == "native":
        token = _native_results.set(checks)
        try:
            native(target or ("windows-x64-controlled-v1" if os.name == "nt" else "linux-x64-controlled-v1"), include_archives=False, require_explicit=True)
        finally:
            _native_results.reset(token)
    return checks

def report(component: str, *, target: str | None = None) -> int:
    checks = inspect(component, target)
    for check in checks:
        print(f"[{check.status}] {check.name}: {check.detail}")
        if check.fix and check.status in ("blocked", "error"):
            print(f"  {check.fix}")
    return int(any(check.status in ("blocked", "error") for check in checks))

def _platform_dependencies(checks: list[Check], desktop: bool) -> None:
    system = platform.system()
    if system == "Windows":
        root, triplet = os.environ.get("VCPKG_ROOT"), os.environ.get("VCPKGRS_TRIPLET")
        ok = bool(
            root
            and triplet == "x64-windows-static-md"
            and (Path(root) / "installed" / triplet / "lib/libxml2.lib").is_file()
        )
        checks.append(
            Check(
                "libxml2",
                "ready" if ok else "blocked",
                "vcpkg x64-windows-static-md",
                "Set VCPKG_ROOT and VCPKGRS_TRIPLET; install libxml2[iconv,zlib]",
            )
        )
        lib = os.environ.get("LIBCLANG_PATH")
        checks.append(
            Check(
                "libclang",
                "ready"
                if lib and (Path(lib) / "libclang.dll").is_file()
                else "blocked",
                lib or "LIBCLANG_PATH not set",
                "Set LIBCLANG_PATH to the directory containing libclang.dll",
            )
        )
        if desktop:
            checks.extend(
                [
                    Check(
                        "MSVC build tools",
                        "warning",
                        "Visual Studio C++ tools and Windows SDK are required; installation was not probed",
                        "See docs/development/environment.md",
                    ),
                    Check(
                        "WebView2",
                        "warning",
                        "Desktop runtime installation was not probed",
                        "See docs/development/environment.md",
                    ),
                ]
            )
        return
    if system not in ("Linux", "Darwin"):
        checks.append(
            Check("platform", "blocked", system, "Use Windows, Linux or macOS")
        )
        return
    packages = ["libxml-2.0"]
    checks.append(
        Check(
            "C linker",
            "ready" if shutil.which("cc") else "blocked",
            "cc on PATH",
            "Install build-essential or Xcode Command Line Tools",
        )
    )
    if desktop and system == "Linux":
        packages += ["webkit2gtk-4.1", "ayatana-appindicator3-0.1", "librsvg-2.0"]
    pkg = shutil.which("pkg-config")
    for package in packages:
        try:
            ok = bool(
                pkg
                and subprocess.run(
                    [pkg, "--exists", package],
                    capture_output=True,
                    timeout=30,
                    check=False,
                ).returncode
                == 0
            )
            checks.append(
                Check(
                    package,
                    "ready" if ok else "blocked",
                    "pkg-config",
                    "Install platform dependencies listed in docs/development/environment.md",
                )
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            checks.append(Check(package, "error", str(error), "Check pkg-config"))
    lib = os.environ.get("LIBCLANG_PATH")
    found = bool(lib and any(Path(lib).glob("libclang*.*"))) or any(
        Path("/usr/lib").glob("llvm-*/lib/libclang*.so*")
    )
    if system == "Linux":
        checks.append(
            Check(
                "libclang",
                "ready" if found else "blocked",
                "bindgen dependency",
                "Install libclang-dev or set LIBCLANG_PATH",
            )
        )
    else:
        found = found or any(
            path.is_file()
            for path in (
                Path("/Library/Developer/CommandLineTools/usr/lib/libclang.dylib"),
                Path(
                    "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/libclang.dylib"
                ),
                Path("/opt/homebrew/opt/llvm/lib/libclang.dylib"),
                Path("/usr/local/opt/llvm/lib/libclang.dylib"),
            )
        )
        checks.append(
            Check(
                "libclang",
                "ready" if found else "blocked",
                "Xcode/LLVM bindgen dependency",
                "Install LLVM/Xcode tools or set LIBCLANG_PATH",
            )
        )
        checks.append(
            Check(
                "macOS native acceptance",
                "warning",
                "Native desktop/IPC and bundles are not verified",
            )
        )


def _check(name: str, ok: bool, detail: str, *, mismatch: bool = False) -> bool:
    status = "ready" if ok else "version_mismatch" if mismatch else "missing"
    collected = _native_results.get()
    if collected is None:
        print(f"{name}: {status} — {detail}")
    else:
        collected.append(Check(name, "ready" if ok else "blocked", detail))
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
                bool(re.search(r"Python (\d+)\.(\d+)", output) and tuple(map(int, re.search(r"Python (\d+)\.(\d+)", output).groups())) >= (3, 11)),
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
        for name, (variable, relative, expected) in ((name, entry) for name, entry in archives().items() if entry[2]):
            checks.append(_archive(name, variable, relative, expected))
    return 0 if all(checks) else 1
