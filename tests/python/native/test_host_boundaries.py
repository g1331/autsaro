"""Real host C fault injection, without adding hooks to delivery code."""
from __future__ import annotations

import os
import platform
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


@unittest.skipUnless(platform.system() == "Linux", "Linux native facilities")
class NativeFacilityTests(unittest.TestCase):
    def test_native_handle_errors_and_resource_reuse(self):
        import os_suites

        with tempfile.TemporaryDirectory() as temporary, os_suites.host_target():
            directory = Path(temporary)
            with os_suites.native_session(directory, "event-errors"):
                binary, _ = os_suites.build(
                    directory, str(ROOT / "tests/fixtures/os_event_errors.c"),
                    link_flags=tuple(f"-Wl,--wrap={name}" for name in (
                        "poll", "read", "write", "close", "pthread_mutex_destroy",
                    )),
                )
                for scenario in ("poll-error", "poll-error-infinite", "poll-invalid", "poll-hangup", "poll-revents-error",
                                 "poll-interrupted", "read-error", "read-zero", "read-short",
                                 "read-interrupted", "wait-read-error", "wait-read-interrupted",
                                 "wait-read-empty", "wait-read-zero", "wait-read-short",
                                 "write-error", "write-interrupted", "mutex-reuse", "mutex-close-error",
                                 "close-interrupted"):
                    with self.subTest(scenario=scenario):
                        result = os_suites.run_native([str(binary), scenario], timeout=10)
                        self.assertEqual(result.returncode, 0, result)
                        self.assertEqual(result.stdout, "event success/reset/failure/interruption observed\n")
                        self.assertEqual(result.stderr, "")

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
