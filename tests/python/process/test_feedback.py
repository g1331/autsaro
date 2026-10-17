"""Daily feedback keeps deadlines and failure evidence through normal tool entries."""

import json
import sys
import tempfile
import time
import unittest
from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from autosar_tooling import assets, c_check, quality
from ecu_tools.owner import OwnershipError
from ecu_tools.process import ProcessSpec, run_bounded


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
