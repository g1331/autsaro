"""Exercise the target compiler's conditional evaluation evidence."""

from __future__ import annotations

import gzip
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from autosar_tooling import c_check, c_preprocessor
from ecu_tools.build import digest, tool_argument


@unittest.skipUnless(shutil.which("gcc"), "GCC is needed for preprocessing evidence")
class PreprocessingEvidenceTests(unittest.TestCase):
    def test_repeated_include_observes_the_current_definition_and_only_evaluated_branches(self):
        with tempfile.TemporaryDirectory() as directory:
            scratch = Path(directory)
            root = scratch / "project"
            root.mkdir()
            (root / "check.h").write_text(
                "#if FLAG == 1\n"
                "int first;\n"
                "#elif FLAG == 2\n"
                "int second;\n"
                "#elif NEVER_EVALUATED\n"
                "#endif\n",
                encoding="utf-8",
            )
            (root / "main.c").write_text(
                "#define FLAG 1\n"
                '#include "check.h"\n'
                "#undef FLAG\n"
                "#define FLAG 2\n"
                '#include "check.h"\n'
                "#if 0\n"
                "#if NEVER_EVALUATED\n"
                "#endif\n"
                "#elif defined(GUARDED) && (GUARDED == 0)\n"
                "#elif UNDEFINED == CLI\n"
                "#elif (__STDC_VERSION__ == 199901L) && (0xffffffffffffffffU > 0)\n"
                "int last = __LINE__;\n"
                "const char *where = __FILE__;\n"
                "#endif\n"
                "#if 2U \\\n"
                "    + 0U\n"
                "int other;\n"
                "#endif\n",
                encoding="utf-8",
            )
            scope = self.reference(scratch, root, "check.h")
            receipt = c_preprocessor.collect(scope, root, scratch, c_check.run)
            report = scratch / receipt["file"]
            self.assertEqual(receipt["sha256"], digest(report))
            with gzip.open(report, "rt", encoding="utf-8") as stream:
                unit = json.load(stream)["translationUnits"]["main.c"]
            events = unit["events"]
            conditions = [event for event in events if event["kind"] in {"if", "elif"}]
            self.assertEqual(
                [(row["file"], row["line"], row["kind"], row["value"]) for row in conditions],
                [
                    ("check.h", 1, "if", 1),
                    ("check.h", 1, "if", 0),
                    ("check.h", 3, "elif", 1),
                    ("main.c", 6, "if", 0),
                    ("main.c", 9, "elif", 0),
                    ("main.c", 10, "elif", 0),
                    ("main.c", 11, "elif", 1),
                    ("main.c", 15, "if", "other"),
                ],
            )
            self.assertEqual(conditions[0]["definitions"]["FLAG"]["body"], "1")
            self.assertEqual(conditions[1]["definitions"]["FLAG"]["line"], 4)
            self.assertEqual(conditions[4]["definedTests"], {"GUARDED": None})
            self.assertEqual(conditions[5]["unexpandedIdentifiers"], ["UNDEFINED"])
            self.assertEqual(conditions[5]["definitions"]["CLI"]["file"], "<command-line>")
            self.assertEqual(conditions[6]["definitions"]["__STDC_VERSION__"]["file"], "<built-in>")
            diagnostics = unit["compilerDiagnostics"]
            self.assertEqual(diagnostics, (scratch / "compiler-origins-0.stderr.log").read_text(
                encoding="utf-8"))
            warnings = [line for line in diagnostics.splitlines() if line.endswith("[-Wundef]")]
            self.assertEqual(len(warnings), 1)
            self.assertIn("main.c:10:", warnings[0])
            self.assertIn("UNDEFINED", warnings[0])
            self.assertEqual(
                [(row["kind"], row["line"]) for row in events if row.get("name") == "FLAG"],
                [("define", 1), ("undef", 3), ("define", 4)],
            )

    def test_collect_rejects_changed_preprocessor_output(self):
        with tempfile.TemporaryDirectory() as directory:
            scratch = Path(directory)
            root = scratch / "project"
            root.mkdir()
            (root / "main.c").write_text(
                "#if 0 /* multi\n"
                "*/ || 1\n"
                "int emitted;\n"
                "#endif\n",
                encoding="utf-8",
            )
            scope = self.reference(scratch, root)
            with self.assertRaisesRegex(
                ValueError, r"^Preprocessing probes changed emitted tokens or include order: main\.c$",
            ):
                c_preprocessor.collect(scope, root, scratch, c_check.run)

    def test_undef_removes_active_definition_at_next_evaluation(self):
        condition = self.condition_after_definition("#undef FLAG\n")
        self.assertEqual(condition["value"], 0)
        self.assertEqual(condition["expanded"], "0")
        self.assertEqual(condition["definitions"], {})
        self.assertEqual(condition["definedTests"], {"FLAG": None})

    def test_defined_test_records_the_active_definition(self):
        condition = self.condition_after_definition("")
        self.assertEqual(condition["value"], 1)
        self.assertEqual(condition["definedTests"], {
            "FLAG": {"file": "main.c", "line": 1, "body": "7", "function": False},
        })

    def condition_after_definition(self, intervening_directives):
        with tempfile.TemporaryDirectory() as directory:
            scratch = Path(directory)
            root = scratch / "project"
            root.mkdir()
            (root / "main.c").write_text(
                f"#define FLAG 7\n{intervening_directives}"
                "#if defined(FLAG)\n"
                "int emitted;\n"
                "#endif\n",
                encoding="utf-8",
            )
            scope = self.reference(scratch, root)
            receipt = c_preprocessor.collect(scope, root, scratch, c_check.run)
            with gzip.open(scratch / receipt["file"], "rt", encoding="utf-8") as stream:
                events = json.load(stream)["translationUnits"]["main.c"]["events"]
            condition, = [event for event in events if event["kind"] == "if"]
            return condition

    def reference(self, scratch, root, *headers):
        scope = self.scope()
        scope["projectTranslationUnits"] = {"main.c": [*headers, "main.c"]}
        c_check.run(
            [scope["diagnosticCompiler"]["path"], *scope["compilerFlags"],
             "-E", "-dD", *c_preprocessor.WARNINGS, tool_argument(root / "main.c")],
            scratch, "compiler-origins-0", 60,
        )
        return scope

    @staticmethod
    def scope():
        compiler = Path(shutil.which("gcc")).resolve()
        return {
            "diagnosticCompiler": {"path": str(compiler)},
            "compilerFlags": ["-std=c99", "-Werror", "-DCLI=1"],
            "compilerWorkers": 1,
            "includePaths": [],
            "target": "fixture",
        }
