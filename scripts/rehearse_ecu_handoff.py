"""Build and run a received ECU package with only Python and its pinned native tools."""

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

# Match the independent component-pipeline receiver vector in test_offline_delivery.py.
PIPELINE_TRACE = (
    b"BEGIN 0\nRX 800 4 78563412\nRX 1792 8 0322123400000000\nCOMMIT\n"
    b"BEGIN 1\nCOMMIT\nBEGIN 10\nRX 1792 8 0322123400000000\nCOMMIT\n"
    b"BEGIN 10\nCOMMIT\nBEGIN 20\nRX 1792 8 0322123400000000\nCOMMIT\n"
)
PIPELINE_FRAMES = [
    ("1", "1800", "8", "0762123400000000"),
    ("10", "801", "4", "78563412"),
    ("10", "1800", "8", "0762123412345678"),
    ("20", "801", "4", "78563412"),
    ("20", "1800", "8", "0762123412345678"),
]


def run(command: list[str], directory: Path, log: Path, *, stdin: bytes = b"") -> str:
    print(f"{log.name}: {subprocess.list2cmdline(command)}", flush=True)
    with log.open("wb") as output:
        subprocess.run(
            command, cwd=directory, input=stdin, stdout=output, stderr=subprocess.STDOUT,
            check=True, timeout=300,
        )
    return log.read_text(encoding="utf-8", errors="replace")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path, help="received sealed project directory")
    parser.add_argument("results", type=Path, help="unused directory outside the package")
    behavior = parser.add_mutually_exclusive_group()
    behavior.add_argument("--verify", action="store_true", help="run the standard ECU behavior oracle")
    behavior.add_argument("--pipeline", action="store_true", help="check the component-pipeline frames")
    args = parser.parse_args()
    package = args.package.resolve()
    results = args.results.resolve()
    entry = package / "tools" / "ecu-tool.py"
    if not entry.is_file():
        parser.error(f"sealed package entry is missing: {entry}")
    if results.is_relative_to(package) or results.exists():
        parser.error("results must be an unused directory outside the package")

    results.mkdir()
    tool = [sys.executable, "-I", "-S", str(entry)]
    for mode in ("host-batch", "probe"):
        run(
            [*tool, "build", "--project", str(package), "--output", str(results / mode),
             "--mode", mode],
            results, results / f"build-{mode}.log",
        )

    suffix = ".exe" if os.name == "nt" else ""
    probe = run([str(results / "probe" / f"ecu_probe{suffix}")], results, results / "probe.log")
    if "ecu_probe completed=20" not in probe.splitlines():
        raise SystemExit(f"probe did not complete 20 ticks; see {results / 'probe.log'}")
    batch_log = results / "host-batch.log"
    batch = run(
        [str(results / "host-batch" / f"ecu_host_batch{suffix}")], results,
        batch_log, stdin=PIPELINE_TRACE if args.pipeline else b"BEGIN 1\nCOMMIT\n",
    )
    epochs = re.findall(r"^COMMIT_OK .*?\bepoch=(\d+)", batch, re.MULTILINE)
    expected_epochs = ["0", "1", "10", "10", "20"] if args.pipeline else ["1"]
    if epochs != expected_epochs:
        raise SystemExit(f"unexpected HostBatch commit receipts {epochs}; see {batch_log}")
    if args.pipeline:
        records = [
            dict(field.split("=", 1) for field in line.split()[1:])
            for line in batch.splitlines()
            if line.startswith("OUT ")
        ]
        frames = [tuple(record[key] for key in ("epoch", "id", "dlc", "data")) for record in records]
        if frames != PIPELINE_FRAMES:
            raise SystemExit(f"unexpected component-pipeline frames {frames}; see {batch_log}")

    if args.verify:
        run(
            [*tool, "verify", "--project", str(package), "--build-directory",
             str(results / "verify")],
            results, results / "verify.log",
        )
    print(f"PASS {package.name}: independent HostBatch and probe")


if __name__ == "__main__":
    main()
