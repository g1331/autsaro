"""Windows stdlib-only adapter for the same bounded owned-command contract.

A suspended process is assigned to a private kill-on-close Job before its
only initial thread is resumed. No taskkill process or shell string is used.
"""

from __future__ import annotations

import ctypes
import json
import ntpath
import os
import subprocess
import sys
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
    kernel.GetSystemDirectoryW.argtypes = [w.LPWSTR, w.UINT]
    kernel.GetSystemDirectoryW.restype = w.UINT
    kernel.CloseHandle.argtypes = [w.HANDLE]
    kernel.CloseHandle.restype = w.BOOL


class ProcessIds(ctypes.Structure):
    _fields_ = [
        ("assigned", w.DWORD),
        ("listed", w.DWORD),
        ("ids", ctypes.c_size_t * 128),
    ]


class SystemThread(ctypes.Structure):
    # SYSTEM_THREAD_INFORMATION, winternl.h. WaitReason is valid in state 5.
    _fields_ = [
        ("times", ctypes.c_int64 * 3),
        ("wait_time", w.DWORD),
        ("start", w.LPVOID),
        ("pid", ctypes.c_size_t),
        ("tid", ctypes.c_size_t),
        ("priority", w.LONG),
        ("base_priority", w.LONG),
        ("switches", w.DWORD),
        ("state", w.DWORD),
        ("reason", w.DWORD),
    ]


class SystemProcess(ctypes.Structure):
    # SYSTEM_PROCESS_INFORMATION through the fixed-size header (winternl.h).
    _fields_ = [
        ("next", w.DWORD),
        ("threads", w.DWORD),
        ("reserved1", ctypes.c_byte * 24),
        ("created", ctypes.c_int64),
        ("times", ctypes.c_int64 * 2),
        ("name_length", w.USHORT),
        ("name_maximum", w.USHORT),
        ("name", w.LPWSTR),
        ("priority", w.LONG),
        ("pid", ctypes.c_size_t),
        ("parent", ctypes.c_size_t),
        ("handles", w.DWORD),
        ("session", w.DWORD),
        ("reserved3", ctypes.c_size_t),
        ("sizes", ctypes.c_size_t * 12),
        ("reserved7", ctypes.c_int64 * 6),
    ]


