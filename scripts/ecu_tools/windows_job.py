"""Windows stdlib-only adapter for the same bounded owned-command contract.

A suspended process is assigned to a private kill-on-close Job before its
only initial thread is resumed. No taskkill process or shell string is used.
"""

from __future__ import annotations

import ctypes
import os
import subprocess
import time
from ctypes import wintypes as w
from pathlib import Path


class BasicLimits(ctypes.Structure):
    _fields_ = [
        ("process_time", ctypes.c_int64),
        ("job_time", ctypes.c_int64),
        ("flags", w.DWORD),
        ("minimum", ctypes.c_size_t),
        ("maximum", ctypes.c_size_t),
        ("active", w.DWORD),
        ("affinity", ctypes.c_size_t),
        ("priority", w.DWORD),
        ("scheduling", w.DWORD),
    ]


class ExtendedLimits(ctypes.Structure):
    _fields_ = [
        ("basic", BasicLimits),
        ("io", ctypes.c_uint64 * 6),
        ("process_memory", ctypes.c_size_t),
        ("job_memory", ctypes.c_size_t),
        ("peak_process", ctypes.c_size_t),
        ("peak_job", ctypes.c_size_t),
    ]


class Accounting(ctypes.Structure):
    _fields_ = [
        ("times", ctypes.c_int64 * 4),
        ("faults", w.DWORD),
        ("total", w.DWORD),
        ("active", w.DWORD),
        ("terminated", w.DWORD),
    ]


class ThreadEntry(ctypes.Structure):
    _fields_ = [
        ("size", w.DWORD),
        ("usage", w.DWORD),
        ("thread", w.DWORD),
        ("process", w.DWORD),
        ("priority", w.LONG),
        ("delta", w.LONG),
        ("flags", w.DWORD),
    ]


if os.name == "nt":
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.CreateJobObjectW.argtypes = [w.LPVOID, w.LPCWSTR]
    kernel.CreateJobObjectW.restype = w.HANDLE
    kernel.SetInformationJobObject.argtypes = [
        w.HANDLE,
        ctypes.c_int,
        w.LPVOID,
        w.DWORD,
    ]
    kernel.SetInformationJobObject.restype = w.BOOL
    kernel.AssignProcessToJobObject.argtypes = [w.HANDLE, w.HANDLE]
    kernel.AssignProcessToJobObject.restype = w.BOOL
    kernel.TerminateJobObject.argtypes = [w.HANDLE, w.UINT]
    kernel.TerminateJobObject.restype = w.BOOL
    kernel.QueryInformationJobObject.argtypes = [
        w.HANDLE,
        ctypes.c_int,
        w.LPVOID,
        w.DWORD,
        w.LPVOID,
    ]
    kernel.QueryInformationJobObject.restype = w.BOOL
    kernel.CreateToolhelp32Snapshot.argtypes = [w.DWORD, w.DWORD]
    kernel.CreateToolhelp32Snapshot.restype = w.HANDLE
    kernel.Thread32First.argtypes = [w.HANDLE, ctypes.POINTER(ThreadEntry)]
    kernel.Thread32First.restype = w.BOOL
    kernel.Thread32Next.argtypes = [w.HANDLE, ctypes.POINTER(ThreadEntry)]
    kernel.Thread32Next.restype = w.BOOL
    kernel.OpenThread.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    kernel.OpenThread.restype = w.HANDLE
    kernel.ResumeThread.argtypes = [w.HANDLE]
    kernel.ResumeThread.restype = w.DWORD
    kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    kernel.OpenProcess.restype = w.HANDLE
    kernel.CloseHandle.argtypes = [w.HANDLE]
    kernel.CloseHandle.restype = w.BOOL


def _check(value: int, operation: str) -> int:
    if not value or value == ctypes.c_void_p(-1).value:
        raise ctypes.WinError(ctypes.get_last_error(), operation)
    return value


