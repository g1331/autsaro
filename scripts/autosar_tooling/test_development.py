"""Regression coverage for task isolation, diagnostics and development plans."""

from __future__ import annotations

import io
import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import Mock, patch

from ecu_tools.process import ProcessSpec

from autosar_tooling import assets, config, dev, diagnostics, quality, runner, verify


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

    def test_resource_identity_comes_from_the_core_and_fixture(self):
        entries = config.archives()
        fixture = json.loads(
            (config.ROOT / "core/tests/fixtures/epic4/manifest.json").read_text()
        )
        self.assertEqual(entries["R24-11 MOD"][2], fixture["external_mod"]["sha256"])
        self.assertEqual(len(entries), 3)
        with patch.dict(os.environ, {"AUTOSAR_MOD_ARCHIVE": "custom/mod.zip"}):
            self.assertEqual(
                config.archive_path("AUTOSAR_MOD_ARCHIVE", "unused"),
                config.ROOT / "custom/mod.zip",
            )


class DiagnosticTests(unittest.TestCase):
    def query(self, output, **options):
        with (
            patch.object(diagnostics.shutil, "which", return_value="/tools/node"),
            patch.object(
                diagnostics.subprocess,
                "run",
                return_value=subprocess.CompletedProcess([], 0, output, ""),
            ),
        ):
            return diagnostics.version_check(
                "Node", ["node", "--version"], "24.19.0", 24, **options
            )

    def test_node_prefix_is_parsed_and_recommended_patch_is_optional(self):
        self.assertEqual(self.query("v24.19.0").status, "ready")
        self.assertEqual(self.query("v24.20.0").status, "warning")
        self.assertEqual(self.query("v24.20.0", exact=True).status, "blocked")
        self.assertEqual(self.query("v24.1.0", minimum="24.19.0").status, "blocked")
        self.assertEqual(self.query("v22.0.0").status, "blocked")

    def test_missing_and_failed_version_queries_are_distinct(self):
        with patch.object(diagnostics.shutil, "which", return_value=None):
            self.assertEqual(
                diagnostics.version_check("Node", ["node", "--version"]).status,
                "blocked",
            )
        with (
            patch.object(diagnostics.shutil, "which", return_value="/tools/node"),
            patch.object(
                diagnostics.subprocess,
                "run",
                side_effect=subprocess.TimeoutExpired("node", 30),
            ),
        ):
            self.assertEqual(
                diagnostics.version_check("Node", ["node", "--version"]).status, "error"
            )

    def test_ui_does_not_query_rust_archives_or_desktop_dependencies(self):
        calls = []

        def version(name, *args, **kwargs):
            calls.append(name)
            return diagnostics.Check(name, "ready", "test")

        with (
            patch.object(diagnostics, "version_check", side_effect=version),
            patch.object(
                diagnostics,
                "archives",
                side_effect=AssertionError("UI requested archives"),
            ),
            patch.object(
                diagnostics,
                "_platform_dependencies",
                side_effect=AssertionError("UI requested desktop libraries"),
            ),
        ):
            diagnostics.inspect("ui")
        self.assertEqual(calls, ["Node", "npm", "Git"])

    def test_json_warnings_do_not_become_blockers(self):
        checks = [diagnostics.Check("npm", "warning", "supported, recommended differs")]
        output = io.StringIO()
        with (
            patch.object(diagnostics, "inspect", return_value=checks),
            redirect_stdout(output),
        ):
            self.assertEqual(diagnostics.report("ui", json_output=True), 0)
        self.assertTrue(json.loads(output.getvalue())["ready"])


