"""Console hosts that finish teardown after their clients must not read as orphans.

The first class drives `WindowsJob.wait` with a simulated Job and clock, so the
decision rules run on every platform. The second class checks the identification
against a real Windows console host.
"""

from __future__ import annotations

import os
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from ecu_tools import windows_job

MS = 1_000_000
SYSTEM_HOST = r"C:\Windows\System32\conhost.exe"


class SimulatedJob:
    """Members that leave the Job at given times, on a clock advanced by sleep."""

    def __init__(self) -> None:
        self.now = 0
        self.members: dict[int, dict] = {}
        self.closed = 0
        self.sleeps = 0
        # Each member query after this time costs `slow_query_ms` of clock.
        self.slow_after_ns = 10**18
        self.slow_query_ns = 0
        # Each sleep after this time overshoots by `stall_ns`.
        self.stall_after_ns = 10**18
        self.stall_ns = 0

    def add(
        self,
        pid: int,
        image: str,
        *,
        exit_code: int,
        leaves_ms: float | None,
        image_readable_ms: float = 10_000,
        created: int | None = None,
    ) -> None:
        self.members[pid] = {
            "image": image,
            "exit_code": exit_code,
            "leaves": None if leaves_ms is None else int(leaves_ms * MS),
            "readable": int(image_readable_ms * MS),
            "created": created if created is not None else 1000 + pid,
        }

    def present(self) -> dict[int, dict]:
        return {
            pid: value
            for pid, value in self.members.items()
            if value["leaves"] is None or value["leaves"] > self.now
        }

    def monotonic_ns(self) -> int:
        return self.now

    def sleep(self, seconds: float) -> None:
        assert seconds >= 0, "time.sleep rejects negative durations"
        self.sleeps += 1
        assert self.sleeps < 5_000, "wait did not terminate"
        if self.now >= self.stall_after_ns:
            self.now += self.stall_ns
        self.now += int(seconds * 1_000_000_000)
        # A mutated wait that never ends must fail instead of hanging.
        assert self.now < 30_000 * MS, "wait did not terminate"

    def pids(self, job: int) -> list[int]:
        return list(self.present())

    def member(self, pid: int, job: int) -> dict:
        if self.now >= self.slow_after_ns:
            self.now += self.slow_query_ns
        value = self.present().get(pid)
        if value is None:
            return {"pid": pid, "open_error": windows_job.ERROR_INVALID_PARAMETER}
        readable = self.now < value["readable"]
        return {
            "pid": pid,
            "created_100ns": value["created"],
            "image": value["image"] if readable else None,
            "image_error": None if readable else 5,
            "exit_code": value["exit_code"],
        }

    def run(
        self,
        *,
        deadline_ms: float = 60_000,
        observe_ms: float = 0,
        close_tree_on_exit: bool = False,
        system_host: str | None = SYSTEM_HOST.lower(),
    ):
        root = windows_job.WindowsJob.__new__(windows_job.WindowsJob)
        root.job = 1
        root.child = SimpleNamespace(pid=100, poll=lambda: 0)
        root.evidence = SimpleNamespace(
            observe=lambda *args: self.sleep(observe_ms / 1000),
            finish=lambda *args: None,
        )
        clock = SimpleNamespace(
            monotonic_ns=self.monotonic_ns, sleep=self.sleep, monotonic=lambda: 0
        )
        with (
            patch.object(windows_job, "system_console_host", lambda: system_host),
            patch.object(windows_job, "time", clock),
            patch.object(windows_job, "job_pids", self.pids),
            patch.object(windows_job, "member", self.member),
            patch.object(
                windows_job.WindowsJob, "_active", lambda _: len(self.present())
            ),
            patch.object(
                windows_job.WindowsJob,
                "close",
                lambda _: setattr(self, "closed", self.closed + 1),
            ),
        ):
            return root.wait(
                int(deadline_ms * MS), close_tree_on_exit=close_tree_on_exit
            )


