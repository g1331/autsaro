"""Failure contracts of the C analysis entry; no installed C tools required."""
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import c_check
from ecu_tools.build import target_configuration


class CAnalysisTests(unittest.TestCase):
    def test_compiler_header_paths_use_the_compiler_working_directory(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve(strict=True)
            include = directory / "include"
            include.mkdir()
            reports = ["include", include.as_posix()]
            if include.drive:
                reports.append(include.as_posix()[len(include.drive):])
            for report in reports:
                with self.subTest(report=report):
                    log = f"ignoring nonexistent directory absent\n#include <...> search starts here:\n {report}\nEnd of search list.\n"
                    self.assertEqual(c_check.compiler_include_paths(log, directory), [include.as_posix()])
            with self.assertRaisesRegex(ValueError, "no system include paths"):
                c_check.compiler_include_paths("no search list", directory)
            for invalid in ("absent", "file.h"):
                (directory / "file.h").write_text("not a directory")
                with self.subTest(invalid=invalid), self.assertRaises((ValueError, FileNotFoundError)):
                    c_check.compiler_include_paths(f"#include <...> search starts here:\n {invalid}\nEnd of search list.", directory)

    def test_build_configuration_rejects_missing_or_duplicated_units(self):
        target = {"compilerFlags": ["-std=c99"], "sources": ["main.c"], "includePaths": ["."]}
        self.assertEqual(target_configuration(["main.c"], target), (["-std=c99"], ["main.c"], ["."]))
        for sources in ([], ["absent.c"], ["main.c", "main.c"], ["../main.c"]):
            with self.subTest(sources=sources), self.assertRaises(ValueError):
                target_configuration(["main.c"], {**target, "sources": sources})

    def test_target_configuration_requires_the_declared_c99_mode(self):
        target = {"sources": ["main.c"], "includePaths": ["."]}
        for flags in ([], ["-Wall"], ["-std=c11"]):
            with self.subTest(flags=flags), self.assertRaises(ValueError):
                target_configuration(["main.c"], {**target, "compilerFlags": flags})

    def test_output_conflicts_preserve_existing_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve(strict=True)
            project = root / "source"
            project.mkdir()
            for output in (project, project / "analysis", root):
                with self.subTest(output=output), self.assertRaises(ValueError):
                    c_check.check(project, "linux-x64-controlled-v1", output)
            occupied = root / "occupied"
            occupied.mkdir()
            (occupied / "keep.txt").write_text("owner")
            with self.assertRaises(ValueError):
                c_check.check(project, "linux-x64-controlled-v1", occupied)
            self.assertEqual((occupied / "keep.txt").read_text(), "owner")

    def test_tool_failures_are_nonzero_and_retained(self):
        for error in (ValueError("missing tool"), ValueError("version mismatch"), RuntimeError("timed out"), OSError("addon crash")):
            with self.subTest(error=str(error)), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary).resolve(strict=True)
                project = root / "source"
                project.mkdir()
                output = root / "analysis"
                with patch.object(c_check, "tool_identity", side_effect=error):
                    self.assertEqual(c_check.check(project, "linux-x64-controlled-v1", output), 1)
                summary = json.loads((output / "summary.json").read_text())
                self.assertFalse(summary["passed"])
                self.assertEqual(summary["error"], str(error))
                self.assertEqual(list(project.iterdir()), [])

    def test_xml_preserves_rule_locations_and_separates_adopted_code(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve(strict=True)
            xml = f'<results><errors><error id="misra-c2012-10.4" severity="style" msg="type mismatch"><location file="{root.as_posix()}/src/Can.c" line="9" column="2"/></error><error id="style" severity="style" msg="kernel"><location file="{root.as_posix()}/kernel/tasks.c" line="1"/></error></errors></results>'
            errors = c_check.parse_diagnostics(xml, root)
            self.assertEqual(errors[0]["locations"][0]["file"], "src/Can.c")
            self.assertFalse(errors[0]["adopted"])
            self.assertTrue(errors[1]["adopted"])
        for xml in ("", "<results/>", "<wrong><errors/></wrong>"):
            with self.subTest(xml=xml), self.assertRaises((ValueError, c_check.ET.ParseError)):
                c_check.parse_diagnostics(xml, Path.cwd())

    def test_wrong_cppcheck_version_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve(strict=True)
            fake = root / "cppcheck"
            fake.write_text("not executed")
            with patch.dict(c_check.os.environ, {"CPPCHECK": str(fake)}), patch.object(c_check, "run", return_value=(0, "Cppcheck 2.20", "")), self.assertRaisesRegex(ValueError, "version mismatch"):
                c_check.tool_identity(root)

    def test_addons_from_a_different_release_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve(strict=True)
            fake = root / "cppcheck"
            fake.write_text("not executed")
            addons = root / "addons"
            addons.mkdir()
            for name in c_check.ADDON_HASHES:
                (addons / name).write_text("different source")
            with patch.dict(c_check.os.environ, {"CPPCHECK": str(fake), "CPPCHECK_ADDON_DIR": str(addons)}), patch.object(c_check, "run", return_value=(0, "Cppcheck 2.21.0", "")), self.assertRaisesRegex(ValueError, "addon hashes"):
                c_check.tool_identity(root)

    def test_incomplete_inventory_never_leaves_a_green_summary(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve(strict=True)
            source = directory / "source"
            source.mkdir()
            (source / "files.sha256").write_text("test seal")
            output = directory / "analysis"
            root = output / "project"

            def run(argv, directory, stage):
                (directory / "cache/files.txt").write_text("absent.a1:::missing.c\n")
                return 0, "", "<results><errors/></results>"

            with patch.object(c_check, "tool_identity", return_value=(directory / "cppcheck", directory, {})), patch.object(c_check, "coverage", return_value={}), patch.object(c_check, "prepare", return_value=(root, ["main.c"], [], {})), patch.object(c_check, "run", side_effect=run):
                self.assertEqual(c_check.check(source, "linux-x64-controlled-v1", output), 1)
            summary = json.loads((output / "summary.json").read_text())
            self.assertFalse(summary["passed"])
            self.assertIn("inventory differs", summary["error"])

    def test_native_entry_programs_keep_separate_link_boundaries_and_failures(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve(strict=True)
            source = directory / "source"
            source.mkdir()
            (source / "files.sha256").write_text("seal")
            output = directory / "analysis"
            root = output / "project"
            units = ["src/common.c", "src/ecu_host_batch.c", "src/ecu_probe.c"]
            database = [{"file": str(root / name)} for name in units]
            calls = []

            def analyze(executable, addons, root, sources, database, scope, target, output):
                calls.append(sources)
                failed = output.name == "legacy-probe"
                return (1 if failed else 0), not failed, [], []

            with patch.object(c_check, "tool_identity", return_value=(directory / "cppcheck", directory, {})), patch.object(c_check, "coverage", return_value={}), patch.object(c_check, "prepare", return_value=(root, units, database, {"profile": "ecu"})), patch.object(c_check, "analyze_program", side_effect=analyze), patch.object(c_check, "sealed_sources"):
                self.assertEqual(c_check.check(source, "linux-x64-controlled-v1", output), 1)
            self.assertEqual(calls, [["src/common.c", "src/ecu_host_batch.c"], ["src/common.c", "src/ecu_probe.c"]])
            summary = json.loads((output / "summary.json").read_text())
            self.assertIsNone(summary["error"])
            self.assertFalse(summary["passed"])
            self.assertEqual([row["exitCode"] for row in summary["programs"]], [0, 1])

    def test_malformed_second_program_receipt_cannot_leave_green_summary(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve(strict=True)
            source = directory / "source"
            source.mkdir()
            (source / "files.sha256").write_text("seal")
            output = directory / "analysis"
            root = output / "project"
            units = ["src/common.c", "src/ecu_host_batch.c", "src/ecu_probe.c"]
            database = [{"file": str(root / name)} for name in units]

            def run(argv, directory, stage):
                commands = json.loads((directory / "compile_commands.json").read_text())
                (directory / "cache/files.txt").write_text("".join("dump:::" + row["file"] + "\n" for row in commands))
                receipts = directory / "addon-receipts"
                receipts.mkdir()
                for index, row in enumerate(commands):
                    data = [] if directory.name == "legacy-probe" else {"units": [row["file"]], "configurations": 1}
                    (receipts / f"{index}.json").write_text(json.dumps(data))
                return 0, "", "<results><errors/></results>"

            with patch.object(c_check, "tool_identity", return_value=(directory / "cppcheck", directory, {})), patch.object(c_check, "coverage", return_value={}), patch.object(c_check, "prepare", return_value=(root, units, database, {"profile": "ecu"})), patch.object(c_check, "run", side_effect=run):
                self.assertEqual(c_check.check(source, "linux-x64-controlled-v1", output), 1)
            summary = json.loads((output / "summary.json").read_text())
            self.assertFalse(summary["passed"])
            self.assertEqual(summary["error"], "Invalid project addon receipt structure")
            self.assertTrue(summary["programs"][0]["passed"])
