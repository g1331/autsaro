"""Low-frequency host CPU and disk queue samples for the duration of a Windows CI job.

`typeperf` only reads performance counters: no collector set, registry value or service is
created, and the sampler ends by itself after `MAX_SAMPLES` if no stop step ever runs.
The counters show what the guest sees; they cannot name another tenant, but a saturated
run queue or a long disk queue with the job's own load low points at the host.
"""

from __future__ import annotations

import csv
import json
import re
import statistics
import subprocess
from pathlib import Path

from autosar_tooling.windows_process_timing import percentile

INTERVAL_S = 5
# Longest job timeout is 30 minutes; the sampler never outlives a job by much.
MAX_SAMPLES = 480
COUNTERS = {
    "cpu_busy_pct": r"\Processor(_Total)\% Processor Time",
    "cpu_privileged_pct": r"\Processor(_Total)\% Privileged Time",
    "cpu_interrupt_pct": r"\Processor(_Total)\% Interrupt Time",
    "cpu_dpc_pct": r"\Processor(_Total)\% DPC Time",
    "run_queue_length": r"\System\Processor Queue Length",
    "context_switches_per_s": r"\System\Context Switches/sec",
    "disk_busy_pct": r"\PhysicalDisk(_Total)\% Disk Time",
    "disk_queue_length": r"\PhysicalDisk(_Total)\Avg. Disk Queue Length",
    "disk_latency_ms": r"\PhysicalDisk(_Total)\Avg. Disk sec/Transfer",
    "disk_transfers_per_s": r"\PhysicalDisk(_Total)\Disk Transfers/sec",
    "memory_available_mb": r"\Memory\Available MBytes",
}
# Windows creation flags; absent elsewhere, where only the parsing runs.
DETACHED = getattr(subprocess, "DETACHED_PROCESS", 0) | getattr(
    subprocess, "CREATE_NEW_PROCESS_GROUP", 0
)


def start(folder: Path) -> None:
    folder.mkdir(parents=True, exist_ok=True)
    argv = [
        "typeperf",
        *COUNTERS.values(),
        "-si",
        str(INTERVAL_S),
        "-sc",
        str(MAX_SAMPLES),
        "-o",
        str(folder / "samples.csv"),
        "-y",
    ]
    with (folder / "typeperf.log").open("wb") as log:
        process = subprocess.Popen(
            argv,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
            creationflags=DETACHED,
        )
    (folder / "sampler.json").write_text(
        json.dumps({"pid": process.pid, "interval_s": INTERVAL_S}), encoding="utf-8"
    )


def terminate(pid: int) -> bool:
    """Stop the recorded sampler; True when it was still running."""
    listing = subprocess.run(
        ["tasklist", "/FI", f"PID eq {pid}", "/FO", "CSV", "/NH"],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    # A PID reused by another program is left alone.
    if "typeperf.exe" not in listing.stdout.lower():
        return False
    subprocess.run(
        ["taskkill", "/PID", str(pid), "/F"],
        capture_output=True,
        timeout=30,
        check=False,
    )
    return True


def read_samples(path: Path) -> dict[str, list[float]]:
    """Counter name to its numeric samples; blank cells (first rate sample) are skipped."""
    with path.open(newline="", encoding="utf-8", errors="replace") as source:
        rows = list(csv.reader(source))
    # A killed sampler can leave a partial last row.
    header, *body = rows
    columns = {
        re.sub(r"^\\\\[^\\]+", "", name): index for index, name in enumerate(header)
    }
    samples = {}
    for key, counter in COUNTERS.items():
        index = columns[counter]
        values = []
        for row in body:
            if len(row) == len(header) and row[index].strip():
                values.append(float(row[index]))
        if key == "disk_latency_ms":
            values = [value * 1000 for value in values]
        samples[key] = values
    return samples


def summarize(folder: Path, still_running: bool) -> dict:
    try:
        samples = read_samples(folder / "samples.csv")
    except FileNotFoundError:
        samples = {}
    if not any(samples.values()):
        log = (folder / "typeperf.log").read_text(errors="replace")
        raise RuntimeError(f"typeperf recorded no samples: {log!r}")
    counters = {}
    for key, values in samples.items():
        ordered = sorted(values)
        counters[key] = {
            "median": statistics.median(values),
            "p95": percentile(ordered, 0.95),
            "max": ordered[-1],
        }
    return {
        "interval_s": INTERVAL_S,
        "samples": len(samples["cpu_busy_pct"]),
        "sampler_running_at_stop": still_running,
        "counters": counters,
    }


def render(summary: dict) -> str:
    lines = [
        "### Windows host load",
        "",
        (
            f"{summary['samples']} samples every {summary['interval_s']} s "
            f"(sampler running at stop: {summary['sampler_running_at_stop']})."
        ),
        "",
        "| counter | median | p95 | max |",
        "| --- | ---: | ---: | ---: |",
    ]
    for key, value in summary["counters"].items():
        lines.append(
            f"| {key} | {value['median']:.2f} | {value['p95']:.2f} | {value['max']:.2f} |"
        )
    return "\n".join(lines) + "\n"


def stop(folder: Path) -> str:
    """Stop the sampler and write summary.json and summary.md; returns the Markdown."""
    pid = json.loads((folder / "sampler.json").read_text(encoding="utf-8"))["pid"]
    summary = summarize(folder, terminate(pid))
    text = render(summary)
    (folder / "summary.json").write_text(
        json.dumps(summary, indent=1), encoding="utf-8"
    )
    (folder / "summary.md").write_text(text, encoding="utf-8")
    return text