class ConsoleHostWaitTests(unittest.TestCase):
    def test_exited_host_finishing_after_the_grace_is_waited_out(self) -> None:
        # The observed failure: the host exited with its client but left the Job
        # 400 ms later, and its image became unreadable before then.
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=404, image_readable_ms=250)
        self.assertEqual(job.run(), ("exited", 0, False))
        self.assertGreaterEqual(job.now, 404 * MS)

    def test_slow_diagnostics_do_not_cost_the_host_its_identity(self) -> None:
        # The root-exit observation itself can stall for the whole grace while the
        # host's image is still readable only at the start.
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=404, image_readable_ms=250)
        self.assertEqual(job.run(observe_ms=300), ("exited", 0, False))

    def test_unreadable_system_directory_keeps_the_strict_rule_and_reclaims(self):
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=404)
        job.add(700, r"C:\Python\python.exe", exit_code=259, leaves_ms=None)
        self.assertEqual(job.run(system_host=None), ("orphaned_members", 0, True))
        self.assertLess(job.now, 300 * MS)
        self.assertEqual(job.closed, 1)

    def test_failure_while_classifying_still_closes_the_job(self) -> None:
        job = SimulatedJob()
        job.add(700, r"C:\Python\python.exe", exit_code=259, leaves_ms=None)

        def broken(pid: int, handle: int) -> dict:
            raise OSError("query failed")

        job.member = broken
        with self.assertRaises(OSError):
            job.run()
        self.assertEqual(job.closed, 1)

    def test_job_first_seen_empty_after_the_deadline_still_fails(self) -> None:
        # Deadline 303 ms, the exited host leaves at 304 ms: not a success.
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=304)
        self.assertEqual(job.run(deadline_ms=303), ("orphaned_members", 0, True))
        self.assertLessEqual(job.now, 303 * MS)

    def test_slow_observation_cannot_turn_an_overdue_job_into_success(self) -> None:
        # A member query stalls for 1 s past the 300 ms deadline; the host is gone
        # when the Job is read again, which is too late.
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=400)
        job.slow_after_ns = 200 * MS
        job.slow_query_ns = 1000 * MS
        self.assertEqual(job.run(deadline_ms=300), ("orphaned_members", 0, True))

    def test_stalled_sleep_cannot_turn_an_overdue_job_into_success(self) -> None:
        # The first pause after the grace overshoots the deadline by 1 s and the
        # host is gone when the Job is read next.
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=404)
        job.stall_after_ns = 200 * MS
        job.stall_ns = 1000 * MS
        self.assertEqual(job.run(deadline_ms=300), ("orphaned_members", 0, True))

    def test_empty_job_first_seen_after_the_bound_fails(self) -> None:
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=5_300)
        job.slow_after_ns = 4_000 * MS
        job.slow_query_ns = 2_000 * MS
        self.assertEqual(job.run(), ("orphaned_members", 0, True))

    def test_host_that_never_finishes_fails_at_the_bound(self) -> None:
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=None)
        self.assertEqual(job.run(), ("orphaned_members", 0, True))
        self.assertLess(job.now, 5_500 * MS)

    def test_empty_job_inside_the_grace_adds_no_wait(self) -> None:
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=50)
        self.assertEqual(job.run(), ("exited", 0, False))
        self.assertLess(job.now, 200 * MS)

    def test_live_descendant_fails_without_the_extra_wait(self) -> None:
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=404)
        job.add(700, r"C:\Python\python.exe", exit_code=259, leaves_ms=None)
        self.assertEqual(job.run(), ("orphaned_members", 0, True))
        self.assertLess(job.now, 300 * MS)

    def test_running_host_is_not_a_teardown(self) -> None:
        # A host that has not exited still serves a client that may be alive.
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=259, leaves_ms=404)
        self.assertEqual(job.run(), ("orphaned_members", 0, True))
        self.assertLess(job.now, 300 * MS)

    def test_only_the_system_console_host_is_excused(self) -> None:
        for image in (
            r"C:\Windows\System32\git.exe",
            r"C:\Temp\conhost.exe",
            r"C:\Windows\conhost.exe",
        ):
            with self.subTest(image=image):
                job = SimulatedJob()
                job.add(632, image, exit_code=0, leaves_ms=404)
                self.assertEqual(job.run(), ("orphaned_members", 0, True))
                self.assertLess(job.now, 300 * MS)

    def test_host_path_matches_case_insensitively(self) -> None:
        job = SimulatedJob()
        job.add(632, r"c:\WINDOWS\system32\CONHOST.EXE", exit_code=0, leaves_ms=404)
        self.assertEqual(job.run(), ("exited", 0, False))

    def test_host_identity_that_was_never_readable_stays_an_orphan(self) -> None:
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=404, image_readable_ms=0)
        self.assertEqual(job.run(), ("orphaned_members", 0, True))

    def test_reused_pid_is_not_the_recorded_host(self) -> None:
        reused = {"created_100ns": 3, "exit_code": 0}
        with (
            patch.object(windows_job, "job_pids", lambda _: [632]),
            patch.object(windows_job, "member", lambda pid, _: reused),
        ):
            self.assertFalse(windows_job.exited_console_hosts_only(1, {(632, 1)}))
            self.assertTrue(windows_job.exited_console_hosts_only(1, {(632, 3)}))

    def test_member_that_left_between_listing_and_opening_is_not_a_leftover(
        self,
    ) -> None:
        gone = {"open_error": windows_job.ERROR_INVALID_PARAMETER}
        exited = {"created_100ns": 3, "exit_code": 0}
        with (
            patch.object(windows_job, "job_pids", lambda _: [632, 700]),
            patch.object(
                windows_job, "member", lambda pid, _: exited if pid == 632 else gone
            ),
        ):
            self.assertTrue(windows_job.exited_console_hosts_only(1, {(632, 3)}))

    def test_unreadable_member_list_is_a_leftover(self) -> None:
        with patch.object(windows_job, "job_pids", lambda _: None):
            self.assertFalse(windows_job.exited_console_hosts_only(1, {(632, 3)}))

    def test_member_list_is_refused_unless_the_kernel_listing_is_complete(self):
        def listing(assigned: int, listed: int, ok: int = 1):
            def query(job, information, reference, size, returned):
                ids = reference._obj
                ids.assigned, ids.listed = assigned, listed
                for index in range(min(listed, len(ids.ids))):
                    ids.ids[index] = 500 + index
                return ok

            return SimpleNamespace(QueryInformationJobObject=query)

        cases = (
            (listing(2, 2), [500, 501]),
            (listing(0, 0), []),
            (listing(2, 1), None),  # more members than the list holds
            (listing(129, 129), None),  # more than the fixed buffer
            (listing(1, 1, ok=0), None),  # the query itself failed
        )
        for kernel, expected in cases:
            with (
                self.subTest(expected=expected),
                patch.object(windows_job, "kernel", kernel, create=True),
            ):
                self.assertEqual(windows_job.job_pids(1), expected)

    def test_system_directory_decides_the_host_path(self) -> None:
        def reading(text: str, length: int):
            def read(buffer, size):
                buffer.value = text
                return length

            return SimpleNamespace(GetSystemDirectoryW=read)

        cases = (
            (r"C:\Windows\System32", 19, r"c:\windows\system32\conhost.exe"),
            ("C:\\", 3, r"c:\conhost.exe"),  # a root directory keeps its backslash
            (r"C:\Windows\System32", 0, None),  # the call failed
            (r"C:\Windows\System32", 400, None),  # does not fit the buffer
        )
        for text, length, expected in cases:
            with self.subTest(length=length):
                kernel = reading(text, length)
                with patch.object(windows_job, "kernel", kernel, create=True):
                    self.assertEqual(windows_job.system_console_host(), expected)

    def test_accounting_grace_is_a_polling_window_judged_by_what_is_observed(self):
        # Pins the pre-existing rule: after a root exit inside its deadline, an
        # empty Job seen at the end of the grace polling passes, even a little
        # after the deadline; no absolute-deadline check applies to that path.
        job = SimulatedJob()
        job.add(700, r"C:\Python\python.exe", exit_code=259, leaves_ms=104)
        self.assertEqual(job.run(deadline_ms=103), ("exited", 0, False))
        self.assertGreater(job.now, 103 * MS)
        # A stalled pause lets the first look come late; it still finds an empty Job.
        job = SimulatedJob()
        job.add(700, r"C:\Python\python.exe", exit_code=259, leaves_ms=404)
        job.stall_after_ns = 0
        job.stall_ns = 1000 * MS
        self.assertEqual(job.run(deadline_ms=103), ("exited", 0, False))
        self.assertGreaterEqual(job.now, 1000 * MS)

    def test_a_member_still_present_when_the_grace_ends_is_an_orphan(self) -> None:
        job = SimulatedJob()
        job.add(700, r"C:\Python\python.exe", exit_code=259, leaves_ms=150)
        self.assertEqual(job.run(), ("exited", 0, False))
        job = SimulatedJob()
        job.add(700, r"C:\Python\python.exe", exit_code=259, leaves_ms=250)
        self.assertEqual(job.run(), ("orphaned_members", 0, True))

    def test_command_deadline_is_not_extended(self) -> None:
        job = SimulatedJob()
        job.add(632, SYSTEM_HOST, exit_code=0, leaves_ms=1000)
        self.assertEqual(job.run(deadline_ms=300), ("orphaned_members", 0, True))
        self.assertLessEqual(job.now, 320 * MS)

    def test_close_policy_still_reports_the_status_as_exited(self) -> None:
        job = SimulatedJob()
        job.add(700, r"C:\Python\python.exe", exit_code=259, leaves_ms=None)
        self.assertEqual(job.run(close_tree_on_exit=True), ("exited", 0, True))


@unittest.skipUnless(os.name == "nt", "Windows console host identification")
class RealConsoleHostTests(unittest.TestCase):
    def test_real_console_host_is_recognised_while_its_client_runs(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            job = windows_job.WindowsJob(
                [sys.executable, "-I", "-S", "-c", "import time; time.sleep(30)"],
                root,
                None,
                root / "stdout.log",
                root / "stderr.log",
            )
            try:
                end = time.monotonic() + 8
                hosts = windows_job.console_hosts(job.job)
                while not hosts and time.monotonic() < end:
                    time.sleep(0.01)
                    hosts = windows_job.console_hosts(job.job)
                self.assertTrue(hosts, "no console host appeared in the Job")
                # The client still runs, so more than exited hosts remain.
                self.assertFalse(windows_job.exited_console_hosts_only(job.job, hosts))
            finally:
                job.close()
                if job.child is not None:
                    job.child.kill()
                    job.child.wait(timeout=5)
