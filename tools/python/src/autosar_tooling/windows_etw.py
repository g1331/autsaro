"""CI-only process/thread ETW lifecycle capture; never invoked per owned command."""

from __future__ import annotations

import datetime
import json
import os
import re
import statistics
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path

# ProcessThread supplies classic process command lines; the manifest provider adds
# FILETIME creation/exit identities. No CPU sampling, context switches or registry changes.
PROFILE = """<?xml version="1.0" encoding="utf-8"?>
<WindowsPerformanceRecorder Version="1.0">
  <Profiles>
    <SystemCollector Id="JobSystemCollector" Name="AUTSARO process lifecycle">
      <BufferSize Value="64"/><Buffers Value="64"/>
    </SystemCollector>
    <EventCollector Id="JobEventCollector" Name="AUTSARO process identities">
      <BufferSize Value="64"/><Buffers Value="64"/>
    </EventCollector>
    <SystemProvider Id="JobSystemProvider">
      <Keywords><Keyword Value="ProcessThread"/></Keywords>
    </SystemProvider>
    <EventProvider Id="JobProcessProvider" Name="Microsoft-Windows-Kernel-Process" Level="5">
      <Keywords><Keyword Value="0x30"/></Keywords>
    </EventProvider>
    <Profile Id="Job.Verbose.File" Name="Job" Description="Job member lifecycle"
             LoggingMode="File" DetailLevel="Verbose">
      <Collectors>
        <SystemCollectorId Value="JobSystemCollector">
          <SystemProviderId Value="JobSystemProvider"/>
        </SystemCollectorId>
        <EventCollectorId Value="JobEventCollector">
          <EventProviders><EventProviderId Value="JobProcessProvider"/></EventProviders>
        </EventCollectorId>
      </Collectors>
    </Profile>
    <Profile Id="Job.Verbose.Memory" Name="Job" Description="Job member lifecycle"
             LoggingMode="Memory" DetailLevel="Verbose">
      <Collectors>
        <SystemCollectorId Value="JobSystemCollector">
          <SystemProviderId Value="JobSystemProvider"/>
        </SystemCollectorId>
        <EventCollectorId Value="JobEventCollector">
          <EventProviders><EventProviderId Value="JobProcessProvider"/></EventProviders>
        </EventCollectorId>
      </Collectors>
    </Profile>
  </Profiles>
</WindowsPerformanceRecorder>
"""


def command(folder: Path, argv: list[str], timeout: int = 60) -> None:
    started = time.perf_counter()
    result = subprocess.run(argv, capture_output=True, timeout=timeout, check=False)
    with (folder / "collector.jsonl").open("a", encoding="utf-8") as output:
        output.write(
            json.dumps(
                {
                    "argv": argv,
                    "exit": result.returncode,
                    "elapsed_s": time.perf_counter() - started,
                    "stdout": result.stdout.decode(errors="replace"),
                    "stderr": result.stderr.decode(errors="replace"),
                }
            )
            + "\n"
        )
    if result.returncode:
        raise RuntimeError(
            f"ETW command failed: {argv}: {result.stdout!r} {result.stderr!r}"
        )


def benchmark() -> list[float]:
    samples = []
    for _ in range(12):
        started = time.perf_counter()
        subprocess.run(
            [sys.executable, "-I", "-S", "-c", "pass"], check=True, timeout=10
        )
        samples.append(time.perf_counter() - started)
    return samples


def _start(folder: Path) -> None:
    folder.mkdir(parents=True, exist_ok=True)
    profile = folder / "job.wprp"
    profile.write_text(PROFILE, encoding="utf-8")
    command(
        folder,
        ["wevtutil", "gp", "Microsoft-Windows-Kernel-Process", "/ge:true", "/gm:true"],
    )
    before = benchmark()
    command(folder, ["wpr", "-start", str(profile) + "!Job", "-filemode"])
    (folder / "started").touch()
    after = benchmark()
    (folder / "overhead.json").write_text(
        json.dumps(
            {
                "without_etw_s": before,
                "with_etw_s": after,
                "median_difference_s": statistics.median(after)
                - statistics.median(before),
            }
        ),
        encoding="utf-8",
    )


def events(path: Path):
    """Stream tracerpt XML; retain all fields instead of assuming an event version."""
    for _, element in ET.iterparse(path, events=("end",)):
        if element.tag.rsplit("}", 1)[-1] != "Event":
            continue
        system = element.find("{*}System")
        if system is not None:
            fields = {
                item.get("Name"): item.text for item in element.findall(".//{*}Data")
            }
            provider = system.find("{*}Provider")
            timestamp = system.find("{*}TimeCreated")
            yield {
                "provider": dict(provider.attrib) if provider is not None else {},
                "event_id": system.findtext("{*}EventID"),
                "opcode": system.findtext("{*}Opcode"),
                "timestamp": dict(timestamp.attrib) if timestamp is not None else {},
                "fields": fields,
            }
        element.clear()


def _stop(folder: Path) -> None:
    if not (folder / "started").exists():
        raise RuntimeError(
            "ETW session never started; collector.jsonl contains setup results"
        )
    command(folder, ["wpr", "-stop", str(folder / "process.etl")], timeout=120)
    command(
        folder,
        [
            "tracerpt",
            str(folder / "process.etl"),
            "-of",
            "XML",
            "-o",
            str(folder / "events.xml"),
            "-summary",
            str(folder / "summary.xml"),
            "-y",
        ],
        timeout=120,
    )
    count = 0
    with (folder / "lifecycle.jsonl").open("w", encoding="utf-8") as output:
        for event in events(folder / "events.xml"):
            output.write(json.dumps(event) + "\n")
            count += 1
    if not count:
        raise RuntimeError("ETW produced no decoded events")
    correlate(folder)
    print(
        f"ETW captured {count} events; raw ETL, XML, summary and lifecycle in {folder}"
    )


