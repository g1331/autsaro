"""Real parent/child/grandchild probes for bounded process ownership."""

from __future__ import annotations

import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from dataclasses import replace
from pathlib import Path
from unittest.mock import patch

from ecu_tools.owner import Owner, OwnershipError
from ecu_tools.process import CompletionPolicy, OwnedProcess, ProcessSpec, run_bounded

ROOT = Path(__file__).resolve().parents[2]


def _records(path: Path, count: int) -> list[tuple[str, int, int]]:
    end = time.monotonic() + 5
    while time.monotonic() < end:
        if path.exists():
            records = [
                (parts[0], int(parts[1]), int(parts[2]))
                for line in path.read_text(encoding="ascii").splitlines()
                if len(parts := line.split()) == 3
            ]
            if len(records) >= count:
                return records
        time.sleep(0.01)
    raise AssertionError(
        f"only {path.read_text() if path.exists() else 'no'} PID records"
    )


def _gone(pid: int) -> bool:
    if os.name == "nt":
        import ctypes
        from ctypes import wintypes

        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        kernel.WaitForSingleObject.restype = wintypes.DWORD
        handle = kernel.OpenProcess(0x101000, False, pid)  # SYNCHRONIZE | QUERY_LIMITED_INFORMATION
        if not handle:
            return True
        try:
            state = kernel.WaitForSingleObject(handle, 0)
            if state == 0xFFFFFFFF:
                raise ctypes.WinError(ctypes.get_last_error())
            return state == 0
        finally:
            kernel.CloseHandle(handle)
    if sys.platform == "linux":
        status = Path(f"/proc/{pid}/stat")
        try:
            # A nested owner root may leave a stopped child to its outer
            # subreaper; a zombie cannot execute and is reaped at root close.
            if status.read_text().rsplit(")", 1)[-1].split()[0] == "Z":
                return True
        except FileNotFoundError:
            return True
    try:
        os.kill(pid, 0)
        return False
    except ProcessLookupError:
        return True


def _assert_closed(
    test: unittest.TestCase, records: list[tuple[str, int, int]]
) -> None:
    end = time.monotonic() + 2
    for role, pid, _ in records:
        while not _gone(pid) and time.monotonic() < end:
            time.sleep(0.01)
        test.assertTrue(_gone(pid), f"{role} PID {pid} survived owned closure")


