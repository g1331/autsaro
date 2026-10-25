"""Real host C fault injection, without adding hooks to delivery code."""
from __future__ import annotations

import difflib
import os
import platform
import re
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

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
        # Delivered projects generate these two headers; the lock fixture needs only the policy type.
        generated = cls.directory / "generated"
        generated.mkdir()
        (generated / "Ecu_Config.h").write_text(
            '#ifndef ECU_CONFIG_H\n#define ECU_CONFIG_H\n#include "Std_Types.h"\n'
            "typedef enum { ECU_TX_SYNCHRONOUS = 0, ECU_TX_QUEUED = 1 } EcuTxConfirmation;\n"
            "typedef struct { EcuTxConfirmation tx_confirmation; } EcuPolicyConfig;\n"
            "extern const EcuPolicyConfig Ecu_Policy;\n#endif\n", encoding="utf-8")
        (generated / "Ecu_Execution.h").write_text(
            '#ifndef ECU_EXECUTION_H\n#define ECU_EXECUTION_H\n#include "Ecu_Config.h"\n#include "Can_Host.h"\n'
            "EcuStatus Ecu_ExecutionTransmit(Can_TxSinkType sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8]);\n"
            "uint64_t Ecu_ExecutionNow(void);\n#endif\n", encoding="utf-8")
        cls.flags = ["-std=c99", "-O1", "-Wall", "-Wextra", "-Werror", "-pedantic",
                     "-I" + str(ROOT / "runtime/bsw/src"), "-I" + str(ROOT / "runtime/bsw/include"),
                     "-I" + str(ROOT / "runtime/host/src"), "-I" + str(ROOT / "runtime/host/include"),
                     # The mainline CAN headers use the generated ComStack_Cfg.h widths.
                     "-I" + str(ROOT / "core/tests/fixtures/bsw-contracts"), "-I" + str(generated)]
        suffix = ".exe" if os.name == "nt" else ""
        cls.lock = cls.directory / ("lock" + suffix)
        cls.production_lock = cls.directory / ("production-lock" + suffix)
        header = ROOT / "tests/fixtures/host_lock_faults.h"
        obj = cls.directory / "lock.o"
        result = cls.execute([cls.compiler, *cls.flags, "-include", str(header), "-c",
                              str(ROOT / "runtime/host/src/Can_HostLock.c"), "-o", str(obj)], "compile-lock")
        if not result.success:
            raise RuntimeError(result.stderr.read_text(encoding="utf-8", errors="replace"))
        sources = [ROOT / "runtime/bsw/src/Can.c", ROOT / "runtime/bsw/src/Det.c"]
        sources += [ROOT / "runtime/host/src" / name for name in ("Can_Host.c", "Det_Host.c", "SchM_CanIf_Host.c")]
        sources.append(ROOT / "core/tests/fixtures/bsw-contracts/execution_sink.c")
        result = cls.execute([cls.compiler, *cls.flags, str(obj), str(ROOT / "tests/fixtures/host_lock_faults.c"),
                              *(str(path) for path in sources),
                              *([] if os.name == "nt" else ["-pthread"]), "-o", str(cls.lock)], "link-lock")
        if not result.success:
            raise RuntimeError(result.stderr.read_text(encoding="utf-8", errors="replace"))

        result = cls.execute([cls.compiler, *cls.flags, str(ROOT / "runtime/host/src/Can_HostLock.c"),
                              str(ROOT / "tests/fixtures/host_lock_faults.c"),
                              *(str(path) for path in sources),
                              *([] if os.name == "nt" else ["-pthread"]), "-o", str(cls.production_lock)], "production-lock")
        if not result.success:
            raise RuntimeError(result.stderr.read_text(encoding="utf-8", errors="replace"))

    def test_all_production_locks_serialize_real_threads(self):
        result = self.execute([str(self.production_lock), "parallel"], "parallel")
        self.assertTrue(result.success, result.stderr.read_text())

    def test_production_fatal_stops_without_signal_handler_or_crt_cleanup(self):
        for case in ("nest-strict", "nest-same", "nest-driver", "exit-driver", "exit-area",
                     "exit-wrong", "exit-other-thread", "exit-driver-as-area", "exit-area-as-driver",
                     "invalid-enter", "negative-enter", "invalid-exit", "negative-exit"):
            with self.subTest(case=case):
                result = self.execute([str(self.production_lock), case], "production-" + case)
                self.assertEqual(result.exit_code, 134, result.stdout.read_text())
                self.assertEqual(result.stdout.read_bytes(), b"")
                self.assertEqual(result.stderr.read_bytes(), b"")

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    @classmethod
    def execute(cls, argv, stage):
        return OwnedProcess(ProcessSpec.seconds(argv, cls.directory, 60, cls.directory, stage)).wait()

    def test_driver_lock_and_areas_admit_sequential_use(self):
        result = self.execute([str(self.lock), "normal"], "normal")
        self.assertTrue(result.success, result.stdout.read_text())

    @unittest.skipUnless(os.name == "nt", "Windows self-termination refusal")
    def test_refused_self_termination_selects_trap_without_returning(self):
        result = self.execute([str(self.lock), "terminate-refused"], "terminate-refused")
        self.assertEqual(result.exit_code, 92, result.stdout.read_text())
        self.assertIn("LOCK_TRAP", result.stdout.read_text())

    def test_invalid_owners_and_areas_are_fatal(self):
        for case in ("nest-strict", "nest-same", "nest-driver", "exit-driver", "exit-area",
                     "exit-wrong", "exit-other-thread", "exit-driver-as-area", "exit-area-as-driver",
                     "invalid-enter", "negative-enter", "invalid-exit", "negative-exit"):
            with self.subTest(case=case):
                result = self.execute([str(self.lock), case], case)
                output = result.stdout.read_text()
                self.assertEqual(result.exit_code, 91, output)
                self.assertIn("LOCK_FATAL", output)
                self.assertNotIn("UNEXPECTED_LOCK", output)
                self.assertNotIn("UNEXPECTED_UNLOCK", output)

    def test_void_lock_failures_select_fatal_policy_and_never_return(self):
        cases = ["once", "mutex-init"] if os.name == "nt" else ["once", "mutex-init", "lock", "unlock"]
        # The driver lock and a module area are separate instances of the helper.
        for case, instance in ((case, instance) for case in cases for instance in ("fatal", "area")):
            with self.subTest(case=case, instance=instance):
                result = self.execute([str(self.lock), case, instance], case + "-" + instance)
                output = result.stdout.read_text()
                self.assertEqual(result.exit_code, 91, output)
                self.assertIn("LOCK_FATAL", output)
                self.assertNotIn("UNEXPECTED_UNLOCK", output)


