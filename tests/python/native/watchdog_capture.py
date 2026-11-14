"""Replay one retained Linux HostBatch and inspect its first stopped shutdown entry."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

from ecu_tools.owner import Owner
from ecu_tools.process import ProcessSpec, run_bounded

FIXTURES = Path(__file__).resolve().parents[3] / "core/tests/fixtures/host-batch"
FIELDS_SCRIPT = FIXTURES / "linux_watchdog_state.py"
OBSERVER = FIXTURES / "linux_watchdog_capture.c"
WRAPPERS = ("Os_BackendShutdown", "Os_BackendRequestShutdown", "Ecu_TargetAbortNative")


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def command(owner, argv, directory, stage, env=None):
    return run_bounded(
        ProcessSpec.seconds(argv, directory, 180, directory, stage, env=env), owner=owner
    )


def build(owner, receipt, directory):
    data = json.loads(receipt.read_text(encoding="utf-8"))
    argv = data.get("instrumented")
    if not isinstance(argv, list) or not all(isinstance(arg, str) for arg in argv):
        raise ValueError("Expected the retained instrumented compile-command array")
    if argv.count("-o") != 1 or "-fsanitize=thread" not in argv:
        raise ValueError("Expected one output and the original TSan configuration")
    argv = list(argv)
    binary = directory / "ecu_probe"
    argv[argv.index("-o") + 1] = str(binary)
    argv += [str(OBSERVER), *(f"-Wl,--wrap={name}" for name in WRAPPERS)]
    (directory / "compile-command.json").write_text(json.dumps(argv, indent=2) + "\n")
    command(owner, argv, directory, "watchdog observer build")
    return binary


def debugger(owner, executable, binary, directory, mode, pid=None):
    env = dict(os.environ, AUTOSAR_WATCHDOG_OUTPUT=str(directory), AUTOSAR_WATCHDOG_MODE=mode)
    argv = [
        executable, "-q", "-nx", "-batch", "-ex", "set pagination off",
        "-ex", "set confirm off", "-ex", "set debuginfod enabled off",
        "-ex", "set print elements 32", str(binary),
    ]
    if pid is not None:
        argv += ["-p", str(pid)]
    argv += ["-x", str(FIELDS_SCRIPT)]
    if pid is not None:
        argv += ["-ex", "kill"]
    command(owner, argv, directory, f"watchdog {mode}", env)
    return json.loads((directory / f"{mode}.json").read_text(encoding="utf-8"))


def tasks(pid):
    result = {}
    for thread in (Path("/proc") / str(pid) / "task").iterdir():
        record = {}
        for name in ("stat", "schedstat", "status", "wchan"):
            try:
                record[name] = (thread / name).read_text()
            except OSError as error:
                record[name] = {"errno": error.errno, "error": str(error)}
        result[thread.name] = record
    return result


class Observation:
    """Read-only interval observations; only the stopped GDB snapshot is coherent."""

    def __init__(self, pid, binary, layout):
        self.pid = pid
        self.binary = binary
        self.relative = layout
        self.layout = None
        with binary.open("rb") as source:
            header = source.read(20)
        if header[:6] != b"\x7fELF\x02\x01":
            raise ValueError("The watchdog observer requires a little-endian ELF64 program")
        self.pie = int.from_bytes(header[16:18], "little") == 3

    def locate(self):
        maps = (Path("/proc") / str(self.pid) / "maps").read_text()
        for line in maps.splitlines():
            if str(self.binary) in line and line.split()[2] == "00000000":
                relocation = int(line.split("-")[0], 16) if self.pie else 0
                self.layout = dict(self.relative)
                for key in ("base", "watch_started", "gate_pointer", "closing"):
                    self.layout[key] += relocation
                return

    def handles(self, memory):
        layout = self.layout
        fields = layout["fields"]
        records = []
        for index in range(layout["count"]):
            address = layout["base"] + index * layout["stride"]
            prefix = os.pread(memory, fields["ended"]["offset"] + 4, address)
            def read(name, prefix=prefix, address=address):
                field = fields[name]
                begin, length = field["offset"], field["size"]
                raw = (prefix[begin:begin + length] if begin + length <= len(prefix)
                       else os.pread(memory, length, address + begin))
                if len(raw) != length:
                    raise OSError(f"Incomplete process-memory read for {name}")
                return int.from_bytes(raw, "little", signed=name == "fd")
            if read("reserved"):
                records.append({"index": index, "address": address,
                                **{name: read(name) for name in fields}})
        return records

    def sample(self):
        record = {"monotonic_ns": time.monotonic_ns(), "wall_ns": time.time_ns()}
        try:
            record["tasks"] = tasks(self.pid)
            if self.layout is None:
                self.locate()
            if self.layout is not None:
                with open(f"/proc/{self.pid}/mem", "rb", buffering=0) as memory:
                    record["handles"] = self.handles(memory.fileno())
                    for name, length in (("watch_started", 8), ("gate_pointer", 8), ("closing", 4)):
                        raw = os.pread(memory.fileno(), length, self.layout[name])
                        if len(raw) != length:
                            raise OSError(f"Incomplete process-memory read for {name}")
                        record[name] = int.from_bytes(raw, "little")
        except OSError as error:
            record["observation_error"] = {"errno": error.errno, "error": str(error)}
        record["observation_completed_ns"] = time.monotonic_ns()
        return record


class Timeline:
    def __init__(self, path, output):
        self.path = path
        self.output = output
        self.position = 0
        self.pending = b""
        self.started = 0

    def sample(self, record):
        with self.path.open("rb") as source:
            source.seek(self.position)
            chunk = source.read()
        self.position += len(chunk)
        parts = (self.pending + chunk).split(b"\n")
        self.pending = parts.pop()
        stamp = {key: record[key] for key in (
            "monotonic_ns", "observation_completed_ns", "wall_ns"
        )}
        for line in parts:
            if line.startswith((b"COMMIT", b"lifecycle=")):
                self.output.write(json.dumps(dict(stamp, line=line.decode("utf-8"))) + "\n")
        started = record.get("watch_started", 0)
        if started and started != self.started:
            self.output.write(json.dumps(dict(stamp, batch_started_monotonic_ms=started)) + "\n")
            self.started = started
        self.output.flush()


def replay(owner, gdb, setarch, binary, script, directory, layout):
    metadata = {
        "binary": str(binary), "binary_sha256": digest(binary),
        "input_sha256": digest(script), "observer_source_sha256": digest(OBSERVER),
        "state_script_sha256": digest(FIELDS_SCRIPT),
        "started_wall_ns": time.time_ns(), "started_monotonic_ns": time.monotonic_ns(),
        "sched_schedstats": Path("/proc/sys/kernel/sched_schedstats").read_text(),
        "cpu_affinity": sorted(os.sched_getaffinity(0)),
        "nice": os.getpriority(os.PRIO_PROCESS, 0),
        "status": "incomplete",
    }
    process = None
    try:
        with script.open("rb") as incoming, (directory / "stdout").open("wb") as output, \
                (directory / "stderr").open("wb") as errors:
            process = subprocess.Popen(
                [setarch, "x86_64", "-R", str(binary)], stdin=incoming,
                stdout=output, stderr=errors,
            )
            metadata["pid"] = process.pid
            (directory / "owner.json").write_text(json.dumps(metadata, indent=2) + "\n")
            observation = Observation(process.pid, binary, layout)
            with (directory / "interval.jsonl").open("w") as intervals, \
                    (directory / "batches.jsonl").open("w") as batches:
                timeline = Timeline(directory / "stdout", batches)
                deadline = time.monotonic() + 30
                while process.poll() is None:
                    record = observation.sample()
                    intervals.write(json.dumps(record) + "\n")
                    intervals.flush()
                    timeline.sample(record)
                    stopped = any(
                        isinstance(task.get("status"), str) and "\nState:\tT " in task["status"]
                        for task in record.get("tasks", {}).values()
                    )
                    if stopped:
                        (directory / "pre-attach.json").write_text(json.dumps(record, indent=2))
                        snapshot = debugger(owner, gdb, binary, directory, "snapshot", process.pid)
                        if record.get("closing") != 0 or not snapshot["before_close"]:
                            raise RuntimeError(
                                "Capture did not preserve the state before first shutdown"
                            )
                        metadata["status"] = "captured_abort"
                        process.wait(timeout=3)
                        return 7
                    if time.monotonic() >= deadline:
                        raise TimeoutError(
                            "HostBatch exceeded its unchanged 30-second outer deadline"
                        )
                    time.sleep(0.05)
                timeline.sample(observation.sample())
            if process.returncode == 7:
                raise RuntimeError("Abort exited without a valid before-close capture")
            metadata["status"] = "exited"
            return process.returncode
    finally:
        if process is not None:
            if process.poll() is None:
                process.kill()
                process.wait()
            metadata["returncode"] = process.returncode
        metadata["ended_monotonic_ns"] = time.monotonic_ns()
        (directory / "result.json").write_text(json.dumps(metadata, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    inputs = parser.add_mutually_exclusive_group(required=True)
    inputs.add_argument("--build-receipt", type=Path)
    inputs.add_argument("--binary", type=Path)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--gdb", default="gdb")
    args = parser.parse_args()
    gdb = shutil.which(args.gdb)
    setarch = shutil.which("setarch")
    if os.name != "posix" or not Path("/proc/self/status").exists() or not gdb or not setarch:
        raise RuntimeError("Linux, GDB and setarch are required before capture")
    script = args.input.resolve(strict=True)
    source = (args.build_receipt or args.binary).resolve(strict=True)
    directory = args.output.resolve()
    directory.mkdir(mode=0o700, parents=True, exist_ok=False)
    owner = Owner.start()
    try:
        binary = (build(owner, source, directory)
                  if args.build_receipt else source)
        layout = debugger(owner, gdb, binary, directory, "layout")
        if not layout["observer"]:
            raise ValueError("The diagnostic binary must link linux_watchdog_capture.c")
        return replay(owner, gdb, setarch, binary, script, directory, layout)
    finally:
        owner.close()


if __name__ == "__main__":
    raise SystemExit(main())