def _resume(pid: int) -> None:
    snapshot = _check(kernel.CreateToolhelp32Snapshot(4, 0), "Thread snapshot")
    try:
        entry = ThreadEntry()
        entry.size = ctypes.sizeof(entry)
        _check(kernel.Thread32First(snapshot, ctypes.byref(entry)), "First thread")
        while entry.process != pid:
            entry.size = ctypes.sizeof(entry)
            if not kernel.Thread32Next(snapshot, ctypes.byref(entry)):
                raise RuntimeError("Suspended process has no initial thread")
        thread = _check(
            kernel.OpenThread(0x42, False, entry.thread), "Open initial thread"
        )
        try:
            if kernel.ResumeThread(thread) != 1:
                raise ctypes.WinError(ctypes.get_last_error(), "Resume initial thread")
        finally:
            kernel.CloseHandle(thread)
    finally:
        kernel.CloseHandle(snapshot)


class WindowsJob:
    def __init__(
        self,
        argv: list[str],
        cwd: Path,
        env: dict[str, str] | None,
        stdout: Path,
        stderr: Path,
        stdin_file: Path | None = None,
    ) -> None:
        job = _check(kernel.CreateJobObjectW(None, None), "Create private job")
        self.job = job
        self.child: subprocess.Popen[bytes] | None = None
        self.closed = False
        self.assigned = False
        try:
            limits = ExtendedLimits()
            limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            _check(
                kernel.SetInformationJobObject(
                    job, 9, ctypes.byref(limits), ctypes.sizeof(limits)
                ),
                "Configure private job",
            )
            with (
                stdout.open("xb") as out,
                stderr.open("xb") as err,
                open(stdin_file or os.devnull, "rb") as input_stream,
            ):
                self.child = subprocess.Popen(
                    argv,
                    cwd=cwd,
                    env=env,
                    stdin=input_stream,
                    stdout=out,
                    stderr=err,
                    creationflags=subprocess.CREATE_NO_WINDOW
                    | 0x00000004,  # CREATE_SUSPENDED
                )
            process = _check(
                kernel.OpenProcess(0x1101, False, self.child.pid),
                "Open suspended command",
            )
            try:
                _check(
                    kernel.AssignProcessToJobObject(job, process),
                    "Assign suspended command",
                )
                self.assigned = True
            finally:
                kernel.CloseHandle(process)
            _resume(self.child.pid)
        except BaseException:
            self.close()
            raise

    @property
    def pid(self) -> int:
        assert self.child is not None
        return self.child.pid

    def _active(self) -> int:
        record = Accounting()
        _check(
            kernel.QueryInformationJobObject(
                self.job, 1, ctypes.byref(record), ctypes.sizeof(record), None
            ),
            "Observe job membership",
        )
        return record.active

    def poll(self) -> int | None:
        assert self.child is not None
        return self.child.poll()

    def wait(
        self, deadline_ns: int, *, close_tree_on_exit: bool = False
    ) -> tuple[str, int | None, bool]:
        while time.monotonic_ns() < deadline_ns:
            code = self.poll()
            if code is not None:
                # The active count can lag a reaped cooperative exit briefly.
                grace = min(deadline_ns, time.monotonic_ns() + 200_000_000)
                while self._active() and time.monotonic_ns() < grace:
                    time.sleep(0.01)
                orphaned = bool(self._active())
                self.close()
                status = "orphaned_members" if orphaned and not close_tree_on_exit else "exited"
                return status, code, orphaned
            time.sleep(0.01)
        self.close()
        return "timeout", self.poll(), False

    def close(self) -> None:
        if self.closed:
            return
        self.closed = True
        try:
            # A failed assignment leaves a suspended process outside the Job.
            # It is still ours and must be terminated through its own handle.
            if self.child is not None and not self.assigned:
                self.child.kill()
            _check(kernel.TerminateJobObject(self.job, 1), "Terminate private job")
            end = time.monotonic() + 5
            while self._active():
                if time.monotonic() >= end:
                    raise RuntimeError(
                        "cleanup_unconfirmed: Windows Job members remain"
                    )
                time.sleep(0.01)
            if self.child is not None:
                self.child.wait(timeout=5)
        finally:
            kernel.CloseHandle(self.job)
