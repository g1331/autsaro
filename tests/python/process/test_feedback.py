"""Daily feedback keeps deadlines and failure evidence through normal tool entries."""

import contextlib
import io
import json
import platform
import sys
import tempfile
import time
import unittest
from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from autosar_tooling import assets, c_check, quality
from ecu_tools import build as ecu_build
from ecu_tools import cli
from ecu_tools.owner import OwnershipError
from ecu_tools.process import ProcessResult, ProcessSpec, run_bounded


class FeedbackTests(unittest.TestCase):
    def hung(self, spec, **kwargs):
        # Keep the production entry and tree owner, shorten only this test's deadline.
        self.assertGreater(spec.deadline_ns, time.monotonic_ns())
        self.assertLessEqual(spec.deadline_ns - time.monotonic_ns(), 30_000_000_000)
        self.commands.append(spec.argv)
        return run_bounded(replace(
            spec, argv=(sys.executable, "-c",
                        ("import sys,time;print('feedback stdout',flush=True);"
                        "print('feedback stderr',file=sys.stderr,flush=True);time.sleep(60)")),
            deadline_ns=time.monotonic_ns() + 500_000_000,
        ), **kwargs)

    def assert_failure(self, operation, stage):
        self.commands = []
        started = time.monotonic()
        with self.assertRaises(OwnershipError) as failure:
            operation()
        self.assertLess(time.monotonic() - started, 10)
        message = str(failure.exception)
        for text in (stage, "argv=", "status=timeout", "exit_code=", "stdout:\nfeedback stdout", "stderr:\nfeedback stderr"):
            self.assertIn(text, message)
        self.assertTrue(self.commands)

    def test_asset_git_queries_use_deadline_and_retain_stage_command_and_output(self):
        for stage in ("tracked-files", "line-ending-attributes"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                paths = root / "paths"
                paths.write_bytes(b"main.py\0")
                def execute(spec, stage=stage, paths=paths):
                    if stage in spec.stage:
                        return self.hung(spec)
                    return SimpleNamespace(stdout=paths)
                with patch.object(assets, "run_bounded", side_effect=execute):
                    self.assert_failure(lambda root=root: assets.check_line_endings(root), stage)
                self.assertIn("git", Path(self.commands[0][0]).stem)

    def test_quality_kernel_patch_has_deadline_and_sample_stage(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary) / "sample"
            (project / "meta").mkdir(parents=True)
            (project / "kernel").mkdir()
            (project / "target.json").write_text(json.dumps({
                "target": "linux-x64-controlled-v1",
                "kernelPatches": [{"path": "runtime/patch.diff", "includes": []}],
            }))
            (project / "meta/integration.json").write_text('{"originalBswSources":{}}')
            with patch.object(quality, "run_bounded", side_effect=self.hung):
                self.assert_failure(lambda: quality.clang_tidy_project(project, "unused"), "sample: kernel-patch")
            self.assertEqual(self.commands[0][1:4], ("-c", "core.autocrlf=false", "apply"))

    def test_c_analysis_timeout_and_nonzero_exit_keep_raw_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with patch.object(c_check, "run_bounded", side_effect=self.hung):
                self.assert_failure(lambda: c_check.run([sys.executable, "--version"], root, "compiler-unit-7", 30), "compiler-unit-7")
            with self.assertRaisesRegex(OwnershipError, "exit_code=17") as failure:
                c_check.run([sys.executable, "-c", "import sys;print('broken unit',file=sys.stderr);sys.exit(17)"],
                            root, "compiler-unit-8", 30)
            self.assertIn("stderr:\nbroken unit", str(failure.exception))
            self.assertIn("compiler-unit-8", str(failure.exception))

    def test_expected_analyzer_exit_does_not_accept_hangs_or_other_failures(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            code, _, _ = c_check.run([sys.executable, "-c", "raise SystemExit(1)"], root, "cppcheck", 30)
            self.assertEqual(code, 1)
            with self.assertRaisesRegex(OwnershipError, "exit_code=2"):
                c_check.run([sys.executable, "-c", "raise SystemExit(2)"], root, "cppcheck", 30)
            self.commands = []
            spec = ProcessSpec.seconds([sys.executable, "-c", "import time;time.sleep(60)"], root, 30, root, "cppcheck")
            with self.assertRaisesRegex(OwnershipError, "status=timeout"):
                run_bounded(replace(spec, deadline_ns=time.monotonic_ns() + 500_000_000), allowed_exit_codes=(0, 1))


class BuildPatchFeedbackTests(unittest.TestCase):
    @contextlib.contextmanager
    def project(self, root):
        project = root / "project"
        (project / "kernel").mkdir(parents=True)
        (project / "kernel/selected.h").write_bytes(b"before\n")
        (project / "change.patch").write_bytes(
            b"--- a/selected.h\n+++ b/selected.h\n@@ -1 +1 @@\n-before\n+after\n")
        target = {"target": next(key for key, host in ecu_build.TARGETS.items() if host == platform.system()),
                  "profile": "ecu", "kernelPatches": [{"path": "runtime/change.patch", "includes": ["selected.h"]}]}
        # Isolate the patch boundary; real output guards, copy, command owner and CLI remain in use.
        with (patch.object(ecu_build, "sealed_sources", return_value=(["change.patch"], target)),
              patch.object(ecu_build, "target_configuration", return_value=([], [], [])),
              patch.object(ecu_build, "pinned_tools", return_value=(Path(sys.executable),) * 3)):
            yield project

    def invoke(self, project, output):
        error = io.StringIO()
        with (patch.object(sys, "argv", ["ecu-tool", "build", "--project", str(project),
                                        "--output", str(output), "--mode", "host-batch"]),
              contextlib.redirect_stderr(error)):
            self.assertEqual(cli.main(), 1)
        return error.getvalue()

    def test_patch_phases_report_actual_result_even_with_empty_logs(self):
        for phase in ("check", "apply"):
            for status, code, reclaimed in (("exited", 17, False), ("timeout", 0, False),
                                             ("orphaned_members", 0, True), ("cleanup_unconfirmed", 0, False)):
                with self.subTest(phase=phase, status=status), tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary).resolve()
                    seen = []
                    def execute(spec, *, seen=seen, phase=phase, code=code, status=status, reclaimed=reclaimed, **kwargs):
                        seen.append(spec)
                        remaining = spec.deadline_ns - time.monotonic_ns()
                        self.assertGreater(remaining, 29_000_000_000)
                        self.assertLessEqual(remaining, 30_000_000_000)
                        stdout, stderr = spec.log_directory / "out", spec.log_directory / "err"
                        stdout.write_bytes(b"")
                        stderr.write_bytes(b"")
                        failed = ("--check" in spec.argv) == (phase == "check")
                        result = ProcessResult("patch-scope", 123, None, code if failed else 0,
                                               status if failed else "exited", stdout, stderr,
                                               reclaimed if failed else False)
                        with patch("ecu_tools.process.OwnedProcess", return_value=SimpleNamespace(wait=lambda: result)):
                            return run_bounded(spec, **kwargs)
                    with self.project(root) as project, patch.object(ecu_build, "run_bounded", side_effect=execute):
                        message = self.invoke(project, root / "output")
                    for text in (f"Kernel patch change.patch ({phase})", f"status={status}", f"exit_code={code}",
                                 "argv=", "scope=patch-scope", "pid=123", "pgid=None",
                                 f"descendants_reclaimed={reclaimed}", "stdout:\n\nstderr:\n"):
                        self.assertIn(text, message)
                    self.assertEqual(len(seen), 1 if phase == "check" else 2)
                    self.assertEqual("--check" in seen[-1].argv, phase == "check")
                    self.assertEqual(seen[-1].cwd, root / "output/kernel")
                    self.assertIn("--include=selected.h", seen[-1].argv)
                    self.assertEqual((project / "kernel/selected.h").read_bytes(), b"before\n")

    def test_real_patch_command_failure_and_timeout_reach_cli_with_output(self):
        for hung in (False, True):
            with self.subTest(hung=hung), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary).resolve()
                def execute(spec, hung=hung):
                    code = ("import sys,time;print('patch stdout',flush=True);"
                            "print('patch stderr',file=sys.stderr,flush=True);"
                            + ("time.sleep(60)" if hung else "sys.exit(23)"))
                    return run_bounded(replace(spec, argv=(sys.executable, "-c", code),
                                              deadline_ns=time.monotonic_ns() + (500_000_000 if hung else 30_000_000_000)))
                with self.project(root) as project, patch.object(ecu_build, "run_bounded", side_effect=execute):
                    message = self.invoke(project, root / "output")
                for text in ("Kernel patch change.patch (check)", "stdout:\npatch stdout", "stderr:\npatch stderr",
                             "status=timeout" if hung else "status=exited", "argv=", "descendants_reclaimed="):
                    self.assertIn(text, message)
                if not hung:
                    self.assertIn("exit_code=23", message)
