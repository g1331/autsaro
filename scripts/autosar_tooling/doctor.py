"""Read-only checks for the developer workbench and its external inputs."""

from __future__ import annotations

import hashlib
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

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
