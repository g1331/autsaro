"""Exercise fault isolation through the sealed package's real build CLI."""
from __future__ import annotations

import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

from ecu_tools.process import OwnedProcess, ProcessSpec


class EcuBuildModeTests(unittest.TestCase):
    def test_production_ignores_fault_environment_and_test_mode_enables_it(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        if not samples:
            self.fail("Generate c_analysis samples and set AUTOSAR_C_ANALYSIS_SAMPLES first")
        project = (Path(samples) / "standard-ecu").resolve(strict=True)
        env = dict(os.environ)
        for variable, tool in [("AUTOSAR_CC", "gcc"), ("AUTOSAR_OBJDUMP", "objdump"), ("AUTOSAR_GIT", "git")]:
            requested = env.get(variable) or shutil.which(tool)
            if not requested:
                self.fail(f"Native build requires {variable} or {tool} on PATH")
            env[variable] = str(Path(requested).resolve(strict=True))
        env.pop("AUTOSAR_OS_FAIL_RESOURCE", None)
        env.pop("AUTOSAR_OS_BAD_GUARANTEE", None)
        seal = (project / "files.sha256").read_bytes()
        with tempfile.TemporaryDirectory(prefix="autosar-build-modes-") as temporary:
            directory = Path(temporary)
            stdin = directory / "batch.stdin"
            stdin.write_bytes(b"BEGIN 1\nCOMMIT\n")

            def execute(argv, stage, run_env, input_file=None):
                return OwnedProcess(ProcessSpec.seconds(argv, directory, 180, directory, stage,
                                    env=run_env, stdin_file=input_file)).wait()

            for mode in ("host-batch", "probe", "test"):
                with self.subTest(mode=mode):
                    output = directory / mode
                    built = execute([sys.executable, "-I", "-S", str(project / "tools/ecu-tool.py"),
                                     "build", "--project", str(project), "--output", str(output),
                                     "--mode", mode], "build-" + mode, env)
                    self.assertTrue(built.success, built.stdout.read_text() + built.stderr.read_text())
                    binary = output / (("ecu_host_batch" if mode == "host-batch" else "ecu_probe")
                                       + (".exe" if os.name == "nt" else ""))
                    for label, faults in [("normal", {}), ("invalid-resource", {"AUTOSAR_OS_FAIL_RESOURCE": "invalid"}),
                                          ("first-resource", {"AUTOSAR_OS_FAIL_RESOURCE": "1"}),
                                          ("stack", {"AUTOSAR_OS_BAD_GUARANTEE": "S"})]:
                        with self.subTest(fault=label):
                            result = execute([str(binary)], mode + "-" + label, {**env, **faults},
                                             stdin if mode == "host-batch" else None)
                            observed = result.stdout.read_text() + result.stderr.read_text()
                            self.assertEqual(result.status, "exited", observed)
                            if mode == "test" and faults:
                                expected_reason = 8 if label == "invalid-resource" else 7
                                self.assertEqual(result.exit_code, expected_reason, observed)
                                self.assertIn(f"reason={expected_reason}", observed)
                                self.assertNotIn("ecu_probe completed=20", observed)
                            else:
                                self.assertTrue(result.success, observed)
                                self.assertIn("READY HostBatchV1" if mode == "host-batch"
                                              else "ecu_probe completed=20", observed)
            self.assertEqual((project / "files.sha256").read_bytes(), seal)
