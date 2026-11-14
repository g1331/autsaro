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
        "Os_Closing", "'Os_Backend.c'::shutdown_reason", "'Os_Backend.c'::ready",
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
    result["capture_errors"] = ERRORS
    result["before_close"] = result["globals"]["Os_Closing"] == "0" and not ERRORS
    (OUTPUT / "all-stacks.txt").write_text(
        gdb.execute("thread apply all bt full", to_string=True), encoding="utf-8"
    )
    return result


mode = os.environ["AUTOSAR_WATCHDOG_MODE"]
if mode == "layout":
    data = layout()
elif mode == "snapshot":
    data = snapshot()
else:
    raise ValueError(f"Unknown watchdog observation mode: {mode}")
(OUTPUT / f"{mode}.json").write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