class BoundedProcessTests(unittest.TestCase):
    def _spec(
        self, root: Path, kind: str, seconds: int = 10
    ) -> tuple[ProcessSpec, Path]:
        pid_file = root / f"{kind}.pids"
        spec = ProcessSpec.seconds(
            [
                sys.executable,
                "-m",
                "autosar_tooling",
                "probe",
                "descendant",
                "--pid-file",
                str(pid_file),
                "--kind",
                kind,
            ],
            ROOT,
            seconds,
            root,
            f"probe:{kind}",
        )
        return spec, pid_file

    def test_normal_child_and_grandchild_close(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            spec, path = self._spec(Path(name), "normal")
            result = run_bounded(spec)
            self.assertTrue(result.success)
            _assert_closed(self, _records(path, 3))

    def test_timeout_closes_registered_group_and_keeps_logs(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            spec, path = self._spec(Path(name), "hang", 1)
            with self.assertRaisesRegex(OwnershipError, "timeout") as failure:
                run_bounded(spec)
            records = _records(path, 3)
            _assert_closed(self, records)
            self.assertIn("stdout=", str(failure.exception))
            self.assertIn("stderr=", str(failure.exception))

    @unittest.skipUnless(os.name == "nt", "Windows suspended Job assignment")
    def test_failed_job_assignment_closes_unstarted_process(self) -> None:
        from ecu_tools import windows_job

        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            spec, pid_file = self._spec(root, "normal")
            created: list[subprocess.Popen[bytes]] = []
            spawn = subprocess.Popen
            assign = windows_job.kernel.AssignProcessToJobObject

            def record_spawn(*args, **kwargs):
                child = spawn(*args, **kwargs)
                created.append(child)
                return child

            def invalid_assignment(job, process):
                return assign(None, process)

            try:
                with (
                    patch.object(windows_job.subprocess, "Popen", record_spawn),
                    patch.object(
                        windows_job.kernel,
                        "AssignProcessToJobObject",
                        invalid_assignment,
                    ),
                    self.assertRaises(OSError),
                ):
                    windows_job.WindowsJob(
                        list(spec.argv), spec.cwd, spec.env,
                        root / "stdout", root / "stderr",
                    )
                self.assertEqual(len(created), 1)
                self.assertIsNotNone(created[0].poll())
                _assert_closed(self, [("unstarted", created[0].pid, os.getpid())])
                self.assertFalse(pid_file.exists(), "Unassigned command executed")
            finally:
                for child in created:
                    if child.poll() is None:
                        child.kill()
                    child.wait(timeout=5)

    def test_normal_parent_exit_with_live_children_is_failure(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            spec, path = self._spec(Path(name), "parent-first")
            with self.assertRaisesRegex(OwnershipError, "orphaned_members"):
                run_bounded(spec)
            _assert_closed(self, _records(path, 3))

    def test_tool_completion_reclaims_tree_without_changing_root_exit(self) -> None:
        for kind, code in (("parent-first", 0), ("parent-first-fail", 7)):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as name:
                spec, path = self._spec(Path(name), kind)
                spec = replace(spec, completion=CompletionPolicy.CLOSE_TREE_ON_EXIT)
                result = OwnedProcess(spec).wait()
                self.assertEqual(result.status, "exited")
                self.assertEqual(result.exit_code, code)
                self.assertEqual(result.success, code == 0)
                self.assertTrue(result.descendants_reclaimed)
                _assert_closed(self, _records(path, 3))
                if code:
                    self.assertIn(
                        "probe failure with live descendants", result.stderr.read_text()
                    )

    def test_tool_reclamation_does_not_close_live_sibling(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            owner = None if os.name == "nt" else Owner.start()
            try:
                sibling_spec, sibling_path = self._spec(root, "hang", 15)
                sibling = OwnedProcess(sibling_spec, owner=owner)
                sibling_records = _records(sibling_path, 3)
                try:
                    spec, path = self._spec(root, "parent-first")
                    result = OwnedProcess(
                        replace(spec, completion=CompletionPolicy.CLOSE_TREE_ON_EXIT),
                        owner=owner,
                    ).wait()
                    self.assertTrue(result.success)
                    _assert_closed(self, _records(path, 3))
                    self.assertTrue(all(not _gone(pid) for _, pid, _ in sibling_records))
                finally:
                    self.assertEqual(sibling.cancel().status, "cancelled")
                    _assert_closed(self, sibling_records)
            finally:
                if owner is not None:
                    owner.close()

    def test_tool_completion_timeout_still_fails_and_closes_tree(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            spec, path = self._spec(Path(name), "hang", 2)
            result = OwnedProcess(
                replace(spec, completion=CompletionPolicy.CLOSE_TREE_ON_EXIT)
            ).wait()
            self.assertEqual(result.status, "timeout")
            self.assertFalse(result.success)
            _assert_closed(self, _records(path, 3))

    @unittest.skipIf(os.name == "nt", "POSIX registered scopes have independent groups")
    def test_root_exit_closes_unfinished_nested_scope_under_both_policies(self) -> None:
        for completion in CompletionPolicy:
            with self.subTest(completion=completion), tempfile.TemporaryDirectory() as name:
                spec, path = self._spec(Path(name), "nested-parent-first")
                result = OwnedProcess(replace(spec, completion=completion)).wait()
                expected = (
                    "exited"
                    if completion is CompletionPolicy.CLOSE_TREE_ON_EXIT
                    else "orphaned_members"
                )
                self.assertEqual(result.status, expected)
                self.assertEqual(result.exit_code, 0)
                self.assertTrue(result.descendants_reclaimed)
                records = _records(path, 4)
                self.assertNotEqual(records[0][2], records[1][2])
                _assert_closed(self, records)

    def test_nonzero_exit_retains_stderr_and_code(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            spec, _ = self._spec(Path(name), "fail")
            with self.assertRaisesRegex(OwnershipError, "exit_code=7") as failure:
                run_bounded(spec)
            stderr = Path(str(failure.exception).split("stderr=")[-1])
            self.assertIn("probe failure on stderr", stderr.read_text(encoding="utf-8"))

    @unittest.skipIf(os.name == "nt", "POSIX supervisor owns independent root scopes")
    def test_explicit_owner_ignores_unrelated_inherited_scope(self) -> None:
        with tempfile.TemporaryDirectory() as name, Owner.start() as owner:
            spec, path = self._spec(Path(name), "normal")
            with patch.dict(
                os.environ,
                {"ECU_OWNER_SOCKET": "/tmp/unrelated-owner.sock", "ECU_OWNER_SCOPE": "unrelated"},
            ):
                result = OwnedProcess(spec, owner=owner).wait()
            self.assertTrue(result.success)
            _assert_closed(self, _records(path, 3))

    def test_cancel_one_scope_does_not_close_another(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            if os.name == "nt":
                first, path = self._spec(root, "hang")
                process = OwnedProcess(first)
                records = _records(path, 3)
                self.assertEqual(process.cancel().status, "cancelled")
                _assert_closed(self, records)
                self.assertTrue(run_bounded(self._spec(root, "normal")[0]).success)
            else:
                with Owner.start() as owner:
                    first, path = self._spec(root, "hang")
                    process = OwnedProcess(first, owner=owner)
                    records = _records(path, 3)
                    sibling, _ = self._spec(root, "normal")
                    second = OwnedProcess(sibling, owner=owner)
                    self.assertEqual(process.cancel().status, "cancelled")
                    _assert_closed(self, records)
                    self.assertTrue(second.wait().success)

    @unittest.skipIf(
        os.name == "nt", "POSIX supervisor has the subreaper/escape contract"
    )
    def test_unregistered_escape_is_explicitly_unconfirmed(self) -> None:
        for completion in CompletionPolicy:
            with self.subTest(completion=completion), tempfile.TemporaryDirectory() as name:
                spec, path = self._spec(Path(name), "escape")
                try:
                    with self.assertRaisesRegex(OwnershipError, "cleanup_unconfirmed"):
                        run_bounded(replace(spec, completion=completion))
                finally:
                    for role, pid, _ in _records(path, 2):
                        if role == "escape":
                            os.kill(
                                pid, signal.SIGKILL
                            )  # Fixture owns this non-cooperative PID.

    @unittest.skipIf(
        os.name == "nt", "nested Unix scopes require the supervisor socket"
    )
    def test_nested_scope_runs_under_same_root(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            spec, path = self._spec(Path(name), "nested")
            result = run_bounded(spec)
            self.assertTrue(result.success)
            records = _records(path, 2)
            self.assertNotEqual(records[0][2], records[1][2])
            _assert_closed(self, records)

    @unittest.skipIf(os.name == "nt", "guardian death is managed by the POSIX root")
    def test_guardian_death_closes_supervised_group(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            pid_file = root / "guardian.pids"
            guardian = subprocess.Popen(
                [
                    sys.executable,
                    "-m",
                    "autosar_tooling",
                    "probe",
                    "descendant",
                    "--pid-file",
                    str(pid_file),
                    "--kind",
                    "guardian",
                ],
                cwd=ROOT,
                env={**os.environ, "TMPDIR": name},
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
            try:
                records = _records(pid_file, 4)
                self.assertEqual(records[-1][0], "guardian")
                guardian.kill()
                guardian.wait(timeout=3)
                _assert_closed(self, records[:-1])
            finally:
                if guardian.poll() is None:
                    guardian.kill()
                    guardian.wait(timeout=3)

    @unittest.skipIf(
        os.name == "nt", "only the POSIX root can die independently of the guardian"
    )
    def test_supervisor_failure_closes_guardian_mirror(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            owner = Owner.start()
            try:
                spec, path = self._spec(Path(name), "hang")
                process = OwnedProcess(spec, owner=owner)
                records = _records(path, 3)
                assert owner.process is not None
                owner.process.kill()
                owner.process.wait(timeout=3)
                with self.assertRaisesRegex(OwnershipError, "supervisor_failed"):
                    process.wait()
                _assert_closed(self, records)
            finally:
                try:
                    owner.close()
                except OwnershipError:
                    pass
                if owner.directory is not None:
                    shutil.rmtree(owner.directory)

    @unittest.skipIf(
        os.name == "nt",
        "POSIX registration gate is distinct from Windows suspended assignment",
    )
    def test_failed_registration_never_runs_a_command(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            public = root / "public"
            public.mkdir(mode=0o755)
            spec, path = self._spec(public, "normal")
            with self.assertRaisesRegex(OwnershipError, "private"):
                run_bounded(spec)
            self.assertFalse(path.exists())

    @unittest.skipIf(os.name == "nt", "POSIX release is gated by the supervisor")
    def test_failed_release_closes_registered_but_unstarted_scope(self) -> None:
        with tempfile.TemporaryDirectory() as name, Owner.start() as owner:
            spec, path = self._spec(Path(name), "normal")
            actual_request = owner.request
            scopes: list[str] = []

            def fail_release(op: str, **fields: object) -> dict[str, object]:
                if op == "release":
                    raise OwnershipError("injected release failure")
                result = actual_request(op, **fields)
                if op == "reserve":
                    scopes.append(str(result["scope"]))
                return result

            owner.request = fail_release  # type: ignore[method-assign]
            with self.assertRaisesRegex(OwnershipError, "injected release failure"):
                OwnedProcess(spec, owner=owner)
            result = actual_request("closed", scope=scopes[0])
            self.assertFalse(owner.groups)
            self.assertEqual(result["status"], "cancelled")
            self.assertFalse(path.exists())


if __name__ == "__main__":
    unittest.main()