def system_processes() -> dict[int, dict]:
    """One handle-free process/thread snapshot, only at failed closure."""
    ntdll = ctypes.WinDLL("ntdll")
    query = ntdll.NtQuerySystemInformation
    query.argtypes = [ctypes.c_int, w.LPVOID, w.ULONG, ctypes.POINTER(w.ULONG)]
    query.restype = w.LONG
    # One bounded buffer avoids extending cleanup with an unbounded resize loop.
    buffer = ctypes.create_string_buffer(16 * 1024 * 1024)
    length = w.ULONG()
    status = query(5, buffer, len(buffer), ctypes.byref(length))
    if status < 0:
        raise OSError(f"NtQuerySystemInformation status=0x{status & 0xFFFFFFFF:08x}")
    result = {}
    offset = 0
    while offset + ctypes.sizeof(SystemProcess) <= length.value:
        info = SystemProcess.from_buffer(buffer, offset)
        end = offset + info.next if info.next else length.value
        thread_offset = offset + ctypes.sizeof(SystemProcess)
        if (
            end > length.value
            or thread_offset + info.threads * ctypes.sizeof(SystemThread) > end
        ):
            raise OSError("Truncated SYSTEM_PROCESS_INFORMATION")
        threads = []
        for index in range(info.threads):
            thread = SystemThread.from_buffer(
                buffer, thread_offset + index * ctypes.sizeof(SystemThread)
            )
            threads.append(
                {
                    "tid": thread.tid,
                    "state": thread.state,
                    "wait_reason": thread.reason if thread.state == 5 else None,
                    "created_100ns": thread.times[2],
                }
            )
        result[info.pid] = {
            "created_100ns": info.created,
            "parent_pid": info.parent,
            "image": ctypes.wstring_at(info.name, info.name_length // 2)
            if info.name_length
            else None,
            "threads": threads,
        }
        if not info.next:
            break
        offset = end
    return result


if os.name == "nt":
    kernel.GetProcessTimes.argtypes = [w.HANDLE] + [ctypes.POINTER(w.FILETIME)] * 4
    kernel.GetProcessTimes.restype = w.BOOL
    kernel.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
    kernel.WaitForSingleObject.restype = w.DWORD
    kernel.QueryFullProcessImageNameW.argtypes = [
        w.HANDLE,
        w.DWORD,
        w.LPWSTR,
        ctypes.POINTER(w.DWORD),
    ]
    kernel.QueryFullProcessImageNameW.restype = w.BOOL
    kernel.IsProcessInJob.argtypes = [w.HANDLE, w.HANDLE, ctypes.POINTER(w.BOOL)]
    kernel.IsProcessInJob.restype = w.BOOL
    kernel.GetExitCodeProcess.argtypes = [w.HANDLE, ctypes.POINTER(w.DWORD)]
    kernel.GetExitCodeProcess.restype = w.BOOL


def member(pid: int, job: int) -> dict:
    handle = kernel.OpenProcess(0x101000, False, pid)
    if not handle:
        return {"pid": pid, "open_error": ctypes.get_last_error()}
    try:
        wait = kernel.WaitForSingleObject(handle, 0)
        wait_error = ctypes.get_last_error() if wait == 0xFFFFFFFF else None
        times = [w.FILETIME() for _ in range(4)]
        times_ok = kernel.GetProcessTimes(handle, *(ctypes.byref(t) for t in times))
        times_error = None if times_ok else ctypes.get_last_error()
        values = [(t.dwHighDateTime << 32) | t.dwLowDateTime for t in times]
        image = ctypes.create_unicode_buffer(32768)
        size = w.DWORD(len(image))
        image_ok = kernel.QueryFullProcessImageNameW(
            handle, 0, image, ctypes.byref(size)
        )
        image_error = None if image_ok else ctypes.get_last_error()
        in_job = w.BOOL()
        membership_ok = kernel.IsProcessInJob(handle, job, ctypes.byref(in_job))
        membership_error = None if membership_ok else ctypes.get_last_error()
        code = w.DWORD()
        exit_ok = kernel.GetExitCodeProcess(handle, ctypes.byref(code))
        exit_error = None if exit_ok else ctypes.get_last_error()
        return {
            "pid": pid,
            "created_100ns": values[0] if times_ok else None,
            "exited_100ns": values[1] if times_ok and wait == 0 else None,
            "kernel_100ns": values[2] if times_ok else None,
            "user_100ns": values[3] if times_ok else None,
            "times_error": times_error,
            "wait": wait,
            "wait_error": wait_error,
            "image": image.value if image_ok else None,
            "image_error": image_error,
            "in_job": bool(in_job.value) if membership_ok else None,
            "membership_error": membership_error,
            "exit_code": code.value if exit_ok else None,
            "exit_error": exit_error,
        }
    finally:
        kernel.CloseHandle(handle)


STILL_ACTIVE = 259
ERROR_INVALID_PARAMETER = 87
GRACE_NS = 200_000_000
# A console host can finish its process teardown well after its last client has
# exited: one CI run held four such hosts for 404 ms. The bound equals the
# five-second budget close() already allows for Job members to leave after a
# TerminateJobObject, and the command's own deadline still applies.
CONSOLE_HOST_TEARDOWN_NS = 5_000_000_000


def job_pids(job: int) -> list[int] | None:
    """Job member PIDs, or None when the list is unavailable or truncated."""
    ids = ProcessIds()
    ok = kernel.QueryInformationJobObject(
        job, 3, ctypes.byref(ids), ctypes.sizeof(ids), None
    )
    if not ok or ids.assigned != ids.listed or ids.listed > len(ids.ids):
        return None
    return [int(pid) for pid in ids.ids[: ids.listed]]


def system_console_host() -> str | None:
    """Normalized path of the system console host, None when it cannot be read."""
    directory = ctypes.create_unicode_buffer(260)
    length = kernel.GetSystemDirectoryW(directory, len(directory))
    if not 0 < length < len(directory):
        return None
    return ntpath.normcase(ntpath.join(directory.value, "conhost.exe"))


def console_hosts(job: int) -> set[tuple[int, int]]:
    """PID and creation time of the Job members that are the system console host.

    Reading the image of a process in teardown soon fails with access denied, so
    callers record identities early and decide with them later. Without the
    system directory no member is recognized, which keeps the strict rule.
    """
    system_host = system_console_host()
    hosts = set()
    for pid in job_pids(job) or []:
        value = member(pid, job)
        image = value.get("image")
        created = value.get("created_100ns")
        # An unreadable system directory (None) matches no image.
        if image and created is not None and ntpath.normcase(image) == system_host:
            hosts.add((pid, created))
    return hosts


def exited_console_hosts_only(job: int, hosts: set[tuple[int, int]]) -> bool:
    """True when every remaining Job member is a known console host that exited.

    A console host exits when its last client disconnects, so an exited host
    means every client in the Job is gone; any other member, a host that is
    still running, or an unreadable list is a real leftover.
    """
    pids = job_pids(job)
    if pids is None:
        return False
    for pid in pids:
        value = member(pid, job)
        if value.get("open_error") == ERROR_INVALID_PARAMETER:
            continue  # left the Job between listing and opening
        if (pid, value.get("created_100ns")) not in hosts:
            return False
        if value.get("exit_code") in (None, STILL_ACTIVE):
            return False
    return True


class Evidence:
    def __init__(self, path: Path):
        self.path = path
        self.started = time.monotonic_ns()
        self.records: list[dict] = []
        self.identities: set[tuple[int, int]] = set()

    def observe(self, phase: str, pid: int, job: int) -> None:
        # Diagnostic errors must never prevent the owned Job's cleanup.
        try:
            accounting = Accounting()
            accounting_ok = kernel.QueryInformationJobObject(
                job, 1, ctypes.byref(accounting), ctypes.sizeof(accounting), None
            )
            accounting_error = None if accounting_ok else ctypes.get_last_error()
            ids = ProcessIds()
            ok = kernel.QueryInformationJobObject(
                job, 3, ctypes.byref(ids), ctypes.sizeof(ids), None
            )
            list_error = None if ok else ctypes.get_last_error()
            members = (
                [member(int(value), job) for value in ids.ids[: min(ids.listed, 128)]]
                if ok
                else []
            )
            snapshot = {}
            snapshot_error = None
            if phase == "before_stop" and members:
                try:
                    snapshot = system_processes()
                except OSError as error:
                    snapshot_error = str(error)
            for value in members:
                created = value.get("created_100ns")
                if created is not None:
                    self.identities.add((value["pid"], created))
                extra = snapshot.get(value["pid"])
                if extra and extra["created_100ns"] == created:
                    value["system_snapshot"] = extra
            prior = []
            if phase == "stopped":
                for identity, created in sorted(self.identities):
                    current = member(identity, job)
                    prior.append(
                        {
                            "pid": identity,
                            "expected_created_100ns": created,
                            "same_identity": current.get("created_100ns") == created,
                            "current": current,
                        }
                    )
            self.records.append(
                {
                    "schema": "windows-job-v2",
                    "phase": phase,
                    "elapsed_us": (time.monotonic_ns() - self.started) // 1000,
                    "owner_pid": os.getpid(),
                    "root_pid": pid,
                    "job_handle": hex(job),
                    "root": member(pid, job),
                    "members": members,
                    "accounting_error": accounting_error,
                    "accounting": {
                        "active": accounting.active,
                        "total": accounting.total,
                        "terminated_by_limit": accounting.terminated,
                        "times_100ns": list(accounting.times),
                    },
                    "assigned": ids.assigned,
                    "listed": ids.listed,
                    "list_error": list_error,
                    "list_complete": bool(ok)
                    and ids.assigned == ids.listed
                    and ids.listed <= 128,
                    "system_snapshot_error": snapshot_error,
                    "previously_observed": prior,
                }
            )
        except (OSError, ValueError) as error:
            self.records.append({"phase": phase, "observation_error": str(error)})

    def finish(self, status: str, code: int | None, reclaimed: bool) -> None:
        if status == "exited" and code == 0 and not reclaimed:
            return
        self.records.append(
            {
                "schema": "windows-job-v2",
                "phase": "result",
                "status": status,
                "exit_code": code,
                "descendants_reclaimed": reclaimed,
            }
        )
        try:
            data = "".join(json.dumps(record) + "\n" for record in self.records)
            self.path.write_text(data, encoding="utf-8")
            if directory := os.environ.get("AUTOSAR_JOB_EVIDENCE_DIRECTORY"):
                target = Path(directory)
                target.mkdir(parents=True, exist_ok=True)
                (target / f"python-{os.getpid()}-{self.started}.jsonl").write_text(
                    data, encoding="utf-8"
                )
        except OSError as error:
            print(
                f"Windows Job evidence write failed path={self.path}: {error}",
                file=sys.stderr,
            )


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
        self.evidence = Evidence(stderr.with_name("windows-job.jsonl"))
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
            self.evidence.observe("assigned", self.pid, job)
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
        hosts: set[tuple[int, int]] = set()
        while time.monotonic_ns() < deadline_ns:
            code = self.poll()
            try:
                if code is None:
                    # The host may lose its image path before poll sees the root exit.
                    hosts.update(console_hosts(self.job))
                else:
                    orphaned = self._orphaned_after_root_exit(deadline_ns, hosts)
            except BaseException:
                # A classification failure must not skip the Job's reclaim.
                self.close()
                raise
            if code is not None:
                self.close()
                status = (
                    "orphaned_members"
                    if orphaned and not close_tree_on_exit
                    else "exited"
                )
                self.evidence.finish(status, code, orphaned)
                return status, code, orphaned
            time.sleep(0.01)
        self.close()
        self.evidence.finish("timeout", self.poll(), False)
        return "timeout", self.poll(), False

    def _orphaned_after_root_exit(
        self, deadline_ns: int, hosts: set[tuple[int, int]]
    ) -> bool:
        # The active count can lag a reaped cooperative exit briefly.
        grace = min(deadline_ns, time.monotonic_ns() + GRACE_NS)
        hosts.update(console_hosts(self.job))
        self.evidence.observe("root_exited", self.pid, self.job)
        while self._active() and time.monotonic_ns() < grace:
            hosts |= console_hosts(self.job)
            time.sleep(0.01)
        return bool(self._active()) and not self._hosts_finished(hosts, deadline_ns)

    def _hosts_finished(self, hosts: set[tuple[int, int]], deadline_ns: int) -> bool:
        """Wait for exited console hosts to leave the Job; False on any leftover.

        Every observation is judged against the limit, so a Job that is first seen
        empty after the limit still fails.
        """
        limit = min(deadline_ns, time.monotonic_ns() + CONSOLE_HOST_TEARDOWN_NS)
        while True:
            active = self._active()
            if time.monotonic_ns() >= limit:
                return False
            if not active:
                return True
            if not exited_console_hosts_only(self.job, hosts):
                return False
            remaining = limit - time.monotonic_ns()
            if remaining <= 0:
                return False
            time.sleep(min(0.01, remaining / 1_000_000_000))

    def close(self) -> None:
        if self.closed:
            return
        self.closed = True
        cleanup_completed = False
        try:
            # A failed assignment leaves a suspended process outside the Job.
            # It is still ours and must be terminated through its own handle.
            if self.child is not None and not self.assigned:
                self.child.kill()
            if self.child is not None and self.assigned:
                self.evidence.observe("before_stop", self.pid, self.job)
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
                self.evidence.observe("stopped", self.pid, self.job)
            cleanup_completed = True
        finally:
            if not cleanup_completed:
                self.evidence.finish("cleanup_unconfirmed", self.poll(), False)
            kernel.CloseHandle(self.job)
