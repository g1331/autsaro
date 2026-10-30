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
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import os_suites


def native_fixture(name):
    path = os_suites.ROOT / "core/tests/fixtures/oracles" / name
    return json.loads(path.read_text(encoding="utf-8"))


class DependencyTests(unittest.TestCase):
    def test_platform_definitions_use_declared_project_types(self):
        def declaration(source, name):
            matches = re.findall(r"\b(?:sint32|int)\s+" + name + r"\s*\([^)]*\)", source)
            self.assertEqual(len(matches), 1, name)
            return [token.text for token in os_suites.lex(matches[0])]

        header = (os_suites.TARGET / "src/Os_Backend.h").read_text(encoding="utf-8")
        for filename in ("0009-interrupt-source-control.patch", "linux/0001-controlled-posix-port.patch"):
            text = (os_suites.ROOT / "runtime/os/patches" / filename).read_text(encoding="utf-8")
            added = "\n".join(line[1:] for line in text.splitlines()
                              if line.startswith("+") and not line.startswith("+++"))
            for name in ("Os_PortInterruptSourceEnabled", "Os_PortInterruptSourceControl"):
                with self.subTest(patch=filename, function=name):
                    self.assertEqual(declaration(added, name), declaration(header, name))

        header = (os_suites.TARGET / "include/Os_IntegrationHooks.h").read_text(encoding="utf-8")
        for filename in ("runtime/ecu/src/Ecu_Target.c", "core/tests/fixtures/os/os_no_consumer_hooks.c"):
            source = (os_suites.ROOT / filename).read_text(encoding="utf-8")
            with self.subTest(file=filename, function="Os_IntegrationTimingAuthorized"):
                self.assertEqual(declaration(source, "Os_IntegrationTimingAuthorized"),
                                 declaration(header, "Os_IntegrationTimingAuthorized"))

    def test_port_callback_parameter_names_match_their_declarations(self):
        def names(source, name):
            values = []
            for declaration in re.findall(r"\b" + name + r"\s*\([^)]*\)", source):
                signature = os_suites.function(os_suites.lex("void " + declaration))
                if signature and "<unnamed>" not in signature[1]:
                    values.append(signature[1])
            return values

        portable = (os_suites.KERNEL / "include/portable.h").read_text(encoding="utf-8")
        task = (os_suites.KERNEL / "include/task.h").read_text(encoding="utf-8")
        backend = (os_suites.TARGET / "src/Os_Backend.c").read_text(encoding="utf-8")
        patch_text = (os_suites.ROOT / "runtime/os/patches/linux/0001-controlled-posix-port.patch").read_text(encoding="utf-8")
        added = "\n".join(line[1:] for line in patch_text.splitlines()
                          if line.startswith("+") and not line.startswith("+++"))
        with self.subTest(function="pxPortInitialiseStack"):
            expected = [parameters for parameters in names(portable, "pxPortInitialiseStack") if len(parameters) == 3]
            self.assertEqual(len(expected), 1)
            self.assertEqual(names(added, "pxPortInitialiseStack"), expected)
        with self.subTest(function="vPortCloseRunningThread"):
            declarations = names(added, "vPortCloseRunningThread")
            self.assertEqual(len(declarations), 2)
            self.assertEqual(declarations[0], declarations[1])
        with self.subTest(function="vApplicationGetIdleTaskMemory"):
            expected = names(task, "vApplicationGetIdleTaskMemory")
            self.assertTrue(expected)
            self.assertTrue(all(parameters == expected[0] for parameters in expected))
            self.assertEqual(names(backend, "vApplicationGetIdleTaskMemory"), [expected[0]])

    def test_expected_native_exit_does_not_hide_sanitizer_reports(self):
        with patch.dict(os.environ, {"AUTOSAR_SANITIZE": "thread"}):
            for code in (7, 13):
                for marker in ("ThreadSanitizer:", "AddressSanitizer:", "LeakSanitizer:", "runtime error:"):
                    for stream in ("stdout", "stderr"):
                        with self.subTest(exit=code, marker=marker, stream=stream):
                            output = {"stdout": "", "stderr": ""}
                            output[stream] = marker + " injected report"
                            result = subprocess.CompletedProcess(["native"], code, **output)
                            with patch.object(os_suites, "run_native", return_value=result), self.assertRaisesRegex(AssertionError, marker):
                                os_suites.execute(Path("native"), "normal")
            control = subprocess.CompletedProcess(["native"], 7, "state=Failed", "stage=expected")
            with patch.object(os_suites, "run_native", return_value=control):
                self.assertEqual(os_suites.execute(Path("native"), "normal")["exit"], 7)

    def test_backend_terminal_paths_do_not_restore_dead_returns(self):
        source = (os_suites.ROOT / "runtime/os/src/Os_Backend.c").read_text(encoding="utf-8")
        for call in (r"Os_BackendShutdown\(E_OS_STATE\)", r"report_and_exit\(\)", r"Sleep\(INFINITE\)"):
            with self.subTest(call=call):
                self.assertNotRegex(source, call + r";\s*return\b")

    def test_status_switch_clauses_use_break_termination(self):
        for relative, name in [
            ("runtime/host/include/Ecu_Status.h", "Ecu_StatusName"),
            ("runtime/os/src/Os_Backend.c", "native_task_state"),
            ("runtime/os/src/Os_Backend.c", "Os_BackendState"),
            ("runtime/os/src/Os_Arti.c", "Os_ArtiTask"),
            ("runtime/os/src/Os_Arti.c", "Os_ArtiService"),
            ("runtime/os/src/Os_Time.c", "action"),
        ]:
            with self.subTest(function=name):
                source = (os_suites.ROOT / relative).read_text(encoding="utf-8")
                body = re.search(r"\b" + name + r"\([^)]*\)\s*\{(.*?)^\}", source, re.DOTALL | re.MULTILINE)
                self.assertIsNotNone(body, name)
                self.assertNotRegex(body[1], r"\bdefault\s*:\s*break\s*;", name)
                tokens = [token.text for token in os_suites.lex(body[1])
                          if not os_suites.is_comment(token)]
                start = tokens.index("{", tokens.index("switch"))
                end = start + 1
                depth = 1
                while depth != 0:
                    depth += (tokens[end] == "{") - (tokens[end] == "}")
                    end += 1
                words = tokens[start + 1:end - 1]
                self.assertNotIn("return", words, name)
                self.assertNotRegex(" ".join(words),
                                    r"(?:assert \( 0 \)|configASSERT \( FALSE \)|"
                                    r"Os_BackendShutdown \( E_OS_STATE \)) ; break",
                                    name)
                self.assertEqual(words.count("break"), words.count("case") + words.count("default"), name)

    def test_transport_and_diagnostic_sections_exclude_private_preprocessing(self):
        # Verifies: SWS_MemMap_00003. Reuse the directive checker, not its OS object-class policy.
        for module, prefix, constant in (("CanTp", "CANTP", "CANTP_E_PADDING"),
                                         ("Dcm", "DCM", "DCM_BUFFER_CAPACITY")):
            with self.subTest(module=module):
                source = (os_suites.ROOT / "runtime/bsw/src" / (module + ".c")).read_text()
                directives = "\n".join(token.text for token in os_suites.lex(source)
                                       if token.text.lstrip().startswith("#"))
                directives = directives.replace(prefix + "_START_SEC_", "OS_START_SEC_")
                directives = directives.replace(prefix + "_STOP_SEC_", "OS_STOP_SEC_")
                directives = directives.replace('"' + module + '_MemMap.h"', '"Os_MemMap.h"')
                os_suites.verify_memory_mapping_source(directives)
                # Restoring a real private definition inside the section must fail this same oracle.
                definition = next(line for line in directives.splitlines()
                                  if line.startswith("#define " + constant + " "))
                self.assertEqual(directives.count(definition), 1)
                mutant = directives.replace(definition + "\n", "", 1)
                include = '#include "Os_MemMap.h"'
                mutant = mutant.replace(include, include + "\n" + definition, 1)
                with self.assertRaisesRegex(AssertionError, "preprocessing inside mapped section"):
                    os_suites.verify_memory_mapping_source(mutant)

    def test_configuration_conditionals_surround_complete_memory_mapping_pairs(self):
        # Verifies: SWS_MemMap_00003. Configuration selection encloses the complete pair.
        for name, prefix, header, conditional in (
            ("runtime/bsw/src/Can.c", "CAN", "Can", "#if (CAN_HOST_QUEUED_COMPLETION == STD_ON)"),
            ("runtime/host/src/Det_Host.c", "DET", "Det", "#if defined(_WIN32)"),
            ("runtime/os/src/Os_PortRestart.h", "OS", "Os", "#ifndef _WIN32"),
            ("runtime/ecu/src/Ecu_Target.c", "OS", "Os",
             "#if defined(ECU_TARGET_TESTS) && (ECU_TARGET_TESTS == STD_ON)"),
        ):
            with self.subTest(source=name):
                source = (os_suites.ROOT / name).read_text()
                directives = "\n".join(token.text for token in os_suites.lex(source)
                                       if token.text.lstrip().startswith("#"))
                directives = directives.replace(prefix + "_START_SEC_", "OS_START_SEC_")
                directives = directives.replace(prefix + "_STOP_SEC_", "OS_STOP_SEC_")
                directives = directives.replace('"' + header + '_MemMap.h"', '"Os_MemMap.h"')
                os_suites.verify_memory_mapping_source(directives)
                self.assertIn(conditional, directives)
                include = '#include "Os_MemMap.h"'
                mutant = directives.replace(include, include + "\n" + conditional + "\n#endif", 1)
                with self.assertRaisesRegex(AssertionError, "preprocessing inside mapped section"):
                    os_suites.verify_memory_mapping_source(mutant)

    def test_memory_mapping_source_rejects_wrong_classes_and_preprocessing(self):
        def section(kind, body):
            return (f'#define OS_START_SEC_{kind}\n#include "Os_MemMap.h"\n'
                    f'{body}\n#define OS_STOP_SEC_{kind}\n#include "Os_MemMap.h"\n')
        for kind, body in [("CODE", "void entry(void) {}"),
                           ("CODE", "typedef uint32 Count; void entry(Count n) { (void)n; }"),
                           ("VAR_CLEARED_32", "struct Native { uint32 field; }; static struct Native state;"),
                           ("CODE", "void worker(void) { typedef uint32 Count; Count n = 0u; (void)n; }"),
                           ("VAR_CLEARED_32", "static uint32 state;"),
                           ("VAR_INIT_32", "static uint32 state = 3u;"),
                           ("CONST_PTR", "static const uint32 *const view = &state;"),
                           ("CONFIG_DATA_PREBUILD_8", "static const uint8 name[] = {65u, 0u};"),
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
            (section("VAR_CLEARED_32", "typedef OS_MEMMAP uint32 Count;"), "section attribute"),
            (section("CODE", "void entry(void) {}").replace("STOP_SEC_CODE", "STOP_SEC_CONST_32"), "mismatched"),
        ]:
            with self.subTest(expected=expected), self.assertRaisesRegex(AssertionError, expected):
                os_suites.verify_memory_mapping_source(source)

    def test_memory_mapping_aggregate_objects_keep_storage_checks(self):
        def section(kind, body):
            return (f'#define OS_START_SEC_{kind}\n#include "Os_MemMap.h"\n'
                    f'{body}\n#define OS_STOP_SEC_{kind}\n#include "Os_MemMap.h"\n')

        for body in ("struct Native { uint32 field; };",
                     "enum Native { FIRST = 1, LAST = 2 };",
                     "struct Native { struct { uint32 field; } member; };"):
            with self.subTest(type=body):
                self.assertEqual(os_suites.verify_memory_mapping_source(section("CODE", body)), set())
        for body in ("struct Native state = {0};",
                     "struct Native { uint32 field; } state = {0};",
                     "union Native state = {0};"):
            with self.subTest(object=body):
                self.assertEqual(os_suites.verify_memory_mapping_source(section("VAR_INIT_32", body)), {"state"})
            for kind, expected in (("CODE", "code/data"),
                                   ("VAR_CLEARED_32", "initializer"),
                                   ("CONST_32", "constant/variable"),
                                   ("VAR_CLEARED_TLS_32", "TLS")):
                with self.subTest(object=body, section=kind), self.assertRaisesRegex(AssertionError, expected):
                    os_suites.verify_memory_mapping_source(section(kind, body))

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

            (target / "src/Os_Entry.c").write_text(
                '#define OS_START_SEC_CODE\n#include "Os_MemMap.h"\n'
                'void SetEvent(void) {}\n'
                '#define OS_STOP_SEC_CODE\n#include "Os_MemMap.h"\n', encoding="utf-8")
            (target / "include/Os_Entry.h").write_text("void Os_SetEvent(void);\n", encoding="utf-8")
            with (patch("os_suites.TARGET", target),
                  patch("os_suites.compiler", side_effect=AssertionError("compiler ran before alias gate")),
                  self.assertRaisesRegex(AssertionError, "include/Os_Entry.h: unmapped owned.*Os_SetEvent")):
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