class HostIoTests(unittest.TestCase):
    """Use the actual native adapter and full OS link, including syscall refusal seams."""

    def test_native_bytes_and_record_completion(self):
        import os_suites

        with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
            directory = Path(temporary)
            target = None
            if os.name == "nt":
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                # Redirect only the selected production TUs after platform declarations.
                for relative in ("src/host/windows/Os_HostWindows.c", "src/Os_Stack.c"):
                    source = target / relative
                    text = source.read_text(encoding="utf-8")
                    text = text.replace('#include "Os_HostIo.h"', '#include "Os_HostIo.h"\n'
                                        'BOOL WINAPI HostIoWriteFile(HANDLE, LPCVOID, DWORD, LPDWORD, LPOVERLAPPED);\n'
                                        '#define WriteFile HostIoWriteFile')
                    source.write_text(text, encoding="utf-8")
            with os_suites.native_session(directory, "host-io"):
                binary, _ = os_suites.build(
                    directory, str(ROOT / "tests/fixtures/host_io.c"),
                    target=target, link_flags=() if os.name == "nt" else tuple(f"-Wl,--wrap={name}" for name in (
                        "write", "sigemptyset", "sigaddset", "sigpending", "sigismember",
                    )),
                )
                expected = {
                    "format": b"zero=0 max=18446744073709551615 hex=ffffffffffffffff fault=000000A5\x00\r\n",
                    "capacity": b"", "hex-width": b"", "hex-value": b"", "hex-case": b"",
                    "broken-pipe": b"", "partial": b"abcdef", "zero": b"", "partial-error": b"ab",
                }
                if os.name != "nt":
                    expected.update({"interrupted": b"abcdef", "pending-pipe": b"", "errno-sequence": b"abcdef"})
                for case, output in expected.items():
                    with self.subTest(case=case):
                        result = OwnedProcess(ProcessSpec.seconds(
                            [str(binary), case], directory, 10, directory, case,
                        )).wait()
                        actual = result.stdout.read_bytes()
                        self.assertEqual(result.exit_code, 0, (case, actual, result.stderr.read_bytes()))
                        self.assertEqual(actual, output, case)

                fault = (b"FATAL state=Failed reason=13 controllers_exhausted=1\nFAULT role=S "
                         + (b"exception=" if os.name == "nt" else b"signal=")
                         + b"00000000 captured=1 record_complete=1\n")
                fatal_cases = ("fatal-partial", "fatal-zero", "fatal-broken-pipe")
                if os.name != "nt":
                    fatal_cases += ("fatal-interrupted", "fatal-errno")
                for case in fatal_cases:
                    with self.subTest(case=case):
                        result = OwnedProcess(ProcessSpec.seconds(
                            [str(binary), case], directory, 10, directory, case,
                        )).wait()
                        output, errors = result.stdout.read_bytes(), result.stderr.read_bytes()
                        self.assertEqual(result.exit_code, 13, (case, output, errors))
                        self.assertEqual(output, fault if case in ("fatal-partial", "fatal-interrupted", "fatal-errno") else b"", case)
                        self.assertEqual(errors, fault if case == "fatal-broken-pipe" else b"", case)


