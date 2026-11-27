"""Consume the real workbench handoff using only receiver tools and independent wire vectors."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

# HostBatch channel numbers are the configured driver IDs, reversed relative to
# the AUTOSAR network suffix. The two projects deliberately reuse CAN ID 800.
TWO_BUS = """\
PROTOCOL HostBatchV2
BEGIN 10
COMMIT
BEGIN 20
RX 1 STANDARD 0 800 4 2eff0000
RX 1 EXTENDED 0 800 4 78563412
RX 0 STANDARD 0 800 4 7f000000
COMMIT
BEGIN 21
RX 1 STANDARD 1 1792 8 03221234cccccccc
COMMIT
BEGIN 22
RX 1 EXTENDED 1 933 12 aaaaaaaaaaaaaaaaaaaaaaaa
COMMIT
BEGIN 23
RX 0 EXTENDED 1 933 12 0102030405060708090a0b0c
COMMIT
BEGIN 24
RX 1 STANDARD 0 933 4 10203040
COMMIT
BEGIN 25
RX 1 STANDARD 0 800 3 000000
COMMIT
BEGIN 26
RX 1 STANDARD 1 1792 8 03221234cccccccc
COMMIT
BEGIN 30
RX 1 STANDARD 0 800 4 34120000
RX 0 STANDARD 0 800 4 83000000
COMMIT
BEGIN 31
RX 1 STANDARD 1 1792 8 03221234cccccccc
COMMIT
"""
FOUR_BUS = """\
PROTOCOL HostBatchV2
BEGIN 10
COMMIT
BEGIN 20
RX 3 STANDARD 0 800 4 00ff0000
RX 3 EXTENDED 0 800 4 67452301
RX 2 STANDARD 0 800 4 a4000000
RX 1 STANDARD 0 800 4 deadbeef
RX 0 STANDARD 0 800 4 efbeadde
COMMIT
BEGIN 21
RX 3 STANDARD 1 1792 8 03221234cccccccc
COMMIT
BEGIN 22
RX 0 EXTENDED 1 933 12 aaaaaaaaaaaaaaaaaaaaaaaa
COMMIT
BEGIN 23
RX 2 EXTENDED 1 933 12 f0e0d0c0b0a0908070605040
COMMIT
BEGIN 24
RX 3 STANDARD 0 933 4 50607080
COMMIT
BEGIN 25
RX 3 STANDARD 0 800 3 000000
COMMIT
BEGIN 26
RX 1 STANDARD 0 933 4 90a0b0c0
COMMIT
BEGIN 27
RX 3 STANDARD 1 1792 8 03221234cccccccc
COMMIT
BEGIN 30
RX 3 STANDARD 0 800 4 fe7f0000
RX 2 STANDARD 0 800 4 31000000
COMMIT
BEGIN 31
RX 3 STANDARD 1 1792 8 03221234cccccccc
COMMIT
"""
# ISO-TP single-frame PCI, UDS positive response with the four application
# bytes, little-endian input vs big-endian output, and byte-preserving routing.
OUTPUTS = {
    "two-bus": [
        (10, 0, "STANDARD", 0, 801, 4, "00000000"),
        (10, 1, "STANDARD", 0, 801, 4, "00110000"),
        (20, 0, "STANDARD", 0, 801, 4, "7f000000"),
        (20, 1, "STANDARD", 0, 801, 4, "ff2e0000"),
        (21, 1, "STANDARD", 1, 1800, 8, "07621234ff2e7f78"),
        (23, 1, "EXTENDED", 1, 934, 12, "0102030405060708090a0b0c"),
        (24, 0, "EXTENDED", 0, 934, 4, "10203040"),
        (26, 1, "STANDARD", 1, 1800, 8, "07621234ff2e7f78"),
        (30, 0, "STANDARD", 0, 801, 4, "83000000"),
        (30, 1, "STANDARD", 0, 801, 4, "12340000"),
        (31, 1, "STANDARD", 1, 1800, 8, "0762123412348378"),
    ],
    "four-bus": [
        (10, 3, "EXTENDED", 1, 801, 4, "ffdf0000"),
        (20, 3, "EXTENDED", 1, 801, 4, "ff000000"),
        (21, 3, "STANDARD", 1, 1800, 8, "07621234ff00a467"),
        (23, 1, "EXTENDED", 1, 934, 12, "f0e0d0c0b0a0908070605040"),
        (24, 2, "EXTENDED", 0, 934, 4, "50607080"),
        (26, 0, "EXTENDED", 0, 934, 4, "90a0b0c0"),
        (27, 3, "STANDARD", 1, 1800, 8, "07621234ff00a467"),
        (30, 3, "EXTENDED", 1, 801, 4, "7ffe0000"),
        (31, 3, "STANDARD", 1, 1800, 8, "076212347ffe3167"),
    ],
}


def receiver_env() -> dict[str, str]:
    kept = ("SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT", "TEMP", "TMP", "PROCESSOR_ARCHITECTURE",
            "TMPDIR", "LANG")
    env = {name: os.environ[name] for name in kept if name in os.environ}
    tools = {name: str(Path(os.environ[name]).resolve(strict=True))
             for name in ("AUTOSAR_CC", "AUTOSAR_OBJDUMP", "AUTOSAR_GIT")}
    if os.name == "nt":
        system = Path(env["SYSTEMROOT"])
        directories = [str(system / "System32"), str(system)]
    else:
        directories = os.defpath.split(os.pathsep)
    directories.extend(str(Path(tool).parent) for tool in tools.values())
    env["PATH"] = os.pathsep.join(dict.fromkeys(directories))
    return env | tools


def run(argv: list[str | Path], cwd: Path, env: dict[str, str], log: str, input_text: str = "") -> str:
    result = subprocess.run(argv, cwd=cwd, env=env, input=input_text, encoding="utf-8",
                            errors="replace", capture_output=True, timeout=180, check=False)
    (cwd / f"{log}.log").write_text(result.stdout + result.stderr, encoding="utf-8")
    assert result.returncode == 0, (argv, result.stdout, result.stderr)
    return result.stdout


def main(name: str, delivery: Path, receiver: Path) -> None:
    package = receiver / "package"
    shutil.copytree(delivery, package)
    build = receiver / "build"
    env = receiver_env()
    seal = (package / "meta/files.sha256").read_bytes()
    metadata = json.loads((package / "meta/verification-inputs.json").read_bytes())
    endpoints = metadata["configuredEndpoints"].values()
    topology = (len(metadata["channels"]), sum(ep["direction"] == "RX" for ep in endpoints),
                sum(ep["direction"] == "TX" for ep in endpoints))
    assert topology == {"two-bus": (2, 7, 5), "four-bus": (4, 12, 5)}[name], topology
    # The real delivered entry verifies the complete seal before compiling.
    run([sys.executable, "-I", "-S", package / "tools/ecu-tool.py", "build",
         "--project", package, "--output", build, "--mode", "host-batch"], receiver, env, "build")
    binary = build / ("ecu_host_batch.exe" if os.name == "nt" else "ecu_host_batch")
    inputs = TWO_BUS if name == "two-bus" else FOUR_BUS
    trace = run([binary], receiver, env, "host-batch", inputs)
    frames = []
    rejected = []
    committed = []
    for line in trace.splitlines():
        kind, _, payload = line.partition(" ")
        if kind not in ("OUT", "REJECT", "COMMIT_OK"):
            continue
        fields = dict(item.split("=", 1) for item in payload.split())
        if kind == "OUT":
            frames.append((int(fields["epoch"]), int(fields["channel"]), fields["format"],
                           int(fields["fd"]), int(fields["id"]), int(fields["length"]), fields["data"]))
        elif kind == "REJECT":
            rejected.append((int(fields["epoch"]), int(fields["status"])))
        else:
            committed.append(int(fields["epoch"]))
    assert sorted(frames) == sorted(OUTPUTS[name]), trace
    # The invalid PDU is staged for epoch 25. REJECT reports the last completed
    # epoch and E_OS_VALUE (8), while the successful receipts omit epoch 25.
    assert rejected == [(24, 8)], trace
    assert committed == [int(line[6:]) for line in inputs.splitlines()
                         if line.startswith("BEGIN ") and line != "BEGIN 25"], trace
    assert (package / "meta/files.sha256").read_bytes() == seal
    print(f"{name}: sealed build and HostBatch vectors passed")


if __name__ == "__main__":
    main(sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3]))
