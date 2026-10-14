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
            "EcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t dlc, const uint8_t data[8]);\n"
            "uint64_t Ecu_ExecutionNow(void);\n#endif\n", encoding="utf-8")
        cls.flags = ["-std=c99", "-O1", "-Wall", "-Wextra", "-Werror", "-pedantic", "-D_XOPEN_SOURCE=700",
                     "-I" + str(ROOT / "runtime/bsw/src"), "-I" + str(ROOT / "runtime/bsw/include"),
                     "-I" + str(ROOT / "runtime/host/src"), "-I" + str(ROOT / "runtime/host/include"),
                     # The mainline CAN headers use the generated ComStack_Cfg.h widths.
                     "-I" + str(ROOT / "core/tests/fixtures/bsw-contracts"), "-I" + str(generated)]
        suffix = ".exe" if os.name == "nt" else ""
        cls.lock = cls.directory / ("lock" + suffix)
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

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    @classmethod
    def execute(cls, argv, stage):
        return OwnedProcess(ProcessSpec.seconds(argv, cls.directory, 60, cls.directory, stage)).wait()

    def test_legacy_area_nesting_and_busy_are_normal_states(self):
        for case in ("normal", "busy"):
            with self.subTest(case=case):
                result = self.execute([str(self.lock), case], case)
                self.assertTrue(result.success, result.stdout.read_text())

    def test_entering_any_area_inside_a_strict_area_is_fatal(self):
        for case in ("nest-strict", "nest-same", "nest-legacy"):
            with self.subTest(case=case):
                result = self.execute([str(self.lock), case], case)
                output = result.stdout.read_text()
                self.assertEqual(result.exit_code, 91, output)
                self.assertIn("LOCK_FATAL", output)

    def test_try_lock_errors_stay_with_the_probe(self):
        cases = ["once", "mutex-init"] if os.name == "nt" else ["once", "attr-init", "attr-type", "mutex-init", "attr-close", "try-error"]
        for case in cases:
            with self.subTest(case=case):
                result = self.execute([str(self.lock), case], case)
                self.assertTrue(result.success, result.stdout.read_text())

    def test_void_lock_failures_select_fatal_policy_and_never_return(self):
        cases = ["once", "mutex-init"] if os.name == "nt" else ["once", "attr-init", "attr-type", "mutex-init", "attr-close", "lock", "unlock"]
        # The driver lock and the CanIf state lock are separate instances of the helper.
        for case, instance in ((case, instance) for case in cases for instance in ("fatal", "state")):
            with self.subTest(case=case, instance=instance):
                result = self.execute([str(self.lock), case, instance], case + "-" + instance)
                output = result.stdout.read_text()
                self.assertEqual(result.exit_code, 91, output)
                self.assertIn("LOCK_FATAL", output)
                if case in ("attr-type", "mutex-init") and os.name != "nt":
                    self.assertIn("attrs=1", output)
                if case == "attr-close":
                    self.assertIn("attrs=1 mutexes=1", output)