def collector_state(folder: Path, phase: str, error: Exception | None = None) -> bool:
    state = {
        "phase": phase,
        "complete": error is None,
        "error": str(error) if error else None,
    }
    try:
        folder.mkdir(parents=True, exist_ok=True)
        (folder / "collector-state.json").write_text(
            json.dumps(state), encoding="utf-8"
        )
    except OSError as write_error:
        print(f"ETW state write failed: {write_error}", file=sys.stderr)
    if error:
        print(f"ETW evidence degraded during {phase}: {error}", file=sys.stderr)
    return error is None


def start(folder: Path) -> bool:
    try:
        _start(folder)
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        return collector_state(folder, "start", error)
    return collector_state(folder, "started")


def stop(folder: Path) -> bool:
    # Failure to collect evidence must not suppress the actual contract suites.
    # Only stop a session for which this job received a successful start receipt.
    try:
        _stop(folder)
    except (
        OSError,
        RuntimeError,
        subprocess.SubprocessError,
        ValueError,
        KeyError,
        AssertionError,
    ) as error:
        return collector_state(folder, "stop", error)
    return collector_state(folder, "complete")


def filetime(value: str) -> int:
    if value.startswith("0x") or value.isdecimal():
        return int(value, 0) if value.startswith("0x") else int(value)
    match = re.fullmatch(r"(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d+))?Z", value)
    if match is None:
        raise ValueError(f"Unknown ETW timestamp: {value}")
    seconds = datetime.datetime.fromisoformat(match[1]).replace(tzinfo=datetime.UTC)
    epoch = datetime.datetime(1601, 1, 1, tzinfo=datetime.UTC)
    delta = seconds - epoch
    return (delta.days * 86400 + delta.seconds) * 10_000_000 + int(
        (match[2] or "").ljust(7, "0")[:7]
    )


def correlate(folder: Path) -> None:
    starts = {}
    stops = {}
    commands = defaultdict(list)
    identities = defaultdict(list)
    with (folder / "lifecycle.jsonl").open(encoding="utf-8") as source:
        for line in source:
            event = json.loads(line)
            fields = {str(key).lower(): value for key, value in event["fields"].items()}
            pid = fields.get("processid")
            if pid is None:
                continue
            pid = int(pid, 0) if pid.startswith("0x") else int(pid)
            if fields.get("createtime"):
                key = (pid, filetime(fields["createtime"]))
                if event["event_id"] == "1":
                    starts[key] = event
                    identities[pid].append(key[1])
                elif event["event_id"] == "2":
                    stops[key] = event
            provider = {key.lower(): value for key, value in event["provider"].items()}
            process_provider = provider.get("guid", "").strip("{}").lower() in {
                "3d6fa8d0-fe05-11d0-9dda-00c04fd7ba7c",  # Classic kernel Process
                "22fb2cd6-0e7b-422b-a0c7-2fad1fd0e716",  # Manifest Kernel-Process
            }
            if (
                fields.get("commandline")
                and event.get("opcode") == "1"
                and process_provider
            ):
                commands[pid].append(event)
    merged = {}
    for key, event in starts.items():
        pid, created = key
        next_created = min(
            (time for time in identities[pid] if time > created),
            default=2**64,
        )
        stopped = stops.get(key)
        if stopped:
            exit_fields = {
                key.lower(): value for key, value in stopped["fields"].items()
            }
            if exit_fields.get("exittime"):
                next_created = min(next_created, filetime(exit_fields["exittime"]) + 1)
        command = []
        for candidate in commands[pid]:
            stamp = candidate["timestamp"].get("SystemTime")
            if stamp and created <= filetime(stamp) < next_created:
                command.append(candidate)
        merged[key] = {
            "pid": pid,
            "created_100ns": created,
            "start": event,
            "stop": stops.get(key),
            "command_events": command,
        }
    with (folder / "members-etw.jsonl").open("w", encoding="utf-8") as output:
        # Both implementations archive failed captures into this same directory.
        for path in sorted(folder.parent.glob("job-evidence/*.jsonl")):
            for line in path.read_text(encoding="utf-8").splitlines():
                record = json.loads(line)
                for member in record.get("members", []):
                    key = (member["pid"], member.get("created_100ns"))
                    output.write(
                        json.dumps(
                            {
                                "capture": path.name,
                                "phase": record["phase"],
                                "member": member,
                                "etw": merged.get(key),
                            }
                        )
                        + "\n"
                    )
    proofs = list(folder.parent.glob("job-evidence/control-*.json"))
    if os.environ.get("AUTOSAR_ETW_REQUIRE_CONTROLS") == "true" and not proofs:
        raise RuntimeError("Windows orphan control produced no identity proof")
    if proofs:
        for path in proofs:
            proof = json.loads(path.read_text(encoding="utf-8"))
            event = merged[(proof["pid"], proof["created_100ns"])]
            fields = {
                key.lower(): value for key, value in event["start"]["fields"].items()
            }
            assert "python" in fields["imagename"].lower(), event
            assert int(fields["parentprocessid"]) > 0, event
            assert event["stop"] is not None, event
            assert event["command_events"], event
        print(
            "ETW exact PID+creation, image, parent, command line and exit verified: "
            f"{len(proofs)} controls"
        )


def main() -> None:
    action, directory = sys.argv[1:]
    {"start": start, "stop": stop}[action](Path(directory))


if __name__ == "__main__":
    main()
