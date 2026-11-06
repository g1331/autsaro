"""Frozen identity and strict consumption contracts, without claiming compliance."""

from __future__ import annotations

import copy
import hashlib
import json
import os
import re
import runpy
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import chdir
from functools import partial
from pathlib import Path
from unittest.mock import patch

import yaml
from autosar_tooling import c_acceptance, c_check, c_retention, c_sdk, c_standard, cli
from autosar_tooling.config import ROOT

TARGET = "linux-x64-controlled-v1"
CASE = "standard-ecu"


class AcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.root = self.directory / "repository"
        for name in (*c_standard.MANUAL_ASSESSMENT_SOURCES,
                     ".agents/skills/misra-c2012/references/baseline-index.json",
                     *(path.relative_to(ROOT).as_posix() for path in (ROOT / c_retention.RECORD_DIRECTORY).glob("*.md"))):
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, destination)
        inventory = json.loads((ROOT / ".agents/skills/misra-c2012/references/baseline-index.json").read_text())
        self.rules = [{"id": row["id"], "category": row["category"],
                        "automaticChecker": row["id"] not in c_standard.CHECKER_UNCOVERED, "assessment": "not_assessed"}
                      for row in inventory["guideline_metadata"]]
        addons = self.directory / "addons"
        addons.mkdir()
        available = [row["id"][1:] for row in self.rules if row["automaticChecker"]]
        (addons / "misra.py").write_text("def getAddonRules(): return " + repr(available)
                                        + "\ndef getCppcheckRules(): return []\n")
        with patch.object(c_standard, "ROOT", self.root):
            coverage = c_check.coverage(addons)
        self.project = self.directory / "analyzed"
        entries = {"host-batch": "host.c", "probe": "probe.c"}
        self.target = {"target": TARGET, "entries": entries,
                       "kernelSha256": {"kernel.h": hashlib.sha256(b"kernel\n").hexdigest()}}
        self.files = {"main.c": b"int main(void) { return 0; }\n", "kernel/kernel.h": b"kernel\n",
                      "host.c": b"host entry\n", "probe.c": b"probe entry\n",
                      "app/User.c": b"application snapshot\n"}
        producer = self.root / "runtime/producer.c"
        producer.parent.mkdir(parents=True)
        producer.write_bytes(b"producer\n")
        self.plan = {"sources": [{"logicalPath": "application.arxml", "rawSha256": "1" * 64}],
                     "runtimeSources": {"runtime/producer.c": c_acceptance.digest(producer)}}
        self.seal()
        self.summary = {"error": None, "passed": False,
                        "tool": {"version": "2.21.0", "expectedSourceRevision": c_check.CPPCHECK_REVISION,
                                 "addons": c_check.ADDON_HASHES},
                        "sourceSealSha256": c_acceptance.digest(self.project / "meta/files.sha256"),
                        "coverage": coverage, "diagnostics": [{"id": "unknown", "locations": [], "programs": ["host-batch"]}],
                        "sourceClassification": {"complete": True},
                        "programs": [{"name": name, "entry": entry, "translationUnits": ["main.c", entry],
                                      "passed": False, "exitCode": 1} for name, entry in entries.items()],
                        "semanticIdentity": {"version": 2, "sdk": "1" * 64, "compiler": {"release": "12"},
                                             "defines": {"main.c": []}, "libraries": {"std": "2" * 64},
                                             "analysisCode": {"c_check.py": "3" * 64}, "addonRuntime": {"runaddon.py": "4" * 64}},
                        "scope": {"target": TARGET, "entries": entries, "compilerFlags": ["-std=c99"],
                                  "includePaths": ["."], "translationUnits": ["main.c", "host.c", "probe.c"], "supplemental": [], "patchedKernel": True}}
        # The contract fixture replaces environment probes, not re-evaluation or
        # source identity. It is not an actual SDK/toolchain acceptance result.
        self.actual_tool = copy.deepcopy(self.summary["tool"])
        self.actual_semantic = copy.deepcopy(self.summary["semanticIdentity"])
        tool_probe = patch.object(c_check, "tool_identity", return_value=(self.directory / "cppcheck", addons, self.actual_tool))
        self.tool_probe = tool_probe.start()
        self.addCleanup(tool_probe.stop)
        self.real_semantic = c_check.semantic_identity
        semantic_probe = patch.object(c_check, "semantic_identity", side_effect=lambda *args: copy.deepcopy(self.actual_semantic))
        self.semantic_probe = semantic_probe.start()
        self.addCleanup(semantic_probe.stop)
        host = patch.object(c_check.platform, "system", return_value="Linux")
        host.start()
        self.addCleanup(host.stop)

    def seal(self):
        self.files["target.json"] = json.dumps(self.target).encode()
        self.files["meta/integration.json"] = json.dumps({"plan": self.plan, "originalBswSources": {}}).encode()
        for name, data in self.files.items():
            destination = self.project / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
        listing = "\n".join(sorted(self.files)) + "\n"
        (self.project / "meta/files.list").write_text(listing, encoding="utf-8", newline="\n")
        hashes = {name: hashlib.sha256(data).hexdigest() for name, data in self.files.items()}
        hashes["meta/files.list"] = hashlib.sha256(listing.encode()).hexdigest()
        (self.project / "meta/files.sha256").write_text("".join(f"{value}  {name}\n" for name, value in sorted(hashes.items())),
                                                       encoding="utf-8", newline="\n")

    def identity(self):
        return c_acceptance.input_identity(self.summary, self.project, root=self.root)

    def context(self, *, identity=None, status="pending", group="generated-retention", case=CASE):
        return {"id": "frozen-" + group, "evidence": {"head": "a" * 40, "receipt": "Synthetic contract receipt"},
                "cases": [{"caseId": case, "target": TARGET, "inputIdentity": identity or self.identity()}],
                "records": {"kind": group, **({"cohortSha256": c_retention.COHORT_SHA256} if group == "generated-retention" else {})},
                "formalStatus": status}

    def bind(self, context):
        path = self.root / c_standard.MANUAL_ASSESSMENT_SOURCES[0]
        with path.open("a", encoding="utf-8", newline="\n") as destination:
            destination.write(c_acceptance.BEGIN + "\n" + json.dumps(context) + "\n" + c_acceptance.END + "\n")
        self.summary["coverage"]["manualAssessment"] = c_standard.manual_assessment(self.rules, self.root)
        return path

    def evaluate(self):
        return c_acceptance.evaluate(self.summary, self.project, root=self.root)

    def bind_groups(self, *, groups=c_acceptance.GROUPS, status="approved", cases=None, statuses=None, identities=None, identity=None):
        for path in (self.root / "docs/standards").rglob("*.md"):
            text = path.read_text(encoding="utf-8")
            path.write_text(re.sub(re.escape(c_acceptance.BEGIN) + r"\n.*?" + re.escape(c_acceptance.END) + r"\n?", "", text, flags=re.DOTALL),
                            encoding="utf-8", newline="\n")
        identity = identity or self.identity()
        for group in groups:
            self.bind(self.context(identity=(identities or {}).get(group, identity), group=group,
                                   status=(statuses or {}).get(group, status), case=(cases or {}).get(group, CASE)))

    def synthetic_complete_inputs(self):
        """Complete contract inputs in a temporary repository, never real approvals."""
        for name in c_standard.MANUAL_ASSESSMENT_SOURCES:
            path = self.root / name
            path.write_text(path.read_text(encoding="utf-8").replace("| retained |", "| satisfied |"), encoding="utf-8")
        self.supplementary_path = self.root / "docs/standards/bsw-general/synthetic-checkers.md"
        categories = {row["id"]: row["category"] for row in self.rules}
        lines = [f"| {row['guideline']} | {categories[row['guideline']]} | Synthetic frozen scope | satisfied | Synthetic assessment |"
                 for row in self.summary["coverage"]["knownCheckerLimitations"]]
        self.supplementary_path.write_text("<!-- misra-checker-assessment-begin -->\n" + "\n".join(lines)
                                           + "\n<!-- misra-checker-assessment-end -->\n", encoding="utf-8")
        self.retained = next(row for row in c_retention.records() if row["rule"] == "2.5" and "S" in row["profiles"].split(","))
        self.files[self.retained["delivered"]] = ("/* MR12 RULE 2.5 VIOLATION: retain synthetic contract fixture. */\n"
                                                 + self.retained["shape"] + "\n").encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.summary["diagnostics"] = [{"id": "misra-c2012-2.5", "severity": "style", "message": "synthetic retained macro",
                                        "programs": ["host-batch", "probe"],
                                        "locations": [{"file": self.retained["delivered"], "line": 2, "column": 0}]}]
        self.bind_groups()

    def test_identity_binds_application_independently_of_arxml_and_plan(self):
        before = self.identity()
        plan = copy.deepcopy(self.plan)
        self.files["app/User.c"] += b"user change\n"
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.assertEqual(self.plan, plan)
        self.assertNotEqual(self.identity(), before)

    def test_identity_binds_configuration_producer_sdk_and_program_closure(self):
        before = self.identity()
        for key in ("semanticIdentity", "programs", "scope"):
            with self.subTest(key=key):
                saved = copy.deepcopy(self.summary[key])
                if key == "semanticIdentity":
                    self.summary[key]["sdk"] = "b" * 64
                elif key == "programs":
                    self.summary[key][0]["translationUnits"].append("other.c")
                else:
                    self.summary[key]["compilerFlags"].append("-DNDEBUG")
                self.assertNotEqual(before, self.identity())
                self.summary[key] = saved
        self.plan["configuration"] = {"PduLengthType": "UINT8"}
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.assertNotEqual(before, self.identity())
        configured = self.identity()
        (self.root / "runtime/producer.c").write_bytes(b"different producer\n")
        self.assertNotEqual(configured, self.identity())
        self.assertIn("producer_provenance_unverified", [row["reason"] for row in self.evaluate()["objects"]])

    def test_context_does_not_self_reference_or_approve_project_metadata(self):
        before = self.identity()
        self.bind(self.context())
        self.assertEqual(before, self.identity())
        result = self.evaluate()
        self.assertEqual((result["caseId"], result["status"]), (CASE, "blocked"))
        self.assertIn("formal_deviation_pending", [row["reason"] for row in result["objects"]])
        self.assertEqual(sum(row["reason"] == "checker-limitation_not_assessed" for row in result["objects"]), 11)
        self.assertTrue(any(row["kind"] == "manual" and row["id"] == "D4.10"
                            and row["reason"] == "input_context_unbound_or_stale" for row in result["objects"]))
        self.assertEqual(len(self.summary["coverage"]["rules"]), 221)
        self.assertEqual(self.summary["coverage"]["manualAssessment"]["checkerUncovered"]["total"], 66)

    def test_retention_records_are_byte_bound_without_binding_self_reference(self):
        context = self.context()
        path = self.root / c_retention.RECORD_DIRECTORY / "README.md"
        before = self.identity()
        with path.open("a", encoding="utf-8", newline="\n") as destination:
            destination.write(c_acceptance.BEGIN + "\n" + json.dumps(context) + "\n" + c_acceptance.END + "\n")
        self.assertEqual(before, self.identity())
        with path.open("a", encoding="utf-8", newline="\n") as destination:
            destination.write("Changed assessment evidence.\n")
        self.assertNotEqual(before, self.identity())
        self.assertIn("input_context_unbound_or_stale", [row["reason"] for row in self.evaluate()["objects"]])

    def test_unknown_stale_and_pending_contexts_preserve_raw_summary(self):
        for identity in (None, "f" * 64):
            with self.subTest(identity=identity):
                if identity:
                    self.bind(self.context(identity=identity))
                before = copy.deepcopy(self.summary)
                result = self.evaluate()
                self.assertEqual(result["status"], "blocked")
                self.assertIsNone(result["caseId"])
                self.assertEqual(self.summary, before)
                self.assertIn("input_context_unbound_or_stale", [row["reason"] for row in result["objects"]])

    def test_unknown_and_sdk_findings_keep_each_occurrence_visible(self):
        self.bind(self.context(status="approved"))
        self.summary["diagnostics"] = [{"id": "checkersReport", "locations": [], "programs": ["host-batch"]},
                                       {"id": "misra-c2012-7.2", "sourceCategory": "sdk", "programs": ["host-batch"],
                                        "locations": [{"file": "main.c", "line": 1}, {"file": "main.c", "line": 2}]}]
        result = self.evaluate()
        findings = [row for row in result["objects"] if row["kind"] == "diagnostic"]
        self.assertEqual(len(findings), 2)
        self.assertEqual({row["reason"] for row in findings}, {"sdk_integration_assessment_missing"})
        self.assertEqual([row["location"]["line"] for row in findings], [1, 2])

    def test_missing_manual_assessment_is_not_filled_by_a_frozen_context(self):
        name = c_standard.MANUAL_ASSESSMENT_SOURCES[0]
        path = self.root / name
        text = path.read_text(encoding="utf-8")
        text, count = re.subn(r"(?m)^\| D1\.1 .*\n", "", text)
        self.assertEqual(count, 1)
        path.write_text(text, encoding="utf-8")
        self.bind(self.context(status="approved"))
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        self.assertIn({"kind": "manual", "id": "D1.1", "reason": "manual_not_assessed", "source": None}, result["objects"])

    def test_malformed_and_duplicate_bindings_are_errors(self):
        context = self.context()
        path = self.bind(context)
        with path.open("a", encoding="utf-8") as destination:
            destination.write(c_acceptance.BEGIN + "\n" + json.dumps(context) + "\n" + c_acceptance.END + "\n")
        self.assertEqual(self.evaluate()["status"], "error")
        for data in (c_acceptance.BEGIN + "\n{}", c_acceptance.END, c_acceptance.BEGIN + "\n{" + "\n" + c_acceptance.END):
            with self.subTest(data=data):
                path.write_text(data)
                self.assertEqual(self.evaluate()["status"], "error")

    def test_same_group_duplicate_input_is_rejected_even_with_distinct_context_ids(self):
        context = self.context()
        self.bind(context)
        context["id"] = "different-context-id"
        self.bind(context)
        result = self.evaluate()
        self.assertEqual(result["status"], "error")
        self.assertIn("Duplicate assessment input binding", result["objects"][-1]["reason"])

    def test_parse_failure_partial_catalog_and_corrupt_final_kernel_are_errors(self):
        for change in ("raw-error", "program-error", "partial-catalog", "kernel"):
            with self.subTest(change=change):
                saved = copy.deepcopy(self.summary)
                if change == "raw-error":
                    self.summary["error"] = "parse failed"
                elif change == "program-error":
                    self.summary["programs"][0]["exitCode"] = 2
                elif change == "partial-catalog":
                    self.summary["coverage"]["rules"].pop()
                else:
                    (self.project / "kernel/kernel.h").write_bytes(b"changed final kernel")
                self.assertEqual(self.evaluate()["status"], "error")
                self.summary = saved

    def receipt(self):
        self.synthetic_complete_inputs()
        summary = copy.deepcopy(self.summary)
        summary["projectAcceptance"] = self.evaluate()
        return summary

    def test_strict_consumer_is_independent_of_raw_failed_and_exit_one(self):
        summary = self.receipt()
        self.assertEqual(summary["projectAcceptance"]["status"], "accepted", summary["projectAcceptance"]["objects"])
        self.assertEqual(len(summary["projectAcceptance"]["contexts"]), 3)
        self.assertEqual(len(summary["coverage"]["rules"]), 221)
        self.assertEqual(summary["coverage"]["manualAssessment"]["checkerUncovered"]["total"], 66)
        supplementary = c_standard.supplementary_assessment(self.rules, summary["coverage"]["knownCheckerLimitations"], self.root)
        self.assertEqual(len(supplementary["records"]), 11)
        self.assertEqual(summary["projectAcceptance"]["occurrences"][0]["entity"], self.retained["name"])
        self.require(summary)
        self.assertFalse(summary["passed"])
        self.assertEqual(summary["programs"][0]["exitCode"], 1)

    def test_strict_replay_recomputes_actual_identity_instead_of_trusting_the_verdict(self):
        accepted = self.receipt()
        for field in ("sdk", "seal", "flags"):
            changed = copy.deepcopy(accepted)
            if field == "sdk":
                changed["semanticIdentity"]["sdk"] = "b" * 64
            elif field == "seal":
                changed["sourceSealSha256"] = "c" * 64
            else:
                changed["scope"]["compilerFlags"].append("-DUNASSESSED")
            with self.subTest(field=field):
                fresh = c_acceptance.evaluate(changed, self.project, root=self.root)
                self.assertNotEqual(fresh["status"], "accepted")
                with self.assertRaises(ValueError):
                    self.require(changed)
        producer = self.root / "runtime/producer.c"
        producer.write_bytes(b"changed actual producer\n")
        self.assertEqual(self.evaluate()["status"], "blocked")
        with self.assertRaisesRegex(ValueError, "actual analysis project and inputs"):
            self.require(accepted)

    def test_strict_replay_requires_the_actual_complete_project(self):
        accepted = self.receipt()
        with self.assertRaises(ValueError):
            c_acceptance.require_accepted([(accepted, self.directory / "missing-project")], [(TARGET, CASE)],
                                         {"prepare": "success", "analysis": "success"}, root=self.root)
        path = self.project / "app/User.c"
        path.write_bytes(b"changed without updating the seal\n")
        with self.assertRaisesRegex(ValueError, "actual analysis project and inputs"):
            self.require(accepted)
        path.unlink()
        with self.assertRaises(ValueError):
            self.require(accepted)

    def test_strict_replay_compares_the_entire_fresh_receipt(self):
        accepted = self.receipt()
        accepted["projectAcceptance"]["unrecordedResultField"] = "not produced by evaluation"
        with self.assertRaisesRegex(ValueError, "actual analysis project and inputs"):
            self.require(accepted)

    def test_strict_replay_rechecks_environment_semantics_and_probes_tool_once(self):
        accepted = self.receipt()
        for field in ("sdk", "analysisCode", "libraries", "addonRuntime"):
            saved = copy.deepcopy(self.actual_semantic)
            self.actual_semantic[field] = "changed actual environment"
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "Actual semantic inputs changed"):
                self.require(accepted)
            self.actual_semantic = saved
        self.tool_probe.reset_mock()
        with self.assertRaises(ValueError):
            c_acceptance.require_accepted([(accepted, self.project), (accepted, self.project)], [(TARGET, CASE)],
                                         {"prepare": "success", "analysis": "success"}, root=self.root)
        self.assertEqual(self.tool_probe.call_count, 1)
        self.assertEqual(self.semantic_probe.call_args.args[2], (self.project.parent.resolve() / "host-batch", self.project.parent.resolve() / "probe"))
        with patch.object(c_check.platform, "system", return_value="Windows"), self.assertRaisesRegex(ValueError, "analysis target"):
            self.require(accepted)

    def test_fixed_checker_mapping_rejects_the_65_manual_obligation_attack(self):
        self.receipt()
        path = self.root / c_standard.MANUAL_ASSESSMENT_SOURCES[0]
        text, count = re.subn(r"(?m)^\| D1\.1 .*\n", "", path.read_text(encoding="utf-8"))
        self.assertEqual(count, 1)
        path.write_text(text, encoding="utf-8")
        next(row for row in self.rules if row["id"] == "D1.1")["automaticChecker"] = True
        self.summary["coverage"]["rules"] = copy.deepcopy(self.rules)
        validate_catalog = c_acceptance._catalog

        def old_mapping(summary, root):
            checked = copy.deepcopy(summary)
            next(row for row in checked["coverage"]["rules"] if row["id"] == "D1.1")["automaticChecker"] = False
            validate_catalog(checked, root)
            return summary["coverage"]["rules"]

        # Reproduce the old attack with three freshly bound contexts and a real
        # evaluated accepted result, then restore the independent mapping guard.
        with patch.object(c_acceptance, "_catalog", side_effect=old_mapping):
            self.bind_groups()
            forged = copy.deepcopy(self.summary)
            forged["projectAcceptance"] = self.evaluate()
            self.assertEqual(forged["projectAcceptance"]["status"], "accepted")
        self.assertEqual(self.summary["coverage"]["manualAssessment"]["checkerUncovered"]["total"], 65)
        current = self.evaluate()
        self.assertEqual(current["status"], "error")
        self.assertIn("66 uncovered guidelines", current["objects"][-1]["reason"])
        with self.assertRaisesRegex(ValueError, "66 uncovered guidelines"):
            self.require(forged)

    def test_fixed_guideline_categories_cannot_be_redefined_by_input_metadata(self):
        self.synthetic_complete_inputs()
        path = self.root / ".agents/skills/misra-c2012/references/baseline-index.json"
        catalog = json.loads(path.read_text(encoding="utf-8"))
        next(row for row in catalog["guideline_metadata"] if row["id"] == "R2.5")["category"] = "Mandatory"
        path.write_text(json.dumps(catalog), encoding="utf-8")
        next(row for row in self.summary["coverage"]["rules"] if row["id"] == "R2.5")["category"] = "Mandatory"
        result = self.evaluate()
        self.assertEqual(result["status"], "error")
        self.assertIn("fixed guideline categories", result["objects"][-1]["reason"])

    def test_strict_replay_uses_real_semantic_sdk_provider_and_driver_byte_checks(self):
        self.synthetic_complete_inputs()
        tool_directory = self.directory / "semantic-tool"
        (tool_directory / "cfg").mkdir(parents=True)
        executable = tool_directory / "cppcheck"
        executable.write_bytes(b"fixture tool carrier")
        for name in ("std.cfg", "posix.cfg"):
            (tool_directory / "cfg" / name).write_bytes(b"fixture library\n")
        for name in ("runaddon.py", "cppcheck.py", "cppcheckdata.py"):
            (tool_directory / name).write_bytes(b"fixture addon runtime\n")
        drivers = self.directory / "analysis-drivers"
        drivers.mkdir()
        for name in ("c_check.py", "c_addon.py", "c_comparison.py"):
            (drivers / name).write_bytes(b"fixture analysis driver\n")
        header = (self.directory / "provider/sdk.h").resolve()
        header.parent.mkdir()
        header.write_bytes(b"#define SDK_VALUE 1U\n")
        package = {"provider": "libc6-dev", "version": "fixture", "header": "usr/include/sdk.h",
                   "algorithm": "sha256", "expected": c_acceptance.digest(header)}
        packages = {str(header): package}
        record = c_sdk.header_identity(header, self.project, TARGET, "x86_64-linux-gnu", packages)
        self.assertEqual(record["category"], "sdk")
        scope = self.summary["scope"]
        scope.update(compilerSemanticInputs={"machine": "x86_64-linux-gnu", "release": "12"},
                     compilerDiscoveredDefines={"main.c": []}, platformModel=None, platformLibrary=None,
                      sdkHeaders=[record], sdkTranslationUnits={"main.c": [record["path"]]}, sdkMacroInputs={})
        self.assertEqual({path for paths in scope["sdkTranslationUnits"].values() for path in paths},
                         {row["path"] for row in scope["sdkHeaders"]})
        self.tool_probe.return_value = (executable, tool_directory, self.actual_tool)
        directories = (self.project.parent / "host-batch", self.project.parent / "probe")
        # Only tool/package discovery is a fixture here. Both semantic consumers
        # execute their real byte/provider checks, without claiming an actual SDK.
        with patch.object(c_check, "semantic_identity", self.real_semantic), \
                patch.object(c_check, "__file__", str(drivers / "c_check.py")), \
                patch.object(c_sdk, "debian_headers", return_value=packages):
            self.summary["semanticIdentity"] = c_check.semantic_identity(executable, copy.deepcopy(scope), directories)
            self.bind_groups()
            accepted = copy.deepcopy(self.summary)
            accepted["projectAcceptance"] = self.evaluate()
            self.assertEqual(accepted["projectAcceptance"]["status"], "accepted")
            self.require(accepted)
            for path in (header, drivers / "c_check.py", tool_directory / "cfg/std.cfg", tool_directory / "runaddon.py"):
                before = path.read_bytes()
                path.write_bytes(before + b"changed actual input\n")
                with self.subTest(path=path.name), self.assertRaises(ValueError):
                    self.require(accepted)
                path.write_bytes(before)
            package["version"] = "changed provider identity"
            with self.assertRaisesRegex(ValueError, "SDK source/provider identity changed"):
                self.require(accepted)

    def test_workflow_strict_step_runs_the_real_cli_and_consumer(self):
        summary = self.receipt()
        workflow = yaml.safe_load((ROOT / ".github/workflows/checks.yml").read_text())
        steps = {row["name"]: row for row in workflow["jobs"]["c-analysis-samples"]["steps"] if "name" in row}
        script = self.directory / "strict.py"
        script.write_text(steps["Strict generated source gate"]["run"], encoding="utf-8")
        output = self.directory / "build/c-analysis-results/renamed-output/summary.json"
        output.parent.mkdir(parents=True)
        shutil.copytree(self.project, output.parent / "project")
        consume = c_acceptance.require_accepted

        def command(argv):
            self.assertEqual(argv[:7], ["uv", "run", "--locked", "python", "-m", "autosar_tooling", "c-acceptance"])
            with patch.object(sys, "argv", ["autosar_tooling", *argv[6:]]), \
                    patch.object(c_acceptance, "require_accepted", partial(consume, root=self.root)):
                return subprocess.CompletedProcess(argv, cli.main())

        for change in ("valid", "blocked", "unknown", "duplicate", "missing", "cancelled", "skipped"):
            current = copy.deepcopy(summary)
            if change == "blocked":
                current["projectAcceptance"]["status"] = "blocked"
            elif change == "unknown":
                current["diagnostics"][0]["id"] = "unknown"
            output.write_text(json.dumps(current), encoding="utf-8")
            duplicate = output.parent.parent / "duplicate/summary.json"
            if change == "duplicate":
                duplicate.parent.mkdir()
                duplicate.write_bytes(output.read_bytes())
            if change == "missing":
                output.unlink()
            env = {"ANALYSIS_TARGET": TARGET, "ANALYSIS_SAMPLES": CASE, "PREPARE_RESULT": "success",
                   "ANALYSIS_RESULT": change if change in {"cancelled", "skipped"} else "success"}
            with self.subTest(change=change), chdir(self.directory), patch.dict(os.environ, env), \
                    patch.object(subprocess, "run", side_effect=command), self.assertRaises(SystemExit) as exit_:
                runpy.run_path(str(script))
            self.assertEqual(exit_.exception.code == 0, change == "valid")
            if duplicate.exists():
                duplicate.unlink()

    def test_every_group_requires_its_own_current_approved_scope(self):
        self.synthetic_complete_inputs()
        for path, identifier in ((self.root / c_standard.MANUAL_ASSESSMENT_SOURCES[0], "D4.10"),
                                 (self.supplementary_path, "R5.1")):
            text = path.read_text(encoding="utf-8")
            row = next(line for line in text.splitlines() if line.startswith("| " + identifier + " |"))
            path.write_text(text.replace(row, row.replace("| satisfied |", "| retained |")), encoding="utf-8")
        for group in c_acceptance.GROUPS:
            for change in ("missing", "stale", "pending"):
                with self.subTest(group=group, change=change):
                    options = {}
                    if change == "missing":
                        options["groups"] = tuple(value for value in c_acceptance.GROUPS if value != group)
                    elif change == "stale":
                        options["identities"] = {group: "f" * 64}
                    else:
                        options["statuses"] = {group: "pending"}
                    self.bind_groups(**options)
                    result = self.evaluate()
                    self.assertEqual(result["status"], "blocked")
                    self.assertTrue(any(row["kind"] == "context" and row["id"] == group for row in result["objects"]))
        self.bind_groups(cases={"manual-assessment": "user-application"})
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        self.assertIn("assessment_case_mismatch", [row["reason"] for row in result["objects"]])

    def test_manual_and_supplementary_statuses_are_independently_required(self):
        self.synthetic_complete_inputs()
        manual_path = self.root / c_standard.MANUAL_ASSESSMENT_SOURCES[0]
        for path, identifier, kind in ((manual_path, "D1.1", "manual"),
                                       (manual_path, "D4.2", "manual"),
                                       (self.supplementary_path, "R5.1", "checker-limitation")):
            text = path.read_text(encoding="utf-8")
            row = next(line for line in text.splitlines() if line.startswith("| " + identifier + " |"))
            cells = row.split("|")
            for status in ("satisfied", "not_applicable", "retained", "not_assessed", "missing"):
                with self.subTest(identifier=identifier, kind=kind, status=status):
                    changed = list(cells)
                    changed[4] = " " + status + " "
                    path.write_text(text.replace(row, "" if status == "missing" else "|".join(changed)), encoding="utf-8")
                    self.bind_groups()
                    result = self.evaluate()
                    self.assertEqual(result["status"], "accepted" if status in {"satisfied", "not_applicable", "retained"} else "blocked")
                    if result["status"] == "accepted":
                        before = copy.deepcopy(self.summary)
                        self.require({**self.summary, "projectAcceptance": result})
                        self.assertEqual(self.summary, before)
                    else:
                        self.assertTrue(any(record["id"] == identifier and record["kind"] == kind for record in result["objects"]))
            path.write_text(text, encoding="utf-8")
        self.bind_groups()

    def test_each_of_the_eleven_supplementary_records_is_required(self):
        self.synthetic_complete_inputs()
        text = self.supplementary_path.read_text(encoding="utf-8")
        for limitation in self.summary["coverage"]["knownCheckerLimitations"]:
            identifier = limitation["guideline"]
            line = next(row for row in text.splitlines() if row.startswith("| " + identifier + " |"))
            with self.subTest(guideline=identifier):
                self.supplementary_path.write_text(text.replace(line, ""), encoding="utf-8")
                self.bind_groups()
                result = self.evaluate()
                self.assertEqual(result["status"], "blocked")
                self.assertTrue(any(row["id"] == identifier and row["reason"] == "checker-limitation_not_assessed" for row in result["objects"]))

    def test_supplementary_source_selection_and_bytes_are_in_the_frozen_identity(self):
        self.synthetic_complete_inputs()
        before = self.identity()
        text = self.supplementary_path.read_text(encoding="utf-8")
        self.supplementary_path.write_text(text + "Changed assessment evidence.\n", encoding="utf-8")
        self.assertNotEqual(before, self.identity())
        self.assertEqual(self.evaluate()["status"], "blocked")
        self.supplementary_path.write_text("No block.\n", encoding="utf-8")
        removed = self.identity()
        self.assertNotEqual(before, removed)
        self.supplementary_path.unlink()
        self.assertEqual(removed, self.identity())

    def test_strict_consumer_rejects_unknown_diagnostics_even_with_old_accepted_verdict(self):
        accepted = self.receipt()
        self.assertEqual(accepted["projectAcceptance"]["status"], "accepted")
        for mutation in ("append", "replace", "delete", "location", "message", "forged-unknown-key"):
            changed = copy.deepcopy(accepted)
            if mutation == "append":
                changed["diagnostics"].append({"id": "unknown", "locations": [], "programs": ["host-batch"]})
            elif mutation == "replace":
                changed["diagnostics"][0]["id"] = "unknown"
            elif mutation == "delete":
                changed["diagnostics"].clear()
            elif mutation == "location":
                changed["diagnostics"][0]["locations"].append({"file": self.retained["delivered"], "line": 3})
            elif mutation == "message":
                changed["diagnostics"][0]["message"] = "different diagnostic"
            else:
                changed["diagnostics"][0]["id"] = "unknown"
                occurrence = changed["projectAcceptance"]["occurrences"][0]
                occurrence["rule"] = "unknown"
                occurrence["diagnosticIdentity"] = c_acceptance._hash(changed["diagnostics"][0])
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.require(changed)

    def require(self, summary):
        c_acceptance.require_accepted([(summary, self.project)], [(TARGET, CASE)], {"prepare": "success", "analysis": "success"}, root=self.root)

    def test_strict_consumer_rejects_missing_duplicate_and_corrupt_receipts(self):
        accepted = self.receipt()
        for receipts in ([], accepted["projectAcceptance"]["occurrences"] * 2):
            changed = copy.deepcopy(accepted)
            changed["projectAcceptance"]["occurrences"] = receipts
            with self.subTest(receipts=len(receipts)), self.assertRaises(ValueError):
                self.require(changed)
        for field, value in (("entity", "UnknownEntity"), ("record", {"file": "unknown.md", "line": 1}),
                             ("producer", "unassessed producer"), ("source", "unknown.c"),
                             ("recordedProfiles", "N"), ("decisionSource", "unknown"),
                             ("profile", "N"), ("programs", ["probe"]), ("locationIndex", 1),
                             ("diagnostic", 99), ("diagnostic", False), ("locationIndex", False),
                             ("contextId", "frozen-manual-assessment"),
                             ("assessment", "blocked"), ("status", "unmatched"), ("reason", "unknown"),
                             ("evidenceClosure", "not_established"),
                             ("rule", "misra-c2012-99.1"), ("diagnosticIdentity", "0" * 64)):
            changed = copy.deepcopy(accepted)
            changed["projectAcceptance"]["occurrences"][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.require(changed)

    def test_receipts_cover_each_original_location_and_omit_only_checkers_report(self):
        self.synthetic_complete_inputs()
        self.summary["diagnostics"][0]["locations"] *= 2
        self.summary["diagnostics"].insert(0, {"id": "checkersReport", "locations": [], "programs": ["host-batch", "probe"]})
        summary = copy.deepcopy(self.summary)
        summary["projectAcceptance"] = self.evaluate()
        self.assertEqual(summary["projectAcceptance"]["status"], "accepted")
        receipts = summary["projectAcceptance"]["occurrences"]
        self.assertEqual([(row["diagnostic"], row["locationIndex"]) for row in receipts], [(1, 0), (1, 1)])
        self.require(summary)
        receipts[1]["locationIndex"] = 0
        with self.assertRaises(ValueError):
            self.require(summary)

    def test_strict_consumer_rejects_missing_duplicate_wrong_and_nonaccepted_results(self):
        accepted = self.receipt()
        for rows in ([], [accepted, accepted], [{}]):
            with self.subTest(rows=len(rows)), self.assertRaises(ValueError):
                c_acceptance.require_accepted([(row, self.project) for row in rows], [(TARGET, CASE)], {"prepare": "success", "analysis": "success"}, root=self.root)
        for field, value in (("status", "blocked"), ("status", "error"), ("status", "skipped"),
                             ("caseId", "unknown"), ("target", "windows-x64-controlled-v1"),
                             ("inputIdentity", "invalid"), ("objects", [{"reason": "unclosed"}])):
            summary = copy.deepcopy(accepted)
            summary["projectAcceptance"][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                self.require(summary)
        for status in ("failure", "skipped", "cancelled", "", "pending"):
            with self.subTest(status=status), self.assertRaises(ValueError):
                c_acceptance.require_accepted([(accepted, self.project)], [(TARGET, CASE)], {"prepare": "success", "analysis": status}, root=self.root)
        with self.assertRaises(ValueError):
            c_acceptance.require_accepted([(accepted, self.project)], [(TARGET, CASE)], {"prepare": "success"}, root=self.root)

    def test_strict_consumer_rechecks_the_recorded_context_not_just_the_verdict(self):
        accepted = self.receipt()
        context_path = self.root / accepted["projectAcceptance"]["contexts"][0]["source"]
        saved = context_path.read_bytes().decode("utf-8")
        for text, reason in ((saved.replace('"approved"', '"pending"'), "formal_deviation_pending"),
                             (saved.replace(accepted["projectAcceptance"]["inputIdentity"], "f" * 64), "input_context_unbound_or_stale"),
                             (saved.split(c_acceptance.BEGIN)[0], "input_context_unbound_or_stale")):
            context_path.write_bytes(text.encode("utf-8"))
            with self.subTest(text=text[-80:]), self.assertRaisesRegex(ValueError, reason):
                self.require(accepted)

    def test_strict_consumer_requires_manual_group_even_when_retention_is_approved(self):
        accepted = self.receipt()
        accepted["projectAcceptance"]["contexts"] = [row for row in accepted["projectAcceptance"]["contexts"]
                                                     if row["records"]["kind"] != "manual-assessment"]
        with self.assertRaisesRegex(ValueError, "receipt no longer matches"):
            self.require(accepted)

    def test_strict_consumer_rejects_pending_manual_group_with_current_raw_manual_identity(self):
        accepted = self.receipt()
        reference = next(row for row in accepted["projectAcceptance"]["contexts"] if row["records"]["kind"] == "manual-assessment")
        path = self.root / reference["source"]
        text = path.read_bytes().decode("utf-8")
        blocks = c_acceptance._blocks(text)
        for block in blocks:
            context = json.loads(block)
            if context["id"] == reference["id"]:
                context["formalStatus"] = "pending"
                text = text.replace(block, json.dumps(context) + "\n")
        path.write_bytes(text.encode("utf-8"))
        accepted["coverage"]["manualAssessment"] = c_standard.manual_assessment(self.rules, self.root)
        with self.assertRaisesRegex(ValueError, "formal_assessment_pending"):
            self.require(accepted)

    def test_pending_context_edits_preserve_explicit_lf_and_crlf_assessment_bytes(self):
        identities = []
        for newline in (b"\n", b"\r\n"):
            with self.subTest(newline=newline):
                self.synthetic_complete_inputs()
                path = self.root / c_standard.MANUAL_ASSESSMENT_SOURCES[0]
                body = path.read_bytes().split(c_acceptance.BEGIN.encode())[0]
                body = body.replace(b"\r\n", b"\n").replace(b"\n", newline)
                path.write_bytes(body)
                identity = self.identity()
                identities.append(identity)
                for group in c_acceptance.GROUPS:
                    self.bind(self.context(identity=identity, status="approved", group=group))
                accepted = copy.deepcopy(self.summary)
                accepted["projectAcceptance"] = self.evaluate()
                self.assertEqual(accepted["projectAcceptance"]["status"], "accepted")
                self.require(accepted)
                data = path.read_bytes()
                for block in c_acceptance._blocks(data.decode("utf-8")):
                    context = json.loads(block)
                    if context["records"]["kind"] == "manual-assessment":
                        context["formalStatus"] = "pending"
                        data = data.replace(block.encode(), (json.dumps(context) + "\n").encode())
                path.write_bytes(data)
                self.assertTrue(path.read_bytes().startswith(body))
                self.assertEqual(identity, self.identity())
                accepted["coverage"]["manualAssessment"] = c_standard.manual_assessment(self.rules, self.root)
                with self.assertRaisesRegex(ValueError, "formal_assessment_pending"):
                    self.require(accepted)
        self.assertNotEqual(*identities)

    def test_strict_consumer_rejects_changed_supplementary_evidence(self):
        accepted = self.receipt()
        self.supplementary_path.write_text(self.supplementary_path.read_text(encoding="utf-8") + "Changed assessment scope.\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "input_context_unbound_or_stale"):
            self.require(accepted)

    def test_cli_calls_the_public_strict_consumer_and_retains_failure(self):
        path = self.project.parent / "summary.json"
        path.write_text(json.dumps(self.receipt()))
        analysis = self.directory / "cli-analysis"
        analysis.mkdir()
        shutil.copytree(self.project, analysis / "project")
        shutil.copyfile(path, analysis / "summary.json")
        argv = ["autosar_tooling", "c-acceptance", "--analysis", str(analysis), "--expected", TARGET + "=" + CASE,
                "--upstream", "prepare=success", "--upstream", "analysis=success"]
        consumer = partial(c_acceptance.require_accepted, root=self.root)
        with patch.object(c_acceptance, "require_accepted", consumer), patch("sys.argv", argv):
            self.assertEqual(cli.main(), 0)
        with patch.object(c_acceptance, "require_accepted", consumer), patch("sys.argv", [*argv, "--upstream", "analysis=success"]):
            self.assertEqual(cli.main(), 1)
        self.summary["diagnostics"].append({"id": "unknown", "locations": [], "programs": ["host-batch"]})
        self.summary["projectAcceptance"] = self.evaluate()
        path.write_text(json.dumps(self.summary))
        shutil.copyfile(path, analysis / "summary.json")
        with patch.object(c_acceptance, "require_accepted", consumer), patch("sys.argv", argv):
            self.assertEqual(cli.main(), 1)
