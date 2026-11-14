"""Keep required CI input selection, coverage and failure propagation safe."""

import ast
import json
import os
import re
import runpy
import subprocess
import sys
import tempfile
import textwrap
import unittest
from importlib.util import resolve_name
from itertools import product
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
    "tools/python/src/autosar_tooling/ci_selection.py",
}
UI_PATH = re.compile(r"""["'/\\]ui[/\\"']""")


def file_listing(paths):
    return json.dumps([[{"filename": path, "status": "modified"} for path in paths]])


def run_script(script):
    with tempfile.TemporaryDirectory() as temporary:
        path = Path(temporary) / "workflow.py"
        path.write_text(script, encoding="utf-8")
        return runpy.run_path(str(path))


def expression_value(expression, event):
    expression = expression.removeprefix("${{").removesuffix("}}").strip()
    for name in sorted(event, key=len, reverse=True):
        expression = expression.replace(name, repr(event[name]))
    expression = expression.replace("||", " or ").replace("&&", " and ")
    return eval(expression, {"__builtins__": {}})


def concurrency_group(expression, event):
    return re.sub(r"\$\{\{(.*?)\}\}",
                  lambda match: str(expression_value(match[1], event)), expression)


class CiSelectionTests(unittest.TestCase):
    def test_master_required_starts_while_previous_extended_runs_and_pr_replaces_old_run(self):
        workflow = yaml.safe_load(WORKFLOW.read_text())
        extended = yaml.safe_load((ROOT / ".github/workflows/extended.yml").read_text())
        running, pending, states = {}, {}, {}

        def arrive(name, event):
            group = concurrency_group(workflow["concurrency"]["group"], event).casefold()
            cancel = expression_value(workflow["concurrency"]["cancel-in-progress"], event)
            if group in running and not cancel:
                if group in pending:
                    states[pending[group]] = "cancelled"
                pending[group] = name
                states[name] = "pending"
            else:
                if group in running:
                    states[running[group]] = "cancelled"
                running[group] = name
                states[name] = "required-running"

        def event(sha, *, pull_request=0):
            return {"github.workflow": workflow["name"], "github.event.action": "synchronize",
                    "github.event.label.name": "", "github.run_id": sha,
                    "github.event.pull_request.number": pull_request, "github.sha": sha,
                    "github.ref": "refs/pull/99/merge" if pull_request else "refs/heads/master",
                    "github.event_name": "pull_request" if pull_request else "push"}

        # A still owns its caller lock after CI required, throughout its Extended tail.
        arrive("A", event("a" * 40))
        states["A"] = "extended-running"
        arrive("B", event("b" * 40))
        arrive("C", event("c" * 40))
        self.assertEqual(states, {"A": "extended-running", "B": "required-running",
                                  "C": "required-running"})
        self.assertEqual(pending, {})
        extended_groups = {concurrency_group(extended["concurrency"]["group"], event(sha * 40))
                           for sha in ("a", "b", "c")}
        self.assertEqual(len(extended_groups), 3)
        self.assertTrue(extended_groups.isdisjoint(running))

        arrive("PR-A", event("d" * 40, pull_request=99))
        states["PR-A"] = "extended-running"
        arrive("PR-B", event("e" * 40, pull_request=99))
        arrive("PR-C", event("f" * 40, pull_request=99))
        self.assertEqual([states[name] for name in ("PR-A", "PR-B", "PR-C")],
                         ["cancelled", "cancelled", "required-running"])
        self.assertEqual(states["A"], "extended-running")

    def test_analysis_and_documentation_selection_follows_consumers(self):
        script = yaml.safe_load(WORKFLOW.read_text())["jobs"]["changes"]["steps"][1]["run"]
        analysis = ["tools/python/src/autosar_tooling/" + name + ".py"
                    for name in ("c_check", "c_acceptance", "c_sdk",
                                 "c_comparison", "c_addon", "c_gate")]
        analysis += ["docs/standards/os/adoption.md", "docs/standards/types/declarations.md",
                     ".agents/skills/misra-c2012/references/baseline-index.json"]
        full = ["tools/python/src/autosar_tooling/c_standard.py",
                "tools/python/src/autosar_tooling/c_retention.py",
                "docs/standards/rte/generated-retention/README.md", "LICENSE", "NOTICE",
                "docs/official/example.zip", "core/src/generator.rs", "unknown/input.txt",
                ".github/workflows/checks.yml", ".github/workflows/extended.yml"]
        cases = [([{"filename": path, "status": "modified"}], "analysis") for path in analysis]
        cases += [([{"filename": path, "status": "modified"}], "full") for path in full]
        cases += [([{"filename": path, "status": "modified"} for path in paths], "docs")
                  for paths in (["README.md", "CHANGELOG.md", "docs/development/testing/README.md"],
                                ["docs/architecture/runtime.md"], ["README.zh-CN.md"])]
        for path in ("core/src/generator.rs", "README.md", "ui/src/App.tsx"):
            cases.append(([{"filename": analysis[0], "status": "modified"},
                           {"filename": path, "status": "modified"}], "full"))
        cases += [([{"filename": analysis[0], "status": status}], "full")
                  for status in ("added", "removed", "renamed")]
        cases += [([{"filename": analysis[0], "status": "modified",
                     "previous_filename": "old.py"}], "full"),
                  ([], "full"), ([{"filename": analysis[0], "status": "modified"}] * 3000, "full")]
        for rows, mode in cases:
            with self.subTest(rows=rows[:2], mode=mode), tempfile.TemporaryDirectory() as temporary:
                output = Path(temporary) / "output"
                env = {"PULL_REQUEST": "1", "GITHUB_REPOSITORY": "example/repo",
                       "GITHUB_OUTPUT": str(output)}
                result = subprocess.CompletedProcess([], 0, json.dumps([rows]))
                with mock.patch.dict(os.environ, env), \
                        mock.patch.object(subprocess, "run", return_value=result), \
                        mock.patch("builtins.print"):
                    run_script(script)
                values = dict(line.split("=", 1) for line in output.read_text().splitlines())
                self.assertEqual(values["code"], str(mode != "docs").lower())
                self.assertEqual(values["analysis"], str(mode != "docs").lower())
                self.assertEqual(values["native"], str(mode == "full").lower())
                self.assertEqual(values["application"], str(mode == "full").lower())
                if mode != "full":
                    self.assertEqual(values["desktop"], "false")
                if rows and rows[0]["filename"] == "unknown/input.txt":
                    self.assertEqual(values["desktop"], "true")

    def test_analysis_drivers_are_outside_skipped_python_consumers(self):
        drivers = {"autosar_tooling." + name for name in
                   ("c_check", "c_acceptance", "c_sdk", "c_comparison", "c_addon", "c_gate")}
        modules = {}
        for path in (ROOT / "tools/python/src").rglob("*.py"):
            name = path.relative_to(ROOT / "tools/python/src").with_suffix("").as_posix()
            modules[name.replace("/", ".").removesuffix(".__init__")] = path
        modules.update({path.stem: path for path in (ROOT / "tests/python/native").glob("*.py")})
        roots = list((ROOT / "tools/python/src/ecu_tools").glob("*.py"))
        roots += [path for path in (ROOT / "tests/python/native").glob("*.py")
                  if path.name not in {"test_c_analysis.py", "test_clang_build.py"}]
        roots += list((ROOT / "core/tests/fixtures").rglob("*.py"))
        seen, pending = set(), list(roots)
        while pending:
            path = pending.pop()
            if path in seen:
                continue
            seen.add(path)
            for node in ast.walk(ast.parse(path.read_text(encoding="utf-8"))):
                names = []
                if isinstance(node, ast.Import):
                    names = [alias.name for alias in node.names]
                elif isinstance(node, ast.ImportFrom):
                    module = node.module or ""
                    if node.level:
                        package = next(name for name, entry in modules.items() if entry == path)
                        if path.name != "__init__.py":
                            package = package.rpartition(".")[0]
                        module = resolve_name("." * node.level + module, package)
                    names = [module] + [module + "." + alias.name for alias in node.names]
                    if module == "autosar_tooling.c_standard":
                        self.assertLessEqual({alias.name for alias in node.names},
                                             {"lex", "is_comment", "function", "code_words",
                                              "verify_memory_mapping_pattern"})
                self.assertFalse(drivers.intersection(names), (path, names))
                # Importing a submodule also executes each containing package initializer.
                packages = [".".join(name.split(".")[:index]) for name in names
                            for index in range(1, len(name.split(".")))]
                pending.extend(modules[name] for name in names + packages if name in modules)
        compiler = (ROOT / ".github/actions/native-analysis-compiler/action.yml").read_text()
        self.assertIn("from ecu_tools.build import compiler_include_paths", compiler)
        self.assertNotIn("autosar_tooling", compiler)
        from autosar_tooling import c_check
        from ecu_tools.build import compiler_include_paths
        self.assertIs(c_check.compiler_include_paths, compiler_include_paths)
        from autosar_tooling.c_retention import RECORD_DIRECTORY
        self.assertEqual(RECORD_DIRECTORY, "docs/standards/rte/generated-retention")
        standard = ast.parse((ROOT / "tools/python/src/autosar_tooling/c_standard.py").read_text())
        functions = {node.name: node for node in standard.body if isinstance(node, ast.FunctionDef)}
        pending = ["lex", "is_comment", "function", "code_words", "verify_memory_mapping_pattern"]
        checked = set()
        while pending:
            name = pending.pop()
            if name in checked:
                continue
            checked.add(name)
            names = {node.id for node in ast.walk(functions[name]) if isinstance(node, ast.Name)}
            self.assertFalse(names & {"ROOT", "BASELINE", "MANUAL_ASSESSMENT_SOURCES"}, name)
            pending.extend(names & functions.keys() - checked)

    def test_extended_is_called_once_after_required_at_the_same_revision(self):
        required = yaml.safe_load(WORKFLOW.read_text())
        extended = yaml.safe_load((ROOT / ".github/workflows/extended.yml").read_text())
        events = extended.get("on", extended.get(True))
        self.assertEqual(set(events), {"workflow_call", "workflow_dispatch"})
        calls = [job for job in required["jobs"].values()
                 if job.get("uses") == "./.github/workflows/extended.yml"]
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0]["needs"], "required")
        self.assertIn("needs.required.result == 'success'", calls[0]["if"])
        # Required can succeed after deliberately skipped ancestors (docs/UI/analysis).
        # A status function overrides Actions' implicit success() for that graph.
        self.assertIn("!cancelled()", calls[0]["if"])
        self.assertNotIn("extended", required["jobs"]["required"]["needs"])
        self.assertEqual(extended["permissions"], {"contents": "read"})
        self.assertTrue(extended["concurrency"]["group"].startswith("extended-"))
        for job in extended["jobs"].values():
            for step in job.get("steps", []):
                if step.get("uses", "").startswith("actions/checkout@"):
                    self.assertEqual(step["with"]["ref"], "${{ github.sha }}")
                    self.assertFalse(step["with"]["persist-credentials"])
        self.assertNotIn("workflow_run", events)
        self.assertNotIn("pull_request_target", events)

    def test_label_events_do_not_cancel_or_replace_required_for_unrelated_labels(self):
        workflow = yaml.safe_load(WORKFLOW.read_text())
        for action, label in (("synchronize", ""), ("labeled", "full-ci"), ("labeled", "area:ci")):
            event = {"github.workflow": "Development checks", "github.event.action": action,
                     "github.event.label.name": label, "github.event_name": "pull_request",
                     "github.run_id": 123, "github.event.pull_request.number": 99,
                     "github.ref": "refs/pull/99/merge", "github.sha": "a" * 40}
            selected = action != "labeled" or label == "full-ci"
            cancelled = expression_value(workflow["concurrency"]["cancel-in-progress"], event)
            self.assertEqual(bool(cancelled), selected)
            name = expression_value(workflow["jobs"]["required"]["name"], event)
            self.assertEqual(name == "CI required", selected)
            for job in ("changes", "quality"):
                self.assertEqual(bool(expression_value(workflow["jobs"][job]["if"], event)), selected)

    def test_combined_application_job_keeps_every_check_inside_twenty_minutes(self):
        workflow = yaml.safe_load(WORKFLOW.read_text())["jobs"]
        self.assertNotIn("ui", workflow)
        job = workflow["desktop"]
        self.assertEqual(job["timeout-minutes"], 20)
        commands = [step["run"] for step in job["steps"] if "run" in step]
        for command in (
            "npm ci --prefix ui", "npm run build --prefix ui", "npm run lint --prefix ui",
            "npm run ipc:check --prefix ui", "cargo build --locked -p autosar-config-desktop",
        ):
            self.assertEqual(commands.count(command), 1)
        self.assertTrue(any("npm run test --prefix ui" in command for command in commands))
        self.assertTrue(any("cargo test --locked --profile dev" in command for command in commands))
        self.assertTrue(any("cargo clippy --locked -p autosar-config-desktop" in command
                            for command in commands))
        for step in job["steps"]:
            if "run" in step:
                self.assertIn("!cancelled()", step["if"])

    def test_context_artifact_keeps_exact_sources_and_raw_summary_without_analyzer_intermediates(self):
        workflow = yaml.safe_load(WORKFLOW.read_text())["jobs"]
        for job in ("c-analysis-gates", "c-analysis-gates-windows"):
            step = next(step for step in workflow[job]["steps"]
                        if step.get("name") == "Retain source assessment inputs")
            options = step["with"]
            self.assertTrue(options["include-hidden-files"])
            self.assertEqual(options["name"],
                             "c-analysis-context-inputs-${{ matrix.target }}-${{ matrix.group }}")
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                result = root / "build/c-analysis-results/standard-ecu"
                files = {"summary.json": b'{"passed":false,"error":null}',
                         "project/meta/files.sha256": b"sealed receipt\n",
                         "project/.source-input": b"byte-bound hidden input\r\n",
                         "project/kernel/tasks.c": b"patched kernel\n",
                         "target/file.dump": b"large intermediate"}
                for name, data in files.items():
                    path = result / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(data)
                selected = {}
                for pattern in options["path"].splitlines():
                    for path in root.glob(pattern):
                        for item in path.rglob("*") if path.is_dir() else (path,):
                            if item.is_file():
                                selected[item.relative_to(result).as_posix()] = item.read_bytes()
                expected = {name: data for name, data in files.items()
                            if name == "summary.json" or name.startswith("project/")}
                self.assertEqual(selected, expected)
            producer = workflow[job.replace("gates", "samples")]
            full = [step for step in producer["steps"]
                    if step.get("with", {}).get("name", "").startswith(
                        "c-analysis-${{ env.ANALYSIS_SIDE == 'baseline'")]
            self.assertEqual(len(full), 1)
            self.assertEqual(full[0]["with"]["path"],
                             "build/${{ env.ANALYSIS_SIDE == 'baseline' && "
                             "'c-analysis-baseline' || 'c-analysis-results' }}")

    def test_cargo_source_digest_uses_tracked_bytes_despite_generated_files_and_timestamps(self):
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))["jobs"]
        scripts = [next(step["run"] for step in workflow[name]["steps"]
                        if step.get("name") == "Identify native build cache inputs")
                   for name in ("native", "native-sanitizers", "c-analysis-prepare",
                                "c-analysis-prepare-windows")]
        self.assertTrue(all(script == scripts[0] for script in scripts))
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            source = root / "tools/python/src/ecu_tools/module.py"
            source.parent.mkdir(parents=True)
            source.write_text("value = 1\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(root), "add", "."], check=True)
            output = root / "github-env"
            env = dict(os.environ, GITHUB_ENV=str(output), ImageOS="test", ImageVersion="1")

            def digest():
                output.write_text("", encoding="utf-8")
                subprocess.run([sys.executable, "-c", scripts[0]], cwd=root, env=env,
                               check=True, capture_output=True, text=True)
                return next(line for line in output.read_text(encoding="utf-8").splitlines()
                            if line.startswith("CACHE_SOURCE="))

            before = digest()
            for timestamp in (1700000000, 1700000300):
                os.utime(source, (timestamp, timestamp))
                subprocess.run([sys.executable, "-m", "py_compile", str(source)], check=True)
                # Future generated formats are excluded by tracking, too.
                (source.parent / "generated.bin").write_bytes(str(timestamp).encode("ascii"))
                self.assertEqual(digest(), before)
            source.write_text("value = 2\n", encoding="utf-8")
            self.assertNotEqual(digest(), before)

    def test_native_filters_partition_existing_and_future_modules(self):
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        native = workflow["jobs"]["native"]
        step = next(step for step in native["steps"]
                    if step.get("name") == "Native library and contract suites")
        outlier = ("integration::single_component::"
                   "short_application_period_saves_builds_and_passes_reference_verifier")
        names = [outlier, "library_unit", "runtime_contracts::can", "host::read", "future::test",
                 "integration::protocol::refusal", "integration::sealed_projects::build",
                 "integration::single_component::build", "integration::os::time",
                 "integration::release_contracts::headers", "integration::future::test"]
        for platform in ("ubuntu-24.04", "windows-latest"):
            observed = []
            for entry in native["strategy"]["matrix"]["include"]:
                if entry["os"] != platform:
                    continue
                with tempfile.TemporaryDirectory() as directory:
                    source = Path(directory) / "native.py"
                    source.write_text(step["run"], encoding="utf-8")
                    with (mock.patch.dict(os.environ, {"SHARD": entry["shard"],
                                                       "RUNNER_TEMP": directory}),
                          mock.patch.object(subprocess, "run") as run,
                          self.assertRaises(SystemExit)):
                        run.return_value = subprocess.CompletedProcess(
                            [], 0, "\n".join(name + ": test" for name in names), ""
                        )
                        runpy.run_path(str(source))
                    argv = run.call_args.args[0]
                    self.assertIn("--lib", argv)
                    self.assertIn("--test", argv)
                    self.assertIn("--no-fail-fast", argv)
                    filters = argv[argv.index("native", argv.index("--test")) + 3:]
                    # libtest combines positive filters with OR and applies every --skip.
                    skipped = [filters[i + 1] for i, value in enumerate(filters)
                               if value == "--skip"]
                    positive = [value for i, value in enumerate(filters)
                                if value not in {"--skip", "--exact"}
                                and (i == 0 or filters[i - 1] != "--skip")]
                    if entry["shard"] != "all":
                        self.assertIn("--exact", filters)
                        self.assertEqual(outlier in positive, entry["shard"] == "rest")
                        self.assertEqual("integration::release_contracts::headers" in positive,
                                         entry["shard"] == "projects")
                    observed.extend(
                        name for name in names
                        if (not positive or any(
                            value == name if "--exact" in filters else value in name
                            for value in positive
                        )) and not any(value in name for value in skipped)
                    )
            self.assertCountEqual(observed, names, platform)

    def test_native_listing_failure_and_empty_output_fail_before_running_tests(self):
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        script = next(step["run"] for step in workflow["jobs"]["native"]["steps"]
                      if step.get("name") == "Native library and contract suites")
        for code in (0, 2):
            with self.subTest(code=code), tempfile.TemporaryDirectory() as directory:
                source = Path(directory) / "native.py"
                source.write_text(script, encoding="utf-8")
                result = subprocess.CompletedProcess([], code, "", "listing failed" if code else "")
                with (mock.patch.dict(os.environ, {"SHARD": "protocol", "RUNNER_TEMP": directory}),
                      mock.patch.object(subprocess, "run", return_value=result) as run):
                    if code:
                        with self.assertRaises(SystemExit) as error:
                            runpy.run_path(str(source))
                        self.assertEqual(error.exception.code, code)
                    else:
                        with self.assertRaisesRegex(RuntimeError, "selected no tests"):
                            runpy.run_path(str(source))
                    self.assertEqual(run.call_count, 1)

    def test_independent_native_steps_run_once_per_platform(self):
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        for job, name, key, allowed in (
            ("native-os", "Host I/O and lock failure boundaries", "shard", {"0/1", "1/2"}),
            ("c-analysis-samples", "Analyzer behavior tests", "group", {"no-diagnostic"}),
        ):
            steps = workflow["jobs"][job]["steps"]
            step = next(step for step in steps if step.get("name") == name)
            selected = set(re.findall(r"matrix\." + key + r" == '([^']+)'", step["if"]))
            self.assertEqual(selected, allowed)
            entries = list(workflow["jobs"][job]["strategy"]["matrix"]["include"])
            if job == "c-analysis-samples":
                windows = workflow["jobs"]["c-analysis-samples-windows"]
                entries += windows["strategy"]["matrix"]["include"]
            for platform in ("ubuntu-24.04", "windows-latest"):
                self.assertEqual(sum(entry[key] in selected for entry in entries
                                     if entry["os"] == platform), 1)
        self.assertFalse(any(step.get("name") == "Analyzer behavior tests"
                             for step in workflow["jobs"]["c-analysis-prepare"]["steps"]))

    def test_parallel_com_refusals_keep_all_fourteen_cases_and_propagate_failure(self):
        path = ROOT / "core/tests/fixtures/com-rejection/consumers.py"
        tree = ast.parse(path.read_text(encoding="utf-8"))
        # Replace only the expensive case executor; run the actual grouping and worker joins.
        for index, node in enumerate(tree.body):
            if isinstance(node, ast.ImportFrom) and node.module == "ecu_tools.build":
                tree.body[index] = ast.Pass()
            if isinstance(node, ast.FunctionDef) and node.name == "run_case":
                node.body = ast.parse("record(name, mode, **kwargs)").body
                node.args.kwonlyargs = []
                node.args.kw_defaults = []
                node.args.kwarg = ast.arg(arg="kwargs")
        ast.fix_missing_locations(tree)
        expected = {"host", "probe", "task-and-isr", "host-batch-capacity", "probe-capacity",
                    "trigger-notify", "periodic-notify", "condition", "pdu", "result",
                    "host-batch-drain", "host-batch-output", "probe-drain", "probe-output"}
        with tempfile.TemporaryDirectory() as temporary:
            script = Path(temporary) / "consumers.py"
            script.write_text(ast.unparse(tree), encoding="utf-8")
            for failure in (None, "condition"):
                observed = []
                def record(name, mode, observed=observed, failure=failure, **kwargs):
                    observed.append((name, mode, kwargs))
                    if name == failure:
                        raise RuntimeError("mutation worker failed")
                with mock.patch.object(sys, "argv", [str(path), "/source", "/output"]):
                    if failure:
                        with self.assertRaisesRegex(RuntimeError, "mutation worker failed"):
                            runpy.run_path(str(script), init_globals={"record": record})
                    else:
                        runpy.run_path(str(script), init_globals={"record": record})
                        self.assertCountEqual([row[0] for row in observed], expected)
                        for name, mode, options in observed:
                            if name.endswith(("-drain", "-output")) or name in {
                                "trigger-notify", "periodic-notify", "condition", "pdu", "result"
                            }:
                                self.assertTrue(options["killed"], name)
                            if name.endswith("-output"):
                                self.assertTrue(options["output_failure"], name)

    def test_platform_pipelines_share_steps_and_require_every_result(self):
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))["jobs"]
        for name in ("c-analysis-prepare", "c-analysis-samples", "native-delivery"):
            linux, windows = workflow[name], workflow[name + "-windows"]
            self.assertEqual(linux["steps"], windows["steps"])
            for job, platform in ((linux, "ubuntu-24.04"), (windows, "windows-latest")):
                self.assertEqual({row["os"] for row in job["strategy"]["matrix"]["include"]},
                                 {platform})
            if name != "c-analysis-prepare":
                self.assertEqual(linux["needs"], ["changes", "c-analysis-prepare"]
                                 if name == "native-delivery" else "c-analysis-prepare")
                self.assertEqual(windows["needs"], ["changes", "c-analysis-prepare-windows"]
                                 if name == "native-delivery" else "c-analysis-prepare-windows")
        upstream = workflow["c-analysis"]["needs"]
        self.assertEqual(set(upstream), {
            "changes", "c-analysis-prepare", "c-analysis-prepare-windows",
            "c-analysis-samples", "c-analysis-samples-windows",
            "c-analysis-baseline", "c-analysis-baseline-windows",
            "c-analysis-gates", "c-analysis-gates-windows",
        })
        self.assertIn("native-delivery-windows", workflow["required"]["needs"])
        gate = next(step for step in workflow["c-analysis"]["steps"]
                    if step.get("name") == "Require every platform and sample group")["run"]
        for name in upstream:
            for status in ("success", "failure", "cancelled", "skipped"):
                results = {key: {"result": status if key == name else "success"}
                           for key in upstream}
                with (self.subTest(name=name, status=status),
                      tempfile.TemporaryDirectory() as directory):
                    script = Path(directory) / "gate.py"
                    script.write_text(gate, encoding="utf-8")
                    with mock.patch.dict(os.environ, {"UPSTREAM_RESULTS": json.dumps(results)}):
                        if status == "success":
                            runpy.run_path(str(script))
                        else:
                            with self.assertRaisesRegex(RuntimeError, "Incomplete platform/sample"):
                                runpy.run_path(str(script))
        for name in ("python", "desktop"):
            job = workflow[name]
            self.assertEqual(set(job["needs"]),
                             {"changes", "c-analysis-prepare", "c-analysis-prepare-windows"})
            # The status function explicitly permits a skipped producer on UI-only PRs.
            condition = "${{ !cancelled() && needs.changes.outputs.code == 'true' }}"
            self.assertEqual(job["if"], condition.replace("outputs.code", "outputs.application")
                             if name == "desktop" else condition)

    def test_analysis_readiness_rejects_acceptance_errors_but_not_known_blockers(self):
        workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
        step = next(row for row in workflow["jobs"]["c-analysis-samples"]["steps"]
                    if row.get("name") == "Verify complete generated-project analysis")
        for side, status in product(("current", "baseline"),
                                    ("accepted", "blocked", "error", "unknown", None)):
            with self.subTest(side=side, status=status), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                results = "c-analysis-baseline" if side == "baseline" else "c-analysis-results"
                inputs = "c-analysis-base-samples" if side == "baseline" else "c-analysis-samples"
                output = root / "build" / results / "standard-ecu/summary.json"
                output.parent.mkdir(parents=True)
                data = {"error": None, "passed": False}
                if status is not None:
                    data["projectAcceptance"] = {"status": status}
                output.write_text(json.dumps(data), encoding="utf-8")
                script = root / "readiness.py"
                script.write_text(step["run"], encoding="utf-8")
                env = {"ANALYSIS_SIDE": side, "ANALYSIS_TARGET": "linux-x64-controlled-v1",
                       "ANALYSIS_SAMPLES": "standard-ecu"}
                previous = Path.cwd()
                try:
                    os.chdir(root)
                    with (
                        mock.patch.dict(os.environ, env),
                        mock.patch.object(subprocess, "run",
                                          return_value=subprocess.CompletedProcess([], 1)) as run,
                        self.assertRaises(SystemExit) as exit_,
                    ):
                        runpy.run_path(str(script))
                finally:
                    os.chdir(previous)
                self.assertEqual(exit_.exception.code == 0, status in {"accepted", "blocked"})
                command = run.call_args.args[0]
                self.assertEqual(command[command.index("--project") + 1],
                                 str(Path("build") / inputs / "standard-ecu"))
                self.assertEqual(command[command.index("--output-directory") + 1],
                                 str(Path("build") / results / "standard-ecu"))

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
                        listing = file_listing([path]) if event == "pull_request" else path + "\n"
                        run.return_value = subprocess.CompletedProcess([], 0, listing)
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
            entries = list(workflow["jobs"][job]["strategy"]["matrix"]["include"])
            if job == "native-delivery":
                windows = workflow["jobs"]["native-delivery-windows"]
                entries += windows["strategy"]["matrix"]["include"]
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
                workflow = yaml.safe_load(text)
                receiver = "All moved samples build and run with receiver Python 3.11"
                script = next(step["run"] for step in workflow["jobs"]["native-delivery"]["steps"]
                              if step.get("name") == receiver)
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
            (["docs/standards/rte/generated-retention/README.md"], True, True),
            (["docs/standards/types/declarations.md"], True, False),
            ([".agents/skills/misra-c2012/references/baseline-index.json"], True, False),
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
                listing = subprocess.CompletedProcess([], 0, stdout=file_listing(paths))
                with (
                    mock.patch.dict(os.environ, environment),
                    mock.patch.object(subprocess, "run", return_value=listing),
                ):
                    runpy.run_path(str(script_path))
                values = dict(line.split("=", 1) for line in output.read_text().splitlines())
                self.assertEqual(values["code"], str(code).lower())
                self.assertEqual(values["native"], str(native).lower())
                desktop = any(path.startswith(("ui/", "runtime/", ".codex-extra/")) for path in paths)
                self.assertEqual(values["desktop"], str(desktop).lower())

    def test_selected_jobs_and_required_gate_agree(self):
        workflow = yaml.safe_load(WORKFLOW.read_text())["jobs"]
        script = workflow["required"]["steps"][0]["run"]
        native_jobs = {"core", "native", "native-os", "native-sanitizers",
                       "native-delivery", "native-delivery-windows"}
        for mode, selected in (
            ("full", {"code": True, "native": True, "desktop": True,
                      "analysis": True, "application": True}),
            ("analysis", {"code": True, "native": False, "desktop": False,
                          "analysis": True, "application": False}),
            ("ui", {"code": True, "native": False, "desktop": True,
                    "analysis": False, "application": True}),
            ("docs", dict.fromkeys(
                ("code", "native", "desktop", "analysis", "application"), False)),
        ):
            for name in workflow["required"]["needs"]:
                selection = ("desktop" if name == "desktop-scenarios" else
                             "application" if name == "desktop" else
                             "analysis" if name == "c-analysis" else
                             "native" if name in native_jobs else "code")
                for status in ("success", "failure", "cancelled", "skipped"):
                    outputs = {key: str(value).lower() for key, value in selected.items()}
                    needs = {job: {"result": "success"} for job in workflow["required"]["needs"]}
                    needs["changes"]["outputs"] = outputs
                    needs[name]["result"] = status
                    with self.subTest(mode=mode, name=name, status=status), \
                            mock.patch.dict(os.environ, {"RESULTS": json.dumps(needs)}), \
                            mock.patch("builtins.print"):
                        try:
                            run_script(script)
                        except SystemExit as error:
                            expected = (status == "success" or status == "skipped"
                                        and name not in {"changes", "quality"}
                                        and not selected[selection])
                            self.assertEqual(error.code == 0, expected)
        for name in native_jobs:
            self.assertIn("outputs.native", workflow[name]["if"])
        for name in ("c-analysis-prepare", "c-analysis-prepare-windows", "c-analysis"):
            self.assertIn("outputs.analysis", workflow[name]["if"])
        self.assertIn("outputs.application", workflow["desktop"]["if"])

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
