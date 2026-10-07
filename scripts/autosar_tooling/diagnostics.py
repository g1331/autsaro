"""Task-specific, read-only developer diagnostics with actionable results."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tomllib
import zipfile
from dataclasses import asdict, dataclass
from pathlib import Path

from autosar_tooling.config import ROOT, archive_path, archives

PROFILES = (
    "ui",
    "tooling",
    "core",
    "desktop",
    "process",
    "integration",
    "native",
    "all",
)


@dataclass
class Check:
    name: str
    status: str
    detail: str
    fix: str = ""


def version_check(
    name: str,
    args: list[str],
    recommended: str | None = None,
    supported_major: int | None = None,
    exact: bool = False,
    minimum: str | None = None,
) -> Check:
    path = shutil.which(args[0])
    if not path:
        return Check(
            name,
            "blocked",
            "Executable not found",
            f"Install {args[0]} and open a new terminal",
        )
    try:
        command = [path, *args[1:]]
        if args[0] == "npm" and os.name == "nt":
            from autosar_tooling.verify import npm_command

            command = npm_command(*args[1:])
        result = subprocess.run(
            command, capture_output=True, text=True, timeout=30, check=False
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return Check(name, "error", str(error), f"Check {path}")
    output = (result.stdout or result.stderr).strip()
    match = re.search(r"(?<!\d)(\d+)\.(\d+)\.(\d+)\b", output)
    if result.returncode or not match:
        return Check(
            name, "error", output or f"exit={result.returncode}", f"Check {path}"
        )
    actual = match[0]
    if (
        (supported_major is not None and int(match[1]) != supported_major)
        or (exact and actual != recommended)
        or (
            minimum
            and tuple(map(int, actual.split("."))) < tuple(map(int, minimum.split(".")))
        )
    ):
        return Check(
            name,
            "blocked",
            f"{actual} at {path}; required {recommended or supported_major}",
            f"Install {name} {recommended or supported_major}",
        )
    status = "warning" if recommended and actual != recommended else "ready"
    return Check(
        name,
        status,
        f"{actual} at {path}" + (f"; recommended {recommended}" if recommended else ""),
    )


def inspect(
    profile: str, strict: bool = False, target: str | None = None
) -> list[Check]:
    import io
    from contextlib import redirect_stdout

    from autosar_tooling import doctor

    checks: list[Check] = []
    ui = profile in ("ui", "desktop", "all")
    core = profile in ("core", "desktop", "integration", "all")
    tooling = profile != "ui"
    desktop = profile in ("desktop", "all")
    if ui:
        package = json.loads((ROOT / "ui/package.json").read_text(encoding="utf-8"))
        node = (ROOT / ".node-version").read_text().strip()
        npm = package["packageManager"].split("@", 1)[1]
        checks.extend(
            [
                version_check(
                    "Node",
                    ["node", "--version"],
                    node,
                    int(node.split(".")[0]),
                    strict,
                    minimum=package["engines"]["node"].split()[0].removeprefix(">="),
                ),
                version_check(
                    "npm", ["npm", "--version"], npm, int(npm.split(".")[0]), strict
                ),
                Check(
                    "UI dependencies",
                    "ready"
                    if (ROOT / "ui/node_modules/typescript/bin/tsc").is_file()
                    else "blocked",
                    "ui/node_modules",
                    "uv run dev setup --profile ui",
                ),
            ]
        )
    if tooling:
        recommended = (ROOT / ".python-version").read_text().strip()
        actual = platform.python_version()
        status = (
            "blocked"
            if sys.version_info[:2] != (3, 12) or (strict and actual != recommended)
            else "warning"
            if actual != recommended
            else "ready"
        )
        checks.append(
            Check(
                "CPython",
                status,
                f"{actual}; recommended {recommended}",
                "uv python install",
            )
        )
        checks.append(
            version_check(
                "Ruff", ["ruff", "--version"], _dependency_version("ruff"), exact=True
            )
        )
    checks.append(version_check("Git", ["git", "--version"]))
    if core:
        rust = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"][
            "channel"
        ]
        checks.extend(
            [
                version_check("Rust", ["rustc", "--version"], rust, exact=True),
                version_check("Cargo", ["cargo", "--version"], rust, exact=True),
                version_check("rustfmt", ["rustfmt", "--version"]),
                version_check("Clippy", ["cargo", "clippy", "--version"]),
            ]
        )
        # Native libxml/bindgen are needed by the core, not by UI development.
        _platform_dependencies(checks, desktop)
    if profile in ("integration", "all"):
        for name, (variable, relative, expected) in archives().items():
            path = archive_path(variable, relative)
            fix = f"Place the legally obtained archive at {path} or set {variable}"
            try:
                with path.open("rb") as source:
                    digest = hashlib.file_digest(source, "sha256").hexdigest()
                valid = digest == expected if expected else zipfile.is_zipfile(path)
                if valid and expected is None:
                    with zipfile.ZipFile(path) as archive:
                        valid = any(
                            name.startswith(
                                "AUTOSAR_CP_EXP_ModelingShowCases/30_MeasurementCalibration/10_Introductory/model/"
                            )
                            and name.endswith(".arxml")
                            for name in archive.namelist()
                        )
                checks.append(
                    Check(
                        name,
                        "ready" if valid else "blocked",
                        str(path),
                        "" if valid else fix,
                    )
                )
            except (OSError, zipfile.BadZipFile) as error:
                checks.append(Check(name, "blocked", str(error), fix))
    if profile in ("process", "native", "all") and sys.platform == "linux":
        proc = Path(f"/proc/{os.getpid()}/task/{os.getpid()}/children")
        checks.append(
            Check(
                "Linux process ownership observability",
                "ready" if proc.is_file() else "blocked",
                str(proc),
                "Use a Linux host exposing /proc task children; restricted synthetic /proc cannot verify escaped descendants",
            )
        )
    if profile in ("native", "all"):
        selected = target or (
            "windows-x64-controlled-v1"
            if os.name == "nt"
            else "linux-x64-controlled-v1"
        )
        output = io.StringIO()
        with redirect_stdout(output):
            doctor.native(selected, include_archives=False, require_explicit=True)
        for line in output.getvalue().splitlines():
            name, _, result = line.partition(": ")
            status, _, detail = result.partition(" — ")
            checks.append(
                Check(
                    name,
                    "ready" if status == "ready" else "blocked",
                    detail,
                    "See docs/development/environment.md" if status != "ready" else "",
                )
            )
    return checks


def _dependency_version(name: str) -> str:
    project = tomllib.loads((ROOT / "pyproject.toml").read_text())
    return next(
        item.split("==")[1]
        for item in project["dependency-groups"]["quality"]
        if item.startswith(name + "==")
    )


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


def report(
    profile: str,
    *,
    strict: bool = False,
    target: str | None = None,
    json_output: bool = False,
) -> int:
    checks = inspect(profile, strict, target)
    blocked = any(check.status in ("blocked", "error") for check in checks)
    if json_output:
        print(
            json.dumps(
                {
                    "profile": profile,
                    "ready": not blocked,
                    "checks": [asdict(check) for check in checks],
                },
                ensure_ascii=False,
                indent=2,
            )
        )
    else:
        for check in checks:
            print(f"[{check.status}] {check.name}: {check.detail}")
            if check.fix and check.status in ("blocked", "error"):
                print(f"  Fix: {check.fix}")
        print(
            f"{profile}: {'blocked; resolve the items above' if blocked else 'ready'}; other profiles were not checked."
        )
    return 1 if blocked else 0
