"""OS observations for a launcher-owned native process, never synthetic JS heap data."""

from __future__ import annotations

import ctypes
import json
import os
import time
from pathlib import Path


def sample(scratch: Path, pid: int, scope: str) -> None:
    request_path = scratch / "native-measure-request.json"
    if not request_path.is_file():
        return
    request = json.loads(request_path.read_text(encoding="utf-8"))
    if os.name == "nt":
        from ctypes import wintypes as w

        class Counters(ctypes.Structure):
            _fields_ = [("size", w.DWORD), ("faults", w.DWORD)] + [
                (name, ctypes.c_size_t) for name in (
                    "peakWorkingSet", "workingSet", "peakPagedPool", "pagedPool",
                    "peakNonPagedPool", "nonPagedPool", "pagefile", "peakPagefile", "privateBytes",
                )
            ]

        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        api = ctypes.WinDLL("psapi", use_last_error=True)
        kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
        kernel.OpenProcess.restype = w.HANDLE
        kernel.CloseHandle.argtypes = [w.HANDLE]
        api.GetProcessMemoryInfo.argtypes = [w.HANDLE, ctypes.POINTER(Counters), w.DWORD]
        handle = kernel.OpenProcess(0x410, False, pid)
        if not handle:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            counters = Counters()
            counters.size = ctypes.sizeof(counters)
            if not api.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.size):
                raise ctypes.WinError(ctypes.get_last_error())
            values = {name: getattr(counters, name) for name in ("workingSet", "peakWorkingSet", "privateBytes", "peakPagefile")}
        finally:
            kernel.CloseHandle(handle)
        processes = [{"pid": pid, **values}]
        measurement = "Windows native Tauri process; WebView and external driver memory not included"
    else:
        parents = {}
        for entry in Path("/proc").iterdir():
            if entry.name.isdecimal():
                try:
                    status = dict(line.split(":", 1) for line in (entry / "status").read_text().splitlines() if ":" in line)
                    parents[int(entry.name)] = (int(status["PPid"].strip()), status)
                except (OSError, KeyError, ValueError):
                    continue
        owned = {pid}
        while True:
            descendants = {process for process, (parent, _) in parents.items() if parent in owned}
            if descendants <= owned:
                break
            owned.update(descendants)
        processes = []
        for process in sorted(owned):
            if process not in parents:
                continue
            status = parents[process][1]
            values = {name: int(status[name].split()[0]) * 1024 for name in ("VmRSS", "VmHWM", "VmSize") if name in status}
            processes.append({"pid": process, "name": status.get("Name", "").strip(), **values})
        measurement = "Owned native WebDriver server subtree including application/WebKit; excludes external Node scenario driver"
    receipt = {**request, "scope": scope, "measurement": measurement, "monotonicNs": time.monotonic_ns(), "processes": processes}
    destination = scratch / "native-measure-result.json"
    staging = destination.with_suffix(".next.json")
    staging.write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    staging.replace(destination)
    with (scratch / "native-memory.jsonl").open("a", encoding="utf-8") as stream:
        stream.write(json.dumps(receipt) + "\n")
    request_path.unlink()
