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

    def test_public_status_names_and_unknown_fallback(self):
        binary = self.directory / ("status-names.exe" if os.name == "nt" else "status-names")
        result = self.execute([self.compiler, *self.flags,
                               str(ROOT / "tests/fixtures/host_status_names.c"),
                               str(ROOT / "runtime/host/src/Ecu_Status.c"), "-o", str(binary)],
                              "status-names-build")
        self.assertEqual(result.exit_code, 0, (result.stdout.read_text(), result.stderr.read_text()))
        result = self.execute([str(binary)], "status-names-run")
        self.assertEqual(result.exit_code, 0, (result.stdout.read_text(), result.stderr.read_text()))
        self.assertEqual(result.stderr.read_text(), "")
        self.assertEqual(result.stdout.read_text().splitlines(), [
            "OK", "CONFIG", "SIGNAL_ID", "SIGNAL_VALUE", "DIRECTION", "FRAME_ID", "FRAME_DLC",
            "CONTROLLER", "TIME", "TP_SEQUENCE", "TP_TIMEOUT", "TP_LENGTH", "TP_FLOW", "TP_BUSY",
            "IO", "NVM", "CAN_BUSY", "INTERNAL",
        ])

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

    @unittest.skipUnless(platform.system() == "Linux", "glibc assertion receipts are Linux-only")
    def test_arti_invalid_switch_selectors_retain_fatal_guards(self):
        import os_suites

        for name, scenario, selector in [(None, None, None),
                                          ("Os_ArtiTask", "reject-task", 99),
                                          ("Os_ArtiService", "reject-service", 255)]:
            with self.subTest(function=name), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                if name is not None:
                    source = target / "src/Os_Arti.c"
                    text = source.read_text(encoding="utf-8")
                    start = text.index("OS_MEMMAP void " + name + "(")
                    end = text.index("\n}\n", start) + 2
                    body = text[start:end]
                    guard = "if (valid == FALSE) {\n        assert(0);\n    }"
                    self.assertEqual(body.count(guard), 1)
                    source.write_text(text[:start] + body.replace(guard, "if (valid == FALSE) { (void)0; }")
                                      + text[end:], encoding="utf-8")
                with os_suites.native_session(directory, "arti-switch-guard"):
                    binary, _ = os_suites.build(directory, "arti.c", ("OS_ARTI_TESTS",), target=target)
                    if name is None:
                        observations = os_suites.check_arti(binary)
                        self.assertEqual(len(observations), 11)
                        print("arti-switch control normal=9 rejected=2 SIGABRT", flush=True)
                    else:
                        result = os_suites.execute(binary, scenario)
                        self.assertEqual(result["exit"], 91, result)
                        self.assertEqual(result["stdout"], "", result)
                        self.assertEqual(result["stderr"],
                                         f"arti_reject function={name} selector={selector}", result)
                        print(f"arti-switch removed={name} exit=91 assertion-bypassed", flush=True)

    def test_rejected_kernel_state_preserves_output_and_critical_cleanup(self):
        import os_suites

        anchor = "OS_MEMMAP void Os_BackendGuardService(void)\n{"
        observer = "\n    extern void Os_StateGuardObserved(void);\n    Os_StateGuardObserved();"
        variants = [
            ("control", None, None, ""),
            ("success-status", "status = E_OS_STATE;", "status = E_OK;",
             "STATE_REJECTION result=0 output=99 queries=1 guards=3 enter=1 exit=1"),
            ("changed-output", "status = E_OS_STATE;", "*state = SUSPENDED; status = E_OS_STATE;",
             "STATE_REJECTION result=7 output=3 queries=1 guards=2 enter=1 exit=1"),
            ("missing-release", "taskEXIT_CRITICAL();", "(void)0;",
             "STATE_REJECTION result=7 output=99 queries=1 guards=2 enter=1 exit=0"),
            ("extra-guard", "if (status == E_OK) {\n                Os_BackendGuardService();\n            }",
             "Os_BackendGuardService();",
             "STATE_REJECTION result=7 output=99 queries=1 guards=3 enter=1 exit=1"),
        ]
        for name, old, new, error in variants:
            with self.subTest(variant=name), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                source = target / "src/Os_Backend.c"
                text = source.read_text(encoding="utf-8")
                self.assertEqual(text.count(anchor), 1)
                text = text.replace(anchor, anchor + observer)
                if old is not None:
                    start = text.index("OS_MEMMAP StatusType Os_BackendState(")
                    end = text.index("#define OS_STOP_SEC_CODE", start)
                    body = text[start:end]
                    self.assertEqual(body.count(old), 1)
                    text = text[:start] + body.replace(old, new) + text[end:]
                source.write_text(text, encoding="utf-8")
                shutil.copyfile(ROOT / "tests/fixtures/os_state_switch.c", target / "tests/os_state_switch.c")
                with os_suites.native_session(directory, "state-switch-" + name):
                    binary, _ = os_suites.build(
                        directory, "os_state_switch.c", target=target,
                        link_flags=tuple("-Wl,--wrap=" + function for function in (
                            "eTaskGetState", "vPortEnterCritical", "vPortExitCritical")),
                    )
                    if old is None:
                        self.assertGreater(len(os_suites.check_lifecycle(binary)), 3)
                    else:
                        result = os_suites.execute(binary, "normal")
                        self.assertEqual(result["exit"], 63, result)
                        self.assertEqual(result["stdout"], "", result)
                        self.assertEqual(result["stderr"], error, result)

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

        retained = os.environ.get("AUTOSAR_OS_OUTPUT_DIRECTORY")
        output = Path(retained) / "kernel-object-types" if retained else None
        with (os_suites.retained_workspace(output) as directory, os_suites.host_target(),
              os_suites.native_session(directory, "kernel-object-types")):
            binary, metadata = os_suites.build(
                directory, str(ROOT / "tests/fixtures/os_kernel_objects.c"),
            )
            result = os_suites.run_native([str(binary)], timeout=10)
            self.assertEqual(result.returncode, 0, result)
            self.assertEqual(result.stdout, "static task and queue types; task stack identity; bounded FIFO/front/overwrite/reset observed\n")
            self.assertEqual(result.stderr, "")

            # These configured-out definitions must not leave callable declarations.
            unavailable = (
                "pvPortCalloc", "pvPortMalloc", "vPortDefineHeapRegions", "vPortFree",
                "vPortGetHeapStats", "vPortHeapResetState", "vPortInitialiseBlocks",
                "xPortGetFreeHeapSize", "xPortGetMinimumEverFreeHeapSize",
                "xPortResetHeapMinimumEverFreeHeapSize", "pvTaskIncrementMutexHeldCount",
                "uxTaskBasePriorityGet", "uxTaskBasePriorityGetFromISR", "vTaskDelay",
                "vTaskDelete", "vTaskPlaceOnEventListRestricted",
                "vTaskPriorityDisinheritAfterTimeout", "xTaskDelayUntil",
                "xTaskGetSchedulerState", "xTaskPeriodicDelay", "xTaskPriorityDisinherit",
                "xTaskPriorityInherit", "vQueueWaitForMessageRestricted", "xQueueCreateMutex",
                "xQueueCreateMutexStatic", "xQueueGiveMutexRecursive", "xQueueTakeMutexRecursive",
            )
            available = ("xTaskCreateStatic", "xQueueGenericCreateStatic", "uxTaskPriorityGet",
                         "vTaskSuspend", "xTaskGetHandle", "xTaskGetTickCount", "xInsideInterrupt")
            unavailable_types = ("HeapRegion_t", "HeapStats_t")
            command = metadata["command"]
            self.assertEqual(command[-2], "-o")
            flags = [argument for argument in command[:-2]
                     if not argument.endswith(".c") and not argument.startswith("-l")]
            for label, names in (("available", available), ("unavailable", unavailable),
                                 ("unavailable-types", unavailable_types)):
                probe = directory / f"{label}.c"
                expressions = [f"sizeof({name})" if label == "unavailable-types" else f"&{name}"
                               for name in names]
                probe.write_text(
                    '#include "FreeRTOS.h"\n#include "task.h"\n#include "queue.h"\n'
                    "int main(void)\n{\n" +
                    "".join(f"    (void){expression};\n" for expression in expressions) +
                    "    return 0;\n}\n", encoding="utf-8",
                )
                compiled = os_suites.run_native([*flags, "-fsyntax-only", str(probe)], timeout=30)
                if label == "available":
                    self.assertEqual(compiled.returncode, 0, compiled)
                else:
                    self.assertNotEqual(compiled.returncode, 0, compiled)
                    for name in names:
                        self.assertRegex(compiled.stderr, rf"error:[^\n]*\b{name}\b[^\n]*undeclared")
            print(f"kernel_declarations available={len(available)} unavailable={len(unavailable)} "
                  f"unavailable_types={len(unavailable_types)} strict_c99=OK", flush=True)

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
        "nanosleep", "sigaddset", "sigaction", "pthread_create", "sigemptyset", "sigfillset",
    )

    def test_native_atomic_storage_constraints(self):
        import os_suites

        cases = (("include/Os_HostTypes.h", (
            ("OS_ATOMIC32_WIDTH", "typedef LONG Os_Atomic32Type;",
             "typedef int64_t Os_Atomic32Type;"),
            ("OS_ATOMIC32_ALIGN", "typedef LONG Os_Atomic32Type;",
             "typedef LONG Os_Atomic32Type __attribute__((aligned(1)));"),
            ("OS_ATOMIC64_WIDTH", "typedef uint64_t Os_Atomic64Type;",
             "typedef uint32_t Os_Atomic64Type __attribute__((aligned(8)));"),
            ("OS_ATOMIC64_ALIGN", "typedef uint64_t Os_Atomic64Type;",
             "typedef uint64_t Os_Atomic64Type __attribute__((aligned(1)));"),
        )), ("include/Arti_Types.h", (
            ("ARTI_ATOMIC32_WIDTH", "typedef int32_t Arti_Atomic32Type;",
             "typedef int64_t Arti_Atomic32Type;"),
        )), ("src/host/linux/Os_HostLinuxTypes.h", (
            ("OS_HOST_LONG_WIDTH", "typedef int32_t LONG;", "typedef int64_t LONG;"),
        )))
        retained = os.environ.get("AUTOSAR_OS_OUTPUT_DIRECTORY")
        output = Path(retained) / "atomic-type-guards" if retained else None
        with (
            os_suites.host_target(),
            os_suites.retained_workspace(output) as directory,
            os_suites.native_session(directory, "atomic-type-guards"),
        ):
            flags = [os_suites.compiler(), "-std=c99", "-O1", "-Wall", "-Wextra",
                     "-Werror", "-pedantic", "-fexceptions",
                     *([] if os.name == "nt" else ["-pthread"]),
                     "-I" + str(os_suites.TARGET / "include"),
                     "-I" + str(os_suites.TARGET / "src"),
                     *([] if os.name == "nt" else ["-I" + str(os_suites.TARGET / "src/host/linux")]),
                     "-I" + str(ROOT / "runtime/bsw/include")]

            def compile_header(name, label, text):
                case = directory / label
                case.mkdir()
                header = case / name
                header.write_text(text, encoding="utf-8")
                source = case / "types.c"
                source.write_text(f'#include "{name}"\n', encoding="utf-8")
                return header, os_suites.run_native(
                    [*flags, "-c", str(source), "-o", str(case / "types.o")],
                    timeout=30, env={"LC_ALL": "C"},
                )

            for relative, variants in cases:
                name = Path(relative).name
                original = (os_suites.TARGET / relative).read_text(encoding="utf-8")
                _, control = compile_header(name, name + "-control", original)
                self.assertEqual(control.returncode, 0, control)
                self.assertEqual(control.stderr, "", control)
                for guard, before, after in variants:
                    with self.subTest(guard=guard):
                        self.assertEqual(original.count(before), 1)
                        changed = original.replace(before, after)
                        pattern = re.compile(r"(?m)^[ \t]*" + guard + r"\s*=\s*[^,]+,")
                        matches = list(pattern.finditer(changed))
                        self.assertEqual(len(matches), 1)
                        match = matches[0]
                        first = changed.count("\n", 0, match.start()) + 1
                        last = changed.count("\n", 0, match.end()) + 1
                        header, rejected = compile_header(name, guard, changed)
                        self.assertNotEqual(rejected.returncode, 0, rejected)
                        diagnostics = rejected.stderr.replace("\\", "/")
                        lines = re.findall(re.escape(header.as_posix()) + r":(\d+):\d+: error:", diagnostics)
                        self.assertTrue(any(first <= int(line) <= last for line in lines), rejected)
                        self.assertIn("negative", diagnostics, rejected)
                        _, unchecked = compile_header(
                            name, guard + "-disabled", pattern.sub(f"    {guard} = 1,", changed),
                        )
                        self.assertEqual(unchecked.returncode, 0, unchecked)
                        self.assertEqual(unchecked.stderr, "", unchecked)
                        print(f"atomic-guard={guard} invalid=rejected disabled=compiled", flush=True)

    def test_startup_gate_failures_stop_before_continuation(self):
        import os_suites

        retained = os.environ.get("AUTOSAR_OS_OUTPUT_DIRECTORY")
        with os_suites.host_target():
            for operation in ("wait", "publish", "context"):
                for unchecked in (False, True):
                    output = Path(retained) / "startup-gates" / f"{operation}-{unchecked}" if retained else None
                    # Subtest failures must leave through the retaining context first.
                    with self.subTest(operation=operation, unchecked=unchecked), os_suites.retained_workspace(output) as directory:
                        target = directory / "target"
                        shutil.copytree(os_suites.TARGET, target)
                        source = target / "src/Os_Backend.c"
                        text = source.read_text(encoding="utf-8")
                        stage = f"startup_gate_failure={operation}\n"
                        continued = f"startup_gate_continued={operation}\n"
                        return_type, arguments, unused, result = (
                            ("DWORD", "HANDLE gate, DWORD interval", "(void)gate;\n    (void)interval;", "WAIT_FAILED")
                            if operation == "wait" else
                            ("int", "HANDLE gate", "(void)gate;", "0")
                        )
                        helper = (
                            f"OS_MEMMAP static {return_type} startup_gate_failure({arguments})\n{{\n"
                            f"    {unused}\n"
                            f'    (void)Os_HostWriteBytes(1, "startup_gate_failure={operation}\\n", {len(stage)}u);\n'
                            f"    return {result};\n}}\n"
                            "OS_MEMMAP static void __attribute__((noreturn)) startup_gate_continued(void)\n{\n"
                            f'    (void)Os_HostWriteBytes(1, "startup_gate_continued={operation}\\n", {len(continued)}u);\n'
                            "    ExitProcess(91u);\n}\n"
                        )
                        if operation == "context":
                            helper = helper[helper.index("OS_MEMMAP static void"):]
                        anchor = "OS_MEMMAP static DWORD WINAPI native_task_start(void *argument)"
                        self.assertEqual(text.count(anchor), 1)
                        text = text.replace(anchor, helper + anchor)
                        if operation == "context":
                            text = text.replace(
                                anchor + "\n{", anchor + "\n{\n    argument = NULL;"
                            )
                            old = ("    if (start == NULL) {\n"
                                   "        ExitProcess(E_OS_STATE);\n    }\n")
                            new = old.replace(
                                "        ExitProcess(E_OS_STATE);",
                                f'        (void)Os_HostWriteBytes(1, "startup_gate_failure=context\\n", {len(stage)}u);\n'
                                "        ExitProcess(E_OS_STATE);",
                            )
                            fatal = "ExitProcess(E_OS_STATE);"
                        elif operation == "wait":
                            old = ("    if (WaitForSingleObject(start->gate, INFINITE) != WAIT_OBJECT_0) {\n"
                                   "        ExitProcess(E_OS_STATE);\n    }\n")
                            new = old.replace("WaitForSingleObject", "startup_gate_failure")
                            fatal = "ExitProcess(E_OS_STATE);"
                        else:
                            old = ("    ++task_thread_count;\n"
                                   "    if (Os_HostSetEvent(start->gate) == 0) {\n"
                                   "        Os_BackendShutdown(E_OS_STATE);\n    }\n")
                            new = old.replace("Os_HostSetEvent", "startup_gate_failure")
                            fatal = "Os_BackendShutdown(E_OS_STATE);"
                        if unchecked:
                            self.assertEqual(new.count(fatal), 1)
                            new = new.replace(fatal, "(void)0;")
                        self.assertEqual(text.count(old), 1)
                        source.write_text(text.replace(old, new + "    startup_gate_continued();\n"), encoding="utf-8")
                        build = directory / "build"
                        build.mkdir()
                        with os_suites.native_session(directory, "startup-gate"):
                            binary, _ = os_suites.build(build, "native_stack.c", target=target)
                            observation = os_suites.execute(binary, "normal")
                        self.assertEqual(observation["exit"], 91 if unchecked else 7, observation)
                        self.assertEqual(observation["stderr"], (stage + (continued if unchecked else "")).strip(), observation)
                        if unchecked or operation in {"wait", "context"}:
                            self.assertEqual(observation["stdout"], "", observation)
                        else:
                            self.assertIn("lifecycle=Closed state=Failed reason=7", observation["stdout"], observation)
                            self.assertIn(" threads=3 ", observation["stdout"], observation)
                        print(observation, flush=True)

    def test_stack_fault_and_ordinary_close_publish_reason_atomically(self):
        import os_suites

        variants = ("atomic", "ordinary-write") if os.environ.get("AUTOSAR_SANITIZE") == "thread" else ("atomic",)
        retained = os.environ.get("AUTOSAR_OS_OUTPUT_DIRECTORY")
        with os_suites.host_target():
            for variant in variants:
                output = Path(retained) / "shutdown-reason" / variant if retained else None
                with self.subTest(variant=variant), os_suites.retained_workspace(output) as directory:
                    target = directory / "target"
                    shutil.copytree(os_suites.TARGET, target)
                    source = target / "src/Os_Backend.c"
                    text = source.read_text()
                    anchor = "#include <string.h>\n"
                    self.assertEqual(text.count(anchor), 1)
                    text = text.replace(anchor, anchor + "void Os_TestBeforeReasonStore(StatusType value);\n"
                                        "void Os_TestAfterReasonStore(StatusType value);\n", 1)
                    pattern = r"(?m)^( +)(__atomic_store_n\(&shutdown_reason, (E_OS_STACKFAULT|error), __ATOMIC_SEQ_CST\);)$"
                    self.assertEqual(len(re.findall(pattern, text)), 4)

                    def handshake(match, variant=variant):
                        indent, operation, value = match.groups()
                        if variant == "ordinary-write":
                            operation = "shutdown_reason = " + value + ";"
                        return (indent + "Os_TestBeforeReasonStore(" + value + ");\n"
                                + indent + operation + "\n" + indent + "Os_TestAfterReasonStore(" + value + ");")

                    source.write_text(re.sub(pattern, handshake, text))
                    shutil.copy2(ROOT / "tests/fixtures/os_shutdown_reason.c", target / "tests/shutdown_reason.c")
                    build = directory / "build"
                    build.mkdir()
                    with os_suites.native_session(directory, "shutdown-reason"):
                        binary, _ = os_suites.build(build, "shutdown_reason.c", ("OS_STACK_TESTS",), target=target)
                        if variant == "atomic":
                            observation = os_suites.execute(binary, "reason-publication")
                            self.assertEqual(observation["exit"], 13, observation)
                            self.assertEqual(observation["stderr"], "reason_publication writers=2 fault=H", observation)
                            self.assertIn("lifecycle=Closed state=Failed reason=13", observation["stdout"], observation)
                            self.assertIn("FAULT role=H ", observation["stdout"], observation)
                            self.assertNotIn("X", observation["stdout"].split("trace=")[1].split()[0], observation)
                        else:
                            with self.assertRaises(AssertionError) as rejected:
                                os_suites.execute(binary, "reason-publication")
                            observation = rejected.exception.args[0]
                            self.assertEqual(observation["exit"], 13, observation)
                            self.assertIn("WARNING: ThreadSanitizer: data race", observation["stderr"], observation)
                            self.assertIn("shutdown_reason", observation["stderr"], observation)
                            self.assertIn("Os_BackendRequestShutdown", observation["stderr"], observation)
                            self.assertIn("Os_BackendSignalStackFault", observation["stderr"], observation)
                    print(observation, flush=True)

    def test_controller_does_not_read_a_pending_first_fault_snapshot(self):
        import os_suites

        variants = ("healthy-owner", "global-read") if os.environ.get("AUTOSAR_SANITIZE") == "thread" else ("healthy-owner",)
        retained = os.environ.get("AUTOSAR_OS_OUTPUT_DIRECTORY")
        helpers = r"""
static volatile LONG snapshot_phase;
void Os_TestBeforeFirstSnapshot(void)
{
    __atomic_store_n(&snapshot_phase, 1, __ATOMIC_SEQ_CST);
    while (__atomic_load_n(&snapshot_phase, __ATOMIC_SEQ_CST) < 2) {}
}
void Os_TestAfterFirstSnapshot(void)
{
    while (__atomic_load_n(&snapshot_phase, __ATOMIC_SEQ_CST) < 3) {}
}
void Os_TestBeforeFaultReader(void)
{
    if (__atomic_load_n(&snapshot_phase, __ATOMIC_SEQ_CST) != 1) { ExitProcess(91u); }
    __atomic_store_n(&snapshot_phase, 2, __ATOMIC_SEQ_CST);
}
void Os_TestAfterFaultReader(void)
{
    const char stage[] = "fault_snapshot reader=healthy first_payload=pending\n";
    __atomic_store_n(&snapshot_phase, 3, __ATOMIC_SEQ_CST);
    if (Os_HostWriteBytes(1, stage, sizeof(stage) - 1u) == 0) { ExitProcess(92u); }
}
"""
        with os_suites.host_target():
            for variant in variants:
                output = Path(retained) / "fault-snapshot" / variant if retained else None
                with self.subTest(variant=variant), os_suites.retained_workspace(output) as directory:
                    target = directory / "target"
                    shutil.copytree(os_suites.TARGET, target)
                    stack = target / "src/host/linux/Os_StackLinux.c"
                    text = stack.read_text()
                    anchor = '#include "Os_Backend.h"\n'
                    self.assertEqual(text.count(anchor), 1)
                    text = text.replace(anchor, anchor + "void Os_TestBeforeFirstSnapshot(void);\nvoid Os_TestAfterFirstSnapshot(void);\n", 1)
                    before = "        Os_Fault.thread_id = record->thread_id;\n"
                    after = "        __atomic_store_n(&Os_Fault.published, 1, __ATOMIC_RELEASE);\n"
                    self.assertEqual((text.count(before), text.count(after)), (1, 1))
                    stack.write_text(text.replace(before, "        Os_TestBeforeFirstSnapshot();\n" + before)
                                     .replace(after, "        Os_TestAfterFirstSnapshot();\n" + after))
                    backend = target / "src/Os_Backend.c"
                    text = backend.read_text()
                    anchor = "#include <string.h>\n"
                    self.assertEqual(text.count(anchor), 1)
                    text = text.replace(anchor, anchor + "void Os_TestBeforeFaultReader(void);\nvoid Os_TestAfterFaultReader(void);\n", 1)
                    read = ("                const LONG healthy =\n"
                            "                    InterlockedCompareExchange(&controller_healthy[controller], 0, 0);\n")
                    self.assertEqual(text.count(read), 1)
                    if variant == "global-read":
                        replacement = "                const LONG healthy = (current->thread_id != Os_Fault.thread_id) ? 1 : 0;\n                (void)controller;\n"
                    else:
                        replacement = read
                    backend.write_text(text.replace(read, "                Os_TestBeforeFaultReader();\n" + replacement
                                                     + "                Os_TestAfterFaultReader();\n", 1))
                    fixture = target / "tests/native_stack.c"
                    text = fixture.read_text()
                    start = text.index("void Os_StackTestBeforeClose(void)")
                    stop = text.index("\nvoid StartupHook(void)", start)
                    # This case deliberately proceeds after the second local fault; the normal suite retains its count==2 oracle.
                    text = text[:start] + "void Os_StackTestBeforeClose(void) {}\n" + text[stop:]
                    old = "current->thread_id == Os_Fault.thread_id"
                    self.assertEqual(text.count(old), 1)
                    text = text.replace(old, "Os_StackThreadHasFault(current->thread_id) != 0", 1)
                    fixture.write_text('#include "Os_HostIo.h"\n' + text + helpers)
                    build = directory / "build"
                    build.mkdir()
                    with os_suites.native_session(directory, "fault-snapshot"):
                        binary, _ = os_suites.build(build, "native_stack.c", target=target)
                        if variant == "healthy-owner":
                            observation = os_suites.execute(binary, "concurrent")
                            self.assertEqual(observation["exit"], 13, observation)
                            self.assertEqual(observation["stderr"], "fault_snapshot reader=healthy first_payload=pending", observation)
                            self.assertIn("FAULT role=T ", observation["stdout"], observation)
                            self.assertIn("FAULT role=D ", observation["stdout"], observation)
                            self.assertIn("lifecycle=Closed state=Failed reason=13", observation["stdout"], observation)
                        else:
                            with self.assertRaises(AssertionError) as rejected:
                                os_suites.execute(binary, "concurrent")
                            observation = rejected.exception.args[0]
                            self.assertEqual(observation["exit"], 13, observation)
                            for evidence in ("WARNING: ThreadSanitizer: data race", "Os_Fault", "publish_fault", "Os_BackendShutdown"):
                                self.assertIn(evidence, observation["stderr"], observation)
                    print(observation, flush=True)

    def test_retired_interrupt_snapshot_drops_borrowed_parameters(self):
        import os_suites

        clear = "interrupt_frames[interrupt_depth].error = (Os_ErrorContextType){0};"
        for retained in (False, True):
            with self.subTest(retained=retained), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                backend = target / "src/Os_Backend.c"
                text = backend.read_text(encoding="utf-8")
                self.assertEqual(text.count(clear), 1)
                if retained:
                    text = text.replace(clear, "")
                anchor = "    current_interrupt = interrupt_frames[interrupt_depth].interrupt;\n}\n"
                self.assertEqual(text.count(anchor), 1)
                text = text.replace(anchor, anchor +
                    "\nOS_MEMMAP const void *Os_TestRetiredErrorPointer(void)\n{\n"
                    "    return interrupt_frames[interrupt_depth].error.parameters.service_GetTaskState.State;\n}\n")
                backend.write_text(text, encoding="utf-8")
                fixture = target / "tests/nested_interrupts.c"
                text = fixture.read_text(encoding="utf-8")
                include = '#include "Os_Backend.h"'
                self.assertEqual(text.count(include), 1)
                text = text.replace(include, include + "\nconst void *Os_TestRetiredErrorPointer(void);")
                anchor = ("check(OSErrorGetServiceId() == service && OSError_GetTaskState_TaskID() == 99u);\n"
                          "        check(OSError_GetTaskState_State() == &parent_state && parent_state == 77u);")
                self.assertEqual(text.count(anchor), 1)
                observe = "check(Os_TestRetiredErrorPointer() == NULL);"
                text = text.replace(anchor, anchor + "\n        " + observe)
                line = next(i for i, value in enumerate(text.splitlines(), 1) if value.strip() == observe)
                fixture.write_text(text, encoding="utf-8")
                with os_suites.native_session(directory, "retired-error-parameters"):
                    binary, _ = os_suites.build(directory, "nested_interrupts.c", target=target)
                    result = os_suites.execute(binary, "hook")
                self.assertEqual(result["exit"], 7 if retained else 0, result)
                self.assertEqual(result["stderr"], (
                    f"nested assertion failed: {fixture}:{line} invariant=Os_TestRetiredErrorPointer() == NULL reason=7"
                    if retained else ""), result)
                expected = ("children=0 grandchildren=1 helpers=0 errors=1 depth=1 maximum=2 trace=APEG reason=7"
                            if retained else "children=1 grandchildren=1 helpers=1 errors=1 depth=0 maximum=2 trace=APEGeCcpHB reason=0")
                self.assertIn("nested scenario=hook " + expected, result["stdout"])
                print(f"retired-error retained={retained} exit={result['exit']}", flush=True)

    def test_disabled_error_parameter_access_retains_no_pointer_fields(self):
        import os_suites

        fields = ("service_GetTaskID.TaskID", "service_GetTaskState.State", "service_GetEvent.Event",
                  "service_GetCounterValue.Value", "service_GetElapsedValue.Value",
                  "service_GetElapsedValue.ElapsedValue", "service_GetAlarmBase.Info",
                  "service_GetAlarm.Tick", "service_GetScheduleTableStatus.ScheduleStatus")
        for service_access, parameter_access, retained in ((1, 1, False), (0, 1, False),
                                                         (1, 0, False), (0, 0, False), (1, 0, True)):
            with self.subTest(service_access=service_access, parameter_access=parameter_access, retained=retained), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                fixture = target / "tests/error_hooks.c"
                text = fixture.read_text(encoding="utf-8")
                anchor = "void ErrorHook(StatusType Error)\n{"
                self.assertEqual(text.count(anchor), 1)
                observations = ("\n#if (OS_USE_PARAMETER_ACCESS == STD_OFF)\n"
                                "    const Os_ErrorParametersType *const captured = Os_ErrorParametersCurrent();\n")
                for field in fields:
                    observations += (f"    if (captured->{field} != NULL) {{\n"
                        f'        if (fprintf(stderr, "disabled_hook retained field={field}\\n") < 0) {{ ExitProcess(97u); }}\n'
                        "        if (fflush(stderr) != 0) { ExitProcess(97u); }\n"
                        "        ExitProcess(71u);\n    }\n")
                text = text.replace(anchor, anchor + observations + "#endif\n")
                fixture.write_text(text, encoding="utf-8")
                if retained:
                    source = target / "src/Os_Error.c"
                    text = source.read_text(encoding="utf-8")
                    selection = ("const Os_ErrorParametersType *const sources[2] = {&empty_parameters, arguments};\n"
                                 "        parameters = *sources[OS_USE_PARAMETER_ACCESS];")
                    self.assertEqual(text.count(selection), 1)
                    source.write_text(text.replace(selection, "parameters = *arguments;"), encoding="utf-8")
                with os_suites.native_session(directory, "disabled-error-parameters"):
                    binary, _ = os_suites.build(directory, "error_hooks.c", target=target,
                        defines=(f"OS_USE_GET_SERVICE_ID={service_access}", f"OS_USE_PARAMETER_ACCESS={parameter_access}"))
                    if retained:
                        result = os_suites.execute(binary, "normal")
                        self.assertEqual(result["exit"], 71, result)
                        self.assertEqual(result["stdout"], "", result)
                        self.assertEqual(result["stderr"], "disabled_hook retained field=service_GetTaskState.State", result)
                    else:
                        observations = os_suites.check_error_hooks(binary, service_access, parameter_access)
                        self.assertEqual(len(observations), 2)
                print(f"error-parameters service={service_access} access={parameter_access} retained={retained}", flush=True)

    def test_native_futex_errors_and_expected_wake_outcomes(self):
        import os_suites

        injected = "futex injected mode={mode} call={call} result={result}\n"
        park_error = "".join(injected.format(mode="park-error", call=i, result=result)
                             for i, result in enumerate((0, -11, -4, -110, -14), 1))
        controls = (
            ("wake-empty", 0, "futex completed mode=wake-empty calls=1 result=0\n", ""),
            ("wait-mismatch", 0, "futex completed mode=wait-mismatch calls=1 result=-11\n", ""),
            ("wait-timeout", 0, "futex completed mode=wait-timeout calls=1 result=-110\n", ""),
            ("wait-interrupted", 0, "futex completed mode=wait-interrupted calls=1 result=-4\n",
             injected.format(mode="wait-interrupted", call=1, result=-4)),
            ("wake-error", 1, "", injected.format(mode="wake-error", call=1, result=-22)),
            ("wait-error", 1, "", injected.format(mode="wait-error", call=1, result=-14)),
            ("park-error", 13, "", park_error),
        )
        for variant in ("control", "wake-unchecked", "wait-unchecked", "park-unchecked"):
            with self.subTest(variant=variant), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                native = target / "src/host/linux/Os_LinuxNative.c"
                text = native.read_text(encoding="utf-8")
                declaration = "OS_MEMMAP static long native_call("
                self.assertEqual(text.count(declaration), 1)
                text = text.replace(declaration, "OS_MEMMAP long Os_TestRealNativeCall(")
                anchor = "OS_MEMMAP int Os_LinuxFutexWait("
                self.assertEqual(text.count(anchor), 1)
                text = text.replace(anchor,
                    "extern long Os_TestNativeCall(long number, const volatile void *first, "
                    "long second, long third, const void *fourth);\n"
                    "#define native_call Os_TestNativeCall\n" + anchor)
                if variant == "wake-unchecked":
                    self.assertEqual(text.count("if (result < 0)"), 1)
                    text = text.replace("if (result < 0)", "(void)result;\n    if (0)")
                native.write_text(text, encoding="utf-8")
                for filename, before, after, selected in (
                    ("Os_HostLinux.c", "OS_MEMMAP static void futex_wait(",
                     "OS_MEMMAP void Os_TestFutexWait(", "wait-unchecked"),
                    ("Os_StackLinux.c", "OS_MEMMAP static void fault_park_forever(",
                     "OS_MEMMAP void Os_TestFutexPark(", "park-unchecked"),
                ):
                    source = target / "src/host/linux" / filename
                    text = source.read_text(encoding="utf-8")
                    self.assertEqual(text.count(before), 1)
                    text = text.replace(before, after)
                    old_name = "futex_wait(" if selected == "wait-unchecked" else "fault_park_forever("
                    new_name = "Os_TestFutexWait(" if selected == "wait-unchecked" else "Os_TestFutexPark("
                    text = text.replace(old_name, new_name)
                    if variant == selected:
                        pattern = (r"if \(\(result != 0\)\s*&&\s*\(result != -EAGAIN\)\s*&&\s*"
                                   r"\(result != -EINTR\)\s*&&\s*\(result != -ETIMEDOUT\)\) \{")
                        matches = list(re.finditer(pattern, text, re.DOTALL))
                        self.assertEqual(len(matches), 1)
                        text = text.replace(matches[0].group(), "(void)result;\n        if (0) {")
                    source.write_text(text, encoding="utf-8")
                cases = controls if variant == "control" else {
                    "wake-unchecked": (("wake-error", 51, "", injected.format(mode="wake-error", call=1, result=-22) +
                                        "futex rejected stage=wake-error-returned calls=1\n"),),
                    "wait-unchecked": (("wait-error", 52, "", injected.format(mode="wait-error", call=1, result=-14) +
                                        "futex rejected stage=wait-error-returned calls=1\n"),),
                    "park-unchecked": (("park-error", 54, "", park_error +
                                        "futex rejected stage=unexpected-retry calls=6\n"),),
                }[variant]
                with os_suites.native_session(directory, "native-futex-errors"):
                    binary, _ = os_suites.build(
                        directory, str(ROOT / "tests/fixtures/os_native_futex_errors.c"), target=target,
                    )
                    for scenario, code, output, error in cases:
                        with self.subTest(scenario=scenario):
                            result = os_suites.run_native([str(binary), scenario], timeout=10)
                            self.assertEqual(result.returncode, code, result)
                            self.assertEqual(result.stdout, output, result)
                            self.assertEqual(result.stderr, error, result)
                            print(f"futex variant={variant} scenario={scenario} exit={result.returncode}", flush=True)

    def test_finite_host_actor_restores_previous_altstack(self):
        self._finite_actor_altstack(mutants=False)

    def test_finite_host_actor_altstack_mutants(self):
        self._finite_actor_altstack(mutants=True)

    def _finite_actor_altstack(self, *, mutants):
        import os_suites

        variants = (
            ("control", None, None),
            ("omit-query", "Os_LinuxAltStack(NULL, &previous)", "0"),
            ("omit-restore", "Os_LinuxAltStack(&previous, NULL)", "0"),
            ("damage-state", "Os_LinuxAltStack(&previous, NULL)",
             "(previous.ss_size ^= 1u, Os_LinuxAltStack(&previous, NULL))"),
            ("ignore-query", "Os_LinuxAltStack(NULL, &previous) != 0",
             "(Os_LinuxAltStack(NULL, &previous), 0)"),
            ("ignore-restore", "Os_LinuxAltStack(&previous, NULL) != 0",
             "(Os_LinuxAltStack(&previous, NULL), 0)"),
        )
        for name, before, after in (variants[1:] if mutants else variants[:1]):
            with self.subTest(variant=name), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                if before is not None:
                    source = target / "src/host/linux/Os_HostLinux.c"
                    text = source.read_text(encoding="utf-8")
                    self.assertEqual(text.count(before), 1)
                    source.write_text(text.replace(before, after), encoding="utf-8")
                cases = {
                    "control": (("normal", 0, ""), ("observer", 0, ""),
                                ("query-error", 1, "altstack_injected query\n"),
                                ("restore-error", 1, "altstack_injected restore\n")),
                    "omit-query": (("normal", 21, "altstack_rejected stage=0 reason=sequence\n"),),
                    "omit-restore": (("normal", 24, "altstack_rejected stage=3 reason=completion-state\n"),),
                    "damage-state": (("normal", 23, "altstack_rejected stage=3 reason=restoration\n"),),
                    "ignore-query": (("query-error", 21,
                                     "altstack_injected query\naltstack_rejected stage=0 reason=sequence\n"),),
                    "ignore-restore": (("restore-error", 24,
                                       "altstack_injected restore\naltstack_rejected stage=3 reason=completion-state\n"),),
                }[name]
                with os_suites.native_session(directory, "altstack-" + name):
                    binary, _ = os_suites.build(
                        directory, str(ROOT / "tests/fixtures/os_native_altstack.c"), target=target,
                        link_flags=("-Wl,--wrap=Os_LinuxAltStack",),
                    )
                    for scenario, code, stderr in cases:
                        result = os_suites.run_native([str(binary), scenario], timeout=10)
                        self.assertEqual(result.returncode, code, result)
                        stdout = (f"altstack actor={scenario} stage={0 if scenario == 'observer' else 4} "
                                  "joined=1 closed=1\n") if code == 0 else ""
                        self.assertEqual(result.stdout, stdout, result)
                        self.assertEqual(result.stderr, stderr, result)

    def test_unwinder_initialization_precedes_native_threads(self):
        import os_suites

        variants = (
            ("control", None, None, None, "normal", 0),
            ("failure", None, None, None, "error", 7),
            ("omit", "src/Os_Backend.c", "if (Os_PortRestartPrepare() == 0)",
             "if (0)", "normal", 81),
            ("skip-provider", "src/Os_PortRestart.c", "_Unwind_Backtrace(&prepare_trace, NULL)",
             "((void)&prepare_trace, _URC_END_OF_STACK)", "normal", 81),
            ("ignore-failure", "src/Os_Backend.c", "if (Os_PortRestartPrepare() == 0)",
             "if ((Os_PortRestartPrepare(), 0))", "error", 82),
        )
        for name, source, before, after, scenario, code in variants:
            with self.subTest(variant=name), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = os_suites.TARGET
                if source is not None:
                    target = directory / "target"
                    shutil.copytree(os_suites.TARGET, target)
                    path = target / source
                    text = path.read_text(encoding="utf-8")
                    self.assertEqual(text.count(before), 1)
                    path.write_text(text.replace(before, after), encoding="utf-8")
                output = directory / "output"
                output.mkdir(mode=0o700)
                with os_suites.native_session(directory, "unwind-prepare"):
                    binary, _ = os_suites.build(
                        output, str(ROOT / "tests/fixtures/os_unwind_prepare.c"), target=target,
                        link_flags=("-Wl,--wrap=_Unwind_Backtrace", "-Wl,--wrap=pthread_create",
                                    "-Wl,--wrap=StartupHook"),
                    )
                    result = os_suites.run_native([str(binary), scenario], timeout=10)
                self.assertEqual(result.returncode, code, result)
                if code == 0:
                    self.assertRegex(result.stderr,
                                     r"\AUNWIND_PREPARE ready trace_calls=1 frames=[1-9][0-9]* before_threads=1\n\Z")
                    self.assertIn("lifecycle=Closed state=Ready reason=0", result.stdout)
                    self.assertIn("trace=ISRAD ", result.stdout)
                elif code == 7:
                    self.assertEqual(result.stderr, "UNWIND_PREPARE injected_failure\n")
                    self.assertIn("lifecycle=Closed state=Failed reason=7", result.stdout)
                    self.assertNotIn("state=Ready", result.stdout)
                else:
                    self.assertEqual(result.stdout, "")
                    expected = ("UNWIND_PREPARE thread_before_trace trace_calls=0 frames=0\n"
                                if code == 81 else "UNWIND_PREPARE injected_failure\n"
                                "UNWIND_PREPARE thread_after_failed_trace trace_calls=1 frames=0\n")
                    self.assertEqual(result.stderr, expected)
                print(f"unwind-prepare variant={name} exit={result.returncode} {result.stderr}", end="", flush=True)

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

    def test_signal_context_reads_only_kernel_control_fields(self):
        import os_suites

        prefix = "const size_t prefix = offsetof(struct ucontext, uc_sigmask) + sizeof(uint64_t);"
        sp_extent = "offsetof(struct ucontext, uc_mcontext.rsp) + sizeof(kernel.uc_mcontext.rsp)"
        variants = (
            ("control", None, None, 0, ""),
            ("user-mask", prefix, "const size_t prefix = sizeof(struct ucontext);", 41,
             "frame-copy rejected capture\n"),
            ("user-tail", "(void)memcpy((void *)&kernel, source, prefix);",
             "(void)memcpy((void *)&kernel, source, sizeof(ucontext_t));", 41,
             "frame-copy rejected capture\n"),
            ("sp-tail", sp_extent, "sizeof(kernel)", 42, "frame-copy rejected sp\n"),
        )
        for name, before, after, code, error in variants:
            with self.subTest(variant=name), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                source = target / "src/host/linux/Os_LinuxNative.c"
                text = source.read_text(encoding="utf-8")
                if before is not None:
                    self.assertEqual(text.count(before), 1)
                    text = text.replace(before, after)
                include = "#include <string.h>"
                self.assertEqual(text.count(include), 1)
                text = text.replace(include, include + "\n"
                    "void *Os_TestNativeCopy(void *destination, const void *source, "
                    "size_t length, size_t capacity, int moving);\n"
                    "#define memcpy(d, s, n) Os_TestNativeCopy((d), (s), (n), __builtin_object_size((d), 0), 0)\n"
                    "#define memmove(d, s, n) Os_TestNativeCopy((d), (s), (n), __builtin_object_size((d), 0), 1)\n")
                source.write_text(text, encoding="utf-8")
                with os_suites.native_session(directory, "signal-frame-projection"):
                    binary, _ = os_suites.build(
                        directory, str(ROOT / "tests/fixtures/os_native_facilities.c"), target=target,
                    )
                    result = os_suites.run_native([str(binary), "frame-copies"], timeout=10)
                self.assertEqual(result.returncode, code, result)
                self.assertEqual(result.stderr, error, result)
                self.assertEqual(result.stdout, "native facilities futex=2 mapping=1 mutex_owner=1 recursion=2 error=classified context=signal+self\n" if code == 0 else "", result)
                print(f"signal-frame variant={name} exit={result.returncode} {result.stderr}", end="", flush=True)

    def test_signal_context_content_mutants(self):
        import os_suites

        for name, before, after, code, stage in (
            ("rsp", "destination->Rsp = kernel.uc_mcontext.rsp;", "destination->Rsp = 0u;", 9, "rsp"),
            ("flags", "destination->ContextFlags = CONTEXT_CONTROL;", "destination->ContextFlags = 0u;", 9, "flags"),
            ("signal-sp", "return kernel.uc_mcontext.rsp;", "return 0u;", 9, "signal-sp"),
            ("fp-content", "sizeof(snapshot.__fpregs_mem));", "1u);", 9, "fp"),
            ("fp-owner", "void *copied = &destination->native.__fpregs_mem;",
             "void *copied = kernel.uc_mcontext.fpstate;", 11, "fp-owner"),
            ("self-copy", "if (kernel.uc_mcontext.fpstate != NULL) {",
             "destination->native = snapshot;\n    if (kernel.uc_mcontext.fpstate != NULL) {", 10, "self-copy"),
            ("absent-fp", "unsigned char *native = (unsigned char *)&snapshot;",
             ("unsigned char *native = (unsigned char *)&snapshot;\n"
              "    (void)memset(&snapshot.__fpregs_mem, 0xa5, sizeof(snapshot.__fpregs_mem));"), 45, "absent-fp"),
        ):
            with self.subTest(variant=name), tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
                directory = Path(temporary)
                target = directory / "target"
                shutil.copytree(os_suites.TARGET, target)
                source = target / "src/host/linux/Os_LinuxNative.c"
                text = source.read_text(encoding="utf-8")
                self.assertEqual(text.count(before), 1)
                source.write_text(text.replace(before, after), encoding="utf-8")
                with os_suites.native_session(directory, "signal-context-content"):
                    binary, _ = os_suites.build(
                        directory, str(ROOT / "tests/fixtures/os_native_facilities.c"), target=target,
                    )
                    result = os_suites.run_native([str(binary), "normal"], timeout=10)
                self.assertEqual(result.returncode, code, result)
                self.assertEqual(result.stdout, "", result)
                self.assertEqual(result.stderr, f"native-context rejected {stage}\n", result)
                print(f"signal-content variant={name} exit={result.returncode} {result.stderr}", end="", flush=True)

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
                    "context-error",
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
                        fatal = scenario in {"registration-error", "identity-error", "context-error"}
                        self.assertEqual(result.returncode, 1 if fatal else 0, result)
                        expected = {
                            "registration-error": "registration_rejected role=H\n",
                            "identity-error": "identity_requested\n",
                            "context-error": "actor_context_rejected\n",
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
                    "context",
                    "if (thread == NULL) {\n        ExitProcess(1u);\n    }\n    current_thread = thread;",
                    "if (thread == NULL) {\n        thread = &host_handles[0];\n    }\n    current_thread = thread;",
                    (("context-error", 0, "actor_context_rejected\n"),),
                ),
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
                                (
                                    code,
                                    "registered_actor entry=completed identity=published\n"
                                    if name == "context" else "",
                                    errors,
                                ),
                                result,
                            )
