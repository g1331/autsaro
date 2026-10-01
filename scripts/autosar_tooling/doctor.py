"""Read-only checks for the developer workbench and its external inputs."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from ecu_tools.process import OwnedProcess, ProcessSpec

ROOT = Path(__file__).resolve().parents[2]
ARCHIVES = {
    "R24-11 XSD": (
        "AUTOSAR_XSD_ARCHIVE",
        "docs/official/R24-11/FO/MethodologyAndTemplates/AUTOSAR_FO_MMOD_XMLSchema.zip",
        "9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766",
    ),
    "R24-11 MOD": (
        "AUTOSAR_MOD_ARCHIVE",
        "docs/official/R24-11/CP/MethodologyAndTemplates/AUTOSAR_CP_MOD_ECUConfigurationParameters.zip",
        "df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb",
    ),
}


def _check(name: str, ok: bool, detail: str, *, mismatch: bool = False) -> bool:
    status = "ready" if ok else "version_mismatch" if mismatch else "missing"
    print(f"{name}: {status} — {detail}")
    return ok


def _command(name: str, executable: str, args: tuple[str, ...], expected: str) -> bool:
    path = shutil.which(executable)
    if not path:
        return _check(
            name, False, f"Install {executable} and make it available in a new terminal"
        )
    try:
        result = subprocess.run(
            [path, *args], capture_output=True, text=True, timeout=30, check=False
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return _check(name, False, f"Cannot query {path}: {error}")
    version = (result.stdout or result.stderr).splitlines()
    observed = version[0] if version else "no version output"
    if result.returncode:
        return _check(name, False, f"{path} exited {result.returncode}: {observed}")
    return _check(
        name,
        expected in (result.stdout or result.stderr),
        f"{observed}; required {expected}; executable {path}",
        mismatch=True,
    )


def _archive(name: str, variable: str, relative: str, expected: str) -> bool:
    path = Path(os.environ[variable]) if os.environ.get(variable) else ROOT / relative
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


def _windows_prerequisites() -> list[bool]:
    root = os.environ.get("VCPKG_ROOT")
    triplet = os.environ.get("VCPKGRS_TRIPLET")
    libclang = os.environ.get("LIBCLANG_PATH")
    return [
        _check(
            "vcpkg libxml2",
            bool(
                root
                and triplet == "x64-windows-static-md"
                and (
                    Path(root) / "installed" / triplet / "lib" / "libxml2.lib"
                ).is_file()
            ),
            "Set VCPKG_ROOT and VCPKGRS_TRIPLET=x64-windows-static-md; install libxml2[iconv,zlib] for that triplet",
        ),
        _check(
            "libclang",
            bool(libclang and (Path(libclang) / "libclang.dll").is_file()),
            "Set LIBCLANG_PATH to the directory containing libclang.dll",
        ),
        _command("Rust MSVC host", "rustc", ("-vV",), "x86_64-pc-windows-msvc"),
        _check(
            "MSVC CPython",
            "MSC" in sys.version,
            "Use the MSVC CPython build and install Visual Studio C++ Build Tools and WebView2",
        ),
    ]


def _pkg_config(package: str, instruction: str) -> bool:
    executable = shutil.which("pkg-config")
    if not executable:
        return _check(package, False, f"Install pkg-config; {instruction}")
    try:
        result = subprocess.run(
            [executable, "--exists", package], timeout=30, check=False
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return _check(package, False, f"Cannot query {package}: {error}")
    return _check(package, result.returncode == 0, instruction)


def _unix_prerequisites(system: str) -> list[bool]:
    checks = [
        _command("pkg-config", "pkg-config", ("--version",), "."),
        _pkg_config(
            "libxml-2.0",
            "Install libxml2-dev (Linux) or provide libxml2 through Xcode/Homebrew (macOS)",
        ),
    ]
    if system == "Linux":
        for package in ("webkit2gtk-4.1", "ayatana-appindicator3-0.1", "librsvg-2.0"):
            checks.append(
                _pkg_config(
                    package,
                    f"Install the Ubuntu development package for {package} (Tauri prerequisites)",
                )
            )
        checks.append(
            _check(
                "libclang",
                bool(shutil.which("clang"))
                and (
                    bool(
                        os.environ.get("LIBCLANG_PATH")
                        and (
                            Path(os.environ["LIBCLANG_PATH"]) / "libclang.so"
                        ).is_file()
                    )
                    or any(Path("/usr/lib").glob("llvm-*/lib/libclang.so"))
                ),
                "Install libclang-dev and clang or set LIBCLANG_PATH to libclang.so",
            )
        )
    else:
        checks.append(_command("Xcode CLT", "xcode-select", ("-p",), "/"))
    return checks


def workbench() -> int:
    system = platform.system()
    machine = platform.machine().lower()
    supported = system in ("Windows", "Linux", "Darwin") and machine in (
        "amd64",
        "x86_64",
        "arm64",
        "aarch64",
    )
    checks = [
        _check(
            "platform",
            supported,
            f"{system}/{machine}; Windows, Linux or macOS workbench required",
        )
    ]
    checks.extend(
        [
            _command("rustc", "rustc", ("--version",), "1.98.1"),
            _command("cargo", "cargo", ("--version",), "1.98.1"),
            _command("rustfmt", "rustfmt", ("--version",), "1.9.0"),
            _command("clippy", "cargo", ("clippy", "--version"), "clippy 0.1.98"),
            _command("node", "node", ("--version",), "v24.19.0"),
            _command("npm", "npm", ("--version",), "11.17.0"),
            _command("uv", "uv", ("--version",), "0.11."),
            _check(
                "CPython",
                sys.version_info[:3] == (3, 12, 9),
                f"{sys.version.split()[0]}; required 3.12.9",
                mismatch=True,
            ),
            _check(
                "uv.lock",
                (ROOT / "uv.lock").is_file(),
                "Run uv lock from the repository root",
            ),
        ]
    )
    for name, (variable, relative, expected) in ARCHIVES.items():
        checks.append(_archive(name, variable, relative, expected))
    if system == "Windows":
        checks.extend(_windows_prerequisites())
    elif system in ("Linux", "Darwin"):
        checks.extend(_unix_prerequisites(system))
    else:
        print("Tauri prerequisites: not_applicable")
    return 0 if all(checks) else 1


def _native_query(executable: Path, *args: str) -> tuple[str, str] | None:
    """Only explicit native checks launch a scoped, deadline-bound process."""
    try:
        with tempfile.TemporaryDirectory(prefix="autosar-doctor-") as temporary:
            spec = ProcessSpec.seconds(
                [str(executable), *args], ROOT, 30, Path(temporary),
                f"native-identity-{executable.name}",
            )
            result = OwnedProcess(spec).wait()
            stdout = result.stdout.read_text(encoding="utf-8", errors="replace")
            stderr = result.stderr.read_text(encoding="utf-8", errors="replace")
            if not result.success:
                _check(
                    executable.name, False,
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
        _check(name, False, f"Set {variable or 'PATH'} to an installed {name} executable")
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
        name, actual == expected, f"{path}; SHA-256 {actual}; required {expected}",
        mismatch=True,
    )


def native(target: str) -> int:
    locks = {
        "windows-x64-controlled-v1": ("Windows", "toolchain.json"),
        "linux-x64-controlled-v1": ("Linux", "toolchain-linux.json"),
    }
    system, lock_name = locks[target]
    pinned = json.loads((ROOT / "runtime/os" / lock_name).read_text(encoding="utf-8"))
    native_host = platform.system() == system and platform.machine().lower() in (
        "amd64", "x86_64",
    )
    checks = [
        _check(
            "native target", native_host,
            f"{platform.system()}/{platform.machine()}: "
            + (f"native execution of {target} is applicable"
               if native_host else f"cannot execute {target}; pure source rendering remains available"),
        )
    ]
    git = _native_path("git", "AUTOSAR_GIT")
    if git:
        version = _native_query(git, "--version")
        checks.append(_check("Git", bool(version and version[0].startswith("git version ")), f"{git}: {version[0].strip() if version else 'identity unavailable'}"))
    else:
        checks.append(False)
    python = _native_path("python", "AUTOSAR_PYTHON")
    if python:
        version = _native_query(python, "--version")
        output = " ".join(version) if version else ""
        checks.append(_check("CPython", output.strip() == "Python 3.12.9", f"{python}: {output.strip() or 'identity unavailable'}", mismatch=True))
    else:
        checks.append(False)
    compiler = _native_path("gcc", "AUTOSAR_CC")
    if compiler:
        version = _native_query(compiler, "--version")
        machine = _native_query(compiler, "-dumpmachine")
        observed = version[0].splitlines()[0] if version and version[0].splitlines() else ""
        expected = pinned["identity"]
        checks.append(_check(
            "compiler version",
            observed.partition(" ")[2] == expected.partition(" ")[2]
            and bool(machine and machine[0].strip() == pinned["target"]),
            f"{compiler}; {observed}; triple {machine[0].strip() if machine else 'unknown'}; required {expected} {pinned['target']}",
            mismatch=True,
        ))
        checks.append(_native_digest("compiler bytes", compiler, pinned["executable_sha256"]))
    else:
        checks.append(False)
    for name in ("objdump", "nm"):
        if name == "nm" and system == "Windows":
            continue
        variable = f"AUTOSAR_{name.upper()}"
        executable = _native_path(name, variable)
        if executable:
            version = _native_query(executable, "--version")
            checks.append(_check(name, bool(version and "GNU " in version[0]), f"{executable}; {version[0].splitlines()[0] if version and version[0].splitlines() else 'identity unavailable'}"))
            expected_hash = pinned.get(f"{name}_sha256")
            if expected_hash:
                checks.append(_native_digest(f"{name} bytes", executable, expected_hash))
        else:
            checks.append(False)
    for name, (variable, relative, expected) in ARCHIVES.items():
        checks.append(_archive(name, variable, relative, expected))
    return 0 if all(checks) else 1
