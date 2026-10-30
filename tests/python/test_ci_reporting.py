"""Reporting is presentation: real check failures keep their original verdict."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from collections import Counter
from pathlib import Path
from unittest import mock

from autosar_tooling import ci_reporting as report


class CiReportingTests(unittest.TestCase):
    def test_real_failure_success_and_exit_code_are_observable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "summary.md"
            test = root / "test_probe.py"
            test.write_text(
                "import unittest\nclass Probe(unittest.TestCase):\n def test_real_failure(self): self.assertEqual(1, 2)\n",
                encoding="utf-8",
            )
            env = {**os.environ, "GITHUB_STEP_SUMMARY": str(output)}
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "autosar_tooling.ci_reporting",
                    "run",
                    "--",
                    sys.executable,
                    "-m",
                    "unittest",
                    "discover",
                    "-s",
                    str(root),
                ],
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertIn(
                "Failed test: test_real_failure (test_probe.Probe.test_real_failure)",
                output.read_text(),
            )
            self.assertIn("AssertionError: 1 != 2", result.stdout)
            with mock.patch.dict(os.environ, env):
                self.assertEqual(
                    report.run([sys.executable, "-c", "raise SystemExit(7)"]), 7
                )
                output.unlink()
                self.assertEqual(report.run([sys.executable, "-c", "pass"]), 0)
            self.assertIn("Exit status: 0", output.read_text())
            self.assertNotIn("Failed test", output.read_text())

    def test_failure_formats_and_requirement_identifiers(self):
        self.assertEqual(
            report.failure_names(
                "test namespace::bad ... FAILED\n FAIL  tests/sample.test.ts > rejects\n"
            ),
            ["namespace::bad", "tests/sample.test.ts > rejects"],
        )
        text = """SWS_Test: 3 official items; verified 1, undisposed 1; 1 stale citations

Undisposed items:
  SWS_Test_00001

Stale citations (not an official item of R24-11):
  tests/example.py:7: SWS_Test_99999
