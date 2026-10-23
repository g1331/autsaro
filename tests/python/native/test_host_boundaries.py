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
                    target=target, link_flags=() if os.name == "nt" else ("-Wl,--wrap=write",),
                )
                expected = {
                    "format": b"zero=0 max=18446744073709551615 hex=ffffffffffffffff fault=000000A5\x00\r\n",
                    "capacity": b"", "hex-width": b"", "hex-value": b"", "hex-case": b"",
                    "broken-pipe": b"", "partial": b"abcdef", "zero": b"", "partial-error": b"ab",
                }
                if os.name != "nt":
                    expected.update({"interrupted": b"abcdef", "pending-pipe": b""})
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
                    fatal_cases += ("fatal-interrupted",)
                for case in fatal_cases:
                    with self.subTest(case=case):
                        result = OwnedProcess(ProcessSpec.seconds(
                            [str(binary), case], directory, 10, directory, case,
                        )).wait()
                        output, errors = result.stdout.read_bytes(), result.stderr.read_bytes()
                        self.assertEqual(result.exit_code, 13, (case, output, errors))
                        self.assertEqual(output, fault if case in ("fatal-partial", "fatal-interrupted") else b"", case)
                        self.assertEqual(errors, fault if case == "fatal-broken-pipe" else b"", case)
