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
        str(TARGET / "tests" / harness),
        str(copied / "tasks.c"),
        str(copied / "list.c"),
        str(copied / "queue.c"),
        str(copied / "portable/MSVC-MingW/port.c"),
        "-lwinmm",
        "-o",
        str(binary),
    ]
    compiled = subprocess.run(command, capture_output=True, text=True, check=False)
    if compiled.returncode:
        raise RuntimeError(f"OS C99 build failed:\n{compiled.stdout}{compiled.stderr}")
    evidence = {
        "kernel_commit": manifest["commit"],
        "compiler": "GCC 16.1.0 x64",
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


def execute(binary: Path, scenario: str, failure: int | str = 0) -> dict:
    env = dict(os.environ)
    env.pop("AUTOSAR_OS_FAIL_RESOURCE", None)
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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--suite", choices=["lifecycle"], default="lifecycle")
    parser.add_argument("--evidence", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="autosar-epic4-os-") as temporary:
        binary, evidence = build(Path(temporary))
        evidence["observations"] = check_lifecycle(binary)
        evidence["status"] = "pass"
        if args.evidence:
            args.evidence.parent.mkdir(parents=True, exist_ok=True)
            args.evidence.write_text(
                json.dumps(evidence, indent=2) + "\n", encoding="utf-8"
            )
        print(
            f"epic4_backend_lifecycle PASS: {len(evidence['observations'])} native vectors"
        )


if __name__ == "__main__":
    main()
