"""Build and run a received ECU package with only Python and its pinned native tools."""

import argparse
import os
import subprocess
import sys
from pathlib import Path


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
    parser.add_argument("--verify", action="store_true", help="run the standard ECU behavior oracle")
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
    batch = run(
        [str(results / "host-batch" / f"ecu_host_batch{suffix}")], results,
        results / "host-batch.log", stdin=b"BEGIN 1\nCOMMIT\n",
    )
    if not any(line.startswith("COMMIT_OK ") and "epoch=1" in line.split()
               for line in batch.splitlines()):
        raise SystemExit(f"missing HostBatch commit receipt; see {results / 'host-batch.log'}")

    if args.verify:
        run(
            [*tool, "verify", "--project", str(package), "--build-directory",
             str(results / "verify")],
            results, results / "verify.log",
        )
    print(f"PASS {package.name}: independent HostBatch and probe{', verify' if args.verify else ''}")


if __name__ == "__main__":
    main()
