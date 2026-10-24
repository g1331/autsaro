"""Exercise fault isolation through the sealed package's real build CLI."""
from __future__ import annotations

import contextlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from ecu_tools import build as ecu_build
from ecu_tools.process import OwnedProcess, ProcessSpec


class EcuBuildModeTests(unittest.TestCase):
    def test_zero_dlc_output_keeps_library_copy_sizes_positive(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        self.assertTrue(samples, "Generate c_analysis samples first")
        project = (Path(samples) / "standard-ecu").resolve(strict=True)
        fixture = Path(__file__).resolve().parents[3] / "core/tests/fixtures/host-batch/zero_dlc_output.c"
        target = json.loads((project / "target.json").read_text(encoding="utf-8"))
        original = ecu_build.query
        commands = []

        def observe(argv, logs, stage, **options):
            if stage == "native-c99-build":
                argv = [*argv, "-fno-builtin-memcpy", "-Wl,--wrap=memcpy"]
                if target["target"].startswith("windows"):
                    argv.append("-Wl,--wrap=__imp_memcpy")
                else:
                    argv.append("-Wl,--wrap=__memcpy_chk")
                commands.append(argv)
            return original(argv, logs, stage, **options)

        with tempfile.TemporaryDirectory(prefix="autosar-zero-dlc-", delete=False) as temporary:
            directory = Path(temporary)
            with patch.object(ecu_build, "query", side_effect=observe):
                binary = ecu_build.build(project, directory / "build", "test", fixture)
            self.assertEqual(len(commands), 1)
            if not target["target"].startswith("windows"):
                bounds = OwnedProcess(ProcessSpec.seconds(
                    [str(binary), "fortify"], directory, 10, directory, "copy-bounds",
                )).wait()
                self.assertEqual(bounds.exit_code, -6, bounds.stdout.read_text() + bounds.stderr.read_text())
                self.assertIn("buffer overflow detected", bounds.stderr.read_text())
            result = OwnedProcess(ProcessSpec.seconds(
                [str(binary)], directory, 10, directory, "zero-dlc",
            )).wait()
            self.assertTrue(result.success,
                            f"exit_code={result.exit_code}\n" +
                            result.stdout.read_text() + result.stderr.read_text())
            self.assertEqual(result.stderr.read_text(), "")
            self.assertEqual([line for line in result.stdout.read_text().splitlines()
                              if line.startswith("zero_dlc ")],
                             ["zero_dlc epoch=1 length=0 confirmed=1",
                              "zero_dlc epoch=2 length=1 confirmed=1",
                              "zero_dlc epoch=3 length=8 confirmed=1",
                              "zero_dlc closed reason=0"], result.stdout.read_text())
            shutil.rmtree(directory)

    def test_one_byte_corruption_after_patching_is_rejected_before_compilation(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        self.assertTrue(samples, "Generate c_analysis samples first")
        project = (Path(samples) / "standard-ecu").resolve(strict=True)
        target = json.loads((project / "target.json").read_text(encoding="utf-8"))
        original = ecu_build.require_kernel_patch_applied
        calls = 0
        with tempfile.TemporaryDirectory(prefix="autosar-corrupt-kernel-") as temporary:
            output = Path(temporary) / "build"

            def corrupt_last_result(patch_path, before):
                nonlocal calls
                original(patch_path, before)
                calls += 1
                if calls == len(target["kernelPatches"]):
                    path = output / "kernel/tasks.c"
                    data = path.read_bytes()
                    # Change a comment byte so the mutant still compiles.
                    index = data.index(b"FreeRTOS Kernel")
                    path.write_bytes(data[:index] + bytes([data[index] ^ 1]) + data[index + 1:])

            with (patch.object(ecu_build, "query", wraps=ecu_build.query) as queries,
                  patch.object(ecu_build, "require_kernel_patch_applied", side_effect=corrupt_last_result),
                  self.assertRaisesRegex(ValueError, "^Kernel identity mismatch: kernel/tasks.c$")):
                ecu_build.build(project, output, "host-batch")
            self.assertEqual(calls, len(target["kernelPatches"]))
            self.assertNotIn("native-c99-build", [call.args[2] for call in queries.call_args_list])

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

            for sample, mode in [("standard-ecu", "host-batch"), ("user-application", "host-batch"),
                                 ("component-pipeline", "host-batch")]:
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

                    # Probe every required tool on the same real pipeline package.
                    if sample == "component-pipeline":
                        failures = [("missing-" + key, {**env, key: ""}, "Missing") for key in tools]
                        failures.append(("wrong-gcc", {**env, "AUTOSAR_CC": wrong_tools["AUTOSAR_CC"]},
                                         "Compiler identity differs"))
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
                    self.assertTrue((output / ("ecu_host_batch" + (".exe" if os.name == "nt" else ""))).is_file())
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
        seal = (project / "meta/files.sha256").read_bytes()
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
            self.assertEqual((project / "meta/files.sha256").read_bytes(), seal)

    def test_integrity_refusals_name_the_entry_and_keep_the_previous_build(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        if not samples:
            self.fail("Generate c_analysis samples and set AUTOSAR_C_ANALYSIS_SAMPLES first")
        env = dict(os.environ)
        for variable, tool in [("AUTOSAR_CC", "gcc"), ("AUTOSAR_OBJDUMP", "objdump"), ("AUTOSAR_GIT", "git")]:
            requested = env.get(variable) or shutil.which(tool)
            if not requested:
                self.fail(f"Native build requires {variable} or {tool} on PATH")
            env[variable] = str(Path(requested).resolve(strict=True))

        def tree(root):
            return {path.relative_to(root).as_posix(): None if path.is_dir() else path.read_bytes()
                    for path in root.rglob("*")}

        with tempfile.TemporaryDirectory(prefix="autosar-integrity-") as temporary:
            directory = Path(temporary).resolve(strict=True)
            package = directory / "package"
            shutil.copytree(Path(samples) / "standard-ecu", package)

            def build(output, stage):
                return OwnedProcess(ProcessSpec.seconds(
                    [sys.executable, "-I", "-S", str(package / "tools/ecu-tool.py"), "build",
                     "--project", str(package), "--output", str(output), "--mode", "host-batch"],
                    directory, 180, directory, stage, env=env)).wait()

            previous = directory / "previous"
            built = build(previous, "previous")
            self.assertTrue(built.success, built.stdout.read_text() + built.stderr.read_text())
            kept = tree(previous)
            source = package / "bsw/src/Com.c"
            original = source.read_bytes()
            external = directory / "external-bsw"

            # Each case restores the package even when its assertions fail.
            @contextlib.contextmanager
            def tampered():
                source.write_bytes(original + b"\n")
                try:
                    yield "Source identity mismatch: bsw/src/Com.c"
                finally:
                    source.write_bytes(original)

            @contextlib.contextmanager
            def extra():
                (package / "bsw/src/Extra.c").write_bytes(b"")
                try:
                    yield "Unlisted source file: bsw/src/Extra.c"
                finally:
                    (package / "bsw/src/Extra.c").unlink()

            @contextlib.contextmanager
            def linked():
                # A directory symbolic link on POSIX, an NTFS junction on Windows.
                (package / "bsw").rename(external)
                try:
                    if os.name == "nt":
                        subprocess.run(["cmd", "/C", "mklink", "/J", str(package / "bsw"), str(external)],
                                       check=True, capture_output=True)
                    else:
                        (package / "bsw").symlink_to(external, target_is_directory=True)
                    yield "Source links/reparse points are refused: bsw"
                finally:
                    if os.name == "nt" and (package / "bsw").exists():
                        (package / "bsw").rmdir()
                    elif (package / "bsw").is_symlink():
                        (package / "bsw").unlink()
                    external.rename(package / "bsw")

            before = tree(package)
            for case in (tampered, extra, linked):
                with self.subTest(case=case.__name__):
                    with case() as expected:
                        output = directory / case.__name__
                        failed = build(output, case.__name__)
                        observed = failed.stdout.read_text() + failed.stderr.read_text()
                        self.assertEqual(failed.status, "exited", observed)
                        self.assertNotEqual(failed.exit_code, 0, observed)
                        self.assertIn(expected, observed)
                        self.assertFalse(output.exists())
                        self.assertEqual(tree(previous), kept)
                    self.assertEqual(tree(package), before)
