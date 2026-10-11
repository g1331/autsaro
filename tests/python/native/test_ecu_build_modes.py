"""Exercise fault isolation through the sealed package's real build CLI."""
from __future__ import annotations

import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

from ecu_tools.process import OwnedProcess, ProcessSpec


class EcuBuildModeTests(unittest.TestCase):
    def test_received_packages_build_and_preserve_failure_diagnostics(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        if not samples:
            self.fail("Generate c_analysis samples and set AUTOSAR_C_ANALYSIS_SAMPLES first")
        tools = {}
        for variable, name in [("AUTOSAR_CC", "gcc"), ("AUTOSAR_OBJDUMP", "objdump"),
                               ("AUTOSAR_GIT", "git")]:
            requested = os.environ.get(variable) or shutil.which(name)
            if not requested:
                self.fail(f"Native build requires {variable} or {name} on PATH")
            tools[variable] = str(Path(requested).resolve(strict=True))

        def tree(root):
            return {path.relative_to(root).as_posix(): None if path.is_dir() else path.read_bytes()
                    for path in root.rglob("*")}

        with tempfile.TemporaryDirectory(prefix="autosar-received-build-") as temporary:
            directory = Path(temporary)
            (directory / "driver-logs").mkdir(mode=0o700)
            # ProcessSpec env overrides the owner's inherited environment; explicitly
            # blank development settings rather than assuming omitted keys are removed.
            env = {name: value if name.upper() in ("PATH", "SYSTEMROOT", "WINDIR", "TEMP", "TMP", "TMPDIR",
                                           "PATHEXT", "COMSPEC", "HOME", "LANG", "LC_ALL") else ""
                   for name, value in os.environ.items()}
            env.update(tools)
            wrong_tools = {}
            for variable in ("AUTOSAR_CC", "AUTOSAR_OBJDUMP"):
                source = Path(tools[variable])
                wrong = directory / (variable + source.suffix)
                shutil.copy2(source, wrong)
                with wrong.open("ab") as output:
                    output.write(b"\nchanged identity for rejection test\n")
                wrong_tools[variable] = str(wrong)

            for sample, mode in [("signals", "host"), ("standard-ecu", "host-batch"),
                                 ("multi-component", "host-batch")]:
                with self.subTest(sample=sample):
                    received = directory / sample / "package"
                    shutil.copytree(Path(samples) / sample, received)
                    before = tree(received)
                    compiler_version = json.loads((received / "target.json").read_text(encoding="utf-8"))[
                        "toolchain"]["identity"].partition(" ")[2].encode()
                    readme = (received / "README.md").read_text(encoding="utf-8")
                    for variable in tools:
                        self.assertIn(variable, readme)
                    self.assertIn("another new or empty external output directory", readme)

                    def build(output, run_env, stage, received=received, mode=mode, sample=sample):
                        argv = [sys.executable, "-I", "-S", str(received / "tools/ecu-tool.py"),
                                "build", "--project", str(received), "--output", str(output),
                                "--mode", mode]
                        return OwnedProcess(ProcessSpec.seconds(
                            argv, received, 180, directory / "driver-logs", sample + "-" + stage,
                            env=run_env)).wait()

                    # Probe every required tool on the same real multi package.
                    if sample == "multi-component":
                        failures = [("missing-" + key, {**env, key: ""}, "Missing") for key in tools]
                        failures.append(("wrong-gcc", {**env, "AUTOSAR_CC": wrong_tools["AUTOSAR_CC"]},
                                         "Compiler identity differs"))
                        # Only Linux declares an objdump digest; Windows only queries its version.
                        if os.name != "nt":
                            failures.append(("wrong-objdump", {**env, "AUTOSAR_OBJDUMP":
                                                               wrong_tools["AUTOSAR_OBJDUMP"]},
                                             "objdump identity differs"))
                        for label, run_env, expected in failures:
                            with self.subTest(failure=label):
                                failed_output = directory / sample / label
                                failed = build(failed_output, run_env, label)
                                observed = failed.stdout.read_text() + failed.stderr.read_text()
                                self.assertEqual(failed.status, "exited", observed)
                                self.assertNotEqual(failed.exit_code, 0, observed)
                                self.assertIn(expected, observed)
                                self.assertTrue((failed_output / "logs").is_dir())
                                if label in ("wrong-gcc", "wrong-objdump"):
                                    # Compiler version/triple queries precede both digest checks.
                                    # objdump's version query is not reached after its digest rejection.
                                    self.assertTrue(any(compiler_version in log.read_bytes()
                                                        for log in (failed_output / "logs").rglob("stdout.log")),
                                                    "Failed tool query must retain its actual version log")
                                old_output = tree(failed_output)
                                retry = build(failed_output, env, label + "-retry")
                                observed = retry.stdout.read_text() + retry.stderr.read_text()
                                self.assertNotEqual(retry.exit_code, 0, observed)
                                self.assertIn("Build output must be new or empty", observed)
                                self.assertEqual(tree(failed_output), old_output)
                                self.assertEqual(tree(received), before)
                    output = directory / sample / "successful-empty-output"
                    output.mkdir()
                    built = build(output, env, "success")
                    self.assertTrue(built.success, built.stdout.read_text() + built.stderr.read_text())
                    binary = "ecu_host" if mode == "host" else "ecu_host_batch"
                    self.assertTrue((output / (binary + (".exe" if os.name == "nt" else ""))).is_file())
                    self.assertEqual(tree(received), before)

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

            initialized = execute([env["AUTOSAR_GIT"], "init", "--quiet", str(directory)], "init-checkout", env)
            self.assertTrue(initialized.success, initialized.stderr.read_text())

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
