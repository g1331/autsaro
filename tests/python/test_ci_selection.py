"""Keep required CI's documentation-only and ui/-only selection safe."""

import json
import os
import re
import runpy
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

import yaml
from autosar_tooling.config import ROOT

WORKFLOW = ROOT / ".github/workflows/checks.yml"
# What the skipped jobs build, run or analyse. Tooling modules that no skipped job relies on
# for ui/ content are exempt: quality runs in full (--scope all) in the always-run quality job,
# and verify, doctor and the CLI argument lists are not run by these jobs.
INPUTS = ("core/", "runtime/", "tools/python/src/", "tests/python/native/")
EXEMPT = {
    "tools/python/src/autosar_tooling/quality.py",
    "tools/python/src/autosar_tooling/verify.py",
    "tools/python/src/autosar_tooling/diagnostics.py",
    "tools/python/src/autosar_tooling/cli.py",
}
UI_PATH = re.compile(r"""["'/\\]ui[/\\"']""")


def jobs() -> dict[str, dict[str, object]]:
    found: dict[str, dict[str, object]] = {}
    current = None
    for line in WORKFLOW.read_text(encoding="utf-8").splitlines():
        if header := re.fullmatch(r"  ([a-z-]+):", line):
            current = found.setdefault(header.group(1), {"needs": set(), "if": ""})
        elif current is not None and (needs := re.fullmatch(r"    needs: \[?([^\]]+)\]?", line)):
            current["needs"] = {name.strip() for name in needs.group(1).split(",")}
        elif current is not None and (condition := re.fullmatch(r"    if: (.+)", line)):
            current["if"] = condition.group(1)
    return found


