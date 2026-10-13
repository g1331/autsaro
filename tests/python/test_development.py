"""Development configuration, diagnostics and resource regression tests."""

from __future__ import annotations

import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

import zipfile

from autosar_tooling import assets, config, diagnostics, official, quality, requirements, verify


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
        (root / "core/resources").mkdir(parents=True)
        (root / "core/resources/official.json").write_text(
            json.dumps([{"environment": requirements.ARCHIVE_VARIABLE, "path": "official/spec.zip"}])
        )
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
        (root / "docs").mkdir()
        (root / "docs/standards.md").write_text(standards, encoding="utf-8")
        (root / "core/tests").mkdir()
        (root / "core/tests/demo.c").write_text(test, encoding="utf-8")

    def run_check(self, root: Path, stale_only: bool = False) -> tuple[int, str]:
        output = io.StringIO()
        with patch.dict(os.environ), redirect_stdout(output):
            os.environ.pop(requirements.ARCHIVE_VARIABLE, None)
            return requirements.check("SWS_Demo", root, stale_only), output.getvalue()

    def test_citations_expand_continuations_and_ranges(self):
        cited = requirements.citations(
            "`SWS_DEMO_00001／00002` and ecuc_demo_00010; (SWS_DEMO_00003,\n * 00005) "
            "SWS_DEMO_00005–00007; SWS_DEMO_CONSTR_00001; SWS_DEMO_00002, 2024; x_SWS_DEMO_00009",
            "f",
            {"SWS_DEMO_", "SWS_DEMO_CONSTR_", "ECUC_DEMO_"},
        )
        self.assertEqual(requirements.citations("SWS_DEMO_CONSTR_00001 rejects_constr_00002", "f", {"CONSTR_"}), [])
        self.assertEqual(
            [(item.identifier, item.line, item.interior) for item in cited],
            [
                ("SWS_DEMO_00001", 1, False), ("SWS_DEMO_00002", 1, False), ("ECUC_DEMO_00010", 1, False),
                ("SWS_DEMO_00003", 1, False), ("SWS_DEMO_00005", 1, False),
                ("SWS_DEMO_00005", 2, False), ("SWS_DEMO_00006", 2, True), ("SWS_DEMO_00007", 2, False),
                ("SWS_DEMO_CONSTR_00001", 2, False), ("SWS_DEMO_00002", 2, False),
            ],
        )

    def test_reports_undisposed_items_and_stale_citations(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.repository(
                root,
                "| Demo | `SWS_DEMO_00001／00002`；`SWS_DEMO_00004` 已删除；`ECUC_Demo_00020` |\n",
                "/* SWS_DEMO_00005–00007 */\n",
            )
            status, output = self.run_check(root)
            self.assertEqual(status, 1)
            self.assertIn("8 official items, 4 disposed, 4 undisposed, 1 stale citations", output)
            undisposed = output.split("Undisposed items:")[1].split("Stale")[0].split()
            self.assertEqual(undisposed, ["ECUC_Demo_00010", "SWS_DEMO_00003", "SWS_DEMO_00009", "(draft)", "SWS_DEMO_CONSTR_00001"])
            self.assertIn("docs/standards.md:1: SWS_DEMO_00004", output)
            (root / "docs/standards.md").write_text("`SWS_DEMO_00001`\n", encoding="utf-8")
            status, output = self.run_check(root, stale_only=True)
            self.assertEqual(status, 0)
            self.assertNotIn("Undisposed items", output)

    def test_complete_disposition_passes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.repository(
                root,
                "`SWS_DEMO_00001–00003`、`SWS_DEMO_00009` 不适用；`ECUC_Demo_00010`；`SWS_DEMO_CONSTR_00001`\n",
                "// SWS_DEMO_00005, 00007\n",
            )
            self.assertEqual(self.run_check(root)[0], 0)

    def test_unknown_document_and_missing_archive_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.repository(root, "", "")
            errors = io.StringIO()
            with patch.object(requirements.sys, "stderr", errors):
                with patch.dict(os.environ), redirect_stdout(io.StringIO()):
                    os.environ.pop(requirements.ARCHIVE_VARIABLE, None)
                    self.assertEqual(requirements.check("SWS_Unknown", root), 1)
                    (root / "official/spec.zip").unlink()
                    self.assertEqual(requirements.check("SWS_Demo", root), 1)
            self.assertIn("no single AUTOSAR_CP_SWS_Unknown.arxml", errors.getvalue())
            self.assertIn("fetch-official", errors.getvalue())


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
        with patch.object(quality, "scoped_paths", return_value=[]), patch.object(quality, "source_paths", return_value=[]), patch.object(quality, "c_syntax_errors", side_effect=AssertionError("UI requested C")), patch.object(quality, "plan_number_errors", return_value=[]), redirect_stdout(io.StringIO()):
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