"""
        lines = report.requirement_lines(text)
        self.assertIn("SWS_Test_00001", lines)
        self.assertIn("tests/example.py:7: SWS_Test_99999", lines)
        self.assertIn("undisposed 1; 1 stale citations", lines[0])

    def test_sarif_paths_messages_and_invalid_locations(self):
        rows = [
            {
                "rule": "misra-c2012-8.4",
                "file": "runtime/a b.c",
                "line": 4,
                "message": '<unsafe & "quoted">',
            }
        ]
        result = report.sarif(rows, "Cppcheck")
        entry = result["runs"][0]["results"][0]
        self.assertEqual(entry["ruleId"], rows[0]["rule"])
        self.assertEqual(entry["message"]["text"], rows[0]["message"])
        physical = entry["locations"][0]["physicalLocation"]
        self.assertEqual(physical["artifactLocation"]["uri"], "runtime/a%20b.c")
        self.assertEqual(physical["region"]["startLine"], 4)
        for name in ("../secret.c", "/tmp/a.c", "C:\\private.c"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                report.sarif([{**rows[0], "file": name}], "Cppcheck")
        with self.assertRaises(ValueError):
            report.sarif([{**rows[0], "line": 0}], "Cppcheck")

    def test_cppcheck_runtime_mapping_new_and_missing_baseline_are_distinct(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "results/sample/project"
            (project / "meta").mkdir(parents=True)
            (project / "runtime").mkdir()
            (root / "runtime").mkdir()
            (project / "runtime/test.c").write_text("preamble\nunique_code();\n")
            (root / "runtime/test.c").write_text("unique_code();\n")
            (project / "meta/integration.json").write_text(
                json.dumps({"originalBswSources": {"runtime/test.c": "runtime/test.c"}})
            )
            result_dir = root / "results/sample"
            result_dir.mkdir(parents=True, exist_ok=True)
            data = {
                "passed": False,
                "error": None,
                "sourceClassification": {"complete": False, "sdkDiagnosticKeys": 2, "compilerPredefineDiagnosticKeys": 3,
                                         "unclassifiedDiagnosticKeys": 1, "unclassifiedHeaders": 1,
                                         "unresolvedMacroDiagnostics": 1},
                "diagnostics": [
                    {
                        "id": "rule",
                        "message": "finding",
                        "locations": [{"file": "runtime/test.c", "line": "2"}],
                    }
                ],
            }
            output = root / "summary.md"
            with mock.patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(output)}):
                (result_dir / "summary.json").write_text(json.dumps(data))
                rows = report.cppcheck(result_dir.parent, root)
                self.assertIn("New diagnostics: unknown", output.read_text())
                self.assertIn("SDK source provenance: complete=False", output.read_text())
                self.assertIn("unclassified keys=1", output.read_text())
                self.assertIn("compiler predefine keys=3", output.read_text())
                self.assertIn("source passed=False", output.read_text())
                self.assertEqual(rows[0]["file"], "runtime/test.c")
                self.assertEqual(rows[0]["line"], 1)
                output.unlink()
                data["comparison"] = {
                    "new": [
                        {
                            "rule": "rule",
                            "file": "runtime/test.c",
                            "source": "unique_code();",
                            "count": 2,
                        }
                    ],
                    "removed": [],
                }
                (result_dir / "summary.json").write_text(json.dumps(data))
                report.cppcheck(result_dir.parent, root)
                self.assertIn("New diagnostics: 2 occurrences", output.read_text())
                self.assertIn("rule runtime/test.c:1", output.read_text())
                self.assertNotIn("unknown", output.read_text())
                output.unlink()
                del data["comparison"]
                (result_dir / "summary.json").write_text(json.dumps(data))
                with mock.patch(
                    "autosar_tooling.c_comparison.compare",
                    side_effect=ValueError("C baseline refused: identity differs"),
                ):
                    report.cppcheck(result_dir.parent, root, root / "baseline")
                self.assertIn(
                    "C baseline refused: identity differs", output.read_text()
                )
                self.assertIn("New diagnostics: unknown", output.read_text())
                self.assertFalse(
                    json.loads((result_dir / "summary.json").read_text())["passed"]
                )

                sarif_path = root / "result.sarif"
                with mock.patch("os.getcwd", return_value=str(root)):
                    self.assertEqual(
                        report.main(
                            [
                                "sarif",
                                "--cppcheck-directory",
                                str(result_dir.parent),
                                "--baseline-directory",
                                str(root / "missing-baseline"),
                                "--output",
                                str(sarif_path),
                            ]
                        ),
                        0,
                    )
                self.assertEqual(
                    json.loads(sarif_path.read_text())["runs"][0]["results"][0][
                        "ruleId"
                    ],
                    "rule",
                )

    def test_clang_tidy_uses_shared_baseline_and_reports_partial_analysis(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "runtime").mkdir()
            (root / "runtime/test.c").write_text("bad();\n")
            artifact = root / "tidy.json"
            artifact.write_text(
                json.dumps(
                    [
                        {
                            "rule": "readability-test",
                            "file": "runtime/test.c",
                            "line": 1,
                            "message": "bad name",
                            "sample": "sample",
                        }
                    ]
                    * 2
                )
            )
            output = root / "summary"
            with (
                mock.patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(output)}),
                mock.patch(
                    "autosar_tooling.quality.read_clang_tidy_baseline",
                    return_value=Counter(),
                ),
            ):
                rows = report.clang_tidy(artifact, root / "samples", root, "failure")
            self.assertEqual(len(rows), 1)
            self.assertIn("report may be partial", output.read_text())
            self.assertIn(
                "new clang-tidy finding [readability-test]", output.read_text()
            )
            self.assertIn("runtime/test.c:1", output.read_text())

    def test_raw_foreign_findings_survive_but_baseline_counts_use_columns_and_scope(
        self,
    ):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in (
                "runtime/test.c",
                "samples/sample/kernel/test.c",
                "samples/sample/generated.c",
            ):
                source = root / name
                source.parent.mkdir(parents=True, exist_ok=True)
                source.write_text("bad();\n")
            base = {
                "rule": "test-rule",
                "file": "runtime/test.c",
                "line": 1,
                "column": 1,
                "message": "bad",
                "sample": "sample",
            }
            raw = [
                base,
                base,
                {**base, "column": 9},
                {**base, "file": "kernel/test.c"},
                {
                    **base,
                    "file": "generated.c",
                    "rule": "readability-identifier-naming",
                },
            ]
            artifact = root / "tidy.json"
            artifact.write_text(json.dumps(raw))
            output = root / "summary"
            baseline = Counter({("test-rule", "runtime/test.c", "bad();"): 2})
            with (
                mock.patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(output)}),
                mock.patch(
                    "autosar_tooling.quality.read_clang_tidy_baseline",
                    return_value=baseline,
                ),
            ):
                rows = report.clang_tidy(artifact, root / "samples", root, "success")
                self.assertEqual(len(rows), 4)
                self.assertIn("No new or fixed baseline findings.", output.read_text())
                self.assertNotIn("new clang-tidy finding", output.read_text())
                artifact.write_text(json.dumps([base]))
                output.unlink()
                report.clang_tidy(artifact, root / "samples", root, "failure")
                self.assertNotIn("fixed finding, remove", output.read_text())
                self.assertIn("report may be partial", output.read_text())

    def test_merged_quality_producer_and_consumer_share_raw_identity_and_origin_mapping(
        self,
    ):
        from autosar_tooling import quality

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "samples/sample"
            for name in (
                "runtime/Test.c",
                "third_party/kernel.h",
                "samples/sample/unit.c",
                "samples/sample/kernel/test.h",
                "samples/sample/bsw/Test.h",
            ):
                source = root / name
                source.parent.mkdir(parents=True, exist_ok=True)
                source.write_text("int x;\n")
            (project / "meta").mkdir()
            (project / "meta/integration.json").write_text(
                json.dumps(
                    {
                        "originalBswSources": {
                            "runtime/Test.c": "unit.c",
                            "third_party/kernel.h": "kernel/test.h",
                        }
                    }
                )
            )
            (project / "target.json").write_text(
                json.dumps(
                    {
                        "target": "linux-test",
                        "kernelPatches": [],
                        "includePaths": ["."],
                        "sources": ["unit.c"],
                        "entries": {},
                        "compilerFlags": ["-std=c99"],
                    }
                )
            )
            output = "".join(
                f"{project}/unit.c:1:{column}: warning: Runtime [test-rule]\n"
                for column in (2, 3, 2)
            )
            output += f"{project}/bsw/Test.h:1:1: warning: Generated [readability-identifier-naming]\n"
            output += f"{project}/kernel/test.h:1:1: warning: Foreign [test-rule]\n"
            artifact = root / "raw.json"
            summary = root / "summary"
            baseline = Counter({("test-rule", "runtime/Test.c", "int x;"): 2})
            with (
                mock.patch.dict(
                    os.environ,
                    {
                        "AUTOSAR_CLANG_TIDY_REPORT": str(artifact),
                        "GITHUB_STEP_SUMMARY": str(summary),
                    },
                ),
                mock.patch.object(quality.shutil, "which", return_value="tidy"),
                mock.patch.object(
                    quality,
                    "run",
                    return_value=subprocess.CompletedProcess([], 0, output, ""),
                ),
                mock.patch.object(
                    quality, "read_clang_tidy_baseline", return_value=baseline
                ),
            ):
                self.assertEqual(quality.clang_tidy_errors(project.parent), [])
                self.assertEqual(len(json.loads(artifact.read_text())), 5)
                rows = report.clang_tidy(artifact, project.parent, root, "success")
            self.assertIn("No new or fixed baseline findings.", summary.read_text())
            self.assertEqual(
                {row["file"] for row in rows},
                {"runtime/Test.c", "third_party/kernel.h", "samples/sample/bsw/Test.h"},
            )

    def test_new_diagnostic_position_survives_trailing_documentation_normalization(
        self,
    ):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "runtime/a.c"
            source.parent.mkdir()
            source.write_text(
                "typedef void *BadPtr; /**< A documented pointer type. */\n"
            )
            artifact = root / "tidy.json"
            artifact.write_text(
                json.dumps(
                    [
                        {
                            "rule": "readability-identifier-naming",
                            "file": "runtime/a.c",
                            "line": 1,
                            "column": 15,
                            "message": "invalid case style",
                            "sample": "probe",
                        }
                    ]
                )
            )
            output = root / "summary"
            with (
                mock.patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(output)}),
                mock.patch(
                    "autosar_tooling.quality.read_clang_tidy_baseline",
                    return_value=Counter(),
                ),
            ):
                rows = report.clang_tidy(artifact, root / "samples", root, "success")
            self.assertIn(
                "new clang-tidy finding [readability-identifier-naming]: typedef void *BadPtr;",
                output.read_text(),
            )
            self.assertIn("runtime/a.c:1", output.read_text())
            self.assertEqual(
                report.sarif(rows, "clang-tidy")["runs"][0]["results"][0]["locations"][
                    0
                ]["physicalLocation"]["region"]["startLine"],
                1,
            )

    def test_relative_sample_cli_maps_actual_generated_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "samples/probe/probe.c"
            source.parent.mkdir(parents=True)
            source.write_text("bad();\n")
            artifact = root / "tidy.json"
            artifact.write_text(
                json.dumps(
                    [
                        {
                            "rule": "test-rule",
                            "file": "probe.c",
                            "line": 1,
                            "message": "bad",
                            "sample": "probe",
                        }
                    ]
                )
            )
            with (
                mock.patch("os.getcwd", return_value=str(root)),
                mock.patch(
                    "autosar_tooling.quality.read_clang_tidy_baseline",
                    return_value=Counter(),
                ),
            ):
                rows = report.clang_tidy(artifact, Path("samples"), root, "success")
            self.assertEqual(rows[0]["file"], "samples/probe/probe.c")
            self.assertEqual(rows[0]["line"], 1)
            # The helper's explicit repository is also the base when process cwd differs.
            with mock.patch(
                "autosar_tooling.quality.read_clang_tidy_baseline",
                return_value=Counter(),
            ):
                rows = report.clang_tidy(artifact, Path("samples"), root, "success")
            self.assertEqual(rows[0]["file"], "samples/probe/probe.c")

    def test_relative_samples_under_a_real_symlinked_repository_root(self):
        with tempfile.TemporaryDirectory() as directory:
            physical = Path(directory) / "physical"
            source = physical / "samples/probe/probe.c"
            source.parent.mkdir(parents=True)
            source.write_text("bad();\n")
            alias = Path(directory) / "alias"
            try:
                alias.symlink_to(physical, target_is_directory=True)
            except (OSError, NotImplementedError) as error:
                self.skipTest(f"Directory symlinks unavailable: {error}")
            artifact = physical / "tidy.json"
            artifact.write_text(
                json.dumps(
                    [
                        {
                            "rule": "test-rule",
                            "file": "probe.c",
                            "line": 1,
                            "message": "bad",
                            "sample": "probe",
                        }
                    ]
                )
            )
            with (
                mock.patch("os.getcwd", return_value=str(alias)),
                mock.patch(
                    "autosar_tooling.quality.read_clang_tidy_baseline",
                    return_value=Counter(),
                ),
            ):
                rows = report.clang_tidy(artifact, Path("samples"), alias, "success")
            self.assertEqual(rows[0]["file"], "samples/probe/probe.c")
            self.assertEqual(rows[0]["line"], 1)

    def test_summary_escapes_external_markdown_and_html(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "summary"
            with mock.patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(path)}):
                report.summary("<title>", ["<script>bad</script>"])
            self.assertNotIn("<script>", path.read_text())
            self.assertIn("&lt;script&gt;", path.read_text())
