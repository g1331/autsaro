"""Development configuration, diagnostics and resource regression tests."""

from __future__ import annotations

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

from autosar_tooling import assets, config, diagnostics, quality, verify


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
        self.assertEqual(len(entries), 3)
        with patch.dict(os.environ, {"AUTOSAR_MOD_ARCHIVE": "custom/mod.zip"}):
            self.assertEqual(
                config.archive_path("AUTOSAR_MOD_ARCHIVE", "unused"),
                config.ROOT / "custom/mod.zip",
            )


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
        with patch.object(quality, "scoped_paths", return_value=[]), patch.object(quality, "source_paths", return_value=[]), patch.object(quality, "c_syntax_errors", side_effect=AssertionError("UI requested C")), redirect_stdout(io.StringIO()):
            self.assertEqual(quality.check("HEAD", scope="ui"), [])


class VerificationTests(unittest.TestCase):
    def test_desktop_and_all_scopes_stop_on_backend_test_failure(self):
        expected = ["cargo", "test", "--locked", "--manifest-path", "src-tauri/Cargo.toml"]
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
                self.assertNotIn(["cargo", "build", "--locked", "--manifest-path", "src-tauri/Cargo.toml"], calls)


class AssetTests(unittest.TestCase):
    def test_source_digest_update_cannot_accept_an_abi_change(self):
        from autosar_tooling import bsw_catalog

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'third_party/freertos').mkdir(parents=True)
            (root / 'runtime/contracts').mkdir(parents=True)
            for name in ('source-manifest.json', 'posix-source-manifest.json'):
                (root / 'third_party/freertos' / name).write_text('{"files": {}}')
            contract = root / 'runtime/contracts/bsw-v1.json'
            original = '{"types": {"Value": "uint8_t"}, "sources": {}}'
            contract.write_text(original)
            with (
                patch.object(bsw_catalog, 'materialize', return_value={'types': {'Value': 'uint16_t'}, 'sources': {}}),
                self.assertRaisesRegex(ValueError, 'BSW ABI changed'),
            ):
                assets.maintain(update=True, root=root)
            self.assertEqual(contract.read_text(), original)

    def test_missing_and_traversal_assets_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("../outside", "/absolute", "missing"):
                with self.assertRaises(ValueError):
                    assets.digest(root, name)

    def test_no_change_update_is_idempotent_and_does_not_write(self):
        import hashlib

        from autosar_tooling import bsw_catalog

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
            (root / "runtime/contracts/bsw-v1.json").write_text('{"sources": {}}')
            (root / "tools/python/src/ecu_tools/workbench-v2-assets.json").write_text(
                '{"files": []}'
            )
            source = root / "owned"
            source.write_bytes(b"original")
            manifest = root / "runtime/contracts/assets-v1.json"
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
            with (
                patch.object(bsw_catalog, "materialize", return_value={"sources": {}}),
                redirect_stdout(io.StringIO()),
            ):
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


class NativeBuiltinModeTests(unittest.TestCase):
    def test_development_mode_keeps_application_tool_boundary_and_reports_real_mode(self):
        from autosar_tooling.acceptance.native_profile import app_environment, prepare

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tool = root / "declared-tool"
            tool.write_text("test tool")
            declared = dict.fromkeys(
                ("AUTOSAR_CC", "AUTOSAR_OBJDUMP", "AUTOSAR_GIT", "AUTOSAR_PYTHON"), str(tool)
            )
            with patch.dict(os.environ, {**declared, "AUTOSAR_XSD_ARCHIVE": "/private/xsd", "CARGO_HOME": "/private/cargo"}):
                prepare(root, "linux", False, None, builtin_only=True)
                environment = app_environment(root, False, builtin_only=True)
            scenario = json.loads((root / "scenario.json").read_text())
            self.assertFalse(scenario["installed"])
            self.assertEqual(scenario["performanceTools"], dict.fromkeys(("compiler", "objdump", "git", "python"), str(tool)))
            receipt = json.loads((root / "application-environment.json").read_text())
            self.assertFalse(receipt["installed"])
            self.assertTrue(all(value is None for value in receipt["tools"].values()))
            self.assertNotIn("AUTOSAR_CC", environment)
            self.assertNotIn("AUTOSAR_XSD_ARCHIVE", environment)
            self.assertNotIn("CARGO_HOME", environment)

    def test_builtin_consumer_tools_fail_before_preparing_scenarios(self):
        from autosar_tooling.acceptance.native_builtin import prepare

        for value in (None, "relative-tool", "/missing-native-consumer-tool"):
            with self.subTest(value=value), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                environment = {} if value is None else {"AUTOSAR_CC": value}
                with patch.dict(os.environ, environment, clear=True), self.assertRaisesRegex(RuntimeError, "AUTOSAR_CC.*existing absolute tool file"):
                    prepare(root, "linux", None, installed=False)
                self.assertEqual(list(root.iterdir()), [])

    def test_installed_mode_still_requires_unavailable_checkout(self):
        from autosar_tooling.acceptance.native_profile import prepare

        with tempfile.TemporaryDirectory() as directory, self.assertRaisesRegex(RuntimeError, "unavailable build checkout"):
            prepare(Path(directory), "linux", True, None, builtin_only=True)
