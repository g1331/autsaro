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

from autosar_tooling import windows_host_load, windows_process_timing

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
    <EventProvider Id="JobProcessProvider" Name="Microsoft-Windows-Kernel-Process" Level="5"
                   NonPagedMemory="true">
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


class ControlProofError(RuntimeError):
    """The trace cannot prove the control processes' identity, command line and exit."""


def controls_required() -> bool:
    return os.environ.get("AUTOSAR_ETW_REQUIRE_CONTROLS") == "true"


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
                "event_guid": element.findtext(".//{*}EventGuid"),
                "timestamp": dict(timestamp.attrib) if timestamp is not None else {},
                "fields": fields,
            }
        element.clear()


def _stop(folder: Path) -> None:
    if not (folder / "started").exists():
        raise RuntimeError(
            "ETW session never started; collector.jsonl contains setup results"
        )
    # Tests have finished: the same quiet-time probe as at start, before the trace stops.
    (folder / "reference-end.json").write_text(
        json.dumps({"samples_s": benchmark()}), encoding="utf-8"
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
    process_events = 0
    loss_counters = []
    with (folder / "lifecycle.jsonl").open("w", encoding="utf-8") as output:
        for event in events(folder / "events.xml"):
            output.write(json.dumps(event) + "\n")
            count += 1
            provider = {key.lower(): value for key, value in event["provider"].items()}
            provider_guid = provider.get("guid", "").strip("{}").lower()
            if provider_guid == "22fb2cd6-0e7b-422b-a0c7-2fad1fd0e716" and event[
                "event_id"
            ] in {"1", "2"}:
                process_events += 1
            fields = event["fields"]
            if "EventsLost" in fields:
                loss_counters.append(
                    {
                        name: int(fields[name])
                        for name in ("EventsLost", "BuffersLost")
                        if name in fields
                    }
                )
    health = {
        "decoded_events": count,
        "process_events": process_events,
        "loss_counters": loss_counters,
    }
    (folder / "trace-health.json").write_text(json.dumps(health), encoding="utf-8")
    if not process_events:
        raise RuntimeError("ETW has no Kernel-Process lifecycle events")
    publish_process_timing(folder)
    if any(value for counters in loss_counters for value in counters.values()):
        raise RuntimeError(
            "ETW lost events or buffers; trace-health.json contains the counters"
        )
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


def publish_process_timing(folder: Path) -> None:
    """The timing summary is extra evidence: it never changes the control proof's verdict."""
    try:
        windows_process_timing.publish(
            windows_process_timing.write_summary(
                folder, os.environ.get("AUTOSAR_ETW_LABEL")
            )
        )
    except (OSError, ValueError, KeyError) as error:
        print(f"Process timing summary unavailable: {error!r}", file=sys.stderr)


def start_host_load(folder: Path) -> None:
    """Host counters are extra evidence: a failure is reported and never stops a job."""
    try:
        windows_host_load.start(folder / "host-load")
    except (OSError, subprocess.SubprocessError) as error:
        print(f"Host load sampler did not start: {error}", file=sys.stderr)


def stop_host_load(folder: Path) -> None:
    try:
        windows_process_timing.publish(windows_host_load.stop(folder / "host-load"))
    except (
        OSError,
        RuntimeError,
        ValueError,
        KeyError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Host load evidence unavailable: {error}", file=sys.stderr)


def start(folder: Path) -> bool:
    start_host_load(folder)
    try:
        _start(folder)
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        return collector_state(folder, "start", error)
    return collector_state(folder, "started")


def stop(folder: Path) -> bool:
    # Failure to collect evidence must not suppress the actual contract suites.
    # Only stop a session for which this job received a successful start receipt.
    stop_host_load(folder)
    try:
        _stop(folder)
    except (
        OSError,
        RuntimeError,
        subprocess.SubprocessError,
        ValueError,
        KeyError,
    ) as error:
        degraded = collector_state(folder, "stop", error)
        # A job that must prove its controls cannot count as a clean run without the
        # evidence (ADR 0031): fail the step. Other jobs keep running on degraded evidence.
        if controls_required():
            raise
        return degraded
    return collector_state(folder, "complete")


def filetime(value: str) -> int:
    value = value.strip()
    if value.startswith("0x") or value.isdecimal():
        return int(value, 0) if value.startswith("0x") else int(value)
    match = re.fullmatch(
        r"(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d+))?(?:Z|\+00:00)", value
    )
    if match is None:
        raise ValueError(f"Unknown ETW timestamp: {value}")
    seconds = datetime.datetime.fromisoformat(match[1]).replace(tzinfo=datetime.UTC)
    epoch = datetime.datetime(1601, 1, 1, tzinfo=datetime.UTC)
    delta = seconds - epoch
    return (delta.days * 86400 + delta.seconds) * 10_000_000 + int(
        (match[2] or "").ljust(7, "0")[:7]
    )


def stamp(event: dict) -> int:
    return filetime(event["timestamp"]["SystemTime"])


def correlate(folder: Path) -> None:
    starts = {}
    stops = {}
    commands = defaultdict(list)
    opened = defaultdict(list)
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
                    opened[pid].append(stamp(event))
                elif event["event_id"] == "2":
                    stops[key] = event
            provider = {key.lower(): value for key, value in event["provider"].items()}
            process_provider = (
                event.get("event_guid") or provider.get("guid", "")
            ).strip("{}").lower() in {
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
        # CreateTime and ExitTime are kernel FILETIME payload; the trace clock stamps
        # event headers and sits several milliseconds away from it, by a different amount
        # in every run. A command line belongs to the process whose creation event sits
        # just before it on the trace clock, so every bound here is a header time.
        began = stamp(event)
        next_began = min((time for time in opened[pid] if time > began), default=2**64)
        stopped = stops.get(key)
        ended = stamp(stopped) if stopped else 2**64
        merged[key] = {
            "pid": pid,
            "created_100ns": created,
            "start": event,
            "stop": stopped,
            "command_events": [
                candidate
                for candidate in commands[pid]
                if began <= stamp(candidate) < next_began and stamp(candidate) <= ended
            ],
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
    if controls_required() and not proofs:
        raise ControlProofError("Windows orphan control produced no identity proof")
    if proofs:
        for path in proofs:
            verify_control(path, merged)
        print(
            "ETW exact PID+creation, image, parent, command line and exit verified: "
            f"{len(proofs)} controls"
        )


def verify_control(path: Path, merged: dict) -> None:
    try:
        proof = json.loads(path.read_text(encoding="utf-8"))
        event = merged.get((proof["pid"], proof["created_100ns"]))
        if event is None:
            raise ControlProofError(
                f"{path.name}: no process start for the control: {proof}"
            )
        fields = {key.lower(): value for key, value in event["start"]["fields"].items()}
        failed = [
            reason
            for reason, ok in (
                ("image is not python", "python" in fields["imagename"].lower()),
                ("no parent", int(fields["parentprocessid"]) > 0),
                ("no exit event", event["stop"] is not None),
                ("no command line event", bool(event["command_events"])),
            )
            if not ok
        ]
    except (KeyError, ValueError) as error:
        raise ControlProofError(f"{path.name}: unreadable proof: {error!r}") from error
    if failed:
        raise ControlProofError(f"{path.name}: {', '.join(failed)}: {event}")


def main() -> None:
    action, directory = sys.argv[1:]
    {"start": start, "stop": stop}[action](Path(directory))


if __name__ == "__main__":
    main()
