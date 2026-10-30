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
from ecu_tools.owner import OwnershipError


class ActualCAnalysisTests(unittest.TestCase):
    def project(self, root: Path, source: str, source_name: str = "bsw/src/main.c", extra_files: dict[str, str] | None = None,
                unbuilt: tuple[str, ...] = ()) -> Path:
        project = root / "sources"
        project.mkdir()
        target = "windows-x64-controlled-v1" if platform.system() == "Windows" else "linux-x64-controlled-v1"
        abi = "x86_64-w64-mingw32" if platform.system() == "Windows" else "x86_64-linux-gnu"
        extra_files = extra_files or {}
        files = {**extra_files, source_name: source, "target.json": json.dumps({
            "format": "autosar-build-target-v3", "target": target, "abi": abi, "profile": "host",
            "compilerFlags": ["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"],
            "sources": [source_name, *(name for name in extra_files if name.endswith(".c") and name not in unbuilt)],
            "includePaths": ["."]})}
        for name, text in files.items():
            path = project / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        (project / "meta").mkdir(exist_ok=True)
        (project / "meta/files.list").write_text("\n".join(sorted(files)) + "\n", encoding="utf-8")
        (project / "meta/files.sha256").write_text("".join(f"{hashlib.sha256((project / name).read_bytes()).hexdigest()}  {name}\n" for name in [*sorted(files), "meta/files.list"]), encoding="utf-8")
        return project

    def analyze(self, source: str, name: str = "bsw/src/main.c", *, cli: bool = False) -> dict:
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

    def test_baseline_comparison_runs_the_local_analyzer_and_preserves_source_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = self.project(root, "int main(void) { return 0; }\n",
                                   extra_files={"meta/integration.json": '{"plan":{}}'})
            target = json.loads((project / "target.json").read_text())["target"]
            baseline = root / "baseline/sources"
            self.assertEqual(c_check.check(project, target, baseline), 0)
            self.assertEqual(c_check.check(project, target, root / "same", baseline), 0)
            summary = json.loads((root / "same/summary.json").read_text())
            self.assertEqual(summary["comparison"], {"new": [], "removed": []})
            # Mutation: returning the baseline result without analyzing modified sources
            # would hide this compiler failure. Update the source seal, not the expectation.
            source = project / "bsw/src/main.c"
            source.write_text("invalid C tokens;\n")
            inventory = project / "meta/files.sha256"
            lines = inventory.read_text().splitlines()
            inventory.write_text("\n".join(
                f"{hashlib.sha256(source.read_bytes()).hexdigest()}  bsw/src/main.c"
                if line.endswith("  bsw/src/main.c") else line for line in lines) + "\n")
            self.assertEqual(c_check.check(project, target, root / "changed", baseline), 1)
            summary = json.loads((root / "changed/summary.json").read_text())
            self.assertIn("compiler-unit-0", summary["error"])
            self.assertNotIn("comparison", summary)

    def test_ecu_analysis_requires_the_same_patched_kernel_bytes_as_builds(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            delivered = "int kernel(void) { return 1; }\n"
            project = self.project(root, "int application(void) { return 0; }\n", extra_files={
                "kernel/unit.c": "int kernel(void) { return 0; }\n",
                "host.c": "int main(void) { return 0; }\n",
                "probe.c": "int main(void) { return 0; }\n",
                "patch.diff": "--- a/unit.c\n+++ b/unit.c\n@@ -1 +1 @@\n-int kernel(void) { return 0; }\n+" + delivered,
            })
            target = json.loads((project / "target.json").read_text())
            target.update(profile="ecu", sources=["bsw/src/main.c"], kernelSources=["unit.c"],
                          entries={"host-batch": "host.c", "probe": "probe.c"},
                          kernelPatches=[{"path": "runtime/patch.diff", "includes": ["unit.c"]}],
                          kernelSha256={"unit.c": hashlib.sha256(delivered.encode()).hexdigest()})
            target["compilerFlags"].append("-fexceptions")
            (project / "target.json").write_text(json.dumps(target))
            # Match the LF bytes of controlled kernel sources and patches on both hosts.
            for name in ("kernel/unit.c", "patch.diff"):
                path = project / name
                path.write_bytes(path.read_text(encoding="utf-8").encode("utf-8"))
            seal = project / "meta/files.sha256"
            names = (project / "meta/files.list").read_text().splitlines() + ["meta/files.list"]
            seal.write_text("".join(f"{hashlib.sha256((project / name).read_bytes()).hexdigest()}  {name}\n"
                                    for name in names))
            for corrupt in (False, True):
                output = root / ("corrupt" if corrupt else "control")
                original = c_check.require_kernel_patch_applied

                def after_patch(path, before, *, corrupt=corrupt, original=original):
                    original(path, before)
                    if corrupt:
                        kernel = next(iter(before))
                        kernel.write_bytes(kernel.read_bytes() + b"\n")  # Valid C, different pinned bytes.

                with patch.object(c_check, "require_kernel_patch_applied", side_effect=after_patch):
                    code = c_check.check(project, target["target"], output)
                summary = json.loads((output / "summary.json").read_text())
                if corrupt:
                    self.assertEqual(code, 1)
                    self.assertFalse(summary["passed"])
                    self.assertEqual(summary["error"], "Kernel identity mismatch: kernel/unit.c")
                    self.assertEqual(list(output.glob("compiler-unit-*.log")), [])
                else:
                    self.assertIn(code, (0, 1))
                    self.assertIsNone(summary["error"], summary)
                    self.assertEqual(len(summary["programs"]), 2)

    def test_prepare_failures_keep_context_and_raw_windows_evidence(self):
        stages = [
            ("compiler-target", "Cannot identify diagnostic compiler target"),
            ("compiler-version", "Cannot identify diagnostic compiler"),
            ("compiler-headers", "Cannot discover compiler include search paths"),
            ("compiler-macros", "Cannot discover compiler predefined macros"),
            ("compiler-unit-0", "Compiler validation failed for bsw/src/main.c"),
            ("compiler-defines-0", "Compiler preprocessing failed for bsw/src/main.c"),
            ("compiler-values-0", "Compiler macro expansion failed for bsw/src/main.c"),
            ("patch-0", "Kernel patch 0 failed"),
        ]
        if platform.system() == "Linux":
            stages.append(("platform-model-check", "Linux library analysis model differs from the actual compiler headers"))
        original = c_check.run_bounded
        for stage, explanation in stages:
            def windows_failure(spec, stage=stage, **kwargs):
                if spec.stage == stage:
                    raise OwnershipError(
                        spec.stage + r": argv=C:\project\bsw\src\main.c exit_code=1"
                        + "\nstdout:\nproducer stdout\nstderr:\nproducer stderr"
                    )
                return original(spec, **kwargs)
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                source = ("#include <signal.h>\nint main(void) { return SIGRTMIN; }\n"
                          if stage == "platform-model-check" else "int main(void) { return 0; }\n")
                extras = {"kernel/unit.c": "before\n", "patch.diff": "+++ b/unit.c\n"} if stage == "patch-0" else None
                project = self.project(root, source, extra_files=extras)
                target = json.loads((project / "target.json").read_text())
                if stage == "patch-0":
                    target["compilerFlags"].append("-fexceptions")
                    target.update(profile="ecu", kernelSources=["unit.c"],
                                  kernelPatches=[{"path": "runtime/patch.diff", "includes": ["unit.c"]}])
                    (project / "target.json").write_text(json.dumps(target))
                    seal = project / "meta/files.sha256"
                    seal.write_text("\n".join(
                        hashlib.sha256((project / "target.json").read_bytes()).hexdigest() + "  target.json"
                        if row.endswith("  target.json") else row for row in seal.read_text().splitlines()) + "\n")
                with patch.object(c_check, "run_bounded", side_effect=windows_failure):
                    self.assertEqual(c_check.check(project, target["target"], root / "analysis"), 1)
                error = json.loads((root / "analysis/summary.json").read_text())["error"]
                for evidence in (explanation, stage, r"C:\project\bsw\src\main.c", "exit_code=1",
                                 "stdout:\nproducer stdout", "stderr:\nproducer stderr"):
                    self.assertIn(evidence, error)

    def test_clean_main_passes(self):
        self.assertTrue(self.analyze("int main(void) { return 0; }\n")["passed"])

    def test_actual_sdk_headers_and_unsigned_macro_have_verified_provider_origins(self):
        windows = platform.system() == "Windows"
        source = ("#include <windows.h>\nunsigned long timeout(void) { return INFINITE; }\n"
                  "unsigned long status(void) { return EXCEPTION_ACCESS_VIOLATION; }\n"
                  "int main(void) { return timeout() == INFINITE && status() != 0U ? 0 : 1; }\n" if windows else
                  "#include <pthread.h>\n#include <unwind.h>\nint main(void) { return 0; }\n")
        summary = self.analyze(source, cli=True)
        origins = summary["scope"]["sdkHeaders"]
        self.assertTrue(summary["sourceClassification"]["complete"], origins)
        self.assertEqual(summary["sourceClassification"]["unclassifiedDiagnosticKeys"], 0)
        self.assertTrue(all(row["category"] == "sdk" for row in origins))
        if windows:
            diagnostics = [row for row in summary["diagnostics"] if row["id"] == "misra-c2012-7.2"]
            self.assertTrue(diagnostics)
            self.assertFalse(summary["passed"])
            for row in diagnostics:
                self.assertEqual(row["sourceCategory"], "sdk", row)
                literal = next(origin for origin in row["sourceOrigins"] if origin["kind"] == "unsigned-literal")
                self.assertIn(literal["macro"], {"INFINITE", "STATUS_ACCESS_VIOLATION"})
                if literal["macro"] == "INFINITE":
                    self.assertEqual(literal["value"], "0xffffffff")
                header = next(header for header in origins if header["key"] == literal["headerKey"])
                self.assertEqual(header["provider"], "mingw-w64-x86_64-headers")
            for macro in ("INFINITE", "STATUS_ACCESS_VIOLATION"):
                self.assertTrue(any(origin.get("macro") == macro and origin["kind"] == "unsigned-literal"
                                    for row in diagnostics for origin in row["sourceOrigins"]), macro)
        else:
            self.assertTrue(any(row["header"].endswith("/pthread.h") and row["provider"] == "libc6-dev" for row in origins))
            self.assertTrue(any(row["header"].endswith("/unwind.h") and row["provider"].startswith("libgcc-") for row in origins))

    def test_builtin_constants_and_macro_references_keep_project_responsibility(self):
        summary = self.analyze("#include <stdint.h>\n#define FIXTURE_LIMIT UINT64_C(1)\n"
                               "int main(void) { unsigned value = 0U; return (int)__atomic_load_n(&value, __ATOMIC_ACQUIRE); }\n", cli=True)
        self.assertTrue(summary["sourceClassification"]["complete"], summary["sourceClassification"])
        self.assertFalse(summary["passed"])
        references = [row for row in summary["diagnostics"] if any(
            origin["kind"] == "macro-reference" and origin.get("macro") == "UINT64_C" for origin in row["sourceOrigins"])]
        builtins = [row for row in summary["diagnostics"] if any(
            origin["kind"] == "compiler-predefine" and origin.get("macro") == "__ATOMIC_ACQUIRE" for origin in row["sourceOrigins"])]
        self.assertTrue(references)
        self.assertTrue(builtins)
        self.assertTrue(all(row["sourceCategory"] == "project" for row in references + builtins))

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
                self.assertIn("compiler-unit-0", summary["error"])

    def test_host_adapter_and_application_are_not_classified_as_bsw(self):
        for name in ("integration/src/adapter.c", "app/Application/Application.c"):
            with self.subTest(name=name):
                summary = self.analyze("extern int pthread_create(void);\nint main(void) { return pthread_create(); }\n", name)
                self.assertFalse(any(row["id"] == "autsaro-hostThreadInBsw" for row in summary["diagnostics"]))

    def test_component_skeletons_are_not_analyzed_as_program_units(self):
        # A skeleton repeats the application's definitions; analyzing it with the program reports false 8.6 duplicates.
        source = "int Component_Run(void);\nint Component_Run(void) { return 0; }\nint main(void) { return Component_Run(); }\n"
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = self.project(root, source, extra_files={"generated/templates/Component.c": source, "integration/src/extra.c": "int Extra(void);\nint Extra(void) { return 1; }\n"},
                                   unbuilt=("generated/templates/Component.c", "integration/src/extra.c"))
            target = json.loads((project / "target.json").read_text())["target"]
            c_check.check(project, target, root / "analysis")
            summary = json.loads((root / "analysis/summary.json").read_text())
        self.assertIsNone(summary["error"], summary)
        self.assertEqual(summary["scope"]["supplemental"], ["integration/src/extra.c"])
        self.assertFalse(any(location["file"] == "generated/templates/Component.c" or row["id"] == "misra-c2012-8.6"
                             for row in summary["diagnostics"] for location in row["locations"]), summary["diagnostics"])

    def test_thread_macro_and_typedef_still_report_direct_call(self):
        summary = self.analyze("typedef int ThreadResult;\nextern ThreadResult pthread_create(void);\n#define SPAWN pthread_create\nint main(void) { return SPAWN(); }\n")
        self.assertTrue(any(row["id"] == "autsaro-hostThreadInBsw" for row in summary["diagnostics"]), summary.get("diagnostics"))

    @unittest.skipUnless(platform.system() == "Linux", "glibc target model")
    def test_linux_dynamic_values_keep_their_actual_function_types(self):
        summary = self.analyze("#include <signal.h>\nint main(void) { return SIGRTMIN > 0; }\n")
        self.assertIsNotNone(summary["scope"]["platformModel"])
        definitions = summary["scope"]["compilerDiscoveredDefines"]["bsw/src/main.c"]
        self.assertTrue(any(value.startswith("-DSIGRTMIN=") and "__libc_current_sigrtmin" in value for value in definitions))

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
            project = self.project(root, "#include <signal.h>\nint main(void) { return SIGRTMIN; }\n")
            with patch.object(c_check, "run", side_effect=unsupported_macro):
                self.assertEqual(c_check.check(project, "linux-x64-controlled-v1", root / "analysis"), 1)
            summary = json.loads((root / "analysis/summary.json").read_text())
            self.assertFalse(summary["passed"])
            self.assertIn("Unsupported Linux system macro expansion: SIGRTMIN", summary["error"])

    def test_standard_integer_constant_macros_use_real_compiler_suffixes(self):
        summary = self.analyze("#include <stdint.h>\nint main(void) { uint32_t value = UINT32_C(0xffffffff); return (value > 0u) ? 0 : 1; }\n")
        self.assertFalse(any(row["id"] == "misra-c2012-7.2" for row in summary["diagnostics"]), summary.get("diagnostics"))
        self.assertTrue(any(value.startswith("-DUINT32_C(") for value in summary["scope"]["compilerDiscoveredDefines"]["bsw/src/main.c"]))

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
                    "bsw/src/second.c": '#include "helper.h"\nuint32_t second(void) { return UINT32_C(2); }\n',
                    "bsw/src/third.c": '#include "helper.h"\nuint32_t third(void) { return UINT32_C(3); }\n',
                    "bsw/src/helper.c": 'int broken( { }\n' if invalid else '#include "helper.h"\nuint32_t helper(void) { return UINT32_C(1); }\n'})
                target = json.loads((project / "target.json").read_text())["target"]
                with patch("autosar_tooling.c_check.os.cpu_count", return_value=64):
                    code = c_check.check(project, target, root / "analysis")
                summary = json.loads((root / "analysis/summary.json").read_text())
                self.assertEqual(code, 0 if summary["passed"] else 1)
                if invalid:
                    self.assertFalse(summary["passed"])
                    self.assertIn("bsw/src/helper.c", summary["error"])
                    self.assertIn("compiler-unit-", summary["error"])
                    self.assertIn("exit_code=1", summary["error"])
                else:
                    self.assertIsNone(summary["error"])
                    self.assertEqual(summary["scope"]["compilerWorkers"], min(4, int(c_check.os.environ.get("CARGO_BUILD_JOBS", "4"))))
                    self.assertEqual(set(summary["scope"]["compilerDiscoveredDefines"]), {"bsw/src/main.c", "bsw/src/helper.c", "bsw/src/second.c", "bsw/src/third.c"})
                    for definitions in summary["scope"]["compilerDiscoveredDefines"].values():
                        self.assertTrue(any(value.startswith("-DUINT32_C(") for value in definitions))


    def test_semantic_inputs_from_real_producer_refuse_changed_macro_and_allow_relocation(self):
        from autosar_tooling.c_comparison import compare
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = self.project(root, "#include <limits.h>\nint main(void) { return CHAR_BIT; }\n",
                                   extra_files={"meta/integration.json": '{"plan":{}}'})
            target = json.loads((project / "target.json").read_text())["target"]
            baseline = root / "baseline"
            self.assertEqual(c_check.check(project, target, baseline), 0)
            original = c_check.run
            def changed(*args, **kwargs):
                code, stdout, stderr = original(*args, **kwargs)
                if args[2] == "compiler-values-0":
                    stdout = stdout.replace("AUTSAR_VALUE_CHAR_BIT 8", "AUTSAR_VALUE_CHAR_BIT 16")
                return code, stdout, stderr
            with patch.object(c_check, "run", side_effect=changed):
                self.assertEqual(c_check.check(project, target, root / "changed", baseline), 1)
            summary = json.loads((root / "changed/summary.json").read_text())
            self.assertIn("Semantic identity differs: defines", summary["error"])
            self.assertEqual(summary["semanticIdentity"]["defines"]["bsw/src/main.c"], ["-DCHAR_BIT=16"])
            self.assertEqual(summary["programs"][0]["exitCode"], 0)
            self.assertNotIn("comparison", summary)
            self.assertEqual(c_check.check(project, target, root / "same", baseline), 0)
            summary = json.loads((root / "same/summary.json").read_text())
            # Paths, scheduling and arbitrary executable carriers are provenance.
            summary["scope"]["diagnosticCompiler"].update(path="other/gcc", sha256="other-carrier")
            summary["scope"]["systemIncludePaths"] = ["other/include"]
            summary["scope"]["compilerWorkers"] = 1
            summary["tool"]["binarySha256"] = "other-carrier"
            self.assertEqual(compare(summary, project, root / "same", baseline), {"new": [], "removed": []})

    def test_semantic_capture_uses_actual_model_library_and_analysis_source_contents(self):
        import shutil

        from autosar_tooling.c_comparison import compare
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = self.project(root, 'extern int pthread_create(void);\nint main(void) { return pthread_create(); }\n',
                                   extra_files={"meta/integration.json": '{"plan":{}}'})
            target = json.loads((project / "target.json").read_text())["target"]
            baseline = root / "baseline"
            c_check.check(project, target, baseline)
            before = json.loads((baseline / "summary.json").read_text())
            self.assertIsNone(before["error"])
            self.assertTrue(any(d["id"] == "autsaro-hostThreadInBsw" for d in before["diagnostics"]))
            original_prepare = c_check.prepare
            # The generated header is actually passed to Cppcheck, changing its diagnostics.
            def modeled(*args):
                copied, units, database, scope = original_prepare(*args)
                model = args[2] / "injected-model.h"
                model.write_text("#define pthread_create() 0\n")
                scope["platformModel"] = {"path": str(model), "sha256": hashlib.sha256(model.read_bytes()).hexdigest()}
                return copied, units, database, scope
            with patch.object(c_check, "prepare", side_effect=modeled):
                c_check.check(project, target, root / "modeled", baseline)
            changed = json.loads((root / "modeled/summary.json").read_text())
            self.assertIn("Semantic identity differs: models", changed["error"])
            self.assertFalse(any(d["id"] == "autsaro-hostThreadInBsw" for d in changed["diagnostics"]))
            # Execute the changed addon from its real file, not a self-reported label.
            scripts = root / "scripts"
            scripts.mkdir()
            for name in ("c_check.py", "c_addon.py", "c_comparison.py"):
                shutil.copyfile(Path(c_check.__file__).with_name(name), scripts / name)
            addon = scripts / "c_addon.py"
            addon.write_text(addon.read_text().replace('"pthread_create", ', ''))
            with patch.object(c_check, "__file__", str(scripts / "c_check.py")):
                c_check.check(project, target, root / "addon", baseline)
            changed = json.loads((root / "addon/summary.json").read_text())
            self.assertIn("Semantic identity differs: analysisCode", changed["error"])
            self.assertFalse(any(d["id"] == "autsaro-hostThreadInBsw" for d in changed["diagnostics"]))
            # Content capture also covers the optional Windows library on Linux.
            scope = before["scope"]
            library = root / "platform-library.cfg"
            library.write_text('<def format="2"/>\n')
            scope["platformLibrary"] = {"path": str(library)}
            executable = Path(before["programs"][0]["command"][0])
            first = c_check.semantic_identity(executable, scope)
            library.write_text('<def format="2"><function name="setvbuf"/></def>\n')
            second = c_check.semantic_identity(executable, scope)
            self.assertNotEqual(first["models"]["platformLibrary"], second["models"]["platformLibrary"])
            # Alter a real cfg copy; selecting this installation loads and fingerprints it.
            install = root / "install"
            (install / "cfg").mkdir(parents=True)
            for name, path in scope["cppcheckLibraries"].items():
                shutil.copyfile(path, install / "cfg" / (name + ".cfg"))
            (install / "addons").mkdir()
            for name, path in scope["addonRuntime"].items():
                shutil.copyfile(path, install / "addons" / name)
            first = c_check.semantic_identity(install / "cppcheck", scope)
            cfg = install / "cfg/std.cfg"
            cfg.write_text(cfg.read_text().replace('<valid>-1.0:1.0</valid>', '<valid>0.0:1.0</valid>', 1))
            second = c_check.semantic_identity(install / "cppcheck", scope)
            self.assertNotEqual(first["libraries"]["std"], second["libraries"]["std"])
            directories = (root / "program-a", root / "program-b")
            for directory in directories:
                directory.mkdir()
                shutil.copyfile(install / "cfg/std.cfg", directory / "std.cfg")
            c_check.semantic_identity(install / "cppcheck", scope, directories)
            (directories[1] / "std.cfg").write_text('<def format="2"/>\n')
            with self.assertRaisesRegex(ValueError, "implicit inputs differ between programs"):
                c_check.semantic_identity(install / "cppcheck", scope, directories)

            # The unchanged original producer result remains comparable.
            self.assertEqual(compare(before, project, baseline, baseline), {"new": [], "removed": []})

    def test_actual_compiler_predefines_and_release_are_comparison_inputs(self):
        source = ('#if __GNUC__ > 90\nint main(void) { int value = 1; if (value) { return 1; } return 0; }\n'
                  '#else\nint main(void) { return 0; }\n#endif\n')
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = self.project(root, source, extra_files={"meta/integration.json": '{"plan":{}}'})
            target = json.loads((project / "target.json").read_text())["target"]
            self.assertEqual(c_check.check(project, target, root / "baseline"), 0)
            original = c_check.run
            for stage in ("compiler-macros", "compiler-release"):
                def changed(*args, stage=stage, **kwargs):
                    code, stdout, stderr = original(*args, **kwargs)
                    if args[2] == stage:
                        if stage == "compiler-release":
                            stdout = "99.1.0\n"
                        else:
                            import re
                            stdout = re.sub(r"#define __GNUC__ [0-9]+", "#define __GNUC__ 99", stdout)
                    return code, stdout, stderr
                with self.subTest(stage=stage), patch.object(c_check, "run", side_effect=changed):
                    self.assertEqual(c_check.check(project, target, root / stage, root / "baseline"), 1)
                summary = json.loads((root / stage / "summary.json").read_text())
                self.assertIn("Semantic identity differs: compiler", summary["error"])
                self.assertIn(summary["programs"][0]["exitCode"], (0, 1))
                if stage == "compiler-macros":
                    self.assertTrue(any(row["id"].startswith("misra-c2012-") for row in summary["diagnostics"]))

    @unittest.skipUnless(platform.system() == "Linux", "ELF carrier and shell relocation positive control")
    def test_actual_relocated_tool_carriers_keep_semantic_equivalence(self):
        import os
        import shutil

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = self.project(root, "int main(void) { return 0; }\n",
                                   extra_files={"meta/integration.json": '{"plan":{}}'})
            target = "linux-x64-controlled-v1"
            self.assertEqual(c_check.check(project, target, root / "baseline"), 0)
            before = json.loads((root / "baseline/summary.json").read_text())
            install = root / "relocated"
            (install / "cfg").mkdir(parents=True)
            original = Path(before["programs"][0]["command"][0])
            executable = install / "cppcheck"
            shutil.copyfile(original, executable)
            # ELF permits trailing carrier data; actual analyzer code is unchanged.
            with executable.open("ab") as carrier:
                carrier.write(b"relocation positive control\n")
            executable.chmod(0o700)
            for name, path in before["scope"]["cppcheckLibraries"].items():
                shutil.copyfile(path, install / "cfg" / (name + ".cfg"))
            shutil.copytree(c_check.addon_directory(original), install / "addons", ignore=shutil.ignore_patterns("__pycache__"))
            compiler = install / "gcc"
            import shlex
            compiler.write_text("#!/bin/sh\nexec " + shlex.quote(before["scope"]["diagnosticCompiler"]["path"]) + ' "$@"\n')
            compiler.chmod(0o700)
            with patch.dict(os.environ, {"CPPCHECK": str(executable), "CPPCHECK_ADDON_DIR": str(c_check.addon_directory(original)),
                                         "AUTOSAR_CC": str(compiler)}):
                self.assertEqual(c_check.check(project, target, root / "relocated-result", root / "baseline"), 0)
            after = json.loads((root / "relocated-result/summary.json").read_text())
            self.assertNotEqual(before["tool"]["binarySha256"], after["tool"]["binarySha256"])
            self.assertNotEqual(before["scope"]["diagnosticCompiler"]["sha256"], after["scope"]["diagnosticCompiler"]["sha256"])
            self.assertEqual(before["semanticIdentity"], after["semanticIdentity"])
            self.assertEqual(after["comparison"], {"new": [], "removed": []})

    def test_implicit_library_runner_and_execution_helpers_are_actual_inputs(self):
        import os
        import shutil

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = self.project(root, 'extern int pthread_create(void);\nextern int review_probe(int);\n'
                                   'int main(void) { return pthread_create() + review_probe(1); }\n',
                                   extra_files={"meta/integration.json": '{"plan":{}}'})
            target = json.loads((project / "target.json").read_text())["target"]
            original = Path(os.environ.get("CPPCHECK") or shutil.which("cppcheck")).resolve()
            install = root / "install"
            (install / "cfg").mkdir(parents=True)
            executable = install / original.name
            shutil.copyfile(original, executable)
            executable.chmod(0o700)
            # Match the real CMake install layout, with all actual runner dependencies.
            scope_output = root / "locate"
            c_check.check(project, target, scope_output)
            scope = json.loads((scope_output / "summary.json").read_text())["scope"]
            for name, path in scope["cppcheckLibraries"].items():
                shutil.copyfile(path, install / "cfg" / (name + ".cfg"))
            shutil.copytree(c_check.addon_directory(original), install / "addons", ignore=shutil.ignore_patterns("__pycache__"))
            modules = root / "modules"
            shutil.copytree(Path(c_check.__file__).parents[1], modules, ignore=shutil.ignore_patterns("__pycache__", "*.egg-info"))
            env = {**os.environ, "PYTHONPATH": str(modules), "PYTHONDONTWRITEBYTECODE": "1",
                   "CPPCHECK": str(executable), "CPPCHECK_ADDON_DIR": str(install / "addons")}
            def run(name, baseline=None):
                argv = [sys.executable, "-B", "-m", "autosar_tooling", "c-check", "--target", target,
                        "--project", str(project), "--output-directory", str(root / name)]
                if baseline:
                    argv.extend(["--baseline", str(root / baseline)])
                result = subprocess.run(argv, env=env, capture_output=True, text=True, check=False)
                summary = json.loads((root / name / "summary.json").read_text())
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn(summary["programs"][0]["exitCode"], (0, 1))
                return summary
            before = run("baseline")
            self.assertIsNone(before["error"])
            shadow = install / "std.cfg"
            shadow.write_text('<def format="2"><function name="review_probe"><arg nr="1"><valid>0</valid></arg></function></def>\n')
            after = run("shadow", "baseline")
            self.assertIn("Semantic identity differs: libraries", after["error"])
            self.assertFalse(any(d["id"] == "invalidFunctionArg" for d in before["diagnostics"]))
            self.assertTrue(any(d["id"] == "invalidFunctionArg" for d in after["diagnostics"]))
            shadow.unlink()
            runner = install / "addons/runaddon.py"
            runner_text = runner.read_text()
            needle = "    runpy.run_path(addon, run_name='__main__')"
            self.assertIn(needle, runner_text)
            runner.write_text(runner_text.replace(needle,
                "    original_report = cppcheckdata.reportError\n"
                "    def filtered(*args, **kwargs):\n"
                "        if len(args) > 4 and args[3:5] == ('autsaro', 'hostThreadInBsw'):\n"
                "            return\n"
                "        return original_report(*args, **kwargs)\n"
                "    cppcheckdata.reportError = filtered\n" + needle))
            after = run("runner", "baseline")
            self.assertIn("Semantic identity differs: addonRuntime", after["error"])
            self.assertFalse(any(d["id"] == "autsaro-hostThreadInBsw" for d in after["diagnostics"]))
            runner.write_text(runner_text)
            if platform.system() != "Windows":
                launcher = modules / "ecu_tools/launch.py"
                text = launcher.read_text()
                needle = "        os.execve(argv[0], argv, os.environ)"
                self.assertIn(needle, text)
                launcher.write_text(text.replace(needle, '        if "--xml" in argv:\n'
                                                '            argv.append("--suppress=autsaro-hostThreadInBsw")\n' + needle))
                after = run("launcher", "baseline")
                self.assertIn("Semantic identity differs: analysisCode", after["error"])
                self.assertFalse(any(d["id"] == "autsaro-hostThreadInBsw" for d in after["diagnostics"]))
                launcher.write_text(text)
            config = install / "cppcheck.cfg"
            config.write_text('{"suppressions":["autsaro-hostThreadInBsw"]}')
            with patch.dict(os.environ, env):
                self.assertEqual(c_check.check(project, target, root / "config"), 1)
            error = json.loads((root / "config/summary.json").read_text())["error"]
            self.assertIn("Implicit Cppcheck config adds unsupported addons, suppressions or safety overrides", error)
            config.write_text('{"safety":true}')
            with patch.dict(os.environ, env):
                self.assertEqual(c_check.check(project, target, root / "safety"), 1)
            error = json.loads((root / "safety/summary.json").read_text())["error"]
            self.assertIn("Implicit Cppcheck config adds unsupported", error)

            config.write_text('{"manualUrl":"https://example.invalid/display-only"}')
            after = run("metadata", "baseline")
            self.assertIsNone(after["error"])
            self.assertEqual(after["comparison"], {"new": [], "removed": []})
