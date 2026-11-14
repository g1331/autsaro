"""The required source gate preserves raw results and consumes exact dispositions."""

import copy
import hashlib
import json
import os
import runpy
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import test_c_comparison as comparison_tests
from autosar_tooling import c_gate


class SourceGateTests(unittest.TestCase):
    def fixture(self, root):
        helper = comparison_tests.CComparisonTests()
        current, _project, output, baseline = helper.fixture(root)
        current["diagnostics"] = [helper.diagnostic("rule", 3)]
        current["projectAcceptance"] = {
            "status": "blocked", "occurrences": [{
                "diagnostic": 0, "locationIndex": 0, "assessment": "blocked",
            }],
        }
        return current, output, baseline

    def test_new_changed_text_and_cross_program_are_blocked_but_moves_pass(self):
        for mutation in ("new", "text", "program", "move"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                current, output, baseline = self.fixture(Path(temporary))
                if mutation == "new":
                    current["diagnostics"].append(copy.deepcopy(current["diagnostics"][0]))
                elif mutation == "text":
                    (output / "project/main.c").write_text("same();\nold();\nrewritten();\n")
                elif mutation == "program":
                    current["diagnostics"][0]["programs"] = ["probe"]
                with patch.object(c_gate.c_acceptance, "evaluate",
                                  return_value=current["projectAcceptance"]):
                    result = c_gate.verdict(current, output, baseline, {})
                self.assertEqual(result["status"], "passed" if mutation == "move" else "blocked")
                self.assertFalse(current["passed"])

    def test_current_exact_disposition_allows_new_but_stale_receipt_and_errors_refuse(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, output, baseline = self.fixture(Path(temporary))
            (output / "project/main.c").write_text("same();\nold();\nnew();\n")
            current["projectAcceptance"]["occurrences"][0]["assessment"] = "accepted"
            with patch.object(c_gate.c_acceptance, "evaluate",
                              return_value=current["projectAcceptance"]):
                self.assertEqual(c_gate.verdict(current, output, baseline, {})["status"], "passed")
            for mutation in ("blocked", "error"):
                recomputed = copy.deepcopy(current["projectAcceptance"])
                recomputed["status"] = mutation
                recomputed["occurrences"][0]["assessment"] = "blocked"
                with patch.object(c_gate.c_acceptance, "evaluate", return_value=recomputed), \
                        self.assertRaisesRegex(ValueError, "receipt differs|execution failed"):
                    c_gate.verdict(current, output, baseline, {})

    def test_disposition_cannot_cover_other_program_or_extra_occurrence(self):
        for mutation in ("program", "multiplicity"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                current, output, baseline = self.fixture(Path(temporary))
                (output / "project/main.c").write_text("same();\nold();\nnew();\n")
                current["projectAcceptance"]["occurrences"][0]["assessment"] = "accepted"
                extra = copy.deepcopy(current["diagnostics"][0])
                if mutation == "program":
                    extra["programs"] = ["probe"]
                current["diagnostics"].append(extra)
                current["projectAcceptance"]["occurrences"].append({
                    "diagnostic": 1, "locationIndex": 0, "assessment": "blocked",
                })
                with patch.object(c_gate.c_acceptance, "evaluate",
                                  return_value=current["projectAcceptance"]):
                    result = c_gate.verdict(current, output, baseline, {})
                    self.assertEqual(result["status"], "blocked")

    def test_two_tree_gate_refuses_different_or_missing_analyzer_binary(self):
        for binary in (None, "other-binary"):
            with self.subTest(binary=binary), tempfile.TemporaryDirectory() as temporary:
                current, output, baseline = self.fixture(Path(temporary))
                current["tool"]["binarySha256"] = binary
                with self.assertRaisesRegex(ValueError, "same analyzer binary"):
                    c_gate.verdict(current, output, baseline, {})

    def test_generator_changes_allowed_only_in_explicit_source_tree_mode(self):
        from autosar_tooling.c_comparison import compare

        helper = comparison_tests.CComparisonTests()
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = helper.runtime_fixture(Path(temporary))
            for root in (project, output / "project"):
                metadata = json.loads((root / "meta/integration.json").read_text())
                metadata["plan"]["runtimeSources"]["core/src/integration/example.rs"] = "d" * 64
                helper.seal(root, metadata)
            from ecu_tools.build import digest
            current["sourceSealSha256"] = digest(project / "meta/files.sha256")
            with self.assertRaisesRegex(ValueError, "configuration differs"):
                compare(current, project, output, baseline)
            self.assertEqual(compare(current, project, output, baseline, source_tree=True),
                             {"new": [], "removed": []})

    def test_rename_keeps_source_counts_and_config_binding(self):
        helper = comparison_tests.CComparisonTests()
        from autosar_tooling.c_comparison import compare

        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = helper.fixture(Path(temporary))
            for root in (project, output / "project"):
                (root / "main.c").rename(root / "renamed.c")
                (root / "target.json").write_text('{"target":"linux","sources":["renamed.c"]}')
            current = json.loads(json.dumps(current).replace("main.c", "renamed.c"))
            result = compare(current, project, output, baseline, source_tree=True,
                             renames={"main.c": "renamed.c"})
            self.assertEqual(result, {"new": [], "removed": []})

    def test_workflow_runs_target_host_gate_and_propagates_its_exit(self):
        import yaml
        from autosar_tooling.config import ROOT

        workflow = yaml.safe_load((ROOT / ".github/workflows/checks.yml").read_text())
        steps = workflow["jobs"]["c-analysis-samples"]["steps"]
        step = next(row for row in steps if row.get("name") == "Strict generated source gate")
        self.assertNotIn("if", step)
        with tempfile.TemporaryDirectory() as temporary:
            script = Path(temporary) / "gate.py"
            script.write_text(step["run"], encoding="utf-8")
            for code in (0, 1, 2):
                with self.subTest(code=code), patch.dict(os.environ, {
                    "ANALYSIS_TARGET": "linux-x64-controlled-v1",
                    "ANALYSIS_SAMPLE": "standard-ecu",
                }), patch.object(subprocess, "run", return_value=
                                 subprocess.CompletedProcess([], code)) as run, \
                        self.assertRaises(SystemExit) as exit_:
                    runpy.run_path(str(script))
                self.assertEqual(exit_.exception.code, code)
                self.assertEqual(run.call_args.args[0], [
                    sys.executable, "-m", "autosar_tooling.c_gate", "scan",
                    "--target", "linux-x64-controlled-v1", "--sample", "standard-ecu",
                ])

    def test_aggregate_refuses_missing_duplicate_changed_or_wrong_run_receipts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            helper = comparison_tests.CComparisonTests()
            summary, _, _, _ = helper.fixture(root / "fixture")
            # Aggregate reuses the full producer completeness validator.
            summary.update(passed=False, projectAcceptance={"inputIdentity": "a" * 64})
            for program in summary["programs"]:
                program["passed"] = False
            for target in c_gate.TARGETS:
                for sample in c_gate.SAMPLES:
                    directory = root / (target + "-" + sample) / sample
                    directory.mkdir(parents=True)
                    current = copy.deepcopy(summary)
                    current["scope"]["target"] = target
                    identity = hashlib.sha256(sample.encode()).hexdigest()
                    current["projectAcceptance"]["inputIdentity"] = identity
                    for name in ("summary.json", "baseline-summary.json"):
                        (directory / name).write_text(json.dumps(current))
                    gate = {"format": c_gate.FORMAT, "status": "passed", "target": target,
                            "sample": sample, "commit": "head", "baseline": "parent",
                            "inputIdentity": dict.fromkeys(("current", "baseline"), identity),
                            "summarySha256": {side: c_gate.sha(directory / name) for side, name in
                                              (("current", "summary.json"),
                                               ("baseline", "baseline-summary.json"))}}
                    (directory / "gate.json").write_text(json.dumps(gate))
            shutil.rmtree(root / "fixture")
            def revision(*args):
                return "head" if args[-1] == "HEAD" else "parent"
            with patch.object(c_gate, "git", side_effect=revision):
                c_gate.aggregate(root)
                path = next(root.glob("*/*/gate.json"))
                original = path.read_bytes()
                for change in ("missing", "duplicate", "run", "status", "identity", "summary"):
                    with self.subTest(change=change):
                        data = json.loads(original)
                        if change == "missing":
                            path.unlink()
                        elif change == "duplicate":
                            extra = root / "extra" / path.parent.name
                            shutil.copytree(path.parent, extra)
                        elif change == "summary":
                            path.with_name("summary.json").write_text("{}")
                        else:
                            fields = {"run": "commit", "status": "status",
                                      "identity": "inputIdentity"}
                            key = fields[change]
                            data[key] = "wrong" if change != "identity" else {"current": "missing"}
                            path.write_text(json.dumps(data))
                        with self.assertRaises((ValueError, KeyError)):
                            c_gate.aggregate(root)
                        path.write_bytes(original)
                        if change == "duplicate":
                            shutil.rmtree(root / "extra")
                        if change == "summary":
                            shutil.copy2(path.with_name("baseline-summary.json"),
                                         path.with_name("summary.json"))
