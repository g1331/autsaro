"""Exercise real working-tree selection and same-invocation verification inputs."""

import contextlib
import io
import json
import os
import runpy
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import yaml
from autosar_tooling import verify
from autosar_tooling.ci_selection import select_changes
from autosar_tooling.config import ROOT


class ChangedVerificationTests(unittest.TestCase):
    def git(self, root, *args):
        return subprocess.check_output(["git", *args], cwd=root, text=True).strip()

    def repository(self, root):
        self.git(root, "init", "--quiet")
        for name in ("README.md", "docs/guide.md", "ui/src/App.tsx", "core/src/lib.rs",
                     "tools/python/src/autosar_tooling/c_check.py", "runtime/test.c"):
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("original\n")
        self.git(root, "add", ".")
        self.git(root, "-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                 "commit", "--quiet", "-m", "baseline")
        return self.git(root, "rev-parse", "HEAD")

    def test_git_collects_committed_staged_unstaged_deleted_and_untracked_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            base = self.repository(root)
            (root / "README.md").write_text("committed\n")
            self.git(root, "add", ".")
            self.git(root, "-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                     "commit", "--quiet", "-m", "documentation")
            (root / "ui/src/App.tsx").write_text("staged\n")
            self.git(root, "add", "ui/src/App.tsx")
            driver = root / "tools/python/src/autosar_tooling/c_check.py"
            driver.write_text("unstaged\n")
            (root / "core/src/lib.rs").unlink()
            (root / "docs/new guide.md").write_text("untracked\n")
            with mock.patch.object(verify, "ROOT", root):
                actual_base, rows = verify.changed_rows(base)
            self.assertEqual(actual_base, base)
            values = {row["filename"]: row["status"] for row in rows}
            self.assertEqual(values, {
                "README.md": "modified", "ui/src/App.tsx": "modified",
                "tools/python/src/autosar_tooling/c_check.py": "modified",
                "core/src/lib.rs": "removed", "docs/new guide.md": "unverified",
            })
            self.assertTrue(all(select_changes(rows).values()))

    def test_opposing_git_layers_keep_runtime_checks(self):
        for layers in ("committed-staged", "committed-unstaged", "staged-unstaged"):
            with self.subTest(layers=layers), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                base = self.repository(root)
                source = root / "runtime/test.c"
                source.write_text("changed\n")
                self.git(root, "add", "runtime/test.c")
                if layers.startswith("committed"):
                    self.git(root, "-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                             "commit", "--quiet", "-m", "runtime change")
                source.write_text("original\n")
                if layers == "committed-staged":
                    self.git(root, "add", "runtime/test.c")
                (root / "README.md").write_text("documentation change\n")
                self.assertEqual(self.git(root, "diff", base, "--name-only"), "README.md")
                with mock.patch.object(verify, "ROOT", root):
                    actual_base, rows = verify.changed_rows(base)
                self.assertEqual(actual_base, base)
                self.assertEqual({row["filename"] for row in rows}, {"runtime/test.c", "README.md"})
                self.assertTrue(all(select_changes(rows).values()))
                plan = verify.changed_commands(rows, base, root / "samples")
                names = {name for name, _, _ in plan}
                self.assertTrue({"python:unit", "core:test", "analysis:quality"} <= names)

    def test_each_git_layer_keeps_rename_source_and_destination(self):
        for layer in ("committed", "staged", "unstaged"):
            with self.subTest(layer=layer), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                base = self.repository(root)
                self.git(root, "mv", "runtime/test.c", "docs/moved.md")
                if layer == "committed":
                    self.git(root, "-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                             "commit", "--quiet", "-m", "move runtime")
                elif layer == "unstaged":
                    self.git(root, "reset", "--quiet", "HEAD")
                    self.git(root, "add", "--intent-to-add", "docs/moved.md")
                with mock.patch.object(verify, "ROOT", root):
                    _, rows = verify.changed_rows(base)
                self.assertIn({"filename": "docs/moved.md", "status": "renamed",
                               "previous_filename": "runtime/test.c"}, rows)
                self.assertTrue(all(select_changes(rows).values()))

    def test_document_analysis_ui_and_renames_agree_with_the_actual_ci_selector(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/checks.yml").read_text())
        script = workflow["jobs"]["changes"]["steps"][1]["run"]
        cases = (("README.md", "docs"),
                 ("tools/python/src/autosar_tooling/c_check.py", "analysis"),
                 ("ui/src/App.tsx", "ui"), ("docs/guide.md", "rename"))
        for name, mode in cases:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                base = self.repository(root)
                if mode == "rename":
                    self.git(root, "mv", name, "docs/moved guide.md")
                else:
                    (root / name).write_text("changed\n")
                with mock.patch.object(verify, "ROOT", root):
                    _, rows = verify.changed_rows(base)
                if mode == "rename":
                    self.assertIn("previous_filename", rows[0])
                    self.assertEqual(rows[0]["previous_filename"], name)
                output = root / "output"
                source = root / "selector.py"
                source.write_text(script)
                with (mock.patch.dict(os.environ, {"PULL_REQUEST": "1",
                       "GITHUB_REPOSITORY": "example/repo", "GITHUB_OUTPUT": str(output)}),
                      mock.patch.object(subprocess, "run", return_value=subprocess.CompletedProcess(
                          [], 0, json.dumps([rows]))), contextlib.redirect_stdout(io.StringIO())):
                    runpy.run_path(str(source))
                actual = dict(line.split("=") for line in output.read_text().splitlines())
                expected = {key: str(value).lower() for key, value in select_changes(rows).items()}
                self.assertEqual(actual, expected)
                self.assertEqual(actual["native"], str(mode == "rename").lower())
                plan = verify.changed_commands(rows, base, root / "samples")
                names = {entry[0] for entry in plan}
                self.assertIn("source:quality", names)
                self.assertEqual("python:unit" in names, mode != "docs")
                self.assertEqual("analysis:quality" in names, mode in {"analysis", "rename"})
                self.assertEqual("core:test" in names, mode == "rename")
                self.assertEqual("ui:build" in names, mode in {"ui", "rename"})
                self.assertEqual("desktop:ipc" in names, mode in {"ui", "rename"})

    def test_unstaged_document_move_is_conservative(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            base = self.repository(root)
            (root / "docs/guide.md").rename(root / "docs/untracked.md")
            with mock.patch.object(verify, "ROOT", root):
                _, rows = verify.changed_rows(base)
            self.assertTrue(all(select_changes(rows).values()))

    def test_full_local_core_keeps_all_targets_and_one_native_feature_set(self):
        rows = [{"filename": "runtime/host/src/Os_Host.c", "status": "modified"}]
        plan = verify.changed_commands(rows, "base", Path("samples"))
        commands = {name: argv for name, argv, _ in plan}
        core = commands["core:test"]
        self.assertIn("--no-fail-fast", core)
        self.assertEqual(core[core.index("--features") + 1], "native-tests")
        self.assertNotIn("--test", core)
        self.assertNotIn("--lib", core)
        self.assertNotIn("analysis:samples", commands)
        self.assertEqual(sum(name == "core:test" for name, _, _ in plan), 1)
        self.assertEqual(commands["analysis:quality"][-2:], ["--c-samples", "samples"])
        self.assertIn("--all-features", commands["core:clippy"])

    def test_renaming_workflow_or_lock_inputs_keeps_their_extra_checks(self):
        for previous, expected in (("Cargo.lock", "dependencies:cargo-deny"),
                                   (".github/workflows/desktop.yml", "workflows:actionlint")):
            rows = [{"filename": "docs/moved.txt", "previous_filename": previous,
                     "status": "renamed"}]
            plan = verify.changed_commands(rows, "base", Path("samples"))
            self.assertIn(expected, {name for name, _, _ in plan})

    def test_analysis_execution_requires_a_supported_native_host(self):
        rows = [{"filename": "docs/standards/os/adoption.md", "status": "modified"}]
        with (mock.patch.object(verify, "changed_rows", return_value=("base", rows)),
              mock.patch.object(verify.sys, "platform", "darwin"),
              mock.patch.object(verify, "run_commands", side_effect=AssertionError("tool started")),
              contextlib.redirect_stdout(io.StringIO()),
              contextlib.redirect_stderr(io.StringIO())):
            self.assertEqual(verify.verify("changed"), 1)

    def test_fresh_inputs_are_shared_and_success_is_cleaned_but_failures_are_retained(self):
        rows = [{"filename": "docs/standards/os/adoption.md", "status": "modified"}]
        seen = []
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)

            def run(plan, env):
                self.assertIn("AUTOSAR_C_ANALYSIS_SAMPLES", env)
                samples = Path(env["AUTOSAR_C_ANALYSIS_SAMPLES"])
                self.assertFalse(samples.exists())
                samples.mkdir()
                (samples / "actual-source").write_text("generated input")
                commands = {name: argv for name, argv, _ in plan}
                self.assertEqual(commands["analysis:quality"][-1], str(samples))
                seen.append(samples)
                return 0 if len(seen) == 1 else 7

            with (mock.patch.object(verify, "ROOT", root),
                  mock.patch.object(verify.sys, "platform", "linux"),
                  mock.patch.object(verify, "changed_rows", return_value=("base", rows)),
                  mock.patch.object(verify, "environment", return_value={}),
                  mock.patch.object(verify, "run_commands", side_effect=run),
                  contextlib.redirect_stdout(io.StringIO())):
                self.assertEqual(verify.verify("changed"), 0)
                self.assertEqual(verify.verify("changed"), 7)
            self.assertNotEqual(seen[0], seen[1])
            self.assertFalse(seen[0].parent.exists())
            self.assertTrue((seen[1] / "actual-source").is_file())
            self.assertEqual((seen[1] / "actual-source").read_text(), "generated input")

    def test_listing_creates_no_outputs_and_starts_no_tools(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            base = self.repository(root)
            (root / "README.md").write_text("changed\n")
            with (mock.patch.object(verify, "ROOT", root),
                  mock.patch.object(verify, "run_commands",
                                    side_effect=AssertionError("tool started")),
                  contextlib.redirect_stdout(io.StringIO()) as output):
                self.assertEqual(verify.verify("changed", base, list_checks=True), 0)
            self.assertIn("source:quality", output.getvalue())
            self.assertNotIn("core:test", output.getvalue())
            self.assertFalse((root / "build").exists())