@unittest.skipUnless(platform.system() in ("Linux", "Windows"), "native kernel object consumers")
class NativeKernelObjectTests(unittest.TestCase):
    def test_native_restart_rejects_wrong_pc_and_exported_continuation(self):
        import os_suites

        for mutation in ("wrong-pc", "exported-continuation"):
            with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                source = target / "src/Os_PortRestart.c"
                text = source.read_text(encoding="utf-8")
                if mutation == "exported-continuation":
                    before, after = (".scl 3; .type 32", ".scl 2; .type 32") if os.name == "nt" else (
                        ".local Os_PortRestartLanding", ".global Os_PortRestartLanding",
                    )
                    self.assertEqual(text.count(before), 1)
                    text = text.replace(before, after)
                else:
                    before = "leaq Os_PortRestartLanding(%rip),%rax"
                    self.assertEqual(text.count(before), 2)
                    text = text.replace(before, "leaq Os_PortRestartRejected(%rip),%rax")
                    if os.name == "nt":
                        before = 'ret\\n.seh_endproc\\n.text\\n'
                        after = ('ret\\n.seh_endproc\\nOs_PortRestartRejected:\\nendbr64\\n'
                                 'mov $93,%ecx\\ncall ExitProcess\\nud2\\n.text\\n')
                    else:
                        before = '.size Os_PortRestartAddress,.-Os_PortRestartAddress\\n.popsection'
                        after = ('.size Os_PortRestartAddress,.-Os_PortRestartAddress\\n'
                                 '.local Os_PortRestartRejected\\nOs_PortRestartRejected:\\nendbr64\\n'
                                 'mov $93,%edi\\ncall _exit\\nud2\\n.popsection')
                    self.assertEqual(text.count(before), 1)
                    text = text.replace(before, after)
                source.write_text(text, encoding="utf-8")
                with patch.object(os_suites, "TARGET", target), os_suites.native_session(directory, mutation):
                    if mutation == "wrong-pc":
                        with self.assertRaises(AssertionError) as caught:
                            os_suites.check_port_restart(directory)
                        result = caught.exception.args[0]
                        self.assertEqual(result.returncode, 93, result)
                        self.assertEqual(result.stdout, "")
                        self.assertEqual(result.stderr, "")
                    else:
                        with self.assertRaisesRegex(ValueError,
                                r"(ELF OS entry is outside its code section|PE OS continuation must be local): Os_PortRestartLanding"):
                            os_suites.check_port_restart(directory)

    def _windows_port_target(self, directory: Path, omitted_patch: str | None,
                             scenario_symbol: str, declarations: str) -> Path:
        import os_suites

        target = directory / "target"
        shutil.copytree(os_suites.TARGET, target)
        if omitted_patch:
            (target / "patches" / omitted_patch).unlink()
        # Redirect only the adopted port, after real platform declarations.
        anchor = '#include "Os_Backend.h"\n'
        before = ('/* Scheduler includes. */\n#include "FreeRTOS.h"\n#include "task.h"\n' + anchor +
                  '\n#ifdef WIN32_LEAN_AND_MEAN\n    #include <winsock2.h>\n#else\n    #include <winsock.h>\n#endif\n')
        after = before.replace(anchor, anchor + declarations)
        name = "portable/MSVC-MingW/port.c"
        (target / "patches/9999-test-native-port.patch").write_text("".join(difflib.unified_diff(
            before.splitlines(True), after.splitlines(True),
            fromfile="a/" + name, tofile="b/" + name,
        )), encoding="utf-8")
        fixture = target / "tests/lifecycle.c"
        text = fixture.read_text(encoding="utf-8")
        entry = "int main(int argc, char **argv)\n{"
        self.assertEqual(text.count(entry), 1)
        fixture.write_text(text.replace(entry, "extern const char *" + scenario_symbol + ";\n" + entry +
            '\n    ' + scenario_symbol + ' = argc == 2 ? argv[1] : "normal";'), encoding="utf-8")
        return target

    @unittest.skipUnless(os.name == "nt", "Windows native handoff failures")
    def test_windows_port_native_handoff_errors_stop_the_os(self):
        import os_suites

        for mutant in (False, True):
            with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = self._windows_port_target(
                    directory, "0019-windows-native-handoff-errors.patch" if mutant else None,
                    "Os_HandoffScenario", (
                    "DWORD WINAPI Os_TestWaitForSingleObject(HANDLE, DWORD);\n"
                    "DWORD WINAPI Os_TestWaitForMultipleObjects(DWORD, const HANDLE *, BOOL, DWORD);\n"
                    "BOOL WINAPI Os_TestReleaseMutex(HANDLE);\n"
                    "BOOL Os_TestSignalEvent(HANDLE);\n"
                    "BOOL WINAPI Os_TestResetEvent(HANDLE);\n"
                    "#define WaitForSingleObject Os_TestWaitForSingleObject\n"
                    "#define WaitForMultipleObjects Os_TestWaitForMultipleObjects\n"
                    "#define ReleaseMutex Os_TestReleaseMutex\n"
                    "#define Os_HostSetEvent Os_TestSignalEvent\n"
                    "#define ResetEvent Os_TestResetEvent\n"
                ))
                with os_suites.native_session(directory, "windows-native-handoff"):
                    binary, _ = os_suites.build(
                        directory, "lifecycle.c", target=target,
                        extra_sources=(ROOT / "tests/fixtures/os_windows_native_handoff.c",),
                    )
                    for scenario in ("normal", "timeout", "wait", "release", "signal", "reset", "wait-multiple", "abandoned"):
                        with self.subTest(mutant=mutant, scenario=scenario):
                            result = os_suites.run_native([str(binary), scenario], timeout=10)
                            refused = scenario not in ("normal", "timeout")
                            self.assertEqual(result.stderr, "")
                            if refused:
                                self.assertIn("native handoff refused=" + scenario, result.stdout)
                                if mutant:
                                    self.assertIn(result.returncode, (0, 91), result)
                                else:
                                    self.assertEqual(result.returncode, 7, result)  # E_OS_STATE, not E_OS_STACKFAULT.
                                    self.assertIn("lifecycle=Closed state=Failed reason=7", result.stdout)
                                    self.assertNotIn("native handoff continued", result.stdout)
                            else:
                                self.assertEqual(result.returncode, 0, result)
                                self.assertIn("state=Ready", result.stdout)
                                if scenario == "timeout":
                                    self.assertIn("native handoff timeout", result.stdout)

    @unittest.skipUnless(os.name == "nt", "Windows thread setup failures")
    def test_windows_port_thread_setup_failures_stop_before_ready(self):
        import os_suites

        for mutant in (False, True):
            with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = self._windows_port_target(
                    directory, "0018-windows-thread-setup-errors.patch" if mutant else None,
                    "Os_ThreadSetupScenario", (
                    "DWORD_PTR WINAPI Os_TestSetThreadAffinityMask(HANDLE, DWORD_PTR);\n"
                    "BOOL WINAPI Os_TestSetThreadPriorityBoost(HANDLE, BOOL);\n"
                    "BOOL WINAPI Os_TestSetThreadPriority(HANDLE, int);\n"
                    "#define SetThreadAffinityMask Os_TestSetThreadAffinityMask\n"
                    "#define SetThreadPriorityBoost Os_TestSetThreadPriorityBoost\n"
                    "#define SetThreadPriority Os_TestSetThreadPriority\n"
                ))
                with os_suites.native_session(directory, "windows-thread-setup"):
                    binary, _ = os_suites.build(
                        directory, "lifecycle.c", target=target,
                        extra_sources=(ROOT / "tests/fixtures/os_windows_thread_setup.c",),
                    )
                    for scenario in ("normal", "task-affinity", "task-boost", "task-priority",
                                     "dispatcher-boost", "dispatcher-affinity"):
                        with self.subTest(mutant=mutant, scenario=scenario):
                            result = os_suites.run_native([str(binary), scenario], timeout=10)
                            refused = scenario != "normal"
                            self.assertEqual(result.returncode, 7 if refused and not mutant else 0, result)
                            self.assertEqual(result.stderr, "")
                            if refused:
                                self.assertIn("native setup refused=" + scenario, result.stdout)
                                if not mutant:
                                    self.assertIn("lifecycle=Closed state=Failed reason=7", result.stdout)
                            self.assertEqual("state=Ready" in result.stdout, not refused or mutant, result)

    def test_static_kernel_object_types_and_queue_storage(self):
        import os_suites

        with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
            directory = Path(temporary)
            with os_suites.native_session(directory, "kernel-object-types"):
                binary, _ = os_suites.build(
                    directory, str(ROOT / "tests/fixtures/os_kernel_objects.c"),
                )
                result = os_suites.run_native([str(binary)], timeout=10)
                self.assertEqual(result.returncode, 0, result)
                self.assertEqual(result.stdout, "static task and queue types; task stack identity; bounded FIFO/front/overwrite/reset observed\n")
                self.assertEqual(result.stderr, "")

    def test_queue_front_pointer_bounds_and_order_mutations(self):
        import os_suites

        candidate = """        if( pxQueue->u.xQueue.pcReadFrom == pxQueue->pcHead )
        {
            pxQueue->u.xQueue.pcReadFrom = ( pxQueue->u.xQueue.pcTail - pxQueue->uxItemSize );
        }
        else
        {
            pxQueue->u.xQueue.pcReadFrom -= pxQueue->uxItemSize;
            mtCOVERAGE_TEST_MARKER();
        }"""
        old = """        pxQueue->u.xQueue.pcReadFrom -= pxQueue->uxItemSize;

        if( pxQueue->u.xQueue.pcReadFrom < pxQueue->pcHead )
        {
            pxQueue->u.xQueue.pcReadFrom = ( pxQueue->u.xQueue.pcTail - pxQueue->uxItemSize );
        }
        else
        {
            mtCOVERAGE_TEST_MARKER();
        }"""
        decrement = "pxQueue->u.xQueue.pcReadFrom -= pxQueue->uxItemSize;"
        observed = ("pxQueue->u.xQueue.pcReadFrom = Os_QueueTestPrevious("
                    "pxQueue->pcHead, pxQueue->u.xQueue.pcReadFrom, pxQueue->uxItemSize);")
        for mutant, code, error in (
            ("control", 0, ""),
            ("old-subtract", 78, "QUEUE_BEFORE_HEAD size=4 offset=0\n"),
            ("no-step", 9, "QUEUE_FRONT_ORDER round=0 front=29 back=29\n"),
            ("wrong-wrap", 9, "QUEUE_FRONT_ORDER round=0 front=17 back=29\n"),
        ):
            with self.subTest(mutant=mutant), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                native = os_suites.run_native
                compiled = []

                def observe(command, *, directory=directory, mutant=mutant, native=native, compiled=compiled, **options):
                    if "-std=c99" in command:
                        queue = directory / "kernel/queue.c"
                        text = queue.read_text(encoding="utf-8")
                        self.assertEqual(text.count(candidate), 1)
                        if mutant == "old-subtract":
                            text = text.replace(candidate, old)
                        self.assertEqual(text.count(decrement), 1)
                        text = text.replace(decrement, observed)
                        if mutant == "no-step":
                            text = text.replace(observed, "(void)pxQueue;")
                        elif mutant == "wrong-wrap":
                            wrap = "( pxQueue->u.xQueue.pcTail - pxQueue->uxItemSize )"
                            self.assertEqual(text.count(wrap), 1)
                            text = text.replace(wrap, "pxQueue->pcHead")
                        include = '#include "queue.h"'
                        self.assertEqual(text.count(include), 1)
                        text = text.replace(include, include + "\nextern int8_t *Os_QueueTestPrevious(int8_t *, int8_t *, size_t);")
                        queue.write_text(text, encoding="utf-8")
                        result = native(command, **options)
                        compiled.append(result.returncode)
                        return result
                    return native(command, **options)

                with os_suites.native_session(directory, "queue-front-bounds"), patch.object(os_suites, "run_native", side_effect=observe):
                    binary, _ = os_suites.build(
                        directory, str(ROOT / "tests/fixtures/os_kernel_objects.c"),
                        defines=("OS_QUEUE_POINTER_TESTS=1",),
                    )
                    self.assertEqual(compiled, [0])
                    result = os_suites.run_native([str(binary)], timeout=10)
                self.assertEqual(result.returncode, code, result)
                self.assertEqual(result.stderr, error, result)
                self.assertEqual(result.stdout, "static task and queue types; task stack identity; bounded FIFO/front/overwrite/reset observed\n" if code == 0 else "", result)


