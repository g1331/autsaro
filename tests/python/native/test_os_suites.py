"""Native OS C99 suites and rejection checks for their verifier.

Run one suite with ``-k``, for example
``python -m unittest discover -s tests/python/native -p test_os_suites.py -k activation``.
Set ``AUTOSAR_OS_OUTPUT_DIRECTORY`` to a new directory to keep every suite's
binaries and command logs; failed suites always keep theirs.
"""

import copy
import json
import os
import re
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import os_suites


def native_fixture(name):
    path = os_suites.ROOT / "core/tests/fixtures/oracles" / name
    return json.loads(path.read_text(encoding="utf-8"))


class DependencyTests(unittest.TestCase):
    def test_memory_mapping_source_rejects_wrong_classes_and_preprocessing(self):
        def section(kind, body):
            return (f'#define OS_START_SEC_{kind}\n#include "Os_MemMap.h"\n'
                    f'{body}\n#define OS_STOP_SEC_{kind}\n#include "Os_MemMap.h"\n')
        for kind, body in [("CODE", "void entry(void) {}"),
                           ("CODE", "void worker(void) { typedef uint32 Count; Count n = 0u; (void)n; }"),
                           ("VAR_CLEARED_32", "static uint32 state;"),
                           ("VAR_INIT_32", "static uint32 state = 3u;"),
                           ("CONST_PTR", "static const uint32 *const view = &state;"),
                           ("VAR_CLEARED_TLS_32", "static OS_THREAD_LOCAL uint32 actor;")]:
            os_suites.verify_memory_mapping_source(section(kind, body))
        for source, expected in [
            (section("CODE", "static uint32 state;"), "code/data"),
            (section("VAR_CLEARED_32", "void entry(void) {}"), "code/data"),
            (section("CODE", "#if FLAG\nvoid entry(void) {}\n#endif"), "preprocessing"),
            (section("CODE", '#include "other.h"\nvoid entry(void) {}'), "preprocessing"),
            (section("CONST_32", "static uint32 state;"), "constant/variable"),
            (section("VAR_CLEARED_32", "static uint32 state = 3u;"), "initializer"),
            (section("VAR_CLEARED_TLS_32", "static uint32 state;"), "TLS"),
            ("static uint32 state;", "unmapped"),
            ("void worker(void) { typedef uint32 Count; Count n = 0u; (void)n; }", "unmapped"),
            (section("VAR_CLEARED_32", "struct Native { uint32 field; };"), "does not allocate"),
            (section("CODE", "void entry(void) {}").replace("STOP_SEC_CODE", "STOP_SEC_CONST_32"), "mismatched"),
        ]:
            with self.subTest(expected=expected), self.assertRaisesRegex(AssertionError, expected):
                os_suites.verify_memory_mapping_source(source)

    def test_memory_mapping_owned_headers_require_mapping(self):
        owned = os_suites.verify_memory_mapping_source(
            '#define OS_START_SEC_CODE\n#include "Os_MemMap.h"\n'
            'void Os_Entry(void) {}\n'
            '#define OS_STOP_SEC_CODE\n#include "Os_MemMap.h"\n')
        with self.assertRaisesRegex(AssertionError, "unmapped owned"):
            os_suites.verify_memory_mapping_source("void Os_Entry(void);", owned)
        os_suites.verify_memory_mapping_source("void Native_Imported(void);", owned)
        os_suites.verify_memory_mapping_source("struct Native;", owned)

    def test_memory_mapping_gate_checks_the_actual_module_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary)
            (target / "src").mkdir()
            (target / "src/Os_Vector.c").write_text(
                '#define OS_START_SEC_CODE\n#include "Os_MemMap.h"\n'
                'static uint32 vector[32];\n'
                '#define OS_STOP_SEC_CODE\n#include "Os_MemMap.h"\n', encoding="utf-8")
            with (patch("os_suites.TARGET", target),
                  patch("os_suites.compiler", side_effect=AssertionError("compiler ran before source gate")),
                  self.assertRaisesRegex(AssertionError, "Os_Vector.c: code/data")):
                os_suites.check_memory_mapping(target / "output")

    def test_memory_mapping_gate_checks_actual_owned_headers(self):
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary)
            (target / "src").mkdir()
            (target / "include").mkdir()
            (target / "src/Os_Entry.c").write_text(
                '#define OS_START_SEC_CODE\n#include "Os_MemMap.h"\n'
                'void Os_Entry(void) {}\n'
                '#define OS_STOP_SEC_CODE\n#include "Os_MemMap.h"\n', encoding="utf-8")
            (target / "include/Os_Entry.h").write_text("void Os_Entry(void);\n", encoding="utf-8")
            with (patch("os_suites.TARGET", target),
                  patch("os_suites.compiler", side_effect=AssertionError("compiler ran before source gate")),
                  self.assertRaisesRegex(AssertionError, "include/Os_Entry.h: unmapped owned.*Os_Entry")):
                os_suites.check_memory_mapping(target / "output")

    def test_explicit_output_preserves_success_and_refuses_existing_content(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'result'
            with os_suites.retained_workspace(output) as directory:
                (directory / 'stdout.log').write_text('passed', encoding='utf-8')
            self.assertEqual((output / 'stdout.log').read_text(encoding='utf-8'), 'passed')
            with self.assertRaises(FileExistsError), os_suites.retained_workspace(output):
                self.fail('Existing output must not be accepted')
            self.assertEqual((output / 'stdout.log').read_text(encoding='utf-8'), 'passed')

    def test_activation_gate_rejects_swapped_fifo_and_missing_observer(self):
        original = native_fixture("native-activation.json")
        with patch("os_suites.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(os_suites.check_activation(Path("unused.exe"))), len(original)
            )
        for mutation in ["order", "observer"]:
            with self.subTest(mutation=mutation):
                records = copy.deepcopy(original)
                result = records[0]
                if mutation == "order":
                    result["stdout"] = result["stdout"].replace(
                        "trace=ISRLAHaCBMZ", "trace=ISRLAHaBCMZ"
                    )
                else:
                    result["stdout"] = re.sub(
                        r"observer=\d+", "observer=0", result["stdout"]
                    )
                with (
                    patch("os_suites.execute", side_effect=records),
                    self.assertRaises(AssertionError),
                ):
                    os_suites.check_activation(Path("unused.exe"))

    def test_finish_gate_rejects_entry_order_return_and_missing_observer(self):
        original = native_fixture("native-finish.json")
        with patch("os_suites.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(os_suites.check_finish(Path("unused.exe"))), len(original)
            )
        for mutation in ["order", "return", "observer"]:
            with self.subTest(mutation=mutation):
                records = copy.deepcopy(original)
                if mutation == "observer":
                    records[0]["stdout"] = re.sub(
                        r"observer=\d+", "observer=0", records[0]["stdout"]
                    )
                else:
                    records[0]["stdout"] = records[0]["stdout"].replace(
                        "trace=ISRLACMZ",
                        "trace=ISRLCAMZ" if mutation == "order" else "trace=ISRLACXMZ",
                    )
                self.assertNotEqual(records[0]["stdout"], original[0]["stdout"])
                with (
                    patch("os_suites.execute", side_effect=records),
                    self.assertRaises(AssertionError),
                ):
                    os_suites.check_finish(Path("unused.exe"))

        # Reject the former whole-OS close instead of accepting a returned Task
        # which failed to finish its activation and run the pending monitor.
        records = copy.deepcopy(original)
        returned = next(row for row in records if row["scenario"] == "missing-end")
        returned["exit"] = 7
        returned["stdout"] = returned["stdout"].replace("trace=ISRLAMZ", "trace=ISRLAZ")
        with (
            patch("os_suites.execute", side_effect=records),
            self.assertRaises(AssertionError),
        ):
            os_suites.check_finish(Path("unused.exe"))

    def test_missing_and_modified_kernel_rejected(self):
        with tempfile.TemporaryDirectory(prefix="os-dependency-") as directory:
            kernel = Path(directory) / "kernel"
            shutil.copytree(os_suites.KERNEL, kernel)
            self.assertEqual(os_suites.verify_sources(kernel)["license"], "MIT")
            source = kernel / "tasks.c"
            source.write_bytes(source.read_bytes() + b"\n/* modified */\n")
            with self.assertRaisesRegex(ValueError, "kernel digest mismatch: tasks.c"):
                os_suites.verify_sources(kernel)
            source.unlink()
            with self.assertRaises(FileNotFoundError):
                os_suites.verify_sources(kernel)

    def test_resource_gate_rejects_preemption_wait_and_mask_order(self):
        original = native_fixture("native-resources.json")
        with patch("os_suites.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(os_suites.check_resources(Path("unused.exe"))), len(original)
            )
        for index, before, after in [
            (1, "trace=ISRAnHaMZ", "trace=ISRAHnaMZ"),
            (4, "trace=ISRABaMZ", "trace=ISRAaBMZ"),
            (10, "trace=ISRApJaMZ", "trace=ISRAJpaMZ"),
            (17, "trace=ISRAaMZ", "trace=ISRAJrHaMZ"),
        ]:
            with self.subTest(index=index):
                records = copy.deepcopy(original)
                records[index]["stdout"] = records[index]["stdout"].replace(
                    before, after
                )
                self.assertNotEqual(records[index]["stdout"], original[index]["stdout"])
                with (
                    patch("os_suites.execute", side_effect=records),
                    self.assertRaises(AssertionError),
                ):
                    os_suites.check_resources(Path("unused.exe"))

    def test_event_gate_rejects_lost_record_ticket_and_missing_restart(self):
        original = native_fixture("native-events.json")
        with patch("os_suites.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(os_suites.check_events(Path("unused.exe"))), len(original)
            )
        for scenario, before, after in [
            ("mailbox-between", "records=2", "records=1"),
            ("mailbox-full", "ticket=256", "ticket=255"),
            ("new-instance", "entries=2", "entries=1"),
            ("category1", "rejected=12", "rejected=11"),
        ]:
            records = copy.deepcopy(original)
            record = next(row for row in records if row["scenario"] == scenario)
            record["stdout"] = record["stdout"].replace(before, after)
            self.assertNotEqual(
                record["stdout"],
                next(row["stdout"] for row in original if row["scenario"] == scenario),
            )
            with (
                patch("os_suites.execute", side_effect=records),
                self.assertRaises(AssertionError),
            ):
                os_suites.check_events(Path("unused.exe"))

    def test_time_gate_rejects_lost_tick_wrap_and_action_error(self):
        original = native_fixture("native-time.json")
        with patch("os_suites.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(os_suites.check_time(Path("unused.exe"))), len(original)
            )
        for scenario, before, after in [
            ("thousand", "ticks=1000", "ticks=999"),
            ("wrap", "kernel=0 counter=1", "kernel=1 counter=1"),
            ("absolute-cycle", "counter=6", "counter=5"),
            ("action-error", "errors=1 last=4", "errors=0 last=0"),
            ("tick-mask-all", "observed=1 kernel=0 pending=1", "observed=1 kernel=1 pending=1"),
            ("tick-mask-os", "observed=1 kernel=0 pending=1", "observed=1 kernel=0 pending=0"),
            ("tick-owner-all", "phase=0 waiting_commit=1", "phase=1 waiting_commit=1"),
            ("tick-owner-os", "phase=0 waiting_commit=1", "phase=0 waiting_commit=0"),
            ("close-in-service", "trace=ISREZ ", "trace=ISRE!467Z "),
        ]:
            records = copy.deepcopy(original)
            record = next(row for row in records if row["scenario"] == scenario)
            record["stdout"] = record["stdout"].replace(before, after)
            self.assertNotEqual(
                record["stdout"],
                next(row["stdout"] for row in original if row["scenario"] == scenario),
            )
            with (
                patch("os_suites.execute", side_effect=records),
                self.assertRaises(AssertionError),
            ):
                os_suites.check_time(Path("unused.exe"))


if __name__ == "__main__":
    unittest.main()


@unittest.skipUnless(os_suites.HOST_TARGET, "native OS suites run on Windows or Linux")
class NativeSuiteTests(unittest.TestCase):
    """One test per suite in ``os_suites.SUITES``, named ``test_<suite>``."""

    def run_suite(self, suite):
        retained = os.environ.get("AUTOSAR_OS_OUTPUT_DIRECTORY")
        with os_suites.host_target():
            os_suites.run_suite(suite, Path(retained) / suite if retained else None)


# AUTOSAR_OS_SHARD="k/n" keeps every n-th suite from k, so CI can split the suites over runners.
_shard, _shards = map(int, os.environ.get("AUTOSAR_OS_SHARD", "0/1").split("/"))
for _suite in os_suites.SUITES[_shard::_shards]:
    setattr(
        NativeSuiteTests,
        "test_" + _suite.replace("-", "_"),
        lambda self, suite=_suite: self.run_suite(suite),
    )
