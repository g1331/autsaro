"""Actual pinned Cppcheck/addon round trip, selected explicitly by C analysis CI."""
from __future__ import annotations

import hashlib
import json
import platform
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import c_check


class ActualCAnalysisTests(unittest.TestCase):
    def project(self, root: Path, source: str, source_name: str = "src/main.c", extra_files: dict[str, str] | None = None) -> Path:
        project = root / "sources"
        project.mkdir()
        target = "windows-x64-controlled-v1" if platform.system() == "Windows" else "linux-x64-controlled-v1"
        abi = "x86_64-w64-mingw32" if platform.system() == "Windows" else "x86_64-linux-gnu"
        extra_files = extra_files or {}
        files = {**extra_files, source_name: source, "target.json": json.dumps({
            "format": "autosar-build-target-v1", "target": target, "abi": abi, "profile": "host",
            "compilerFlags": ["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"],
            "sources": [source_name, *(name for name in extra_files if name.endswith(".c"))], "includePaths": ["."]})}
        for name, text in files.items():
            path = project / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        (project / "files.list").write_text("\n".join(sorted(files)) + "\n", encoding="utf-8")
        (project / "files.sha256").write_text("".join(f"{hashlib.sha256((project / name).read_bytes()).hexdigest()}  {name}\n" for name in [*sorted(files), "files.list"]), encoding="utf-8")
        return project

    def analyze(self, source: str, name: str = "src/main.c", *, cli: bool = False) -> dict:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = self.project(root, source, name)
            before = {path.relative_to(project).as_posix(): path.read_bytes() for path in project.rglob("*") if path.is_file()}
            target = json.loads((project / "target.json").read_text())["target"]
            if cli:
                result = subprocess.run([sys.executable, "-m", "autosar_tooling", "c-check", "--target", target,
                                         "--project", str(project), "--output-directory", str(root / "analysis")],
                                        capture_output=True, text=True, encoding="utf-8", errors="replace", check=False)
                code = result.returncode
            else:
                code = c_check.check(project, target, root / "analysis")
            summary = json.loads((root / "analysis/summary.json").read_text())
            self.assertIsNone(summary["error"], summary)
            self.assertEqual(code, 0 if summary["passed"] else 1, summary.get("diagnostics"))
            after = {path.relative_to(project).as_posix(): path.read_bytes() for path in project.rglob("*") if path.is_file()}
            self.assertEqual(before, after)
            return summary

    def test_clean_main_passes(self):
        self.assertTrue(self.analyze("int main(void) { return 0; }\n")["passed"])

    def test_misra_violation_fails_through_the_real_entry(self):
        summary = self.analyze("int main(void) { int value = 1; if (value) { return 1; } return 0; }\n", cli=True)
        self.assertFalse(summary["passed"])
        self.assertTrue(any(row["id"].startswith("misra-c2012-") for row in summary["diagnostics"]))

    def test_allocator_macro_and_typedef_still_report_direct_call(self):
        summary = self.analyze("#include <stdlib.h>\n#define ALLOC malloc\ntypedef void * Buffer;\nint main(void) { Buffer p = ALLOC(4u); free(p); return 0; }\n")
        self.assertTrue(any(row["id"] == "autsaro-dynamicAllocation" for row in summary["diagnostics"]), summary.get("diagnostics"))

    def test_thread_call_reports_but_declaration_and_comment_do_not(self):
        summary = self.analyze("extern int pthread_create(void);\nint main(void) { return pthread_create(); }\n")
        self.assertTrue(any(row["id"] == "autsaro-hostThreadInBsw" for row in summary["diagnostics"]), summary.get("diagnostics"))
        summary = self.analyze("extern int pthread_create(void);\n/* pthread_create() is not called. */\nint main(void) { return 0; }\n")
        self.assertFalse(any(row["id"] == "autsaro-hostThreadInBsw" for row in summary["diagnostics"]))

    def test_missing_header_and_invalid_c_are_analysis_failures(self):
        for source in ('#include "missing-project-header.h"\nint main(void) { return 0; }\n',
                       'int main(void) { invalid C tokens; }\n'):
            with self.subTest(source=source), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                project = self.project(root, source)
                target = json.loads((project / "target.json").read_text())["target"]
                self.assertEqual(c_check.check(project, target, root / "analysis"), 1)
                summary = json.loads((root / "analysis/summary.json").read_text())
                self.assertFalse(summary["passed"])
                self.assertIn("Compiler validation failed", summary["error"])

    def test_host_adapter_and_application_are_not_classified_as_bsw(self):
        for name in ("src/host/adapter.c", "src/Application.c"):
            with self.subTest(name=name):
                summary = self.analyze("extern int pthread_create(void);\nint main(void) { return pthread_create(); }\n", name)
                self.assertFalse(any(row["id"] == "autsaro-hostThreadInBsw" for row in summary["diagnostics"]))

    def test_thread_macro_and_typedef_still_report_direct_call(self):
        summary = self.analyze("typedef int ThreadResult;\nextern ThreadResult pthread_create(void);\n#define SPAWN pthread_create\nint main(void) { return SPAWN(); }\n")
        self.assertTrue(any(row["id"] == "autsaro-hostThreadInBsw" for row in summary["diagnostics"]), summary.get("diagnostics"))

    @unittest.skipUnless(platform.system() == "Linux", "glibc target model")
    def test_linux_dynamic_values_keep_their_actual_function_types(self):
        summary = self.analyze("#define _GNU_SOURCE 1\n#include <pthread.h>\n#include <signal.h>\nint main(void) { return PTHREAD_MUTEX_RECURSIVE + (SIGRTMIN > 0) + (PTHREAD_STACK_MIN > 0); }\n")
        self.assertIsNotNone(summary["scope"]["platformModel"])
        definitions = summary["scope"]["compilerDiscoveredDefines"]["src/main.c"]
        self.assertTrue(any(value.startswith("-DSIGRTMIN=") and "__libc_current_sigrtmin" in value for value in definitions))
        self.assertTrue(any(value.startswith("-DPTHREAD_STACK_MIN=") and "__sysconf" in value for value in definitions))

    @unittest.skipUnless(platform.system() == "Linux", "glibc target model")
    def test_unrecognized_linux_dynamic_macro_is_not_silently_dropped(self):
        original = c_check.run

        def unsupported_macro(*args, **kwargs):
            code, stdout, stderr = original(*args, **kwargs)
            if args[2] == "compiler-values-0":
                stdout = "\n".join("AUTSAR_VALUE_SIGRTMIN unsupported_signal_expression ()"
                                   if line.startswith("AUTSAR_VALUE_SIGRTMIN ") else line
                                   for line in stdout.splitlines())
            return code, stdout, stderr

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = self.project(root, "#define _GNU_SOURCE 1\n#include <signal.h>\nint main(void) { return SIGRTMIN; }\n")
            with patch.object(c_check, "run", side_effect=unsupported_macro):
                self.assertEqual(c_check.check(project, "linux-x64-controlled-v1", root / "analysis"), 1)
            summary = json.loads((root / "analysis/summary.json").read_text())
            self.assertFalse(summary["passed"])
            self.assertIn("Unsupported Linux system macro expansion: SIGRTMIN", summary["error"])

    def test_standard_integer_constant_macros_use_real_compiler_suffixes(self):
        summary = self.analyze("#include <stdint.h>\nint main(void) { uint32_t value = UINT32_C(0xffffffff); return (value > 0u) ? 0 : 1; }\n")
        self.assertFalse(any(row["id"] == "misra-c2012-7.2" for row in summary["diagnostics"]), summary.get("diagnostics"))
        self.assertTrue(any(value.startswith("-DUINT32_C(") for value in summary["scope"]["compilerDiscoveredDefines"]["src/main.c"]))

    def test_stdio_model_accepts_actual_modes_and_rejects_invalid_mode(self):
        summary = self.analyze("#include <stdio.h>\nint main(void) { return setvbuf(stdout, NULL, _IONBF, 0u); }\n")
        self.assertFalse(any(row["id"] == "invalidFunctionArg" for row in summary["diagnostics"]), summary.get("diagnostics"))
        summary = self.analyze("#include <stdio.h>\nint main(void) { return setvbuf(stdout, NULL, 42, 0u); }\n")
        self.assertTrue(any(row["id"] == "invalidFunctionArg" for row in summary["diagnostics"]), summary.get("diagnostics"))

    def test_parallel_compiler_units_keep_macro_results_and_report_invalid_unit(self):
        for invalid in (False, True):
            with self.subTest(invalid=invalid), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                project = self.project(root, '#include "helper.h"\nint main(void) { return helper() == UINT32_C(1) ? 0 : 1; }\n', extra_files={
                    "helper.h": '#include <stdint.h>\nuint32_t helper(void);\nuint32_t second(void);\nuint32_t third(void);\n',
                    "src/second.c": '#include "helper.h"\nuint32_t second(void) { return UINT32_C(2); }\n',
                    "src/third.c": '#include "helper.h"\nuint32_t third(void) { return UINT32_C(3); }\n',
                    "src/helper.c": 'int broken( { }\n' if invalid else '#include "helper.h"\nuint32_t helper(void) { return UINT32_C(1); }\n'})
                target = json.loads((project / "target.json").read_text())["target"]
                with patch("autosar_tooling.c_check.os.cpu_count", return_value=64):
                    code = c_check.check(project, target, root / "analysis")
                summary = json.loads((root / "analysis/summary.json").read_text())
                self.assertEqual(code, 0 if summary["passed"] else 1)
                if invalid:
                    self.assertFalse(summary["passed"])
                    self.assertIn("Compiler validation failed for src/helper.c", summary["error"])
                else:
                    self.assertIsNone(summary["error"])
                    self.assertEqual(summary["scope"]["compilerWorkers"], 4)
                    self.assertEqual(set(summary["scope"]["compilerDiscoveredDefines"]), {"src/main.c", "src/helper.c", "src/second.c", "src/third.c"})
                    for definitions in summary["scope"]["compilerDiscoveredDefines"].values():
                        self.assertTrue(any(value.startswith("-DUINT32_C(") for value in definitions))
