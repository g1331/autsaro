"""Development configuration, diagnostics and resource regression tests."""

from __future__ import annotations

import hashlib
import io
import json
import math
import os
import re
import subprocess
import sys
import tempfile
import unittest
import zipfile
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch
from xml.etree import ElementTree

from autosar_tooling import (
    assets,
    config,
    coverage,
    diagnostics,
    official,
    quality,
    requirements,
    verify,
)


class ConfigurationTests(unittest.TestCase):
    def test_environment_overrides_local_paths_without_global_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "dev.local.toml").write_text(
                '[environment]\nAUTOSAR_MOD_ARCHIVE="references/mod.zip"\nAUTOSAR_CC="/local/gcc"\n'
            )
            with patch.dict(os.environ, {"AUTOSAR_CC": "/selected/gcc"}, clear=True):
                result = config.environment(root)
                self.assertEqual(result["AUTOSAR_CC"], "/selected/gcc")
                self.assertEqual(
                    result["AUTOSAR_MOD_ARCHIVE"], str(root / "references/mod.zip")
                )
                self.assertNotIn("AUTOSAR_MOD_ARCHIVE", os.environ)
                self.assertEqual(
                    result["AUTOSAR_PYTHON"], str(Path(sys.executable).absolute())
                )
                # Native suites run at the default test parallelism unless chosen locally.
                self.assertNotIn("RUST_TEST_THREADS", result)

    def test_unknown_settings_fail_instead_of_changing_arbitrary_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for text in (
                '[environment]\nHOME="somewhere"\n',
                '[unexpected]\nx="value"\n',
            ):
                (root / "dev.local.toml").write_text(text)
                with self.assertRaises(ValueError):
                    config.environment(root)

    def test_resource_identity_comes_from_the_official_resource_catalog(self):
        entries = config.archives()
        fixture = json.loads(
            (config.ROOT / "core/resources/official.json").read_text()
        )
        self.assertEqual(entries["R24-11 MOD"][2], next(item["sha256"] for item in fixture if item["environment"] == "AUTOSAR_MOD_ARCHIVE"))
        self.assertEqual(len(entries), 4)
        with patch.dict(os.environ, {"AUTOSAR_MOD_ARCHIVE": "custom/mod.zip"}):
            self.assertEqual(
                config.archive_path("AUTOSAR_MOD_ARCHIVE", "unused"),
                config.ROOT / "custom/mod.zip",
            )


class OfficialArchiveTests(unittest.TestCase):
    def fetch(self, root: Path, content: bytes) -> tuple[int, str]:
        errors = io.StringIO()
        with patch.object(official, "tls_context", return_value=None), patch.object(official.urllib.request, "urlopen", return_value=io.BytesIO(content)), redirect_stdout(io.StringIO()), patch.object(official.sys, "stderr", errors):
            return official.fetch(root), errors.getvalue()

    def catalog(self, root: Path, content: bytes) -> Path:
        (root / "core/resources").mkdir(parents=True)
        entry = {"name": "archive", "environment": "UNSET_TEST_ARCHIVE", "path": "official/a.zip", "url": "https://example.invalid/a.zip", "sha256": hashlib.sha256(content).hexdigest()}
        (root / "core/resources/official.json").write_text(json.dumps([entry]))
        return root / "official/a.zip"

    def test_download_is_installed_only_with_the_pinned_digest(self):
        with tempfile.TemporaryDirectory() as directory:
            target = self.catalog(Path(directory), b"archive")
            status, errors = self.fetch(Path(directory), b"tampered")
            self.assertEqual(status, 1)
            self.assertIn("SHA-256 mismatch", errors)
            self.assertEqual(list(target.parent.iterdir()), [])
            self.assertEqual(self.fetch(Path(directory), b"archive")[0], 0)
            self.assertEqual(target.read_bytes(), b"archive")

    def test_existing_archive_is_verified_and_never_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            target = self.catalog(Path(directory), b"archive")
            target.parent.mkdir()
            target.write_bytes(b"own copy")
            with patch.object(official, "tls_context", side_effect=AssertionError("downloaded an existing archive")):
                status, errors = self.fetch(Path(directory), b"archive")
            self.assertEqual(status, 1)
            self.assertIn("existing", errors)
            self.assertEqual(target.read_bytes(), b"own copy")

    def test_intermediate_certificate_requires_the_pinned_fingerprint(self):
        with patch.object(official.urllib.request, "urlopen", return_value=io.BytesIO(b"other certificate")), self.assertRaisesRegex(official.FetchError, "fingerprint"):
            official.tls_context()