class CiSelectionTests(unittest.TestCase):
    def test_desktop_gate_selection_and_failure_propagation(self):
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        detect = workflow["jobs"]["changes"]["steps"][1]["run"]
        gate = workflow["jobs"]["required"]["steps"][0]["run"]
        self.assertIn("desktop-scenarios", workflow["jobs"]["required"]["needs"])
        self.assertEqual(workflow["jobs"]["desktop-scenarios"]["uses"], "./.github/workflows/desktop.yml")
        for event in ("pull_request", "push"):
            for path, expected in (("docs/guide.md", False), ("ui/src/main.tsx", True),
                                   ("core/src/lib.rs", True), ("runtime/bsw/src/Com.c", True),
                                   ("tests/desktop/startup.test.mjs", True), ("Cargo.lock", True),
                                   (".github/workflows/checks.yml", True),
                                   (".github/workflows/desktop.yml", True), ("uv.lock", True)):
                with self.subTest(event=event, path=path), tempfile.TemporaryDirectory() as directory:
                    output = Path(directory) / "output"
                    source = Path(directory) / "detect.py"
                    source.write_text(detect, encoding="utf-8")
                    env = {"PULL_REQUEST": "91" if event == "pull_request" else "",
                           "GITHUB_REPOSITORY": "g1331/autsaro", "BASE": "base",
                           "GITHUB_SHA": "head", "GITHUB_OUTPUT": str(output)}
                    with mock.patch.dict(os.environ, env), mock.patch.object(subprocess, "run") as run:
                        run.return_value = subprocess.CompletedProcess([], 0, path + "\n")
                        runpy.run_path(str(source))
                    self.assertIn("desktop=" + str(expected).lower() + "\n", output.read_text())
        for listing, status, base in (("", 0, "base"), ("docs/a.md\n" * 3000, 0, "base"),
                                      ("", 1, "base"), ("docs/a.md", 0, "")):
            with self.subTest(status=status, base=base, length=len(listing)), tempfile.TemporaryDirectory() as directory:
                source = Path(directory) / "detect.py"
                source.write_text(detect, encoding="utf-8")
                output = Path(directory) / "output"
                env = {"PULL_REQUEST": "", "BASE": base, "GITHUB_SHA": "head", "GITHUB_OUTPUT": str(output)}
                with mock.patch.dict(os.environ, env), mock.patch.object(subprocess, "run") as run:
                    run.return_value = subprocess.CompletedProcess([], status, listing)
                    runpy.run_path(str(source))
                self.assertIn("desktop=true\n", output.read_text())
        for selected in ("true", "false", None):
            for result in ("success", "failure", "cancelled", "skipped"):
                with self.subTest(selected=selected, result=result), tempfile.TemporaryDirectory() as directory:
                    source = Path(directory) / "gate.py"
                    source.write_text(gate, encoding="utf-8")
                    outputs = {"code": "false", "native": "false"}
                    if selected is not None:
                        outputs["desktop"] = selected
                    needs = {"changes": {"result": "success", "outputs": outputs},
                             "desktop-scenarios": {"result": result}}
                    with mock.patch.dict(os.environ, {"RESULTS": json.dumps(needs)}):
                        try:
                            runpy.run_path(str(source))
                        except SystemExit as error:
                            self.assertEqual(error.code == 0, result == "success" or selected == "false" and result == "skipped")

    def test_push_rename_out_of_gui_inputs_still_selects_desktop(self):
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        script = workflow["jobs"]["changes"]["steps"][1]["run"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, text=True).strip()
            git("init", "--quiet")
            original = root / "ui/public/example.svg"
            original.parent.mkdir(parents=True)
            original.write_text("<svg/>", encoding="utf-8")
            git("add", ".")
            git("-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit", "--quiet", "-m", "fixture")
            base = git("rev-parse", "HEAD")
            moved = root / "docs/images/example.svg"
            moved.parent.mkdir(parents=True)
            original.rename(moved)
            git("add", "--all")
            git("-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit", "--quiet", "-m", "move")
            self.assertIn("R100", git("diff", "--name-status", "-M", base, "HEAD"))
            source = root / "detect.py"
            source.write_text(script, encoding="utf-8")
            output = root / "output"
            env = {"PULL_REQUEST": "", "BASE": base, "GITHUB_SHA": "HEAD", "GITHUB_OUTPUT": str(output)}
            run = subprocess.run
            with mock.patch.dict(os.environ, env), mock.patch.object(subprocess, "run") as command:
                command.side_effect = lambda argv, **kwargs: run(argv, cwd=root, **kwargs)
                runpy.run_path(str(source))
            self.assertIn("desktop=true\n", output.read_text())

    def test_extended_does_not_duplicate_automatic_desktop(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/extended.yml").read_text(encoding="utf-8"))
        script = workflow["jobs"]["changes"]["steps"][1]["run"]
        for event, full in (("push", "false"), ("pull_request", "false"),
                            ("pull_request", "true"), ("workflow_dispatch", "true")):
            with self.subTest(event=event, full=full), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "output"
                source = Path(directory) / "select.py"
                source.write_text(script, encoding="utf-8")
                env = {"GITHUB_EVENT_NAME": event, "BASE": "base", "FULL": full,
                       "GITHUB_SHA": "head", "GITHUB_OUTPUT": str(output)}
                with mock.patch.dict(os.environ, env), mock.patch.object(subprocess, "run") as run:
                    run.return_value = subprocess.CompletedProcess([], 0, ".github/workflows/extended.yml\n")
                    runpy.run_path(str(source))
                self.assertIn("desktop=" + str(event == "workflow_dispatch").lower() + "\n", output.read_text())

    def test_pull_requests_fuzz_direct_inputs_and_pushes_fuzz_every_input(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/extended.yml").read_text(encoding="utf-8"))
        script = workflow["jobs"]["changes"]["steps"][1]["run"]
        for path, pull_request, push in (
            ("fuzz/fuzz_targets/dcm_request.rs", True, True),
            ("core/src/arxml/projection.rs", True, True),
            ("core/src/definitions/extension.rs", True, True),
            ("runtime/bsw/src/Dcm.c", True, True),
            ("runtime/host/src/SchM_Diagnostic_Host.c", True, True),
            ("core/src/integration/rte.rs", False, True),
            ("core/resources/templates/standard-ecu.arxml", False, True),
            ("runtime/bsw/include/ComStack_Types.h", False, True),
            ("runtime/host/src/Os_Host.c", False, True),
            ("core/src/generator.rs", False, False),
            ("docs/guide.md", False, False),
        ):
            for event, expected in (("pull_request", pull_request), ("push", push)):
                with self.subTest(path=path, event=event), tempfile.TemporaryDirectory() as directory:
                    output = Path(directory) / "output"
                    source = Path(directory) / "select.py"
                    source.write_text(script, encoding="utf-8")
                    env = {"GITHUB_EVENT_NAME": event, "BASE": "base", "FULL": "false",
                           "GITHUB_SHA": "head", "GITHUB_OUTPUT": str(output)}
                    with mock.patch.dict(os.environ, env), mock.patch.object(subprocess, "run") as run:
                        run.return_value = subprocess.CompletedProcess([], 0, path + "\n")
                        runpy.run_path(str(source))
                    self.assertIn("fuzz=" + str(expected).lower() + "\n", output.read_text())

    def test_split_jobs_run_every_step_once_per_platform_or_sanitizer(self):
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        for job, key, names in (
            ("native-delivery", "os", ("Sealed ECU build mode fault isolation",
                                       "CMake build matches the sealed ECU build",
                                       "All moved samples build and run with receiver Python 3.11")),
            ("native-sanitizers", "sanitize", ("Instrumented native programs", "Instrumented sealed ECU protocols",
                                               "Instrumented OS suites", "Check concurrent native constructor rollback")),
        ):
            entries = workflow["jobs"][job]["strategy"]["matrix"]["include"]
            steps = {step["name"]: step for step in workflow["jobs"][job]["steps"] if "name" in step}
            for value in {entry[key] for entry in entries}:
                parts = [entry["part"] for entry in entries if entry[key] == value]
                for name in names:
                    condition = steps[name].get("if", "")
                    # Only part exclusions select these steps, so the count below is exact.
                    self.assertRegex(condition, r"^(matrix\.part != '[a-z]+')?$")
                    excluded = re.findall(r"'([a-z]+)'", condition)
                    with self.subTest(job=job, value=value, step=name):
                        self.assertEqual(sum(part not in excluded for part in parts), 1)

    def test_receiver_sample_selection_defaults_restricts_and_refuses_unknown_names(self):
        module = runpy.run_path(str(ROOT / "tests/python/native/test_offline_delivery.py"))
        with mock.patch.dict(os.environ, {}, clear=True):
            self.assertEqual(module["receiver_samples"](), module["SAMPLES"])
            os.environ["AUTOSAR_RECEIVER_SAMPLES"] = "standard-ecu"
            self.assertEqual(module["receiver_samples"](), ("standard-ecu",))
            for invalid in ("", "unknown", "standard-ecu unknown"):
                os.environ["AUTOSAR_RECEIVER_SAMPLES"] = invalid
                with self.subTest(invalid=invalid), self.assertRaisesRegex(ValueError, "AUTOSAR_RECEIVER_SAMPLES"):
                    module["receiver_samples"]()

    def test_receiver_versions_keep_running_after_failure_and_select_expected_samples(self):
        for filename in ("checks.yml", "extended.yml"):
            extended = filename == "extended.yml"
            text = (ROOT / ".github/workflows" / filename).read_text(encoding="utf-8")
            # Read the complete run block, ending at the next YAML job/step.
            lines = text.splitlines()
            if extended:
                start = next(i for i, line in enumerate(lines) if "for version in ('3.11'" in line)
                end = next(i for i in range(start, len(lines)) if "sys.exit('Receiver Python failed:" in lines[i])
                script = "import os, subprocess, sys\nfailed = []\n" + textwrap.dedent("\n".join(lines[start:end + 1]))
            else:
                block = text.split("      - name: All moved samples build and run with receiver Python 3.11\n", 1)[1]
                script = textwrap.dedent(block.split("        run: |\n", 1)[1].split("\n  c-analysis-samples:", 1)[0])
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "receiver.py"
                path.write_text(script, encoding="utf-8")
                with (
                    mock.patch.object(subprocess, "check_output", side_effect=["3.11", "3.12", "3.13", "3.14"] if extended else ["3.11"]),
                    mock.patch.object(subprocess, "run") as run,
                    mock.patch("builtins.print") as output,
                ):
                    run.side_effect = lambda argv, extended=extended, **kwargs: subprocess.CompletedProcess(
                        argv, int(kwargs.get("env", {}).get("AUTOSAR_RECEIVER_PYTHON") == ("3.12" if extended else "3.11"))
                    )
                    with self.assertRaisesRegex(SystemExit, "Receiver Python failed: 3.12" if extended else "1"):
                        runpy.run_path(str(path))
                if not extended:
                    self.assertEqual(run.call_args_list[0].args[0], ["uv", "python", "install", "--no-bin", "3.11"])
                    self.assertTrue(any("::error title=Receiver Python 3.11::" in str(call) for call in output.call_args_list))
                receivers = [call.kwargs["env"] for call in run.call_args_list if "env" in call.kwargs]
                self.assertEqual([env["AUTOSAR_RECEIVER_PYTHON"] for env in receivers], ["3.11", "3.12", "3.13", "3.14"] if extended else ["3.11"])
                for env in receivers:
                    expected = ["standard-ecu", "user-application", "component-pipeline", "no-diagnostic"]
                    self.assertEqual(env["AUTOSAR_RECEIVER_SAMPLES"].split(), expected)

    def test_codex_hooks_use_lightweight_checks_without_hiding_code_changes(self):
        script = textwrap.dedent(
            WORKFLOW.read_text(encoding="utf-8")
            .split("        run: |\n", 1)[1]
            .split("\n  desktop-scenarios:", 1)[0]
        )
        cases = (
            ([".codex/hooks.json"], False, False),
            (
                [".codex/hooks.json", ".agents/skills/example/SKILL.md", "README.md"],
                False,
                False,
            ),
            ([".codex/hooks.json", "ui/src/App.tsx"], True, False),
            ([".codex/hooks.json", "runtime/bsw/src/Can.c"], True, True),
            ([".codex-extra/hooks.json"], True, True),
        )
        for paths, code, native in cases:
            with self.subTest(paths=paths), tempfile.TemporaryDirectory() as directory:
                script_path = Path(directory) / "detect.py"
                script_path.write_text(script, encoding="utf-8")
                output = Path(directory) / "outputs"
                environment = {
                    "PULL_REQUEST": "184",
                    "GITHUB_REPOSITORY": "example/repo",
                    "GITHUB_OUTPUT": str(output),
                }
                listing = subprocess.CompletedProcess([], 0, stdout="\n".join(paths))
                with (
                    mock.patch.dict(os.environ, environment),
                    mock.patch.object(subprocess, "run", return_value=listing),
                ):
                    runpy.run_path(str(script_path))
                self.assertEqual(
                    output.read_text(encoding="utf-8"),
                    f"code={str(code).lower()}\nnative={str(native).lower()}\ndesktop={str(any(path.startswith(('ui/', 'runtime/')) for path in paths)).lower()}\n",
                )

    def test_skipped_jobs_match_the_required_gate(self):
        workflow = jobs()
        skipped = {name for name, job in workflow.items() if "outputs.native" in job["if"]}
        # A job that needs a skipped job is skipped with it.
        while extra := {name for name, job in workflow.items() if job["needs"] & skipped} - skipped:
            skipped |= extra
        gate = re.search(r"UI_INDEPENDENT = \{([^}]+)\}", WORKFLOW.read_text(encoding="utf-8"))
        self.assertIsNotNone(gate)
        listed = set(re.findall(r"'([a-z-]+)'", gate.group(1)))
        self.assertEqual(listed, skipped & workflow["required"]["needs"])
        self.assertIn("core", skipped)

    def test_skipped_job_inputs_do_not_read_ui(self):
        tracked = subprocess.run(
            ["git", "ls-files", "-z", "--", *INPUTS], cwd=ROOT, capture_output=True, check=True
        ).stdout.decode("utf-8").split("\0")
        readers = []
        for path in filter(None, tracked):
            if path in EXEMPT or path.startswith("core/tests/fixtures/"):
                continue
            try:
                text = (ROOT / path).read_text(encoding="utf-8")
            except (UnicodeDecodeError, FileNotFoundError):
                continue
            readers.extend(f"{path}:{number}" for number, line in enumerate(text.splitlines(), 1) if UI_PATH.search(line))
        self.assertEqual(readers, [], "Jobs skipped for ui/-only pull requests read ui/; update checks.yml")


if __name__ == "__main__":
    unittest.main()
