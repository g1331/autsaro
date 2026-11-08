"""Failure contracts of the C analysis entry; no installed C tools required."""
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path, PureWindowsPath
from unittest.mock import patch

from autosar_tooling import c_check
from ecu_tools.build import target_configuration

ENTRIES = {"host-batch": "integration/src/ecu_host_batch.c", "probe": "integration/src/ecu_probe.c"}


class CAnalysisTests(unittest.TestCase):
    def test_analyzer_inputs_normalizes_root_and_file_aliases(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve()
            root = directory / "project"
            (root / "child").mkdir(parents=True)
            (directory / "sdk").mkdir()
            source = root / "main.c"
            sdk = directory / "sdk/types.h"
            model = directory / "model.h"
            unknown = directory / "unknown.h"
            for path in (source, sdk, model, unknown):
                path.write_bytes(b"fixture\n")
            data = {"source": str(source), "files": sorted([
                str(root / "child/../main.c"), str(source), str(root / "../sdk/types.h"),
                str(root / "../model.h"), str(unknown)])}
            scope = {"sdkHeaders": [{"path": sdk.as_posix()}],
                     "platformModel": {"path": str(model)}}
            unit, inputs = c_check.analyzer_inputs(data, root / "child/..", scope)
            self.assertEqual(unit, "main.c")
            self.assertEqual(inputs, {"project": ["main.c"], "sdk": [sdk.as_posix()],
                                      "unbound": [unknown.as_posix()]})

    def test_analyzer_inputs_windows_drives_separators_case_and_short_aliases(self):
        class WindowsPath(PureWindowsPath):
            def resolve(self):
                # Windows realpath expands 8.3 aliases; emulate that mapping on Linux.
                return type(self)(*("RunnerAdmin" if part.casefold() == "runner~1" else part
                                    for part in self.parts))

        root = WindowsPath(r"C:\Users\RUNNER~1\Temp\PROJECT")
        data = {"source": "c:/users/RunnerAdmin/temp/project/main.c", "files": sorted([
            r"C:\Users\RUNNER~1\TEMP\Project\main.c",
            "c:/USERS/RunnerAdmin/temp/project/config.h", r"d:\SDK\Include\Types.h",
            "d:/ANALYSIS/model.h", "E:/external/unknown.h"])}
        scope = {"sdkHeaders": [{"path": "D:/sdk/include/Types.h"}],
                 "platformModel": {"path": r"D:\analysis\MODEL.h"}}
        with patch.object(c_check, "Path", WindowsPath):
            unit, inputs = c_check.analyzer_inputs(data, root, scope)
            self.assertEqual(unit, "main.c")
            self.assertEqual(inputs, {"project": ["config.h", "main.c"],
                "sdk": ["D:/sdk/include/Types.h"], "unbound": ["E:/external/unknown.h"]})
            data["source"] = "D:/other/main.c"
            with self.assertRaisesRegex(ValueError, "outside the project"):
                c_check.analyzer_inputs(data, root, scope)

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

    def test_automotive_target_requires_unwind_metadata(self):
        target = {"profile": "ecu", "compilerFlags": ["-std=c99"],
                  "sources": ["main.c"], "includePaths": ["."]}
        with self.assertRaisesRegex(ValueError, "exception-unwind metadata"):
            target_configuration(["main.c"], target)
        target["compilerFlags"].append("-fexceptions")
        self.assertEqual(target_configuration(["main.c"], target)[0],
                         ["-std=c99", "-fexceptions"])

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
                self.assertEqual(summary["projectAcceptance"]["status"], "error")
                self.assertEqual(list(project.iterdir()), [])

    def test_xml_preserves_rule_locations_and_separates_adopted_code(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve(strict=True)
            xml = f'<results><errors><error id="misra-c2012-10.4" severity="style" msg="type mismatch"><location file="{root.as_posix()}/src/Can.c" line="9" column="2"/></error><error id="style" severity="style" msg="kernel"><location file="{root.as_posix()}/kernel/tasks.c" line="1"/></error></errors></results>'
            errors = c_check.parse_diagnostics(xml, root)
            self.assertEqual(errors[0]["locations"][0]["file"], "src/Can.c")
            self.assertFalse(errors[0]["adopted"])
            self.assertTrue(errors[1]["adopted"])
            # The same kernel line changed by a pinned patch is project code.
            errors = c_check.parse_diagnostics(xml, root, {"kernel/tasks.c": [1]})
            self.assertFalse(errors[1]["adopted"])
            self.assertTrue(errors[1]["patched"])
            self.assertFalse(errors[0]["patched"])

    def test_patched_kernel_lines_are_the_added_and_changed_lines(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve(strict=True)
            for side, text in (("project", "a\nb\nc\n"), ("root", "a\nB\nc\nd\n")):
                (base / side / "kernel").mkdir(parents=True)
                (base / side / "kernel/tasks.c").write_text(text)
            path = base / "root/kernel/tasks.c"
            change = base / "change.patch"
            change.write_bytes(b"--- a/tasks.c\n+++ b/tasks.c\n"
                               b"@@ -1,3 +1,4 @@\n a\n-b\n+B\n c\n+d\n")
            lines = c_check.kernel_patched_lines({path: (base / "project/kernel/tasks.c").read_bytes()},
                                                base / "root", {}, change, "")
            self.assertEqual(lines, {"kernel/tasks.c": [2, 4]})
            # A later patch restores upstream text and inserts a line above it.
            # Final upstream-vs-delivery diff alone would lose ownership of b.
            before = {path: path.read_bytes()}
            path.write_bytes(b"a\r\ninserted\r\nb\r\nc\r\n")
            change.write_bytes(b"--- a/tasks.c\n+++ b/tasks.c\n"
                               b"@@ -1,4 +1,4 @@\n a\n-B\n+inserted\n+b\n c\n-d\n")
            lines = c_check.kernel_patched_lines(before, base / "root", lines, change, "")
            self.assertEqual(lines, {"kernel/tasks.c": [2, 3]})
            # Delete only the insertion: the untouched restored line stays owned.
            before = {path: path.read_bytes()}
            path.write_text("a\nb\nc\n")
            change.write_bytes(b"--- a/tasks.c\n+++ b/tasks.c\n"
                               b"@@ -1,4 +1,3 @@\n a\n-inserted\n b\n c\n")
            lines = c_check.kernel_patched_lines(before, base / "root", lines, change, "")
            self.assertEqual(lines, {"kernel/tasks.c": [2]})
            xml = '<results><errors><error id="style"><location file="kernel/tasks.c" line="2"/></error></errors></results>'
            diagnostic = c_check.parse_diagnostics(xml, base / "root", lines)[0]
            self.assertTrue(diagnostic["patched"])
            self.assertFalse(diagnostic["adopted"])
        for xml in ("", "<results/>", "<wrong><errors/></wrong>"):
            with self.subTest(xml=xml), self.assertRaises((ValueError, c_check.ET.ParseError)):
                c_check.parse_diagnostics(xml, Path.cwd())

    def test_patch_hunk_offsets_come_from_git_application(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "kernel/tasks.c"
            path.parent.mkdir()
            before = {path: b"z\nx\na\nb\nc\ny\n"}
            path.write_bytes(b"z\nx\na\nb\nb\nc\ny\n")
            change = root / "change.patch"
            change.write_bytes(b"--- a/tasks.c\n+++ b/tasks.c\n@@ -2,3 +2,4 @@\n a\n+b\n b\n c\n")
            report = "Checking patch tasks.c...\nHunk #1 succeeded at 3 (offset 1 line).\n"
            self.assertEqual(c_check.kernel_patched_lines(before, root, {}, change, report),
                             {"kernel/tasks.c": [4]})
            with self.assertRaisesRegex(ValueError, "hunk provenance differs"):
                c_check.kernel_patched_lines(before, root, {}, change, "")

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
            (source / "meta").mkdir()
            (source / "meta/files.sha256").write_text("test seal")
            output = directory / "analysis"
            root = output / "project"

            def run(argv, directory, stage):
                (directory / "cache/files.txt").write_text("absent.a1:::missing.c\n")
                return 0, "", "<results><errors/></results>"

            with patch.object(c_check, "tool_identity", return_value=(directory / "cppcheck", directory, {})), patch.object(c_check, "coverage", return_value={}), patch.object(c_check, "semantic_identity", return_value={}), patch.object(c_check, "prepare", return_value=(root, ["main.c"], [], {"cppcheckLibraries": {}})), patch.object(c_check, "run", side_effect=run):
                self.assertEqual(c_check.check(source, "linux-x64-controlled-v1", output), 1)
            summary = json.loads((output / "summary.json").read_text())
            self.assertFalse(summary["passed"])
            self.assertIn("inventory differs", summary["error"])

    def test_native_entry_programs_keep_separate_link_boundaries_and_failures(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve(strict=True)
            source = directory / "source"
            source.mkdir()
            (source / "meta").mkdir()
            (source / "meta/files.sha256").write_text("seal")
            output = directory / "analysis"
            root = output / "project"
            units = ["src/common.c", "integration/src/ecu_host_batch.c", "integration/src/ecu_probe.c"]
            database = [{"file": str(root / name)} for name in units]
            calls = []

            def analyze(executable, addons, root, sources, database, scope, target, output):
                calls.append(sources)
                failed = output.name == "probe"
                return (1 if failed else 0), not failed, [], [], {name: {"project": [name], "sdk": [], "unbound": []} for name in sources}

            with patch.object(c_check, "tool_identity", return_value=(directory / "cppcheck", directory, {})), patch.object(c_check, "coverage", return_value={}), patch.object(c_check, "semantic_identity", return_value={}), patch.object(c_check, "prepare", return_value=(root, units, database, {"profile": "ecu", "entries": ENTRIES, "cppcheckLibraries": {}})), patch.object(c_check, "analyze_program", side_effect=analyze), patch.object(c_check, "sealed_sources"):
                self.assertEqual(c_check.check(source, "linux-x64-controlled-v1", output), 1)
            self.assertEqual(calls, [["src/common.c", "integration/src/ecu_host_batch.c"], ["src/common.c", "integration/src/ecu_probe.c"]])
            summary = json.loads((output / "summary.json").read_text())
            self.assertIsNone(summary["error"])
            self.assertFalse(summary["passed"])
            self.assertEqual([row["exitCode"] for row in summary["programs"]], [0, 1])

    def test_malformed_second_program_receipt_cannot_leave_green_summary(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve(strict=True)
            source = directory / "source"
            source.mkdir()
            (source / "meta").mkdir()
            (source / "meta/files.sha256").write_text("seal")
            output = directory / "analysis"
            root = output / "project"
            units = ["src/common.c", "integration/src/ecu_host_batch.c", "integration/src/ecu_probe.c"]
            database = [{"file": str(root / name)} for name in units]

            def run(argv, directory, stage):
                commands = json.loads((directory / "compile_commands.json").read_text())
                (directory / "cache/files.txt").write_text("".join("dump:::" + row["file"] + "\n" for row in commands))
                receipts = directory / "addon-receipts"
                receipts.mkdir()
                for index, row in enumerate(commands):
                    data = [] if directory.name == "probe" else {"units": [row["file"]], "configurations": 1, "source": row["file"], "files": [row["file"]]}
                    (receipts / f"{index}.json").write_text(json.dumps(data))
                return 0, "", "<results><errors/></results>"

            with patch.object(c_check, "tool_identity", return_value=(directory / "cppcheck", directory, {})), patch.object(c_check, "coverage", return_value={}), patch.object(c_check, "semantic_identity", return_value={}), patch.object(c_check, "prepare", return_value=(root, units, database, {"profile": "ecu", "entries": ENTRIES, "cppcheckLibraries": {}})), patch.object(c_check, "run", side_effect=run):
                self.assertEqual(c_check.check(source, "linux-x64-controlled-v1", output), 1)
            summary = json.loads((output / "summary.json").read_text())
            self.assertFalse(summary["passed"])
            self.assertEqual(summary["error"], "Invalid project addon receipt structure")
            self.assertEqual(summary["projectAcceptance"]["status"], "error")
            self.assertTrue(summary["programs"][0]["passed"])

    def test_changed_semantic_inputs_during_scan_cannot_leave_green_summary(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            source = directory / "source"
            (source / "meta").mkdir(parents=True)
            (source / "meta/files.sha256").write_text("seal")
            output = directory / "analysis"
            with patch.object(c_check, "tool_identity", return_value=(directory / "cppcheck", directory, {})), \
                    patch.object(c_check, "coverage", return_value={}), \
                    patch.object(c_check, "prepare", return_value=(output / "project", ["main.c"], [], {})), \
                    patch.object(c_check, "semantic_identity", side_effect=[{"version": 1}, {"version": 2}]), \
                    patch.object(c_check, "analyze_program", return_value=(0, True, [], [], {})), \
                    patch.object(c_check, "sealed_sources"):
                self.assertEqual(c_check.check(source, "linux-x64-controlled-v1", output), 1)
            summary = json.loads((output / "summary.json").read_text())
            self.assertEqual(summary["error"], "Semantic inputs changed during analysis")
            self.assertFalse(summary["passed"])
            self.assertTrue(summary["programs"][0]["passed"])

    def test_project_acceptance_follows_rechecks_and_does_not_replace_raw_success(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            source = directory / "source"
            (source / "meta").mkdir(parents=True)
            (source / "meta/files.sha256").write_text("seal")
            output = directory / "analysis"
            events = []

            def semantic(*args):
                events.append("semantic")
                return {"version": 2}

            def acceptance(summary, project):
                events.append("acceptance")
                self.assertTrue(summary["passed"])
                self.assertEqual(summary["programs"][0]["exitCode"], 0)
                self.assertEqual(project, output / "project")
                return {"status": "blocked", "objects": [{"reason": "unassessed"}]}

            with patch.object(c_check, "tool_identity", return_value=(directory / "cppcheck", directory, {})), \
                    patch.object(c_check, "coverage", return_value={"completeCompliance": False}), \
                    patch.object(c_check, "prepare", return_value=(output / "project", ["main.c"], [], {})), \
                    patch.object(c_check, "semantic_identity", side_effect=semantic), \
                    patch.object(c_check, "analyze_program", return_value=(0, True, [], [], {})), \
                    patch.object(c_check, "sealed_sources", side_effect=lambda *args: events.append("seal")), \
                    patch.object(c_check.c_acceptance, "evaluate", side_effect=acceptance):
                self.assertEqual(c_check.check(source, "linux-x64-controlled-v1", output), 0)
            self.assertEqual(events, ["semantic", "seal", "semantic", "acceptance"])
            summary = json.loads((output / "summary.json").read_text())
            self.assertTrue(summary["passed"])
            self.assertIsNone(summary["error"])
            self.assertFalse(summary["coverage"]["completeCompliance"])
            self.assertEqual(summary["projectAcceptance"]["status"], "blocked")
