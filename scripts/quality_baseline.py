"""Audit legacy quality debt and standards evidence without changing product claims."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BSW_SOURCES = {
    "Can.c",
    "CanIf.c",
    "CanTp.c",
    "Com.c",
    "Dcm.c",
    "Dem.c",
    "LSduR.c",
    "NvM.c",
    "Os.c",
    "PduR.c",
}
RTE_SOURCES = {"Rte.c"}
HOST_SOURCES = {
    "ecu_host_main.c",
    "Ecu_Runtime.c",
    "Ecu_Status.c",
    "Security.c",
    "Can_HostLock.c",
}


def c_scope_errors(root: Path) -> list[str]:
    actual = {path.name for path in (root / "runtime" / "src").glob("*.c")}
    assigned = BSW_SOURCES | RTE_SOURCES | HOST_SOURCES
    errors = [
        f"Unclassified runtime C source: {name}" for name in sorted(actual - assigned)
    ]
    errors.extend(
        f"Missing classified runtime C source: {name}"
        for name in sorted(assigned - actual)
    )
    return errors


def misra_addon(executable: str) -> Path | None:
    binary = Path(executable).resolve()
    candidates = (
        binary.parent.parent / "share" / "cppcheck" / "addons" / "misra.py",
        Path("/usr/share/cppcheck/addons/misra.py"),
        Path("/usr/local/share/cppcheck/addons/misra.py"),
    )
    return next((path for path in candidates if path.is_file()), None)


def run_check(label: str, command: list[str], *, input_text: str | None = None) -> bool:
    print(f"\n[{label}] {' '.join(command)}", flush=True)
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            input=input_text,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
    except OSError as error:
        print(f"FAILED: {error}")
        return False
    combined = result.stdout + result.stderr
    output = combined.splitlines()
    for line in output[:35]:
        print(line)
    if len(output) > 35:
        print(f"... {len(output) - 35} more output lines")
    if label == "C API Doxygen":
        print(f"Reported Doxygen warnings: {sum('error:' in line for line in output)}")
    if "MISRA scan" in label:
        print(
            f"Reported MISRA findings: {len(re.findall(r'\[misra-c2012-', combined))}"
        )
    print(
        f"{'PASS' if result.returncode == 0 else 'FAIL'}: {label} (exit {result.returncode})"
    )
    return result.returncode == 0


def spec_evidence_gaps(state: dict) -> list[str]:
    gaps = []
    for capability in state["capabilities"]:
        gate = capability["gates"]["spec_obligations"]
        if gate["status"] not in {"passed", "not_applicable"}:
            gaps.append(f"{capability['id']}: spec_obligations={gate['status']}")
    return gaps


def doxygen_header_errors(root: Path) -> list[str]:
    errors = []
    headers = sorted((root / "runtime/include").glob("*.h"))
    if not headers:
        return ["runtime/include: no public C headers found"]
    for header in headers:
        try:
            marked = header.read_text(encoding="utf-8").lstrip().startswith("/** @file")
        except (OSError, UnicodeError) as error:
            errors.append(
                f"{header.relative_to(root).as_posix()}: cannot read header: {error}"
            )
            continue
        if not marked:
            errors.append(
                f"{header.relative_to(root).as_posix()}: missing Doxygen @file header"
            )
    return errors


def doxygen_check() -> bool:
    header_errors = doxygen_header_errors(ROOT)
    if header_errors:
        print("\n[C API Doxygen]")
        for error in header_errors:
            print(error)
        return False
    try:
        config = (ROOT / "runtime/Doxyfile").read_text(encoding="utf-8")
    except OSError as error:
        print(f"[C API Doxygen] Cannot read runtime/Doxyfile: {error}")
        return False
    with tempfile.TemporaryDirectory() as temporary:
        output = Path(temporary).as_posix()
        return run_check(
            "C API Doxygen",
            ["doxygen", "-"],
            input_text=f'{config}\nOUTPUT_DIRECTORY = "{output}"\n',
        )


def generated_input_errors(directory: Path) -> list[str]:
    required = {"Ecu_Config.c", "Dcm_Externals.h", "include/Ecu_Config.h"}
    try:
        listing = (directory / "files.list").read_bytes()
        names = listing.decode("utf-8").splitlines()
        if not names or listing != ("\n".join(names) + "\n").encode("utf-8"):
            return ["Generated files.list is empty or malformed"]
        if names != sorted(set(names)) or any(
            "\\" in name
            or Path(name).is_absolute()
            or Path(name).drive
            or ".." in Path(name).parts
            for name in names
        ):
            return ["Generated files.list has unsafe or duplicate paths"]
        missing = [
            f"Generated output is missing {name}"
            for name in sorted(required)
            if name not in names or not (directory / name).is_file()
        ]
        if missing:
            return missing
        if (directory / "files.list").is_symlink() or (
            directory / "files.sha256"
        ).is_symlink():
            return ["Generated integrity files must be regular files"]
        record = []
        for name in names:
            source = directory / name
            if not source.is_file() or source.is_symlink():
                return [f"Generated output has missing or linked file: {name}"]
            record.append(f"{hashlib.sha256(source.read_bytes()).hexdigest()}  {name}")
        record.append(f"{hashlib.sha256(listing).hexdigest()}  files.list")
        if (directory / "files.sha256").read_bytes() != (
            "\n".join(record) + "\n"
        ).encode("utf-8"):
            return ["Generated files.sha256 does not match output bytes"]
    except (OSError, UnicodeError) as error:
        return [f"Cannot verify generated output: {error}"]
    return []


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--generated-dir",
        type=Path,
        help="Representative generated ECU project to scan",
    )
    arguments = parser.parse_args()
    npm = "npm.cmd" if sys.platform == "win32" else "npm"
    ruff = (
        ROOT
        / ".quality-venv"
        / ("Scripts/ruff.exe" if sys.platform == "win32" else "bin/ruff")
    )
    checks = [
        (
            "Supported source formatting and syntax",
            [sys.executable, "-B", "scripts/quality.py", "--all-format"],
        ),
        ("Python lint", [str(ruff), "check", "scripts"]),
        ("Python formatting", [str(ruff), "format", "--check", "scripts"]),
        ("UI lint", [npm, "run", "lint", "--prefix", "ui"]),
        (
            "Core full Clippy",
            [
                "cargo",
                "clippy",
                "--manifest-path",
                "core/Cargo.toml",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        (
            "Desktop full Clippy",
            [
                "cargo",
                "clippy",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
    ]
    failed = [label for label, command in checks if not run_check(label, command)]
    if not doxygen_check():
        failed.append("C API Doxygen")

    generated_scanned = False
    scope_errors = c_scope_errors(ROOT)
    if scope_errors:
        print("\n[C source classification]")
        for error in scope_errors:
            print(error)
        failed.append("C source classification")
    else:
        cppcheck = shutil.which("cppcheck")
        addon = misra_addon(cppcheck) if cppcheck else None
        if not cppcheck or addon is None:
            print(
                "\n[Partial MISRA scan] Cppcheck with misra.py is required; see README setup."
            )
            failed.append("Partial MISRA scan")
        else:
            version = subprocess.run(
                [cppcheck, "--version"],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            if version.returncode or version.stdout.strip() != "Cppcheck 2.21.0":
                print(
                    f"\n[Cppcheck version] Expected 2.21.0, found {version.stdout.strip() or version.stderr.strip()}"
                )
                failed.append("Cppcheck version")
            common = [
                cppcheck,
                "--std=c99",
                "--language=c",
                "--enable=warning,style,performance,portability",
                "--inconclusive",
                "--error-exitcode=1",
                "--quiet",
                "--template=gcc",
                f"--addon={addon}",
            ]
            for label, names in (
                ("BSW partial MISRA scan", BSW_SOURCES),
                ("RTE partial MISRA scan", RTE_SOURCES),
            ):
                command = [
                    *common,
                    "-Iruntime/include",
                    *(f"runtime/src/{name}" for name in sorted(names)),
                ]
                if not run_check(label, command):
                    failed.append(label)
            if arguments.generated_dir is not None and not generated_input_errors(
                arguments.generated_dir
            ):
                generated = arguments.generated_dir.resolve()
                command = [
                    *common,
                    f"-I{generated / 'include'}",
                    f"-I{generated}",
                    str(generated / "Ecu_Config.c"),
                ]
                generated_scanned = True
                if not run_check("Generated C partial MISRA scan", command):
                    failed.append("Generated C partial MISRA scan")

    if arguments.generated_dir is None:
        print(
            "\n[Generated C partial MISRA scan] NOT RUN: pass --generated-dir with a generated ECU project."
        )
        failed.append("Generated C partial MISRA scan")
    else:
        generated_errors = generated_input_errors(arguments.generated_dir)
        if generated_errors:
            print("\n[Generated C partial MISRA scan]")
            for error in generated_errors:
                print(error)
            failed.append("Generated C partial MISRA scan")
        elif not generated_scanned:
            print(
                "\n[Generated C partial MISRA scan] NOT RUN: C source scope or Cppcheck is unavailable."
            )
            failed.append("Generated C partial MISRA scan")

    try:
        state = json.loads(
            (ROOT / "docs/workflow/state.json").read_text(encoding="utf-8")
        )
        gaps = spec_evidence_gaps(state)
    except (OSError, ValueError, KeyError, TypeError) as error:
        gaps = [f"Cannot read spec evidence state: {error}"]
    print("\n[Per-capability AUTOSAR evidence]")
    for gap in gaps:
        print(gap)
    if gaps:
        failed.append("Per-capability AUTOSAR evidence")
    print(
        "\nOpen-source Cppcheck MISRA coverage is partial; a clean scan cannot establish MISRA or AUTOSAR compliance."
    )
    print(
        "Target-specific integration and complete generated C coverage remain subject to per-configuration evidence."
    )
    print(
        f"\nBaseline result: {len(failed)} failed section(s): {', '.join(failed) if failed else 'none'}"
    )
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
