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


def check_native_output_backpressure(project: Path, directory: Path) -> None:
    """Called by the normal Rust protocol test with its freshly generated project."""
    fixture = Path(__file__).resolve().parents[3] / "core/tests/fixtures/host-batch/host_batch_faults.c"
    directory.mkdir()
    source = directory / "host-batch-faults.c"
    source.write_text(
        "#define main production_main\n#define batch production_batch\n#define control production_control\n"
        + (project / "integration/src/ecu_host_batch.c").read_text()
        + "\n#undef main\n#undef batch\n#undef control\n" + fixture.read_text(), encoding="utf-8",
    )
    original = ecu_build.query
    before = (project / "meta/files.sha256").read_bytes()
    host_source = "os/src/host/windows/Os_HostWindows.c" if sys.platform == "win32" else "os/src/host/linux/Os_HostLinux.c"
    for variant in ("control", "no-observer", "no-exit"):
        output = directory / variant
        output.mkdir(mode=0o700)
        replacements = {}
        for name in ("integration/src/Ecu_Target.c", "os/src/Os_Backend.c", "integration/src/Ecu_HostBridge.c", host_source):
            path = project / name
            text = path.read_text()
            if name.endswith("Ecu_Target.c"):
                entry = "StatusType Ecu_TargetConfirmOutput(uint64_t ticket, PduIdType pdu)\n{"
                assert text.count(entry) == 1
                text = text.replace(entry, entry + "\n    extern void HostFaultConfirm(uint64_t, PduIdType);\n    HostFaultConfirm(ticket, pdu);")
                if variant == "no-exit":
                    exit_call = "Os_HostExit((uint32_t)failure);"
                    assert text.count(exit_call) == 1
                    text = text.replace(exit_call, "(void)failure;")
            elif name.endswith("Os_Backend.c"):
                report = "    if (Os_StackReport() != 0) {"
                assert text.count(report) == 1
                text = text.replace(report, "    extern void HostFaultReport(StatusType);\n    HostFaultReport(shutdown_reason);\n" + report)
            elif name == host_source:
                if sys.platform == "win32":
                    call = "            if ((WriteFile(handle, &data[offset], requested, &written, NULL) == 0) ||"
                    observer = "            extern void HostFaultNativeWrite(int);\n            HostFaultNativeWrite(error_stream);\n"
                else:
                    call = "        errno = 0;\n        count = write(descriptor, &data[offset], length - offset);"
                    observer = "        extern void HostFaultNativeWrite(int);\n        HostFaultNativeWrite(descriptor == STDOUT_FILENO ? 0 : 1);\n"
                assert text.count(call) == 1
                text = text.replace(call, observer + call)
            elif variant == "no-observer":
                spawn = "monitor = Os_HostSpawnObserverThread(&watch, NULL);"
                assert text.count(spawn) == 1
                text = text.replace(spawn, "monitor = host_watch.cancel; (void)&watch;")
            else:
                continue
            copy = output / path.name
            copy.write_text(text, encoding="utf-8")
            replacements[ecu_build.tool_argument(path)] = ecu_build.tool_argument(copy)
        commands = []

        def observe(argv, logs, stage, *, replacements=replacements, commands=commands, **options):
            if stage == "native-c99-build":
                assert all(argv.count(path) == 1 for path in replacements), argv
                argv = [replacements.get(argument, argument) for argument in argv]
                commands.append(argv)
            return original(argv, logs, stage, **options)

        # Reuse the existing test compiler seam; sealed inputs and build guards are unchanged.
        with patch.object(ecu_build, "query", side_effect=observe):
            binary = ecu_build.build(project, output / "build", "probe", source)
        assert len(commands) == 1
        receipt = {"observed": commands[0]}
        kinds = os.environ.get("AUTOSAR_SANITIZE")
        if kinds in ("thread", "address,undefined"):
            assert sys.platform == "linux", kinds
            replay = list(commands[0])
            binary = output / "sanitized-probe"
            replay[replay.index("-o") + 1] = str(binary)
            replay[1:1] = [f"-fsanitize={kinds}", "-fno-sanitize-recover=all", "-fno-omit-frame-pointer", "-g"]
            original(replay, output / "build/logs", "sanitized-c99-build")
            receipt["sanitized"] = replay
        (output / "compile-command.json").write_text(json.dumps(receipt, indent=2) + "\n")
        modes = ("pipe-drain", "pipe-block") if variant == "control" else ("pipe-block",)
        for mode in modes:
            result = OwnedProcess(ProcessSpec.seconds(
                [str(binary), mode], directory, 8, output, mode,
            )).wait()
            stdout = result.stdout.read_text()
            errors = result.stderr.read_text()
            evidence = f"{variant}/{mode}: {result}\n{stdout}\n{errors}"
            assert "pipe_full mode=" in errors, evidence
            tickets = [line.removeprefix("pipe_write_enter stage=output ticket=")
                       for line in errors.splitlines() if line.startswith("pipe_write_enter stage=output ticket=")]
            assert tickets and all(int(ticket) > 0 for ticket in tickets), evidence
            if mode == "pipe-drain":
                assert result.success, evidence
                for ticket in tickets:
                    assert f"pipe_confirm ticket={ticket} pdu=" in errors, evidence
                assert "written=1\n" in errors, evidence
                assert "OUT epoch=" in errors and "COMMIT_OK batch=" in errors, evidence
                assert "pipe_write_complete stage=receipt" in errors, evidence
            else:
                assert len(tickets) == 1, evidence
                assert "pipe_confirm " not in errors and "pipe_write_complete" not in errors, evidence
                assert "COMMIT_OK" not in errors and "COMMIT_OK" not in stdout, evidence
                if variant == "no-observer":
                    assert "pipe_report_enter" not in errors, evidence
                else:
                    assert "pipe_report_enter reason=7" in errors, evidence
                    assert "pipe_report_native_write stream=0" in errors, evidence
                if variant == "control":
                    assert result.status == "exited" and result.exit_code == 7, evidence
                else:
                    assert result.status == "timeout", evidence
            print(f"backpressure variant={variant} mode={mode} status={result.status} exit={result.exit_code}\n{errors}", flush=True)
    assert (project / "meta/files.sha256").read_bytes() == before
    ecu_build.sealed_sources(project)


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

    def test_patched_kernel_stack_scan_uses_each_byte(self):
        from os_suites import sanitizer_flags

        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        self.assertTrue(samples, "Generate c_analysis samples first")
        project = (Path(samples) / "standard-ecu").resolve(strict=True)
        original = ecu_build.query
        with tempfile.TemporaryDirectory(prefix="autosar-kernel-cursors-") as temporary:
            directory = Path(temporary).resolve()
            for variant in ("control", "stack-step"):
                output = directory / variant
                source = directory / (variant + ".c")
                # Include the real task TU to observe its private stack scanner;
                # the build seam below removes only its duplicate compiler input.
                source.write_text('#include "' + (output / "kernel/tasks.c").as_posix() + '"\n' + r'''
int main(void)
{
    StackType_t stack[3];
    uint8_t * bytes = (uint8_t *)stack;
    size_t index;
    /* One complete free stack word followed by a non-fill sentinel. */
    for (index = 0U; index < sizeof(stack); index++) {
        bytes[index] = index < sizeof(StackType_t) ? (uint8_t)0xa5U : (uint8_t)0U;
    }
    if (prvTaskCheckFreeStackSpace(bytes) != 1U) {
        return 1;
    }
    if (prvTaskCheckFreeStackSpace(&bytes[1]) != 0U) {
        return 2;
    }
    return 0;
}
''', encoding="utf-8")

                def compile_cursor(argv, logs, stage, *, output=output, variant=variant, **options):
                    if stage == "native-c99-build":
                        task = output / "kernel/tasks.c"
                        self.assertEqual(argv.count(ecu_build.tool_argument(task)), 1)
                        argv = [argument for argument in argv if argument != ecu_build.tool_argument(task)]
                        argv = [argv[0], *sanitizer_flags(), *argv[1:]]
                        if variant == "stack-step":
                            before = "&pucCursor[-portSTACK_GROWTH]"
                            text = task.read_text(encoding="utf-8")
                            self.assertEqual(text.count(before), 1)
                            task.write_text(text.replace(before, "&pucCursor[-portSTACK_GROWTH + 1]"), encoding="utf-8")
                    return original(argv, logs, stage, **options)

                with patch.object(ecu_build, "query", side_effect=compile_cursor):
                    binary = ecu_build.build(project, output, "probe", source)
                result = OwnedProcess(ProcessSpec.seconds([str(binary)], directory, 10, directory, variant)).wait()
                self.assertEqual(result.exit_code, 0 if variant == "control" else 1)
                self.assertEqual(result.stderr.read_bytes(), b"")

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
