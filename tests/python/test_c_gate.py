"""The required source gate preserves raw results and consumes exact dispositions."""

import copy
import gzip
import hashlib
import json
import os
import runpy
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import test_c_comparison as comparison_tests
from autosar_tooling import c_gate, c_rename


class SourceGateTests(unittest.TestCase):
    def test_consumer_recomputes_input_digests_and_refuses_producer_tampering(self):
        from autosar_tooling import c_baseline, c_fingerprint

        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            (project / "meta").mkdir()
            (project / "target.json").write_text('{"target":"verified"}', encoding="utf-8")
            (project / c_baseline.FILE_HASHES).write_text("sealed\n", encoding="utf-8")
            (project / "meta/integration.json").write_text(json.dumps({
                "plan": {"ruleSetIdentity": {"rules": "original"}, "sources": ["source"],
                         "runtimeSources": {"core/src/lib.rs": "original"}},
            }), encoding="utf-8")
            with patch.object(c_baseline, "analysis_inputs_identity", return_value="a" * 64):
                identity = {**c_baseline.sealed_project_identity(project), "inputs": "a" * 64,
                            "runner": {"ImageOS": "verified", "ImageVersion": "original"}}
                manifest = {"identity": identity}
                digest = c_fingerprint.sha(manifest)
                c_gate.validate_consumed_inputs(manifest, project, digest)
                for name, replacement in (
                        ("inputs", "b" * 64),
                        ("configuration", {"target": "b" * 64, "sources": ["other"]}),
                        ("generator", {"core/src/lib.rs": "b" * 64}),
                        ("rules", {"rules": "b" * 64})):
                    with self.subTest(name=name), self.assertRaisesRegex(ValueError, name):
                        changed = {"identity": {**identity, name: replacement}}
                        c_gate.validate_consumed_inputs(changed, project, c_fingerprint.sha(changed))
                changed = {"identity": {**identity, "runner": {"ImageOS": "verified", "ImageVersion": "fake"}}}
                with self.assertRaisesRegex(ValueError, "producer receipt"):
                    c_gate.validate_consumed_inputs(changed, project, digest)

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

    def test_injected_outside_diagnostic_is_visible_and_inside_still_blocks(self):
        for name, expected in (("os/src/host/linux/injected.c", "passed"),
                               ("bsw/src/injected.c", "blocked")):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                current, output, baseline = self.fixture(Path(temporary))
                destination = output / "project" / name
                destination.parent.mkdir(parents=True)
                destination.write_text("injected();\n")
                current["diagnostics"].append({
                    "id": "injected-rule", "locations": [{"file": name, "line": "1"}],
                    "programs": ["target"]})
                with patch.object(c_gate.c_acceptance, "evaluate",
                                  return_value=current["projectAcceptance"]):
                    result = c_gate.verdict(current, output, baseline, {})
                self.assertEqual(result["status"], expected)
                self.assertEqual(result["comparison"]["new"][0]["rule"], "injected-rule")
                self.assertEqual(len(result["outOfScopeNew"]), int(expected == "passed"))
                counts = result["complianceScope"]
                self.assertEqual(counts["total"], counts["inScope"] + counts["outOfScope"])
                self.assertEqual(len(current["diagnostics"]), 2)

    def test_kernel_diagnostic_entering_scope_blocks_even_when_raw_counts_match(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, output, baseline = self.fixture(Path(temporary))
            previous = copy.deepcopy(current)
            name = "kernel/tasks.c"
            patch_chain = [{"path": "runtime/os/patches/reclassify.patch", "includes": ["tasks.c"]}]
            for root, summary, patches, owned in (
                (baseline / "project", previous, [], {}),
                (output / "project", current, patch_chain, {name: [1]}),
            ):
                source = root / name
                source.parent.mkdir()
                source.write_bytes(b"step();\nstep();\n")
                target = root / "target.json"
                config = json.loads(target.read_text())
                config.update(kernelPatches=patches, kernelSha256={"tasks.c": c_gate.sha(source)})
                target.write_text(json.dumps(config))
                summary["scope"].update(patchedKernel=patches, patchedLines=owned)
                summary["diagnostics"] = [
                    {"id": "rule", "locations": [{"file": name, "line": str(line)}], "programs": ["target"]}
                    for line in (1, 2)
                ]
            (baseline / "summary.json").write_text(json.dumps(previous))
            for approved, expected in ((None, "blocked"), (1, "blocked"), (0, "passed")):
                with self.subTest(approved=approved):
                    acceptance = current["projectAcceptance"]
                    acceptance["occurrences"] = [
                        {"diagnostic": index, "locationIndex": 0,
                         "assessment": "accepted" if index == approved else "blocked"}
                        for index in range(2)
                    ]
                    with patch.object(c_gate.c_acceptance, "evaluate", return_value=acceptance):
                        gate = c_gate.verdict(current, output, baseline, {})
                    self.assertEqual(gate["comparison"], {"new": [], "removed": []})
                    self.assertFalse(gate["outOfScopeNew"])
                    self.assertEqual(gate["status"], expected)
                    self.assertEqual(gate["unacceptedNew"], [] if expected == "passed" else [
                        {"rule": "rule", "file": name, "source": "step();", "program": "target", "count": 1},
                    ])

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

    def test_existing_accepted_location_cannot_approve_identical_unaccepted_addition(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, output, baseline = self.fixture(Path(temporary))
            helper = comparison_tests.CComparisonTests()
            current["diagnostics"] = [helper.diagnostic("rule", line) for line in (1, 2, 3)]
            current["projectAcceptance"]["occurrences"] = [
                {"diagnostic": index, "locationIndex": 0,
                 "assessment": "accepted" if index == 0 else "blocked"}
                for index in range(3)]
            with patch.object(c_gate.c_acceptance, "evaluate",
                              return_value=current["projectAcceptance"]):
                self.assertEqual(c_gate.verdict(current, output, baseline, {})["status"],
                                 "blocked")
            current["projectAcceptance"]["occurrences"][0]["assessment"] = "blocked"
            current["projectAcceptance"]["occurrences"][2]["assessment"] = "accepted"
            with patch.object(c_gate.c_acceptance, "evaluate",
                              return_value=current["projectAcceptance"]):
                self.assertEqual(c_gate.verdict(current, output, baseline, {})["status"],
                                 "passed")

    def test_new_header_exact_disposition_has_no_existing_position(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, output, baseline = self.fixture(Path(temporary))
            (output / "project/added.h").write_text("new();\n")
            current["diagnostics"][0]["locations"] = [{"file": "added.h", "line": 1}]
            current["projectAcceptance"]["occurrences"][0]["assessment"] = "accepted"
            with patch.object(c_gate.c_acceptance, "evaluate",
                              return_value=current["projectAcceptance"]):
                self.assertEqual(c_gate.verdict(current, output, baseline, {})["status"],
                                 "passed")

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

    def test_generator_inventory_hashes_vary_but_rule_version_and_config_stay_bound(self):
        from autosar_tooling.c_comparison import compare
        from ecu_tools.build import digest

        helper = comparison_tests.CComparisonTests()
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = helper.runtime_fixture(Path(temporary))
            for root in (project, output / "project", baseline / "project"):
                metadata = json.loads((root / "meta/integration.json").read_text())
                identity = "a" * 64 if root == baseline / "project" else "b" * 64
                metadata["plan"].update(
                    ruleSetIdentity={"release": "R24-11", "rulesVersion": "1.1.0",
                                     "sha256": identity},
                    validationDependencies={"rules.release": "R24-11", "rules.version": "1.1.0",
                                            "rules.sha256": identity,
                                            "definitions.sha256": identity},
                )
                helper.seal(root, metadata)
            current["sourceSealSha256"] = digest(project / "meta/files.sha256")
            previous = json.loads((baseline / "summary.json").read_text())
            previous["sourceSealSha256"] = digest(baseline / "project/meta/files.sha256")
            helper.write(baseline, previous)
            with self.assertRaisesRegex(ValueError, "configuration differs"):
                compare(current, project, output, baseline)
            self.assertEqual(compare(current, project, output, baseline, source_tree=True),
                             {"new": [], "removed": []})
            for field in ("ruleSetIdentity", "validationDependencies", "configuration"):
                with self.subTest(field=field):
                    original = (project / "meta/integration.json").read_text()
                    metadata = json.loads(original)
                    if field == "configuration":
                        metadata["plan"][field] = "different"
                    elif field == "ruleSetIdentity":
                        metadata["plan"][field]["rulesVersion"] = "2.0.0"
                    else:
                        metadata["plan"][field]["rules.version"] = "2.0.0"
                    for root in (project, output / "project"):
                        helper.seal(root, metadata)
                    current["sourceSealSha256"] = digest(project / "meta/files.sha256")
                    with self.assertRaisesRegex(ValueError, "configuration differs"):
                        compare(current, project, output, baseline, source_tree=True)
                    for root in (project, output / "project"):
                        helper.seal(root, json.loads(original))

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

    def test_rename_can_change_sorted_translation_unit_order(self):
        from autosar_tooling.c_comparison import compare

        helper = comparison_tests.CComparisonTests()
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = helper.fixture(Path(temporary))
            for root in (project, output / "project", baseline / "project"):
                (root / "other.c").write_text("other();\n")
                (root / "target.json").write_text(json.dumps({"sources": ["main.c", "other.c"]}))
            current["scope"]["translationUnits"] = ["main.c", "other.c"]
            current["programs"][0]["translationUnits"] = ["main.c", "other.c"]
            current["semanticIdentity"]["defines"]["other.c"] = []
            helper.write(baseline, current)
            for root in (project, output / "project"):
                (root / "main.c").rename(root / "renamed.c")
                (root / "target.json").write_text(
                    json.dumps({"sources": ["other.c", "renamed.c"]}))
            current = json.loads(json.dumps(current).replace("main.c", "renamed.c"))
            current["scope"]["translationUnits"].sort()
            current["programs"][0]["translationUnits"].sort()
            self.assertEqual(compare(current, project, output, baseline, source_tree=True,
                                     renames={"main.c": "renamed.c"}), {"new": [], "removed": []})

    def test_same_run_consumer_rejects_transferred_input_changes(self):
        for changed in (False, True):
            with self.subTest(changed=changed), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                current, output, baseline = self.fixture(root / "fixture")
                current["scope"]["target"] = "windows-x64-controlled-v1"
                current["projectAcceptance"]["inputIdentity"] = "a" * 64
                for name in ("c-analysis-results", "c-analysis-baseline"):
                    directory = root / "build" / name / "standard-ecu"
                    shutil.copytree(output if name.endswith("results") else baseline, directory)
                    (directory / "summary.json").write_text(json.dumps(current))
                    (directory / "baseline-source.json").write_text(json.dumps({
                        "kind": "same-run", "tree": "commit",
                    }))
                with patch.object(c_gate, "ROOT", root), \
                        patch.object(c_gate.c_acceptance, "_complete"), \
                        patch.object(c_gate.c_acceptance, "input_identity",
                                     return_value=("b" if changed else "a") * 64) as identity, \
                        patch.object(c_gate, "rename_paths", return_value={}), \
                        patch.object(c_gate, "git", return_value="commit"), \
                        patch.object(c_gate, "verdict", return_value={"status": "passed"}) as gate:
                    if changed:
                        with self.assertRaisesRegex(ValueError, "Transferred baseline"):
                            c_gate.scan("windows-x64-controlled-v1", "standard-ecu")
                        gate.assert_not_called()
                    else:
                        c_gate.scan("windows-x64-controlled-v1", "standard-ecu")
                        gate.assert_called_once()
                        self.assertEqual([call.kwargs["producer_revision"]
                                          for call in identity.call_args_list], ["commit", None])

    def scan_with_unavailable_evidence(self, root, damage):
        helper = comparison_tests.CComparisonTests()
        current, _, output, baseline = helper.fixture(root / "fixture")
        current["scope"]["cppcheckInputs"] = {"main.c": {"project": ["main.c"], "sdk": [], "unbound": []}}
        current["projectAcceptance"] = {"status": "blocked", "occurrences": [], "inputIdentity": "a" * 64}
        current["diagnostics"] = [helper.diagnostic("misra-c2012-10.3", 1)]
        current["diagnostics"][0]["locations"][0]["column"] = "12"
        directories = {}
        for side, source, job, name in (
                ("baseline", baseline, "c-analysis-baseline", "storage"),
                ("current", output, "c-analysis-results", "counters")):
            directory = root / "build" / job / "standard-ecu"
            directories[side] = directory
            shutil.copytree(source, directory)
            project = directory / "project"
            code = f"static int {name};\n"
            (project / "main.c").write_text(code, encoding="utf-8")
            helper.seal(project, {"plan": {"configuration": "same"}})
            helper.write(directory, {**current, "sourceSealSha256": c_gate.sha(project / "meta/files.sha256")})
            proof = c_rename._proof_path(directory, "main.c")
            proof.parent.mkdir()
            proof.write_bytes(gzip.compress(json.dumps({
                "gcc": '# 1 "@project@/main.c"\n' + code,
                "bindings": {"target": [{"externs": [], "uses": [[1, 12, name, 1, 12]]}]},
            }).encode()))
            (directory / "analysis-identity.json").write_text(json.dumps({
                "summarySha256": c_gate.sha(directory / "summary.json"),
                "identifierEvidence": {"main.c": c_gate.sha(proof)},
            }), encoding="utf-8")
        (directories["baseline"] / "baseline-source.json").write_text(
            '{"kind":"same-run","tree":"commit"}', encoding="utf-8")
        damage(directories)
        with (patch.object(c_gate, "ROOT", root), patch.object(c_gate, "git", return_value="commit"),
              patch.object(c_gate.c_acceptance, "_complete"),
              patch.object(c_gate.c_acceptance, "input_identity", return_value="a" * 64),
              patch.object(c_gate.c_acceptance, "evaluate", return_value=current["projectAcceptance"]),
              self.assertRaisesRegex(ValueError, "New source diagnostics")):
            c_gate.scan("linux", "standard-ecu")
        gate = c_gate.read(directories["current"] / "gate.json")
        self.assertEqual(gate["status"], "blocked")
        self.assertEqual(gate["unacceptedNew"], gate["comparison"]["new"])
        self.assertEqual(len(gate["unacceptedNew"]), 1)
        return gate

    def test_scan_records_both_missing_evidence_manifests(self):
        def damage(directories):
            for directory in directories.values():
                (directory / "analysis-identity.json").unlink()

        with tempfile.TemporaryDirectory() as temporary:
            gate = self.scan_with_unavailable_evidence(Path(temporary), damage)
        reason = gate["identifierEvidenceError"]
        for side in ("baseline", "current"):
            self.assertIn(f"{side}: Identifier evidence manifest missing or unreadable (FileNotFoundError)",
                          reason)

    def test_scan_rejects_deeply_nested_evidence_json(self):
        nested = b"[" * 20_000 + b"0" + b"]" * 20_000
        for location in ("manifest", "gzip"):
            def damage(directories, location=location):
                directory = directories["current"]
                manifest = directory / "analysis-identity.json"
                if location == "manifest":
                    manifest.write_bytes(nested)
                else:
                    proof = c_rename._proof_path(directory, "main.c")
                    proof.write_bytes(gzip.compress(nested))
                    updated = c_gate.read(manifest)
                    updated["identifierEvidence"]["main.c"] = c_gate.sha(proof)
                    manifest.write_text(json.dumps(updated), encoding="utf-8")

            with self.subTest(location=location), tempfile.TemporaryDirectory() as temporary:
                gate = self.scan_with_unavailable_evidence(Path(temporary), damage)
                # A decoder may reject the nesting or reach artifact validation.
                self.assertTrue(gate["identifierEvidenceError"].startswith("current: "))

    def test_workflow_runs_same_run_consumer_and_propagates_its_exit(self):
        import yaml
        from autosar_tooling.config import ROOT

        workflow = yaml.safe_load((ROOT / ".github/workflows/checks.yml").read_text())
        steps = workflow["jobs"]["c-analysis-gates"]["steps"]
        step = next(row for row in steps if row.get("name") == "Strict generated source gate")
        self.assertNotIn("if", step)
        with tempfile.TemporaryDirectory() as temporary:
            script = Path(temporary) / "gate.py"
            script.write_text(step["run"], encoding="utf-8")
            for mode in ("pr", "strict"):
                for code in (0, 1, 2):
                    with self.subTest(mode=mode, code=code), patch.dict(os.environ, {
                        "ANALYSIS_TARGET": "linux-x64-controlled-v1",
                        "ANALYSIS_SAMPLE": "standard-ecu", "GATE_MODE": mode,
                    }), patch.object(subprocess, "run", return_value=
                                     subprocess.CompletedProcess([], code)) as run, \
                            self.assertRaises(SystemExit) as exit_:
                        runpy.run_path(str(script))
                    self.assertEqual(exit_.exception.code, code)
                    self.assertEqual(run.call_args.args[0], [
                        sys.executable, "-m", "autosar_tooling.c_gate", "scan",
                        "--target", "linux-x64-controlled-v1", "--sample", "standard-ecu",
                        "--scope-directory", "build/c-analysis-scope", "--mode", mode,
                    ])

    def test_both_scan_sides_share_prepare_and_run_without_cross_dependency(self):
        import yaml
        from autosar_tooling.config import ROOT

        jobs = yaml.safe_load((ROOT / ".github/workflows/checks.yml").read_text())["jobs"]
        for suffix in ("", "-windows"):
            current = jobs["c-analysis-samples" + suffix]
            baseline = jobs["c-analysis-baseline" + suffix]
            prepare = "c-analysis-prepare" + suffix
            self.assertEqual(set(current["needs"]), {prepare, "c-analysis-generate" + suffix})
            self.assertEqual(set(baseline["needs"]),
                             {prepare, "c-analysis-baseline-generate" + suffix})
            self.assertEqual(jobs["c-analysis-generate" + suffix]["needs"], "changes")
            self.assertEqual(jobs["c-analysis-baseline-generate" + suffix]["needs"], "changes")
            self.assertEqual(jobs[prepare]["needs"], "changes")
            platform = "windows-2025-vs2026" if suffix else "ubuntu-24.04"
            target = "windows-x64-controlled-v1" if suffix else "linux-x64-controlled-v1"
            for name in (prepare, "c-analysis-generate" + suffix,
                         "c-analysis-baseline-generate" + suffix):
                self.assertEqual(jobs[name]["strategy"]["matrix"]["include"],
                                 [{"os": platform, "target": target}])
            for name in ("c-analysis-generate" + suffix, "c-analysis-baseline-generate" + suffix):
                self.assertEqual(jobs[name]["env"]["CARGO_TARGET_DIR"],
                                 "${{ github.workspace }}/target")
            self.assertNotIn("Generate complete analysis samples",
                             [step.get("name") for step in jobs[prepare]["steps"]])
            self.assertEqual(current["steps"], baseline["steps"])
            step_ids = [step.get("id") for step in current["steps"]]
            headers = step_ids.index("linux-uapi-headers")
            self.assertEqual(current["steps"][headers]["if"], "runner.os == 'Linux'")
            self.assertLess(headers, step_ids.index("analyze"))
            for job in (current, baseline):
                groups = job["strategy"]["matrix"]["include"]
                self.assertCountEqual([row[key] for row in groups for key in ("first", "second")],
                                      ["standard-ecu", "user-application", "component-pipeline", "no-diagnostic"])
            action = ROOT / ".github/actions/retain-analysis-sample/action.yml"
            uploads = yaml.safe_load(action.read_text())["runs"]["steps"]
            artifact = next(step["with"] for step in uploads
                            if step.get("with", {}).get("name", "").startswith(
                                "c-analysis-raw-"))
            self.assertIn("${{ inputs.index }}", artifact["name"])
            self.assertIn("${{ inputs.side }}", artifact["name"])
            self.assertEqual([path.rsplit("/", 1)[1] for path in artifact["path"].splitlines()],
                             ["summary.json", "project", "analysis-identity.json",
                              "diagnostic-fingerprints.json", "identifier-evidence",
                              "baseline-source.json", "platform-model.h", "platform-library.cfg"])
            self.assertEqual(current["env"]["ANALYSIS_SIDE"], "current")
            self.assertEqual(baseline["env"]["ANALYSIS_SIDE"], "baseline")
            gate = jobs["c-analysis-gates" + suffix]
            self.assertEqual(gate["runs-on"], "ubuntu-24.04" if suffix else "${{ matrix.os }}")
            self.assertFalse(any(step.get("uses") == "./.github/actions/native-analysis-compiler"
                                 for step in gate["steps"]))
            checkout = next(step for step in gate["steps"]
                            if step.get("name") == "Match target checkout bytes")
            self.assertEqual(checkout["if"], "matrix.target == 'windows-x64-controlled-v1'")
            self.assertEqual(checkout["run"],
                             "git read-tree --empty\n"
                             "git -c core.autocrlf=true reset --hard HEAD\n")
            self.assertEqual(set(gate["needs"]), {
                "c-analysis-samples", "c-analysis-baseline",
                "c-analysis-samples-windows", "c-analysis-baseline-windows",
            })
            self.assertTrue(any(step.get("with", {}).get("pattern") == "c-analysis-scope-*"
                                for step in gate["steps"]))
            self.assertFalse(any("'c-check'" in step.get("run", "")
                                 for step in gate["steps"]))

    def test_target_checkout_step_rewrites_existing_files_and_honors_attributes(self):
        import yaml
        from autosar_tooling.config import ROOT

        jobs = yaml.safe_load((ROOT / ".github/workflows/checks.yml").read_text())["jobs"]
        command = next(step["run"] for step in jobs["c-analysis-gates"]["steps"]
                       if step.get("name") == "Match target checkout bytes")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            subprocess.run(["git", "config", "core.autocrlf", "false"], cwd=root, check=True)
            (root / ".gitattributes").write_bytes(b"fixed.c text eol=lf\nnative.h text eol=crlf\n")
            for name in ("parser.py", "record.md", "producer.rs", "fixed.c", "native.h"):
                (root / name).write_bytes(b"content\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True, capture_output=True)
            subprocess.run(["git", "-c", "user.name=CI test", "-c",
                            "user.email=ci@example.invalid", "commit", "--quiet", "-m", "fixture"],
                           cwd=root, check=True)
            for line in command.splitlines():
                subprocess.run(shlex.split(line), cwd=root, check=True,
                               capture_output=True, text=True)
            for name in ("parser.py", "record.md", "producer.rs", "native.h"):
                self.assertEqual((root / name).read_bytes(), b"content\r\n", name)
            self.assertEqual((root / "fixed.c").read_bytes(), b"content\n")

    def test_pairing_refuses_missing_or_duplicate_raw_scans(self):
        import yaml
        from autosar_tooling.config import ROOT

        jobs = yaml.safe_load((ROOT / ".github/workflows/checks.yml").read_text())["jobs"]
        script = next(step["run"] for step in jobs["c-analysis-gates"]["steps"]
                      if step.get("name") == "Require exactly one scan per side")
        original_glob = Path.glob
        for side in ("current", "baseline"):
            for count in (0, 1, 2):
                with self.subTest(side=side, count=count), tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    script_path = root / "pair.py"
                    script_path.write_text(script, encoding="utf-8")
                    for name in ("current", "baseline"):
                        for index in range(count if name == side else 1):
                            path = root / "build/c-analysis-raw" / (
                                f"c-analysis-raw-{name}-linux-x64-controlled-v1-"
                                f"standard-ecu-{index}")
                            path.mkdir(parents=True)
                    with patch.dict(os.environ, {
                        "ANALYSIS_TARGET": "linux-x64-controlled-v1",
                        "ANALYSIS_SAMPLE": "standard-ecu",
                    }), patch("pathlib.Path.glob", autospec=True) as glob, \
                            patch.object(c_gate.shutil, "copytree") as copy:
                        glob.side_effect = lambda path, pattern, root=root: original_glob(
                            root / path, pattern)
                        if count == 1:
                            runpy.run_path(str(script_path))
                            self.assertEqual(copy.call_count, 2)
                        else:
                            with self.assertRaisesRegex(RuntimeError, "Missing or duplicated scan"):
                                runpy.run_path(str(script_path))

    def test_duplicate_matrix_identity_retains_two_receipts_for_aggregate(self):
        import yaml
        from autosar_tooling.config import ROOT

        workflow = yaml.safe_load((ROOT / ".github/workflows/checks.yml").read_text())
        job = workflow["jobs"]["c-analysis-gates"]
        template = next(step["with"]["name"] for step in job["steps"]
                        if step.get("with", {}).get("name", "").startswith(
                            "c-analysis-summaries-"))
        rows = job["strategy"]["matrix"]["include"]
        duplicated = [*rows, rows[0]]
        names = [template.replace("${{ matrix.target }}", row["target"])
                 .replace("${{ matrix.group }}", row["group"])
                 .replace("${{ strategy.job-index }}", str(index))
                 for index, row in enumerate(duplicated)]
        self.assertEqual(len(set(names)), len(duplicated))
        # The duplicate sample identities are then rejected by the real aggregate.
        self.assertEqual(duplicated[0], duplicated[-1])

    def test_baseline_source_requires_exact_parent_and_complete_artifact_provenance(self):
        current = {"kind": "same-run", "tree": "parent"}
        cached = {"kind": "master-artifact", "tree": "parent", "run": 1,
                  "attempt": 1, "job": 3, "artifact": 2, "digest": "sha256:" + "a" * 64}
        for source in (current, cached):
            self.assertEqual(c_gate.baseline_source(source, "parent"), source)
        for field in ("tree", "kind", "run", "attempt", "job", "artifact", "digest"):
            with self.subTest(field=field):
                source = {**cached, field: "invalid"}
                with self.assertRaises(ValueError):
                    c_gate.baseline_source(source, "parent")
                source.pop(field)
                with self.assertRaises((KeyError, ValueError)):
                    c_gate.baseline_source(source, "parent")

    def test_baseline_staging_keeps_parent_production_and_current_sample_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            current, baseline = root / "current", root / "baseline"
            subprocess.run(["git", "init", "--quiet", str(current)], check=True)
            contents = {"core/src/generator.rs": b"old generator\n",
                        "runtime/owned.c": b"old runtime\n",
                        "core/tests/c_analysis.rs": b"old harness\n",
                        "core/resources/templates/input.arxml": b"old config\n",
                        "Cargo.lock": b"old dependencies\n"}
            for name, data in contents.items():
                path = current / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            def commit(directory):
                subprocess.run(["git", "add", "."], cwd=directory, check=True,
                               capture_output=True)
                subprocess.run(["git", "-c", "user.name=CI test", "-c",
                                "user.email=ci@example.invalid", "commit", "--quiet",
                                "-m", "input"],
                               cwd=directory, check=True)
            commit(current)
            subprocess.run(["git", "-c", "core.autocrlf=true", "clone", "--quiet",
                            str(current), str(baseline)], check=True)
            original = {name: (baseline / name).read_bytes() for name in contents}
            self.assertEqual(original["core/src/generator.rs"], b"old generator\r\n")
            for name in contents:
                (current / name).write_bytes(b"current input\n")
            commit(current)
            with patch.object(c_gate, "ROOT", current):
                c_gate.stage(baseline)
                for name in contents:
                    expected = (original[name] if name.startswith(("core/src/", "runtime/"))
                                else b"current input\n")
                    self.assertEqual((baseline / name).read_bytes(), expected, name)
                output = root / "samples"
                for sample in c_gate.SAMPLES:
                    (output / sample).mkdir(parents=True)
                with patch.object(c_gate, "stage") as stage, \
                        patch.object(c_gate.subprocess, "run") as cargo:
                    c_gate.prepare(baseline, output)
                    stage.assert_called_once_with(baseline)
                    self.assertEqual(cargo.call_args.kwargs["cwd"], baseline)
                    environment = cargo.call_args.kwargs["env"]
                    self.assertEqual(environment["CARGO_TARGET_DIR"], str(baseline / "target"))
                    self.assertEqual(environment["AUTOSAR_C_ANALYSIS_SAMPLES"], str(output))

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
                            "outOfScopeNew": [],
                            "sample": sample, "commit": "head", "baseline": "parent",
                            "baselineSource": {"kind": "same-run", "tree": "parent"},
                            "inputIdentity": dict.fromkeys(("current", "baseline"), identity),
                            "summarySha256": {side: c_gate.sha(directory / name) for side, name in
                                              (("current", "summary.json"),
                                               ("baseline", "baseline-summary.json"))}}
                    (directory / "gate.json").write_text(json.dumps(gate))
            paths = list(root.glob("*/*/gate.json"))
            scope = c_gate.c_comparison.scope_matrix([
                c_gate.c_comparison.scope_receipt(c_gate.read(path.with_name("summary.json")),
                                                 c_gate.read(path)["sample"])
                for path in paths])
            for path in paths:
                current = c_gate.read(path.with_name("summary.json"))
                c_gate.c_comparison.annotate_scope(current, set(scope["targetHeaders"]))
                stale = "a" * 64
                current["projectAcceptance"].update(
                    scopeMatrix=scope, complianceScope=current["complianceScope"],
                    fingerprintDispositions={"accepted": [], "stale": [stale]},
                    occurrences=[{"diagnostic": index, "locationIndex": position,
                                  "assessment": "blocked"}
                                 for index, position, _row, _location
                                 in c_gate.c_acceptance._positions(current["diagnostics"])])
                path.with_name("summary.json").write_text(json.dumps(current))
                data = c_gate.read(path)
                data["baselineScopeMatrix"] = scope
                data["inScopeNew"] = []
                inventories = {
                    side: [{"diagnostic": index, "locationIndex": 0, "program": "target",
                            "fingerprint": hashlib.sha256(f"{side}/{index}".encode()).hexdigest()}
                           for index in range(len(current["diagnostics"]))]
                    for side in ("current", "baseline")
                }
                data["diagnosticFingerprints"] = c_gate.fingerprint_comparison(
                    {"current": current, "baseline": c_gate.read(path.with_name("baseline-summary.json"))},
                    inventories, {"current": scope, "baseline": scope}, mode="pr")
                self.assertEqual(data["diagnosticFingerprints"]["status"], "blocked")
                data["summarySha256"]["current"] = c_gate.sha(path.with_name("summary.json"))
                path.write_text(json.dumps(data))
            shutil.rmtree(root / "fixture")
            def revision(*args):
                return "head" if args[-1] == "HEAD" else "parent"
            with patch.object(c_gate, "git", side_effect=revision):
                c_gate.aggregate(root, mode="pr")
                path = next(root.glob("*/*/gate.json"))
                original = path.read_bytes()
                summary_bytes = path.with_name("summary.json").read_bytes()
                for change in ("missing", "duplicate", "run", "status", "identity",
                               "summary", "source", "occurrence"):
                    with self.subTest(change=change):
                        data = json.loads(original)
                        if change == "missing":
                            path.unlink()
                        elif change == "duplicate":
                            extra = root / "extra" / path.parent.name
                            shutil.copytree(path.parent, extra)
                        elif change in ("summary", "occurrence"):
                            path.with_name("summary.json").write_text("{}")
                        elif change == "occurrence":
                            changed = json.loads(summary_bytes)
                            changed["projectAcceptance"]["occurrences"][0]["assessment"] = "out_of_scope"
                            path.with_name("summary.json").write_text(json.dumps(changed))
                            data["summarySha256"]["current"] = c_gate.sha(path.with_name("summary.json"))
                            path.write_text(json.dumps(data))
                        elif change == "source":
                            data.pop("baselineSource")
                            path.write_text(json.dumps(data))
                        else:
                            fields = {"run": "commit", "status": "status",
                                      "identity": "inputIdentity"}
                            key = fields[change]
                            data[key] = "wrong" if change != "identity" else {"current": "missing"}
                            path.write_text(json.dumps(data))
                        with self.assertRaises((ValueError, KeyError)):
                            c_gate.aggregate(root, mode="pr")
                        path.write_bytes(original)
                        if change == "duplicate":
                            shutil.rmtree(root / "extra")
                        if change in ("summary", "occurrence"):
                            path.with_name("summary.json").write_bytes(summary_bytes)