def specification(*items: str, states: dict[str, str] | None = None) -> bytes:
    traces = "".join(
        f"<{tag}><SHORT-NAME>{name}</SHORT-NAME><CATEGORY>SPECIFICATION_ITEM</CATEGORY></{tag}>"
        for tag, name in (item.split(":") for item in items)
    )
    infos = "".join(
        f"<LIFE-CYCLE-INFO><LC-OBJECT-REF BASE=\"ArTrace\" DEST=\"TRACEABLE\">{name}</LC-OBJECT-REF>"
        f"<LC-STATE-REF DEST=\"LIFE-CYCLE-STATE\">/AUTOSAR/GenDef/LifeCycleStateDefinitionGroups/AutosarLifeCycleStates/{state}</LC-STATE-REF></LIFE-CYCLE-INFO>"
        for name, state in (states or {}).items()
    )
    return f'<AUTOSAR xmlns="http://autosar.org/schema/r4.0"><LIFE-CYCLE-INFOS>{infos}</LIFE-CYCLE-INFOS>{traces}</AUTOSAR>'.encode()


class RequirementTests(unittest.TestCase):
    """Uses the invented prefixes DEMO/Demo so that these sources never cite a real module's ID."""

    def repository(self, root: Path, standards: str, test: str) -> None:
        (root / "official").mkdir()
        with zipfile.ZipFile(root / "official/spec.zip", "w") as archive:
            archive.writestr(
                "CP_SWS_Demo_1/AUTOSAR_CP_SWS_Demo.arxml",
                specification(
                    "TRACEABLE-TABLE:SWS_DEMO_00001", "TRACE:SWS_DEMO_00002", "TRACE:SWS_DEMO_00003",
                    "TRACE:SWS_DEMO_00005", "TRACE:SWS_DEMO_00007", "TRACE:ECUC_Demo_00010",
                    "TRACE:SWS_DEMO_CONSTR_00001", "TRACE:SWS_DEMO_00009",
                    states={"SWS_DEMO_00009": "draft"},
                ),
            )
            archive.writestr("CP_SWS_DemoTT_2/AUTOSAR_CP_SWS_DemoTT.arxml", specification("TRACE:ECUC_Demo_00020"))
        (root / "core/resources").mkdir(parents=True)
        digest = hashlib.sha256((root / "official/spec.zip").read_bytes()).hexdigest()
        (root / "core/resources/official.json").write_text(
            json.dumps([{"environment": requirements.ARCHIVE_VARIABLE, "path": "official/spec.zip", "sha256": digest}])
        )
        (root / "docs/standards").mkdir(parents=True)
        (root / "docs/standards/demo.md").write_text(standards, encoding="utf-8")
        (root / "core/tests").mkdir()
        (root / "core/tests/demo.c").write_text(test, encoding="utf-8")

    def run_check(self, root: Path, stale_only: bool = False) -> tuple[int, str]:
        output = io.StringIO()
        with patch.dict(os.environ), redirect_stdout(output):
            os.environ.pop(requirements.ARCHIVE_VARIABLE, None)
            return requirements.check("SWS_Demo", root, stale_only), output.getvalue()

    PREFIXES = frozenset({"SWS_DEMO_", "SWS_DEMO_CONSTR_", "ECUC_DEMO_"})
    STANDARDS = f"{requirements.STANDARDS}/demo.md"

    def cited(self, text: str, path: str) -> list[tuple[str, str | None]]:
        return [(item.identifier, item.category) for item in requirements.scan(text, path, self.PREFIXES).citations]

    def test_citations_expand_continuations_and_ranges(self):
        cited = requirements.scan(
            "`SWS_DEMO_00001／00002` and ecuc_demo_00010; (SWS_DEMO_00003,\n * 00005) "
            "SWS_DEMO_00005–00007; SWS_DEMO_CONSTR_00001; SWS_DEMO_00002, 2024; x_SWS_DEMO_00009",
            "f",
            self.PREFIXES,
        ).citations
        self.assertEqual(requirements.scan("SWS_DEMO_CONSTR_00001 rejects_constr_00002", "f", {"CONSTR_"}).citations, [])
        # An item without a number is the whole ID and is cited as a word.
        whole = requirements.scan(
            "已核对（原文）：SWS_DEMO_NA、SWS_DEMO_00001; SWS_DEMO_NAX", self.STANDARDS, self.PREFIXES | {"SWS_DEMO_NA"}
        ).citations
        self.assertEqual(
            [(item.identifier, item.category) for item in whole],
            [("SWS_DEMO_NA", "inspected"), ("SWS_DEMO_00001", "inspected")],
        )
        self.assertEqual(
            [(item.identifier, item.line, item.interior) for item in cited],
            [
                ("SWS_DEMO_00001", 1, False), ("SWS_DEMO_00002", 1, False), ("ECUC_DEMO_00010", 1, False),
                ("SWS_DEMO_00003", 1, False), ("SWS_DEMO_00005", 1, False),
                ("SWS_DEMO_00005", 2, False), ("SWS_DEMO_00006", 2, True), ("SWS_DEMO_00007", 2, False),
                ("SWS_DEMO_CONSTR_00001", 2, False), ("SWS_DEMO_00002", 2, False),
            ],
        )

    def test_only_ids_after_a_marker_of_their_source_are_disposed(self):
        test = (
            "/* Verifies: SWS_DEMO_00001／00002, SWS_DEMO_00003,\n * SWS_DEMO_00004\n * SWS_DEMO_00005 differs */\n"
            "// 不适用（归属：Can）：SWS_DEMO_00007\n"
        )
        expected = [
            ("SWS_DEMO_00001", "verified"), ("SWS_DEMO_00002", "verified"), ("SWS_DEMO_00003", "verified"),
            ("SWS_DEMO_00004", "verified"), ("SWS_DEMO_00005", None), ("SWS_DEMO_00007", None),
        ]
        self.assertEqual(self.cited(test, "core/tests/fixtures/demo.c"), expected)
        # A Windows checkout of a text file has CRLF line ends.
        self.assertEqual(self.cited(test.replace("\n", "\r\n"), "core/tests/fixtures/demo.c"), expected)
        standards = (
            "| 不适用（配置：ECUC_Demo_00010=false）：`SWS_DEMO_00007`、`SWS_DEMO_00011` | 差距 #12：SWS_DEMO_00009；SWS_DEMO_00013 |\n"
            "| 已核对（头文件比对）：SWS_DEMO_CONSTR_00001 | 不适用：SWS_DEMO_00014 | 不适用（未启用）：SWS_DEMO_00015 |"
            " 不适用（归属：集成方）：SWS_DEMO_00016 | Verifies: SWS_DEMO_00017 |"
        )
        self.assertEqual(
            self.cited(standards, self.STANDARDS),
            [
                ("ECUC_DEMO_00010", None), ("SWS_DEMO_00007", "configuration"), ("SWS_DEMO_00011", "configuration"),
                ("SWS_DEMO_00009", "gap"), ("SWS_DEMO_00013", None), ("SWS_DEMO_CONSTR_00001", "inspected"),
                ("SWS_DEMO_00014", None), ("SWS_DEMO_00015", None), ("SWS_DEMO_00016", "owner"), ("SWS_DEMO_00017", None),
            ],
        )
        self.assertEqual(
            requirements.scan(standards, self.STANDARDS, self.PREFIXES).conditions,
            [requirements.Condition(self.STANDARDS, 1, ("ECUC_DEMO_00010",))],
        )

    def test_rejects_counts_only_in_tests(self):
        text = "// Rejects: ECUC_Can_00316, `ECUC_Demo_00010`\n// ECUC_Can_00326 is accepted\n"
        self.assertEqual(requirements.scan(text, "core/tests/x.rs", self.PREFIXES).rejected, {"ECUC_CAN_00316", "ECUC_DEMO_00010"})
        self.assertEqual(requirements.scan(text, self.STANDARDS, self.PREFIXES).rejected, set())

    def test_reports_undisposed_items_and_stale_citations(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.repository(
                root,
                # An unmarked mention, such as a legacy "located" row, disposes of nothing.
                "| Demo | `SWS_DEMO_00001／00002` 已定位；不适用（归属：Can）：`SWS_DEMO_00004`；`ECUC_Demo_00020` |\n",
                "/* Verifies: SWS_DEMO_00005–00007 */\n",
            )
            status, output = self.run_check(root)
            self.assertEqual(status, 1)
            self.assertIn(
                "8 official items; verified 2, inspected 0, not applicable (configuration) 0, not applicable (owner) 0, gap 0,"
                " undisposed 6; 1 stale citations",
                output,
            )
            undisposed = output.split("Undisposed items:")[1].split("Stale")[0].split()
            self.assertEqual(
                undisposed,
                ["ECUC_Demo_00010", "SWS_DEMO_00001", "SWS_DEMO_00002", "SWS_DEMO_00003", "SWS_DEMO_00009", "(draft)", "SWS_DEMO_CONSTR_00001"],
            )
            self.assertIn("docs/standards/demo.md:1: SWS_DEMO_00004", output)
            (root / "docs/standards/demo.md").write_text("`SWS_DEMO_00001`\n", encoding="utf-8")
            status, output = self.run_check(root, stale_only=True)
            self.assertEqual(status, 0)
            self.assertNotIn("Undisposed items", output)

    def test_configuration_inapplicability_needs_a_rejection_test(self):
        standards = (
            "不适用（配置：ECUC_Demo_00010=false）：`SWS_DEMO_00001–00003`、`SWS_DEMO_00009`；差距 #7：`ECUC_Demo_00010`；"
            "已核对（头文件比对）：`SWS_DEMO_CONSTR_00001`\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.repository(root, standards, "// Verifies: SWS_DEMO_00005, 00007\n// Rejects: ECUC_Demo_00010\n")
            status, output = self.run_check(root)
            self.assertEqual(status, 0, output)
            self.assertIn("verified 2, inspected 1, not applicable (configuration) 4, not applicable (owner) 0, gap 1, undisposed 0;", output)
            (root / "core/tests/demo.c").write_text("// Verifies: SWS_DEMO_00005, 00007\n", encoding="utf-8")
            status, output = self.run_check(root)
            self.assertEqual(status, 1)
            self.assertIn("docs/standards/demo.md:1: ECUC_DEMO_00010", output.split("without a Rejects: test:")[1])
            (root / "docs/standards/demo.md").write_text(standards.replace("ECUC_Demo_00010=false", "未启用"), encoding="utf-8")
            status, output = self.run_check(root)
            self.assertEqual(status, 1)
            self.assertIn("docs/standards/demo.md:1: names no ECUC parameter", output)

    def test_committed_catalog_is_checked_offline_and_against_the_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            standards = "不适用（归属：Can）：`SWS_DEMO_00001–00003`、`SWS_DEMO_00009`、`SWS_DEMO_CONSTR_00001`；差距 #7：`ECUC_Demo_00010`\n"
            self.repository(root, standards, "// Verifies: SWS_DEMO_00005, 00007\n")
            output, errors = io.StringIO(), io.StringIO()
            with patch.object(requirements.sys, "stderr", errors), patch.dict(os.environ), redirect_stdout(output):
                os.environ.pop(requirements.ARCHIVE_VARIABLE, None)
                self.assertEqual(requirements.check("SWS_Demo", root, update=True), 0)
                catalog = root / requirements.CATALOGS / "SWS_Demo.json"
                written = catalog.read_text(encoding="utf-8")
                self.assertEqual(json.loads(written)["items"]["SWS_DEMO_00009"], "draft")
                self.assertEqual(json.loads(written)["elsewhere"], ["ECUC_DEMO_00020"])
                self.assertEqual(requirements.check("SWS_Demo", root), 0)
                catalog.write_text(written.replace('"SWS_DEMO_00009": "draft"', '"SWS_DEMO_00009": "valid"'), encoding="utf-8")
                self.assertEqual(requirements.check("SWS_Demo", root), 1)
                self.assertIn("SWS_Demo.json differs from spec.zip", errors.getvalue())
                catalog.write_text(written, encoding="utf-8")
                # The required check reads only the catalog.
                (root / "official/spec.zip").unlink()
                output.seek(0)
                output.truncate()
                self.assertEqual(requirements.check_catalogs(root), 0, output.getvalue())
                self.assertIn("SWS_Demo: 8 official items; verified 2,", output.getvalue())
                (root / "core/tests/demo.c").write_text("// Verifies: SWS_DEMO_00005\n", encoding="utf-8")
                self.assertEqual(requirements.check_catalogs(root), 1)
                self.assertIn("  SWS_DEMO_00007\n", output.getvalue())
                (root / "core/tests/demo.c").write_text("// Verifies: SWS_DEMO_00005, 00007\n", encoding="utf-8")
                official_json = root / "core/resources/official.json"
                official_json.write_text(official_json.read_text().replace('"sha256": "', '"sha256": "0'))
                self.assertEqual(requirements.check_catalogs(root), 1)
                self.assertIn("comes from archive", errors.getvalue())

    def test_general_items_are_disposed_per_module(self):
        """Another module's or an unqualified disposition of a general item does not dispose of it for this module."""
        standards = (
            "已核对（头文件比对）[CanIf]：SWS_DEMO_00001；"
            "不适用（归属：集成方）[*]：SWS_DEMO_00002–00003、SWS_DEMO_00009、SWS_DEMO_CONSTR_00001、ECUC_Demo_00010\n"
        )
        test = "// Verifies[Com]: SWS_DEMO_00005, 00007\n// Verifies: SWS_DEMO_00007\n// Verifies[CanIf]: SWS_DEMO_00005\n"
        with tempfile.TemporaryDirectory() as directory, patch.object(requirements, "GENERAL", frozenset({"SWS_Demo"})):
            root = Path(directory)
            self.repository(root, standards, test)
            output, errors = io.StringIO(), io.StringIO()
            with patch.object(requirements.sys, "stderr", errors), patch.dict(os.environ), redirect_stdout(output):
                os.environ.pop(requirements.ARCHIVE_VARIABLE, None)
                self.assertEqual(requirements.check("SWS_Demo", root, module="CanIf"), 1)
                self.assertIn("SWS_Demo for CanIf: 8 official items; verified 1, inspected 1, not applicable (configuration) 0, not applicable (owner) 5,", output.getvalue())
                self.assertEqual(output.getvalue().split("Undisposed items:")[1].split(), ["SWS_DEMO_00007"])
                output.seek(0)
                output.truncate()
                self.assertEqual(requirements.check("SWS_Demo", root, module="Com"), 1)
                self.assertEqual(output.getvalue().split("Undisposed items:")[1].split(), ["SWS_DEMO_00001"])
                # Without a module or a registered one, a general document is not accounted.
                self.assertEqual(requirements.check("SWS_Demo", root), 1)
                self.assertEqual(requirements.check("SWS_Demo", root, update=True), 1)
                self.assertIn("accounted per module", errors.getvalue())
                self.assertEqual(requirements.check("SWS_Demo", root, stale_only=True), 0)

                (root / "core/tests/demo.c").write_text(test + "// Verifies[CanIf]: SWS_DEMO_00007\n", encoding="utf-8")
                self.assertEqual(requirements.check("SWS_Demo", root, update=True, module="CanIf"), 0)
                catalog = root / requirements.CATALOGS / "SWS_Demo.json"
                self.assertEqual(json.loads(catalog.read_text(encoding="utf-8"))["modules"], ["CanIf"])
                output.seek(0)
                output.truncate()
                self.assertEqual(requirements.check("SWS_Demo", root), 0, output.getvalue())
                self.assertIn("SWS_Demo for CanIf:", output.getvalue())
                self.assertEqual(requirements.check_catalogs(root), 0)
                (root / "core/tests/demo.c").write_text(test, encoding="utf-8")
                self.assertEqual(requirements.check_catalogs(root), 1)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.repository(root, "", "")
            with patch.object(requirements.sys, "stderr", io.StringIO()), redirect_stdout(io.StringIO()):
                self.assertEqual(requirements.check("SWS_Demo", root, module="CanIf"), 1)

    def test_unknown_document_and_missing_archive_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.repository(root, "", "")
            errors = io.StringIO()
            with patch.object(requirements.sys, "stderr", errors), patch.dict(os.environ), redirect_stdout(io.StringIO()):
                os.environ.pop(requirements.ARCHIVE_VARIABLE, None)
                self.assertEqual(requirements.check("SWS_Unknown", root), 1)
                (root / "official/spec.zip").unlink()
                self.assertEqual(requirements.check("SWS_Demo", root), 1)
            self.assertIn("no single AUTOSAR_CP_SWS_Unknown.arxml", errors.getvalue())
            self.assertIn("fetch-official", errors.getvalue())


class CoverageTests(unittest.TestCase):
    def test_table_lists_each_archive_document_once(self):
        official = {module.document for module in coverage.TABLE}
        coverage.check_table(official)
        for documents, problem in (
            (official - {"CANInterface"}, "unknown ['CANInterface']"),
            (official | {"NewModule"}, "missing ['NewModule']"),
        ):
            with self.assertRaisesRegex(requirements.RequirementError, re.escape(problem)):
                coverage.check_table(documents)
        with (
            patch.object(coverage, "TABLE", [*coverage.TABLE, coverage.m("Again", "CANInterface")]),
            self.assertRaisesRegex(requirements.RequirementError, re.escape("duplicated ['CANInterface']")),
        ):
            coverage.check_table(official)

    def test_tally_counts_verified_gaps_and_unreviewed_items(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            RequirementTests.repository(
                self, root, "已核对（头文件比对）：`SWS_DEMO_CONSTR_00001`；差距 #7：`ECUC_Demo_00010`\n", "// Verifies: SWS_DEMO_00005, 00007\n"
            )
            archive = root / "official/spec.zip"
            self.assertEqual(coverage.documents(archive), {"Demo", "DemoTT"})
            self.assertEqual(coverage.tally(archive, root, "Demo"), coverage.Tally(total=8, verified=3, gap=1, open=4))

    def test_ring_segments_follow_the_counts_in_order(self):
        tallies = {module.document: coverage.Tally(200, 50, 10, 20) for module in coverage.TABLE if module.status == coverage.DELIVERED}
        canvas = coverage.Canvas(100, 100, tallies)
        canvas.ring(50, 50, "CANInterface", 10)
        circumference = 2 * math.pi * 10
        arcs = [(float(length), float(offset)) for length, offset in re.findall(r'stroke-dasharray="([\d.]+) [\d.]+" stroke-dashoffset="(-?[\d.]+)"', canvas.svg())]
        expected = [(circumference / 4, 0.0), (circumference / 20, -circumference / 4), (circumference / 10, -circumference * 0.3)]
        for (length, offset), (want_length, want_offset) in zip(arcs, expected, strict=True):
            self.assertAlmostEqual(length, want_length, delta=0.1)
            self.assertAlmostEqual(offset, want_offset, delta=0.1)
        self.assertIn(">25<", canvas.svg())
        for render in (coverage.focused, coverage.full):
            svg = render(tallies)
            ElementTree.fromstring(svg)
            self.assertIn("Delivered: 11 of 123 BSW modules", svg)


class DiagnosticTests(unittest.TestCase):
    def test_supported_patches_do_not_require_a_specific_node_version(self):
        for version in ("24.0.0", "24.20.0"):
            with patch.object(diagnostics.shutil, "which", return_value="/tools/node"), patch.object(diagnostics.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "v" + version, "")):
                self.assertEqual(diagnostics.version_check("Node", ["node", "--version"], major=24).status, "ready")
        with patch.object(diagnostics.shutil, "which", return_value="/tools/node"), patch.object(diagnostics.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "v22.0.0", "")):
            self.assertEqual(diagnostics.version_check("Node", ["node", "--version"], major=24).status, "blocked")

    def test_ui_does_not_query_rust_archives_or_desktop_dependencies(self):
        calls = []
        def version(name, *args, **kwargs):
            calls.append(name)
            return diagnostics.Check(name, "ready", "test")
        with patch.object(diagnostics, "version_check", side_effect=version), patch.object(diagnostics, "archives", side_effect=AssertionError("UI requested archives")), patch.object(diagnostics, "_platform_dependencies", side_effect=AssertionError("UI requested desktop libraries")):
            diagnostics.inspect("ui")
        self.assertEqual(calls, ["Git", "Node", "npm"])

class QualityScopeTests(unittest.TestCase):
    def test_scoped_formatting_does_not_request_unrelated_tools(self):
        with patch.object(quality, "scoped_paths", return_value=[]), patch.object(quality, "source_paths", return_value=[]), patch.object(quality, "c_syntax_errors", side_effect=AssertionError("UI requested C")), patch.object(quality, "plan_number_errors", return_value=[]), patch.object(quality, "document_size_errors", return_value=[]), redirect_stdout(io.StringIO()):
            self.assertEqual(quality.check("HEAD", scope="ui"), [])


class VerificationTests(unittest.TestCase):
    def test_desktop_and_all_scopes_stop_on_backend_test_failure(self):
        expected = ["cargo", "test", "--locked", "-p", "autosar-config-desktop"]
        for scope in ("desktop", "all"):
            with self.subTest(scope=scope):
                calls = []

                def run(command, calls=calls, **kwargs):
                    calls.append(command)
                    return subprocess.CompletedProcess(command, 7 if command == expected else 0)

                with (
                    patch.object(verify.sys, "platform", "linux"),
                    patch.object(verify, "executable", side_effect=lambda name: name),
                    patch.object(verify, "npm_command", side_effect=lambda *args: ["npm", *args]),
                    patch.object(verify, "environment", return_value={}),
                    patch.object(verify.subprocess, "run", side_effect=run),
                    redirect_stdout(io.StringIO()),
                ):
                    self.assertEqual(verify.verify(scope), 7)
                self.assertEqual(calls[-1], expected)
                self.assertEqual(calls.count(expected), 1)
                self.assertNotIn(["cargo", "build", "--locked", "-p", "autosar-config-desktop"], calls)


class AssetTests(unittest.TestCase):
    def test_missing_and_traversal_assets_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("../outside", "/absolute", "missing"):
                with self.assertRaises(ValueError):
                    assets.digest(root, name)

    def test_no_change_update_is_idempotent_and_does_not_write(self):
        import hashlib

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for folder in (
                "third_party/freertos",
                "runtime/contracts",
                "tools/python/src/ecu_tools",
            ):
                (root / folder).mkdir(parents=True)
            for name in ("source-manifest.json", "posix-source-manifest.json"):
                (root / "third_party/freertos" / name).write_text('{"files": {}}')
            (root / "tools/python/src/ecu_tools/build-tool-inventory.json").write_text(
                '{"files": []}'
            )
            source = root / "owned"
            source.write_bytes(b"original")
            manifest = root / "runtime/contracts/assets.json"
            manifest.write_text(
                json.dumps(
                    {
                        "assets": [
                            {
                                "path": "owned",
                                "license": "Apache-2.0",
                                "sha256": hashlib.sha256(b"original").hexdigest(),
                            }
                        ]
                    }
                )
            )
            before = manifest.read_bytes()
            with redirect_stdout(io.StringIO()):
                self.assertEqual(assets.maintain(update=True, root=root), 0)
                self.assertEqual(manifest.read_bytes(), before)
                source.write_bytes(b"reviewed change")
                self.assertEqual(assets.maintain(root=root), 1)
                self.assertEqual(manifest.read_bytes(), before)
                self.assertEqual(assets.maintain(update=True, root=root), 0)
                self.assertEqual(assets.maintain(root=root), 0)

    def test_upstream_change_cannot_be_accepted_by_asset_update(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            kernel = root / "third_party/freertos"
            kernel.mkdir(parents=True)
            (kernel / "tasks.c").write_bytes(b"changed upstream")
            manifest = {"files": {"tasks.c": "0" * 64}}
            (kernel / "source-manifest.json").write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "Modified upstream"):
                assets.maintain(update=True, root=root)
            self.assertEqual(
                json.loads((kernel / "source-manifest.json").read_text()), manifest
            )
