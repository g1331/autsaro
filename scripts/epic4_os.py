"""Build and independently verify the fixed offline Epic 4 OS target."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KERNEL = ROOT / "third_party/freertos"
TARGET = ROOT / "runtime/os"


def verify_sources(kernel: Path = KERNEL) -> dict:
    manifest = json.loads((kernel / "source-manifest.json").read_text())
    if manifest["commit"] != "054e14f3397023aa83813a65aa065fc4597d481b":
        raise ValueError("fixed kernel identity mismatch")
    for name, expected in manifest["files"].items():
        if hashlib.sha256((kernel / name).read_bytes()).hexdigest() != expected:
            raise ValueError(f"kernel digest mismatch: {name}")
    if manifest["license"] != "MIT":
        raise ValueError("kernel license mismatch")
    return manifest


def compiler() -> str:
    cc = os.environ.get("AUTOSAR_CC", "gcc")
    version = subprocess.run(
        [cc, "--version"], capture_output=True, text=True, check=True
    )
    machine = subprocess.run(
        [cc, "-dumpmachine"], capture_output=True, text=True, check=True
    )
    if "16.1.0" not in version.stdout or machine.stdout.strip() != "x86_64-w64-mingw32":
        raise ValueError("target requires GCC 16.1.0 x86_64-w64-mingw32")
    return cc


def build(directory: Path, harness: str = "lifecycle.c") -> tuple[Path, dict]:
    manifest = verify_sources()
    cc = compiler()
    copied = directory / "kernel"
    shutil.copytree(KERNEL, copied)
    patches = sorted((TARGET / "patches").glob("*.patch"))
    for patch in patches:
        subprocess.run(
            ["git", "apply", "--ignore-space-change", "--check", str(patch)],
            cwd=copied,
            check=True,
        )
        subprocess.run(
            ["git", "apply", "--ignore-space-change", str(patch)],
            cwd=copied,
            check=True,
        )
    binary = directory / "os_harness.exe"
    command = [
        cc,
        "-std=c99",
        "-O1",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-I",
        str(TARGET),
        "-I",
        str(TARGET / "include"),
        "-I",
        str(TARGET / "src"),
        "-I",
        str(copied / "include"),
        "-I",
        str(copied / "portable/MSVC-MingW"),
        str(TARGET / "src/Os.c"),
        str(TARGET / "src/Os_Backend.c"),
        str(TARGET / "src/Os_Stack.c"),
        str(TARGET / "tests" / harness),
        str(copied / "tasks.c"),
        str(copied / "list.c"),
        str(copied / "queue.c"),
        str(copied / "portable/MSVC-MingW/port.c"),
        "-lwinmm",
        "-o",
        str(binary),
    ]
    if harness == "native_stack.c":
        command.insert(1, "-DOS_STACK_TESTS")
    compiled = subprocess.run(command, capture_output=True, text=True, check=False)
    if compiled.returncode:
        raise RuntimeError(f"OS C99 build failed:\n{compiled.stdout}{compiled.stderr}")
    tool = Path(shutil.which(cc) or cc).parent / "objdump.exe"
    symbols = subprocess.run(
        [str(tool), "-t", str(binary)], capture_output=True, text=True, check=True
    ).stdout
    if "__emutls" in symbols:
        raise ValueError(
            "native exception path requires PE TLS without emutls allocation"
        )
    evidence = {
        "kernel_commit": manifest["commit"],
        "compiler": "GCC 16.1.0 x64",
        "tls_abi": "PE native TLS; no emutls helper",
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "patches": {
            p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in patches
        },
        "product_sources": {
            p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(TARGET.rglob("*"))
            if p.is_file()
        },
        "command": command,
    }
    return binary, evidence


def execute(
    binary: Path, scenario: str, failure: int | str = 0, invalid_stack: str = ""
) -> dict:
    env = dict(os.environ)
    env.pop("AUTOSAR_OS_FAIL_RESOURCE", None)
    env.pop("AUTOSAR_OS_BAD_GUARANTEE", None)
    if invalid_stack:
        env["AUTOSAR_OS_BAD_GUARANTEE"] = invalid_stack
    if failure:
        env["AUTOSAR_OS_FAIL_RESOURCE"] = str(failure)
    result = subprocess.run(
        [str(binary), scenario],
        env=env,
        timeout=10,
        capture_output=True,
        text=True,
        check=False,
    )
    return {
        "scenario": scenario,
        "failure": failure,
        "invalid_stack": invalid_stack,
        "exit": result.returncode,
        "stdout": result.stdout.strip(),
        "stderr": result.stderr.strip(),
    }


def require(condition: bool, observation: dict) -> None:
    if not condition:
        raise AssertionError(observation)


def check_lifecycle(binary: Path) -> list[dict]:
    observations = []
    for scenario, trace in [
        ("normal", "ISRAD"),
        ("mode2", "ISRBD"),
        ("priority", "ISRBD"),
    ]:
        result = execute(binary, scenario)
        require(result["exit"] == 0 and f"trace={trace} " in result["stdout"], result)
        require("input_closed=1 tick_closed=1" in result["stdout"], result)
        require("FORBIDDEN" not in result["stdout"], result)
        observations.append(result)
    for scenario, code in [
        ("zero-count", 8),
        ("capacity", 8),
        ("duplicate", 3),
        ("bad-id", 3),
        ("zero-priority", 8),
        ("bad-priority", 8),
        ("null-entry", 8),
        ("bad-stack", 8),
        ("small-stack", 8),
        ("large-stack", 8),
        ("null-tasks", 8),
        ("bad-autostart", 8),
    ]:
        result = execute(binary, scenario)
        require(result["exit"] == code, result)
        require("rejected_before_threads=1" in result["stdout"], result)
        observations.append(result)
    for scenario, trace, reason in [
        ("bad-mode", "ID", 8),
        ("startup-failure", "ISD", 7),
        ("repeat-start", "ISID", 7),
    ]:
        result = execute(binary, scenario)
        require(
            result["exit"] == reason and f"trace={trace} " in result["stdout"], result
        )
        require("state=Failed" in result["stdout"], result)
        observations.append(result)
    for failure in ["invalid", "-1", "429496729600", "0"]:
        result = execute(binary, "normal", failure)
        require(result["exit"] == 8 and "trace=ID " in result["stdout"], result)
        observations.append(result)
    normal = observations[0]["stdout"]
    calls = int(normal.split("resource_calls=")[1].split()[0])
    # Every actual native resource acquisition, including bootstrap and idle.
    for failure in range(1, calls + 1):
        result = execute(binary, "normal", failure)
        require(result["exit"] == 7 and "state=Failed" in result["stdout"], result)
        require(
            "trace=ID " in result["stdout"] and "FORBIDDEN" not in result["stdout"],
            result,
        )
        observations.append(result)
    return observations


def check_stack(binary: Path) -> list[dict]:
    observations = []
    for scenario, role, code in [
        ("normal", None, None),
        ("task", "T", "C00000FD"),
        ("startup", "B", "C00000FD"),
        ("isr", "S", "C00000FD"),
        ("control", "C", "C00000FD"),
        ("backup", "D", "C00000FD"),
        ("idle", "I", "C00000FD"),
        ("boundary", "T", "C0000005"),
        ("boundary-read", "T", "C0000005"),
        ("sp-corrupt", "T", "00000000"),
    ]:
        result = execute(binary, scenario)
        output = result["stdout"]
        require(result["exit"] == (0 if role is None else 13), result)
        require("captured=1" in output if role else "FAULT" not in output, result)
        require("X" not in output.split("trace=")[1].split()[0], result)
        require("D" in output.split("trace=")[1].split()[0], result)
        if role:
            require(
                f"FAULT role={role} " in output and f"exception={code}" in output,
                result,
            )
        records = [
            dict(field.split("=", 1) for field in line.split()[1:])
            for line in output.splitlines()
            if line.startswith("STACK ")
        ]
        require({r["role"] for r in records} == {"T", "B", "I", "S", "C", "D"}, result)
        for record in records:
            low, high, sp = (int(record[key]) for key in ["low", "high", "sp"])
            require(low < sp < high and high - low == int(record["reserve"]), result)
            require(
                int(record["commit"]) > 0
                and int(record["guard"]) >= 4096
                and int(record["guarantee"]) >= 16384
                and int(record["observations"]) > 0,
                result,
            )
            if record["role"] != "S":
                require(int(record["reserve"]) == 262144, result)
            if record["role"] in {"T", "B", "I"}:
                buffer_low = int(record["buffer_low"])
                buffer_high = int(record["buffer_high"])
                require(
                    buffer_low < buffer_high and not buffer_low <= sp < buffer_high,
                    result,
                )
        observations.append(result)
    concurrent = execute(binary, "concurrent")
    require(concurrent["exit"] == 13, concurrent)
    fault_lines = [
        dict(field.split("=", 1) for field in line.split()[1:])
        for line in concurrent["stdout"].splitlines()
        if line.startswith("FAULT ")
    ]
    stack_lines = [
        dict(field.split("=", 1) for field in line.split()[1:])
        for line in concurrent["stdout"].splitlines()
        if line.startswith("STACK ")
    ]
    require(bool(fault_lines), concurrent)
    require({fault["role"] for fault in fault_lines} == {"T", "D"}, concurrent)
    for fault in fault_lines:
        require(
            fault["role"] in {"T", "D"} and fault["exception"] == "C00000FD", concurrent
        )
        require(
            any(
                row["tid"] == fault["tid"] and row["role"] == fault["role"]
                for row in stack_lines
            ),
            concurrent,
        )
    require("X" not in concurrent["stdout"].split("trace=")[1].split()[0], concurrent)
    observations.append(concurrent)
    double = execute(binary, "double-control")
    require(
        double["exit"] == 13 and "controllers_exhausted=1" in double["stdout"], double
    )
    require(
        "FAULT role=C " in double["stdout"]
        and "exception=C00000FD" in double["stdout"],
        double,
    )
    observations.append(double)
    output_failure = execute(binary, "double-control-output")
    require(
        output_failure["exit"] == 13
        and "controllers_exhausted=1" in output_failure["stderr"],
        output_failure,
    )
    require(
        "FAULT role=C " in output_failure["stderr"]
        and "exception=C00000FD" in output_failure["stderr"],
        output_failure,
    )
    observations.append(output_failure)
    unrelated = execute(binary, "unrelated")
    require(
        unrelated["exit"] & 0xFFFFFFFF == 0xC0000005
        and "FAULT" not in unrelated["stdout"],
        unrelated,
    )
    observations.append(unrelated)
    for scenario in ["api-suspend", "api-context", "api-resume"]:
        result = execute(binary, scenario)
        require(result["exit"] == 7 and "FAULT" not in result["stdout"], result)
        trace = result["stdout"].split("trace=")[1].split()[0]
        require(
            trace == "ISRAFD" and "input_closed=1 tick_closed=1" in result["stdout"],
            result,
        )
        observations.append(result)
    for role in ["S", "T", "B", "I", "C", "D"]:
        result = execute(binary, "normal", invalid_stack=role)
        require(result["exit"] == 7 and "state=Failed" in result["stdout"], result)
        require(
            "FAULT" not in result["stdout"]
            and "R" not in result["stdout"].split("trace=")[1].split()[0],
            result,
        )
        observations.append(result)
    return observations


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--suite", choices=["lifecycle", "stack"], default="lifecycle")
    parser.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="autosar-epic4-os-") as temporary:
        binary, evidence = build(
            Path(temporary),
            "lifecycle.c" if args.suite == "lifecycle" else "native_stack.c",
        )
        evidence["observations"] = (
            check_lifecycle(binary)
            if args.suite == "lifecycle"
            else check_stack(binary)
        )
        evidence["status"] = "pass"
        if args.evidence:
            args.evidence.parent.mkdir(parents=True, exist_ok=True)
            args.evidence.write_text(
                json.dumps(evidence, indent=2) + "\n", encoding="utf-8"
            )
        name = (
            "epic4_backend_lifecycle"
            if args.suite == "lifecycle"
            else "epic4_native_stack_fault_shutdown"
        )
        print(f"{name} PASS: {len(evidence['observations'])} native vectors")


if __name__ == "__main__":
    main()
