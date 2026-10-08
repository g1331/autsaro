"""Real host C fault injection, without adding hooks to delivery code."""
from __future__ import annotations

import os
import shutil
import tempfile
import unittest
from pathlib import Path

from autosar_tooling.config import ROOT
from ecu_tools.process import OwnedProcess, ProcessSpec


class HostBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory()
        cls.directory = Path(cls.temporary.name)
        requested = os.environ.get("AUTOSAR_CC") or shutil.which("gcc")
        if not requested or not Path(requested).is_file():
            raise RuntimeError("Host boundary tests require the native GCC")
        cls.compiler = str(Path(requested).resolve())
        cls.flags = ["-std=c99", "-O1", "-Wall", "-Wextra", "-Werror", "-pedantic", "-D_XOPEN_SOURCE=700",
                     "-I" + str(ROOT / "runtime/include"), "-I" + str(ROOT / "runtime/src"),
                     "-I" + str(ROOT / "runtime/host/include")]
        suffix = ".exe" if os.name == "nt" else ""
        cls.lock = cls.directory / ("lock" + suffix)
        cls.storage = cls.directory / ("storage" + suffix)
        for name, module in [("lock", "Can_HostLock.c"), ("storage", "NvM_HostStorage.c")]:
            header = ROOT / "tests/fixtures" / ("host_" + ("lock" if name == "lock" else "storage") + "_faults.h")
            obj = cls.directory / (name + ".o")
            result = cls.execute([cls.compiler, *cls.flags, "-include", str(header), "-c",
                                  str(ROOT / "runtime/src" / module), "-o", str(obj)], "compile-" + name)
            if not result.success:
                raise RuntimeError(result.stderr.read_text(encoding="utf-8", errors="replace"))
            fixture = ROOT / "tests/fixtures" / ("host_" + name + "_faults.c")
            sources = [ROOT / "runtime/src/Can.c", ROOT / "runtime/host/src/Can_Execution.c"] if name == "lock" else [ROOT / "runtime/src/NvM.c"]
            result = cls.execute([cls.compiler, *cls.flags, str(obj), str(fixture), *(str(path) for path in sources),
                                  *([] if os.name == "nt" else ["-pthread"]), "-o", str(getattr(cls, name))], "link-" + name)
            if not result.success:
                raise RuntimeError(result.stderr.read_text(encoding="utf-8", errors="replace"))

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    @classmethod
    def execute(cls, argv, stage):
        return OwnedProcess(ProcessSpec.seconds(argv, cls.directory, 60, cls.directory, stage)).wait()

    def test_recursive_lock_and_busy_are_normal_states(self):
        for case in ("normal", "busy"):
            with self.subTest(case=case):
                result = self.execute([str(self.lock), case], case)
                self.assertTrue(result.success, result.stdout.read_text())

    def test_try_lock_errors_reach_can_error_return(self):
        cases = ["once", "mutex-init"] if os.name == "nt" else ["once", "attr-init", "attr-type", "mutex-init", "attr-close", "try-error"]
        for case in cases:
            with self.subTest(case=case):
                result = self.execute([str(self.lock), case], case)
                self.assertTrue(result.success, result.stdout.read_text())

    def test_void_lock_failures_select_fatal_policy_and_never_return(self):
        cases = ["once", "mutex-init"] if os.name == "nt" else ["once", "attr-init", "attr-type", "mutex-init", "attr-close", "lock", "unlock"]
        for case in cases:
            with self.subTest(case=case):
                result = self.execute([str(self.lock), case, "fatal"], case)
                output = result.stdout.read_text()
                self.assertEqual(result.exit_code, 91, output)
                self.assertIn("LOCK_FATAL", output)
                if case in ("attr-type", "mutex-init") and os.name != "nt":
                    self.assertIn("attrs=1", output)
                if case == "attr-close":
                    self.assertIn("attrs=1 mutexes=1", output)

    def test_close_failure_invalidates_stream_and_prevents_false_reopen_success(self):
        for case in ("close", "reopen", "init"):
            with self.subTest(case=case):
                path = self.directory / (case + ".bin")
                result = self.execute([str(self.storage), case, str(path)], case)
                self.assertTrue(result.success, result.stdout.read_text())
