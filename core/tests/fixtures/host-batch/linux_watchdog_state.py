"""Read a controlled Linux HostBatch's layout or frozen state inside GDB."""
from __future__ import annotations

import json
import os
import time
from pathlib import Path

import gdb

OUTPUT = Path(os.environ["AUTOSAR_WATCHDOG_OUTPUT"])
FIELDS = (
    "reserved", "kind", "fd", "manual", "mutex_owner", "mutex_depth",
    "mutex_generation", "thread_id", "requested", "acknowledged", "resumed",
    "ended", "mutex_wait_address",
)
ERRORS = []


def value(expression):
    try:
        return str(gdb.parse_and_eval(expression))
    except gdb.error as error:
        ERRORS.append({"expression": expression, "error": str(error)})
        return None


def layout():
    handles = gdb.parse_and_eval("&host_handles").type.target()
    cell = handles.target()
    return {
        "base": int(gdb.parse_and_eval("&host_handles[0]")),
        "stride": cell.sizeof,
        "count": handles.range()[1] + 1,
        "fields": {
            field.name: {"offset": field.bitpos // 8, "size": field.type.sizeof}
            for field in cell.fields() if field.name in FIELDS
        },
        "watch_started": int(gdb.parse_and_eval("&host_watch.started")),
        "gate_pointer": int(gdb.parse_and_eval("&interrupt_gate")),
        "closing": int(gdb.parse_and_eval("&Os_Closing")),
        "observer": value("&__wrap_Ecu_TargetAbortNative"),
        "gdb_version": gdb.VERSION,
    }


def thread_state(thread, pid):
    thread.switch()
    result = {
        "ptid": thread.ptid,
        "gdb_number": thread.num,
        "current_thread": value("current_thread"),
        "held_depth": value("held_depth"),
        "backtrace": gdb.execute("bt", to_string=True),
        "proc": {},
    }
    root = Path("/proc") / str(pid) / "task" / str(thread.ptid[1])
    for name in ("status", "stat", "schedstat", "wchan"):
        try:
            result["proc"][name] = (root / name).read_text()
        except OSError as error:
            result["proc"][name] = {"errno": error.errno, "error": str(error)}
            ERRORS.append({"tid": thread.ptid[1], "file": name, "error": str(error)})
    return result


def automatic_budget():
    records = gdb.parse_and_eval("capture_budget")
    result = {}
    for index, role in enumerate(("H", "S")):
        thread = records[index]
        sequence = int(thread["sequence"])
        samples = []
        size = thread["samples"].type.strip_typedefs().range()[1] + 1
        for number in range(max(0, sequence - size), sequence):
            sample = thread["samples"][number % size]
            if int(sample["sequence"]) != number + 1:
                continue  # A freeze may interrupt the oldest slot's replacement.
            samples.append({
                name: int(sample[name]) for name in
                ("sequence", "wall_ns", "cpu_ns", "outside_cpu_ns", "phase", "entering")
            } | {
                name: [int(sample[name][phase]) for phase in range(5)]
                for name in ("calls", "phase_exclusive_cpu_ns", "phase_cpu_ns", "phase_wall_ns")
            })
        result[role] = {
            name: int(thread[name]) for name in
            ("tid", "active", "revision", "entered_wall_ns", "entered_cpu_ns", "accounted_cpu_ns", "outside_cpu_ns", "sequence", "clock_errors")
        } | {
            "phases": [{name: int(thread["phases"][phase][name]) for name in
                        ("calls", "completed", "exclusive_cpu_ns", "cpu_ns", "wall_ns")} for phase in range(5)],
            "samples": samples,
        }
    return result


def automatic_state(result):
    try:
        result["sched_schedstats"] = Path("/proc/sys/kernel/sched_schedstats").read_text().strip()
    except OSError as error:
        result["sched_schedstats"] = {"error": str(error)}
    result["freeze_monotonic_ns"] = int(gdb.parse_and_eval("capture_freeze_ns"))
    result["captured_reason"] = int(gdb.parse_and_eval("capture_reason"))
    if result["captured_reason"] != 7:
        ERRORS.append({"error": "stop was not a captured reason=7"})
    stacks = gdb.parse_and_eval("'Os_StackLinux.c'::stacks")
    result["roles"] = {
        chr(int(cell["role"])): int(cell["thread_id"])
        for cell in (stacks[index] for index in range(stacks.type.range()[1] + 1))
        if int(cell["role"]) != 0
    }
    gate = gdb.parse_and_eval("interrupt_gate").dereference()
    result["gate"] = {name: int(gate[name]) for name in
                      ("mutex_owner", "mutex_depth", "mutex_generation")}
    result["budget"] = automatic_budget()
    result["phase_names"] = ["wait", "suspend", "resume", "drain", "post"]
    for role in ("H", "S"):
        if result["budget"][role]["clock_errors"]:
            ERRORS.append({"role": role, "error": "budget clock failed"})


def snapshot():
    inferior = gdb.selected_inferior()
    result = {
        "pid": inferior.pid,
        "monotonic_ns": time.monotonic_ns(),
        "wall_ns": time.time_ns(),
        "gdb_version": gdb.VERSION,
    }
    expressions = (
        "interrupt_gate", "*interrupt_gate", "port_running", "'port.c'::pending",
        "Os_Closing", "'Os_Backend.c'::shutdown_reason", "'Os_Backend.c'::os_backend_ready",
        "'Ecu_Target.c'::lifecycle", "host_watch.started",
    )
    result["globals"] = {name: value(name) for name in expressions}
    result["threads"] = [thread_state(thread, inferior.pid) for thread in inferior.threads()]
    result["handles"] = []
    handles = gdb.parse_and_eval("host_handles")
    for index in range(handles.type.range()[1] + 1):
        cell = handles[index]
        if int(cell["reserved"]) != 0:
            record = {"index": index, "address": str(cell.address)}
            record.update({name: str(cell[name]) for name in FIELDS})
            if int(cell["fd"]) >= 0:
                path = Path("/proc") / str(inferior.pid) / "fdinfo" / str(int(cell["fd"]))
                try:
                    record["fdinfo"] = path.read_text()
                except OSError as error:
                    record["fdinfo_error"] = str(error)
                    ERRORS.append({"fdinfo": str(path), "error": str(error)})
            result["handles"].append(record)
    if os.environ["AUTOSAR_WATCHDOG_MODE"] == "automatic":
        automatic_state(result)
    result["capture_errors"] = ERRORS
    result["before_close"] = result["globals"]["Os_Closing"] == "0" and not ERRORS
    (OUTPUT / "all-stacks.txt").write_text(
        gdb.execute("thread apply all bt full", to_string=True), encoding="utf-8"
    )
    return result


mode = os.environ["AUTOSAR_WATCHDOG_MODE"]
if mode == "layout":
    data = layout()
elif mode in ("snapshot", "automatic"):
    data = snapshot()
else:
    raise ValueError(f"Unknown watchdog observation mode: {mode}")
(OUTPUT / f"{mode}.json").write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