class PlanTests(unittest.TestCase):
    def test_checks_do_not_install_dependencies_and_have_consistent_quality(self):
        for scope in dev.SCOPES:
            stages = dev.check_plan(scope, None)
            self.assertIn("source:quality", [name for name, _, _ in stages])
            self.assertFalse(any(("ci" in argv for _, argv, _ in stages)))
        core = dev.check_plan("core", None)
        self.assertFalse(
            any(("official-oracles,native-tests" in argv for _, argv, _ in core))
        )
        self.assertTrue(
            any(
                (
                    "official-oracles,native-tests" in argv
                    for _, argv, _ in dev.check_plan("core", None, True)
                )
            )
        )

    def test_fast_and_native_suites_preserve_separate_dependencies(self):
        fast = dev.test_plan("fast")[0][1]
        self.assertNotIn("autosar_tooling.test_process", fast)
        self.assertIn("autosar_tooling.test_development", fast)
        self.assertIn("--skip", dev.test_plan("core")[0][1])
        self.assertIn("native-tests", dev.test_plan("native")[-1][1])
        self.assertIn("official-oracles", dev.test_plan("integration")[0][1])

    def test_dry_run_does_not_start_owned_processes(self):
        output = io.StringIO()
        with (
            patch.object(
                verify,
                "run_live",
                side_effect=AssertionError("dry run launched process"),
            ),
            redirect_stdout(output),
        ):
            self.assertEqual(
                verify.execute(
                    [("example", ["missing"], 1)], plan=True, json_output=True
                ),
                0,
            )
        self.assertEqual(json.loads(output.getvalue())[0]["stage"], "example")

    def test_scoped_formatting_does_not_request_unrelated_tools(self):
        with (
            patch.object(quality, "scoped_paths", return_value=[]),
            patch.object(quality, "source_paths", return_value=[]),
            patch.object(
                quality,
                "clang_format",
                side_effect=AssertionError("UI requested clang-format"),
            ),
            patch.object(
                quality, "c_syntax_errors", side_effect=AssertionError("UI requested C")
            ),
            redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(quality.check("HEAD", scope="ui"), [])


class AssetTests(unittest.TestCase):
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
                "scripts/ecu_tools",
            ):
                (root / folder).mkdir(parents=True)
            for name in ("source-manifest.json", "posix-source-manifest.json"):
                (root / "third_party/freertos" / name).write_text('{"files": {}}')
            (root / "runtime/contracts/bsw-v1.json").write_text('{"sources": {}}')
            (root / "scripts/ecu_tools/workbench-v2-assets.json").write_text(
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


class LiveOutputTests(unittest.TestCase):
    def test_interrupt_closes_windows_job_even_when_wait_marks_finished(self):
        with tempfile.TemporaryDirectory() as directory:
            process = Mock()
            process.registration = {
                "stdout": str(Path(directory) / "stdout"),
                "stderr": str(Path(directory) / "stderr"),
            }
            process.finished = True
            process.wait.side_effect = KeyboardInterrupt
            with (
                patch.object(runner, "OwnedProcess", return_value=process),
                self.assertRaises(KeyboardInterrupt),
            ):
                runner.run(Mock())
            process.windows.close.assert_called_once_with()

    def test_output_streams_before_completion_preserving_unicode_and_failure(self):

        class Capture(io.StringIO):
            def __init__(self):
                super().__init__()
                self.first_write = None

            def write(self, text):
                if self.first_write is None and text:
                    self.first_write = time.monotonic()
                return super().write(text)

        out, err = (Capture(), io.StringIO())
        script = "import os,time,sys; b=chr(0x4e2d).encode(); os.write(1,b[:1]); time.sleep(.2); os.write(1,b[1:]); time.sleep(.5); print('error',file=sys.stderr); sys.exit(7)"
        with tempfile.TemporaryDirectory() as directory:
            spec = ProcessSpec.seconds(
                [sys.executable, "-u", "-c", script],
                config.ROOT,
                10,
                Path(directory),
                "live-test",
            )
            with redirect_stdout(out), redirect_stderr(err):
                result = runner.run(spec)
            completed = time.monotonic()
            self.assertEqual(result.exit_code, 7)
            self.assertFalse(result.success)
            self.assertEqual(out.getvalue(), "中")
            self.assertIn("error", err.getvalue())
            self.assertIsNotNone(out.first_write)
            self.assertGreater(completed - out.first_write, 0.15)


if __name__ == "__main__":
    unittest.main()