@unittest.skipUnless(platform.system() == "Linux", "Linux native facilities")
class NativeFacilityTests(unittest.TestCase):
    errno_functions = (
        "clock_gettime", "sysconf", "mmap", "mprotect", "munmap", "eventfd",
        "nanosleep", "sigaddset", "sigaction", "pthread_create",
    )

    def test_mutex_initial_owner_and_capacity_mutations(self):
        import os_suites

        for name, before, after, refused in (
            ("unowned", "} else if (owner != 0) {", "} else if (owner == 0) {", 28),
            ("capacity", "if (held_depth < 32u) {", "if (held_depth < 31u) {", 27),
        ):
            with self.subTest(mutant=name), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                source = target / "src/host/linux/Os_HostLinux.c"
                text = source.read_text(encoding="utf-8")
                self.assertEqual(text.count(before), 1)
                source.write_text(text.replace(before, after), encoding="utf-8")
                with os_suites.native_session(directory, "mutex-boundaries"):
                    binary, _ = os_suites.build(
                        directory, str(ROOT / "tests/fixtures/os_native_facilities.c"), target=target,
                    )
                    result = os_suites.run_native([str(binary), "owned-capacity"], timeout=10)
                    self.assertEqual(result.returncode, refused, result)
                    self.assertEqual(result.stdout, "", result)
                    self.assertEqual(result.stderr, "", result)

    def test_suspend_generation_wrap_preserves_context_and_terminal_stop(self):
        import os_suites

        for scenario in ("initial", "wrap", "wrap-mutant", "wrap-handler-mutant", "wrap-resume-mutant", "stop", "stop-mutant"):
            with self.subTest(scenario=scenario), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                source = target / "src/host/linux/Os_HostLinux.c"
                text = source.read_text(encoding="utf-8")
                if scenario.startswith("wrap"):
                    # Seed only the test copy; exercise real signals across the reserved stop value.
                    for field in ("requested", "acknowledged", "resumed"):
                        old = f"thread->{field} = 0;"
                        self.assertEqual(text.count(old), 1)
                        text = text.replace(old, f"thread->{field} = INT32_MAX - 2;")
                if scenario == "wrap-mutant":
                    old = "generation = (previous == (INT32_MAX - 1)) ? 1 : (previous + 1);"
                    self.assertEqual(text.count(old), 1)
                    text = text.replace(old, "generation = previous + 1;")
                elif scenario == "wrap-handler-mutant":
                    old = "if (generation != __atomic_load_n(&thread->acknowledged, __ATOMIC_ACQUIRE))"
                    self.assertEqual(text.count(old), 1)
                    text = text.replace(old, old.replace(" != ", " > "))
                elif scenario == "wrap-resume-mutant":
                    old = "if (resumed == generation)"
                    self.assertEqual(text.count(old), 1)
                    text = text.replace(old, "if (resumed >= generation)")
                fixture_text = (ROOT / "core/tests/fixtures/os/os_signal_lifecycle.c").read_text(encoding="utf-8")
                if scenario.startswith("stop"):
                    # Deterministically publish stop immediately before the competing request CAS.
                    matches = list(re.finditer(
                        r"(?m)^\s*if \(__atomic_compare_exchange_n\(&thread->requested,\s*"
                        r"&previous,\s*generation,\s*false,\s*__ATOMIC_ACQ_REL,\s*__ATOMIC_ACQUIRE\)\) \{",
                        text,
                    ))
                    self.assertEqual(len(matches), 1)
                    cas = matches[0].group()
                    replacement = cas if scenario == "stop" else (
                        "    __atomic_store_n(&thread->requested, generation, __ATOMIC_RELEASE);\n    if (1) {")
                    text = text.replace(cas, "    (void)Os_HostStopActor(thread, 2000u);\n" + replacement)
                    pauses = list(re.finditer(
                        r"(?m)^    if \(SuspendThread\(thread\) != 0u \|\| SuspendThread\(thread\) != 1u \|\|\s*"
                        r"valid_context\(thread, &paused\) == 0\) \{", fixture_text,
                    ))
                    self.assertEqual(len(pauses), 1)
                    first_pause = pauses[0].group()
                    fixture_text = fixture_text.replace(first_pause, (
                        "    if (SuspendThread(thread) != UINT32_MAX || SuspendThread(thread) != UINT32_MAX ||\n"
                        "        ResumeThread(thread) != UINT32_MAX || valid_context(thread, &stopped) == 0) {\n"
                        "        return 16;\n    }\n    puts(\"concurrent stop preserved\");\n    return 0;\n" + first_pause))
                source.write_text(text, encoding="utf-8")
                fixture = directory / "signal-lifecycle.c"
                fixture.write_text(fixture_text +
                    "\nvoid StartupHook(void) {}\nvoid ShutdownHook(StatusType reason) { (void)reason; }\n",
                    encoding="utf-8",
                )
                with os_suites.native_session(directory, "park-generation"):
                    binary, _ = os_suites.build(
                        directory, str(fixture), target=target,
                        link_flags=("-Wl,--wrap=pthread_detach",),
                    )
                    result = os_suites.run_native([str(binary)], timeout=10)
                    self.assertEqual(result.returncode, {
                        "wrap-mutant": 14, "wrap-handler-mutant": 5, "wrap-resume-mutant": 17,
                        "stop-mutant": 16,
                    }.get(scenario, 0), result)
                    self.assertEqual(result.stderr, "", result)
                    expected = {
                        "wrap-mutant": "", "wrap-handler-mutant": "", "wrap-resume-mutant": "",
                        "stop-mutant": "", "stop": "concurrent stop preserved\n",
                    }.get(scenario, "suspend=acknowledged resume=progress context=preserved "
                          "terminal=irreversible running=stopped closed=detached\n")
                    self.assertEqual(result.stdout, expected, result)

    def test_native_handle_errors_and_resource_reuse(self):
        import os_suites

        with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
            directory = Path(temporary)
            with os_suites.native_session(directory, "event-errors"):
                binary, _ = os_suites.build(
                    directory, str(ROOT / "tests/fixtures/os_event_errors.c"),
                    link_flags=tuple(f"-Wl,--wrap={name}" for name in (
                        "poll", "read", "__poll_chk", "__read_chk", "write", "close", "pthread_mutex_destroy", "clock_gettime",
                    )),
                )
                for scenario in ("fortify-read", "fortify-poll"):
                    with self.subTest(scenario=scenario):
                        result = os_suites.run_native([str(binary), scenario], timeout=10)
                        self.assertEqual(result.returncode, -6, result)
                        self.assertEqual(result.stdout, "", result)
                        self.assertIn("buffer overflow detected", result.stderr, result)
                for scenario in ("poll-error", "poll-error-infinite", "poll-invalid", "poll-hangup", "poll-revents-error",
                                 "poll-interrupted", "read-error", "read-zero", "read-short",
                                 "read-interrupted", "wait-read-error", "wait-read-interrupted",
                                 "wait-read-empty", "wait-read-zero", "wait-read-short",
                                 "write-error", "write-interrupted", "mutex-reuse", "mutex-close-error", "event-close-error-reuse",
                                 "close-interrupted", "clock-single-normal", "clock-any-normal", "clock-all-normal",
                                 "clock-any-initial", "clock-all-initial", "clock-single-later", "clock-any-later", "clock-all-later"):
                    with self.subTest(scenario=scenario):
                        result = os_suites.run_native([str(binary), scenario], timeout=10)
                        self.assertEqual(result.returncode, 0, result)
                        self.assertEqual(result.stdout, "event success/reset/failure/interruption observed\n")
                        self.assertEqual(result.stderr, "")

    def test_concurrent_event_construction_and_rollback(self):
        import os_suites

        with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
            directory = Path(temporary)
            with os_suites.native_session(directory, "native-event-release"):
                binary, _ = os_suites.build(
                    directory, str(ROOT / "tests/fixtures/os_native_event_release.c"),
                    extra_sources=(ROOT / "runtime/os/src/Os_HostServices.c",),
                    link_flags=("-Wl,--wrap=eventfd",),
                )
                for scenario in ("success", "failure"):
                    with self.subTest(scenario=scenario):
                        result = os_suites.run_native([str(binary), scenario], timeout=10)
                        self.assertEqual(result.returncode, 0, result)
                        self.assertEqual(result.stdout, "concurrent native event construction and failure reuse observed\n")
                        self.assertEqual(result.stderr, "")

    def test_sleep_remainder_protocol_and_mutations(self):
        import os_suites

        call = "status = nanosleep(&pause_time, &pause_time);"
        loop = "} while ((status != 0) && (errno == EINTR));"
        trace = "sleep_request call=1 sec=1 nsec=250000000 errno=0\n"
        for mutation in ("control", "original-request", "discard-remainder", "no-retry", "wrong-error"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                source = target / "src/host/linux/Os_HostLinux.c"
                text = source.read_text(encoding="utf-8")
                self.assertEqual(text.count(call), 1)
                self.assertEqual(text.count(loop), 1)
                if mutation == "original-request":
                    text = text.replace(call, "pause_time.tv_sec = 1; pause_time.tv_nsec = 250000000L; " + call)
                elif mutation == "discard-remainder":
                    text = text.replace(call, "status = nanosleep(&pause_time, NULL);")
                elif mutation == "no-retry":
                    text = text.replace(loop, "} while (0); (void)status;")
                elif mutation == "wrong-error":
                    text = text.replace(loop, "} while ((status != 0) && (errno != 0));")
                source.write_text(text, encoding="utf-8")
                with os_suites.native_session(directory, "sleep-protocol"):
                    binary, _ = os_suites.build(
                        directory, str(ROOT / "tests/fixtures/os_native_errno.c"), target=target,
                        link_flags=tuple(f"-Wl,--wrap={name}" for name in self.errno_functions),
                    )
                    cases = ("sleep-remainder", "sleep-success", "sleep-error") if mutation == "control" else (
                        "sleep-error" if mutation == "wrong-error" else "sleep-remainder",
                    )
                    for case in cases:
                        result = os_suites.run_native([str(binary), case], timeout=10)
                        if mutation == "control":
                            kind = {"sleep-remainder": 1, "sleep-success": 2, "sleep-error": 3}[case]
                            calls = 3 if kind == 1 else 1
                            self.assertEqual(result.returncode, 0, result)
                            self.assertEqual(result.stdout, f"sleep_protocol case={kind} calls={calls}\n")
                            self.assertEqual(result.stderr, trace + (
                                "sleep_request call=2 sec=0 nsec=750000000 errno=0\n"
                                "sleep_request call=3 sec=0 nsec=250000000 errno=0\n" if kind == 1 else ""
                            ))
                        else:
                            self.assertEqual(result.stdout, "", result)
                            code, tail = {
                                "original-request": (91, "sleep_request call=2 sec=1 nsec=250000000 errno=0\nsleep_request_mismatch call=2\n"),
                                "discard-remainder": (91, "sleep_request call=2 sec=1 nsec=250000000 errno=0\nsleep_request_mismatch call=2\n"),
                                "no-retry": (92, "sleep_count expected=3 actual=1\n"),
                                "wrong-error": (93, "sleep_request call=2 sec=1 nsec=250000000 errno=0\nsleep_extra_call case=3 call=2\n"),
                            }[mutation]
                            self.assertEqual(result.returncode, code, result)
                            self.assertEqual(result.stderr, trace + tail, result)

    def test_native_errno_across_initialization_wait_and_cleanup(self):
        import os_suites

        with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
            directory = Path(temporary)
            with os_suites.native_session(directory, "native-errno"):
                binary, _ = os_suites.build(
                    directory, str(ROOT / "tests/fixtures/os_native_errno.c"),
                    link_flags=tuple(f"-Wl,--wrap={name}" for name in self.errno_functions),
                )
                for scenario in ("normal", "sleep-interrupted", "protect-error", "create-error"):
                    with self.subTest(scenario=scenario):
                        result = os_suites.run_native([str(binary), scenario], timeout=10)
                        self.assertEqual(result.returncode, 0, result)
                        self.assertEqual(result.stdout, "native errno initialized across signal, clock, sleep, mapping, event and cleanup calls\n")
                        self.assertEqual(result.stderr, "")
                for scenario in ("protect-rollback-error", "create-rollback-error"):
                    with self.subTest(scenario=scenario):
                        result = os_suites.run_native([str(binary), scenario], timeout=10)
                        self.assertEqual(result.returncode, 1, result)
                        self.assertEqual(result.stdout, "", result)
                        self.assertEqual(result.stderr, f"rollback_munmap_failed stage={scenario}\n", result)

    def test_kernel_calls_native_mutex_and_stack_capacity(self):
        import os_suites

        with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
            directory = Path(temporary)
            target = directory / "target"
            shutil.copytree(os_suites.TARGET, target)
            source = target / "src/host/linux/Os_HostLinux.c"
            text = source.read_text(encoding="utf-8")
            text = text.replace('#include "Os_LinuxNative.h"', '#include "Os_LinuxNative.h"\n'
                                'int Os_FacilityTryLock(pthread_mutex_t *);\n'
                                '#define pthread_mutex_trylock Os_FacilityTryLock')
            source.write_text(text, encoding="utf-8")
            with os_suites.native_session(directory, "native-facilities"):
                binary, _ = os_suites.build(
                    directory, str(ROOT / "tests/fixtures/os_native_facilities.c"), target=target,
                )
                for scenario in ("normal", "error"):
                    result = os_suites.run_native([str(binary), scenario], timeout=10)
                    self.assertEqual(result.returncode, 0, result)
                    self.assertEqual(result.stdout, "native facilities futex=2 mapping=1 mutex_owner=1 recursion=2 error=classified context=signal+self\n")
                    self.assertEqual(result.stderr, "")
                for scenario, receipt in (
                    ("owned-capacity", "native mutex initially-owned depth=32 overflow=rejected mask=restored\n"),
                    ("actor-id", "native actor identity=published completion=joined-once ended=rejected\n"),
                ):
                    with self.subTest(scenario=scenario):
                        result = os_suites.run_native([str(binary), scenario], timeout=10)
                        self.assertEqual(result.returncode, 0, result)
                        self.assertEqual(result.stdout, receipt)
                        self.assertEqual(result.stderr, "")
                if not os.environ.get("AUTOSAR_SANITIZE"):
                    depth = os_suites.run_native([str(binary), "depth"], timeout=15)
                    self.assertEqual(depth.returncode, 0, depth)
                    self.assertIn("native-stack depth=50000 completed", depth.stdout)
                    self.assertEqual(depth.stderr, "")
                    print(depth.stdout, end="", flush=True)
                    cet = os_suites.run_native([str(binary), "cet-depth"], timeout=15)
                    if cet.returncode == 77:
                        print(cet.stdout, end="", flush=True)
                    else:
                        self.assertEqual(cet.returncode, 0, cet)
                        self.assertIn("native-stack CET status=1 result=0", cet.stdout)
                        self.assertIn("native-stack depth=50000 completed", cet.stdout)
                        self.assertEqual(cet.stderr, "")
                        self.assertGreaterEqual(int(cet.stdout.split(" shadow=", 1)[1].split()[0]), 50001 * 8)
                        print(cet.stdout, end="", flush=True)
                        # Confirm the capacity repair on the same hardware and execution stack.
                        mutant_target = directory / "small-shadow-target"
                        shutil.copytree(target, mutant_target)
                        host = mutant_target / "src/host/linux/Os_HostLinux.c"
                        source = host.read_text(encoding="utf-8")
                        selected = "(reserve > OS_HOST_DEFAULT_STACK) ? reserve : OS_HOST_DEFAULT_STACK"
                        self.assertEqual(source.count(selected), 1)
                        host.write_text(source.replace(selected, "OS_HOST_DEFAULT_STACK"), encoding="utf-8")
                        mutant_output = directory / "small-shadow-output"
                        mutant_output.mkdir(mode=0o700)
                        mutant, _ = os_suites.build(
                            mutant_output, str(ROOT / "tests/fixtures/os_native_facilities.c"),
                            target=mutant_target,
                        )
                        small = os_suites.run_native([str(mutant), "cet-depth"], timeout=15)
                        self.assertIn("native-stack CET status=1 result=0", small.stdout)
                        shadow = int(small.stdout.split(" shadow=", 1)[1].split()[0])
                        if small.returncode == 0:
                            # Legacy clone allocates from RLIMIT_STACK instead of pthread size.
                            self.assertIn("native-stack depth=50000 completed", small.stdout)
                            self.assertGreaterEqual(shadow, 50001 * 8)
                            print("native-stack capacity mutation not_assessed: kernel already supplies sufficient shadow stack", flush=True)
                        else:
                            self.assertEqual(small.returncode, -11, small)
                            self.assertLess(shadow, 50001 * 8)
                            self.assertNotIn("native-stack depth=50000 completed", small.stdout)
                            print(f"native-stack capacity mutation exit={small.returncode} {small.stdout}", end="", flush=True)

    def test_native_thread_lifecycle(self):
        import os_suites

        with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
            directory = Path(temporary)
            with os_suites.native_session(directory, "thread-lifecycle"):
                binary, _ = os_suites.build(
                    directory,
                    str(ROOT / "tests/fixtures/os_native_thread_lifecycle.c"),
                    link_flags=tuple(
                        f"-Wl,--wrap={name}"
                        for name in (
                            "Os_StackRegister",
                            "Os_LinuxThreadId",
                            "pthread_create",
                            "pthread_join",
                            "pthread_detach",
                        )
                    ),
                )
                for scenario in (
                    "registered-normal",
                    "registration-error",
                    "identity-error",
                    "close-live",
                    "close-ended",
                    "join-error",
                    "detach-error",
                    "concurrent-wait",
                ):
                    with self.subTest(scenario=scenario):
                        result = os_suites.run_native(
                            [str(binary), scenario], timeout=10
                        )
                        fatal = scenario in {"registration-error", "identity-error"}
                        self.assertEqual(result.returncode, 1 if fatal else 0, result)
                        expected = {
                            "registration-error": "registration_rejected role=H\n",
                            "identity-error": "identity_requested\n",
                            "join-error": "join_rejected attempt=1\n",
                            "detach-error": "detach_rejected attempt=1\n",
                        }.get(scenario, "")
                        self.assertEqual(result.stderr, expected, result)
                        self.assertEqual(
                            result.stdout,
                            ""
                            if fatal
                            else (
                                "registered_actor entry=completed identity=published\n"
                                if scenario == "registered-normal"
                                else f"thread_lifecycle scenario={scenario} ownership=retained completion=observed\n"
                            ),
                            result,
                        )
            mutations = (
                (
                    "registration",
                    "if (registered == 0) {\n        ExitProcess(1u);",
                    "if (registered == 0) {\n        result = UINT32_MAX;",
                    (("registration-error", 5, "registration_rejected role=H\n"),),
                ),
                (
                    "identity",
                    "/* The actor owns its callback context and may already be running. */\n            ExitProcess(1u);",
                    "return NULL;",
                    (
                        (
                            "identity-error",
                            4,
                            "identity_requested\ncreation_returned_null\n",
                        ),
                    ),
                ),
                (
                    "early-close",
                    "closed = 0;\n        if (InterlockedCompareExchange(&thread->ended",
                    "closed = 1;\n        if (InterlockedCompareExchange(&thread->ended",
                    (
                        ("close-live", 50, "thread_state_rejected stage=live-close\n"),
                        (
                            "close-ended",
                            52,
                            "thread_state_rejected stage=ended-close\n",
                        ),
                    ),
                ),
                (
                    "failed-join",
                    "(void)InterlockedExchange(&handle->joined, 0);",
                    "(void)handle;",
                    (
                        (
                            "join-error",
                            51,
                            "join_rejected attempt=1\nthread_state_rejected stage=join-completion\n",
                        ),
                    ),
                ),
                (
                    "early-join",
                    "InterlockedCompareExchange(&handle->joined, 2, 0)",
                    "InterlockedExchange(&handle->joined, 1)",
                    (
                        (
                            "concurrent-wait",
                            54,
                            "thread_state_rejected stage=concurrent-completion\n",
                        ),
                    ),
                ),
                (
                    "failed-detach",
                    "(void)InterlockedExchange(&thread->joined, 0);",
                    "(void)thread;",
                    (
                        (
                            "detach-error",
                            53,
                            "detach_rejected attempt=1\nthread_state_rejected stage=detach-completion\n",
                        ),
                    ),
                ),
            )
            for name, before, after, scenarios in mutations:
                with self.subTest(mutation=name):
                    changed = directory / name
                    changed.mkdir(mode=0o700)
                    target = changed / "target"
                    shutil.copytree(os_suites.TARGET, target)
                    source = target / "src/host/linux/Os_HostLinux.c"
                    text = source.read_text(encoding="utf-8")
                    self.assertEqual(text.count(before), 1)
                    source.write_text(text.replace(before, after), encoding="utf-8")
                    with os_suites.native_session(changed, "thread-lifecycle-mutation"):
                        binary, _ = os_suites.build(
                            changed,
                            str(ROOT / "tests/fixtures/os_native_thread_lifecycle.c"),
                            target=target,
                            link_flags=tuple(
                                f"-Wl,--wrap={name}"
                                for name in (
                                    "Os_StackRegister",
                                    "Os_LinuxThreadId",
                                    "pthread_create",
                                    "pthread_join",
                                    "pthread_detach",
                                )
                            ),
                        )
                        for scenario, code, errors in scenarios:
                            result = os_suites.run_native(
                                [str(binary), scenario], timeout=10
                            )
                            self.assertEqual(
                                (result.returncode, result.stdout, result.stderr),
                                (code, "", errors),
                                result,
                            )
