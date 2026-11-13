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
                     "docs/adr/0025-misra-layered-scope.md",
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
        self.origins = {}
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
        self.files["meta/integration.json"] = json.dumps({
            "plan": self.plan, "originalBswSources": self.origins,
        }).encode()
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
        # These synthetic tests model every file as opened by each fixture TU.
        self.summary["scope"]["projectTranslationUnits"] = {
            unit: sorted(set(self.files) | {unit})
            for unit in self.summary["scope"]["translationUnits"]}
        self.summary["scope"]["cppcheckInputs"] = {
            unit: {"project": [unit], "sdk": [], "unbound": []}
            for unit in self.summary["scope"]["translationUnits"]}
        return c_acceptance.input_identity(self.summary, self.project, root=self.root)

    def context(self, *, identity=None, status="pending", group="generated-retention", case=CASE):
        self.identity()
        if identity is None and group in {"owned-source", "generated-retention"}:
            identity = c_acceptance.evidence_identity(self.summary, self.project, group, self.root)
        return {"id": "frozen-" + group, "evidence": {"head": "a" * 40, "receipt": "Synthetic contract receipt"},
                "cases": [{"caseId": case, "target": TARGET, "inputIdentity": identity or self.identity()}],
                "records": {"kind": group, **({"cohortSha256": c_retention.COHORT_SHA256} if group == "generated-retention" else {})},
                "formalStatus": status}

    def bind(self, context):
        name = (c_acceptance.ADOPTION if context["records"]["kind"] == "freertos-integration"
                else c_standard.MANUAL_ASSESSMENT_SOURCES[0])
        path = self.root / name
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
            self.bind(self.context(identity=(identities or {}).get(group,
                c_acceptance.evidence_identity(self.summary, self.project, group, self.root)
                if group in {"owned-source", "generated-retention"} else identity), group=group,
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
        self.obligation_path = self.root / "docs/standards/bsw-general/synthetic-obligations.md"
        obligations = c_standard.checker_obligations(self.rules, self.root)
        lines = [f"| {row['id']} | {row['category']} | Synthetic scope | "
                 "satisfied | Synthetic evidence |"
                 for row in obligations["records"]]
        self.obligation_path.write_text(
            "<!-- misra-checker-obligations-begin -->\n" + "\n".join(lines)
            + "\n<!-- misra-checker-obligations-end -->\n", encoding="utf-8")
        self.retained = next(row for row in c_retention.records() if row["rule"] == "2.5" and "S" in row["profiles"].split(","))
        self.files[self.retained["delivered"]] = ("/* MR12 RULE 2.5 VIOLATION: retain synthetic contract fixture. */\n"
                                                 + self.retained["shape"] + "\n").encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.summary["diagnostics"] = [{"id": "misra-c2012-2.5", "severity": "style", "message": "synthetic retained macro",
                                        "programs": ["host-batch", "probe"],
                                        "locations": [{"file": self.retained["delivered"], "line": 2, "column": 0}]}]
        self.bind_groups()

    def test_cppcheck_only_header_bytes_and_new_diagnostics_expire_freertos_evidence(self):
        self.freertos_inputs()
        name = "os/HiddenReview.h"
        self.files[name] = b"#define HIDDEN_VALUE 1\n"
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.identity()
        scope = self.summary["scope"]
        for paths in scope["projectTranslationUnits"].values():
            paths.remove(name)
        scope["cppcheckInputs"]["main.c"]["project"].append(name)
        scope["cppcheckInputs"]["main.c"]["project"].sort()
        identity = c_acceptance.freertos_identity(self.summary, self.project, self.root)
        context = {"id": "conditional-header-control", "source": c_acceptance.ADOPTION,
                   "evidence": {"head": "a" * 40, "receipt": "Synthetic conditional header control"},
                   "cases": [{"caseId": CASE, "target": TARGET, "inputIdentity": identity}],
                   "records": {"kind": "freertos-integration"}, "formalStatus": "approved"}
        with patch.object(c_acceptance, "contexts", return_value=[context]):
            self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "accepted")
            with self.subTest(change="header bytes"):
                raw_diagnostics = copy.deepcopy(self.summary["diagnostics"])
                self.files[name] = b"#define HIDDEN_VALUE 2\n"
                self.seal()
                self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
                self.assertEqual(self.summary["diagnostics"], raw_diagnostics)
                self.assertNotEqual(c_acceptance.freertos_identity(self.summary, self.project, self.root), identity)
                self.assertEqual(self.evaluate()["occurrences"][0]["reason"],
                                 "freertos_integration_context_unbound_or_stale")
            self.files[name] = b"#define HIDDEN_VALUE 1\n"
            self.seal()
            self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
            self.assertEqual(c_acceptance.freertos_identity(self.summary, self.project, self.root), identity)
            with self.subTest(change="new diagnostic"):
                self.summary["diagnostics"].append({"id": "misra-c2012-20.4", "programs": ["probe"],
                    "locations": [{"file": name, "line": 1, "column": 1}]})
                self.assertNotEqual(c_acceptance.freertos_identity(self.summary, self.project, self.root), identity)
                self.assertEqual(self.evaluate()["occurrences"][0]["reason"],
                                 "freertos_integration_context_unbound_or_stale")
        scope["cppcheckInputs"]["main.c"]["unbound"] = ["/unknown/conditional.h"]
        with self.assertRaisesRegex(ValueError, "Cppcheck input provenance unbound"):
            c_acceptance.freertos_identity(self.summary, self.project, self.root)
        del scope["cppcheckInputs"]
        with self.assertRaisesRegex(ValueError, "Cppcheck input closure"):
            c_acceptance.freertos_identity(self.summary, self.project, self.root)

    def test_local_evidence_closures_reject_each_input_and_ignore_rte_producer(self):
        self.sdk_inputs()
        (self.root / c_acceptance.ADOPTION).write_bytes(b"Synthetic kernel assessment\n")
        third_party = self.root / "third_party/freertos"
        third_party.mkdir(parents=True)
        for name in ("source-manifest.json", "posix-source-manifest.json"):
            (third_party / name).write_bytes(b'{"files":{}}\n')
        retained = next(row["delivered"] for row in c_retention.records() if "S" in row["profiles"])
        self.files[retained] = b"synthetic retention source\n"
        self.files.update({"kernel/tasks.c": b'#include "FreeRTOSConfig.h"\n',
                           "config/FreeRTOSConfig.h": b"#define configUSE_PREEMPTION 1\n",
                           "config/transitive.h": b"#define VALUE 1\n",
                           "os/patches/change.patch": b"patch bytes\n",
                           "generated/Rte_Unrelated.c": b"unrelated output\n"})
        self.target["kernelSha256"]["tasks.c"] = hashlib.sha256(self.files["kernel/tasks.c"]).hexdigest()
        self.target["kernelPatches"] = [{"path": "runtime/os/patches/change.patch", "includes": ["tasks.c"]}]
        self.summary["scope"]["translationUnits"].append("kernel/tasks.c")
        for row in self.summary["programs"]:
            row["translationUnits"].append("kernel/tasks.c")
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.identity()
        scope = self.summary["scope"]
        kernel_files = ["config/FreeRTOSConfig.h", "config/transitive.h", "kernel/tasks.c", retained]
        scope["projectTranslationUnits"] = {unit: sorted({unit, *kernel_files})
                                            for unit in scope["translationUnits"]}
        scope["sdkTranslationUnits"]["kernel/tasks.c"] = [self.sdk_header["path"]]
        scope["sdkMacroInputs"]["kernel/tasks.c"] = {}
        self.summary["semanticIdentity"]["defines"]["kernel/tasks.c"] = ["-DVALUE=1"]
        groups = ("freertos-integration", "owned-source", "generated-retention", "sdk-integration")

        def identities():
            return {group: c_acceptance.evidence_identity(self.summary, self.project, group, self.root)
                    for group in groups}

        # Real generated deliveries call runtime ownership "product"; provenance
        # mapping, rather than the human label, selects their source evidence.
        ledger_path = self.project / "meta/workbench-ownership.json"
        ledger = json.loads(ledger_path.read_bytes())
        ledger["files"][0]["owner"] = "product"
        self.files["meta/workbench-ownership.json"] = json.dumps(ledger).encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        before = identities()
        ordinary_before = c_acceptance.input_identity(self.summary, self.project, root=self.root)
        # Producer carrier bytes and output outside every selected TU are independent.
        producer = self.root / "core/src/rte_template.rs"
        producer.parent.mkdir(parents=True)
        producer.write_bytes(b"RTE template changed\n")
        self.plan["runtimeSources"]["core/src/rte_template.rs"] = c_acceptance.digest(producer)
        self.files["generated/Rte_Unrelated.c"] = b"new unrelated output\n"
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.assertNotEqual(c_acceptance.input_identity(self.summary, self.project, root=self.root),
                            ordinary_before)
        self.assertEqual(identities(), before)
        for name in kernel_files:
            with self.subTest(file=name):
                path = self.project / name
                data = path.read_bytes()
                path.write_bytes(data + b"changed\n")
                for group, value in identities().items():
                    self.assertNotEqual(value, before[group], group)
                path.write_bytes(data)
        path = self.project / "os/patches/change.patch"
        data = path.read_bytes()
        path.write_bytes(data + b"changed patch\n")
        self.assertNotEqual(identities()["freertos-integration"], before["freertos-integration"])
        path.write_bytes(data)
        for collection, key, changed in (
                (scope, "compilerFlags", ["-std=c99", "-DCHANGED=1"]),
                (self.summary["semanticIdentity"], "defines",
                 {unit: ["-DVALUE=2"] for unit in scope["translationUnits"]}),
                (scope, "includePaths", ["config", "."]),
                (scope, "sdkHeaders", [{**self.sdk_header, "sha256": "f" * 64}])):
            with self.subTest(input=key):
                saved = collection[key]
                collection[key] = changed
                for group, value in identities().items():
                    self.assertNotEqual(value, before[group], group)
                collection[key] = saved
        for field in ("dependencies", "dependency-assignment", "programs", "compiler", "owners", "origin", "assessment", "manifest"):
            with self.subTest(component=field):
                saved_summary = copy.deepcopy(self.summary)
                path = None
                if field == "dependencies":
                    for paths in scope["projectTranslationUnits"].values():
                        paths.remove("config/transitive.h")
                elif field == "dependency-assignment":
                    scope["projectTranslationUnits"]["kernel/tasks.c"].remove("config/transitive.h")
                elif field == "programs":
                    self.summary["programs"][0]["translationUnits"].remove(self.sdk_name)
                elif field == "compiler":
                    self.summary["semanticIdentity"]["compiler"]["release"] = "different"
                elif field == "owners":
                    path = self.project / "meta/workbench-ownership.json"
                elif field == "origin":
                    path = self.root / "runtime/os/src/host/linux/Os_HostLinux.c"
                elif field == "assessment":
                    path = self.root / c_acceptance.GRP
                else:
                    path = third_party / "source-manifest.json"
                if path is not None:
                    saved_bytes = path.read_bytes()
                    if field == "owners":
                        ledger = json.loads(saved_bytes)
                        ledger["files"][0]["sha256"] = "e" * 64
                        path.write_bytes(json.dumps(ledger).encode())
                    else:
                        path.write_bytes(saved_bytes + b"changed\n")
                changed = identities()
                expected = (("freertos-integration",)
                            if field in {"manifest", "dependency-assignment"} else groups)
                for group in expected:
                    self.assertNotEqual(changed[group], before[group], group)
                if path is not None:
                    path.write_bytes(saved_bytes)
                self.summary = saved_summary
                scope = self.summary["scope"]
        self.summary["diagnostics"].append({"id": "misra-c2012-2.5", "programs": ["probe"],
            "locations": [{"file": "kernel/tasks.c", "line": 1},
                          {"file": self.sdk_name, "line": 1}]})
        diagnostic_bound = identities()
        for group in groups:
            self.assertNotEqual(diagnostic_bound[group], before[group], group)
        self.summary["diagnostics"][-1]["locations"].append({"file": "other.c", "line": 4})
        for group, value in identities().items():
            self.assertNotEqual(value, diagnostic_bound[group], group)
        del scope["projectTranslationUnits"]
        with self.assertRaisesRegex(ValueError, "TU dependency closure"):
            identities()

    def test_shared_local_identity_joins_case_labels_without_last_case_winning(self):
        self.synthetic_complete_inputs()
        context = self.context(group="generated-retention", status="approved")
        context["cases"].append({**context["cases"][0], "caseId": "user-application"})
        self.bind_groups(groups=tuple(group for group in c_acceptance.GROUPS
                                     if group != "generated-retention"))
        self.bind(context)
        result = self.evaluate()
        self.assertEqual(result["status"], "accepted")
        self.assertEqual(result["caseId"], CASE)
        # The extra label may occur first or last without changing consumption.
        path = self.root / c_standard.MANUAL_ASSESSMENT_SOURCES[0]
        data = path.read_text(encoding="utf-8")
        context["cases"].reverse()
        blocks = list(c_acceptance._blocks(data))
        block = next(block for block in blocks
                     if json.loads(block)["records"]["kind"] == "generated-retention")
        path.write_text(data.replace(block, json.dumps(context) + "\n"),
                        encoding="utf-8", newline="\n")
        self.summary["coverage"]["manualAssessment"] = c_standard.manual_assessment(self.rules, self.root)
        self.assertEqual(self.evaluate(), result)

    def test_identity_binds_application_independently_of_arxml_and_plan(self):
        before = self.identity()
        plan = copy.deepcopy(self.plan)
        self.files["app/User.c"] += b"user change\n"
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.assertEqual(self.plan, plan)
        self.assertNotEqual(self.identity(), before)

    def sdk_inputs(self):
        self.sdk_name = "os/src/host/linux/Os_HostLinux.c"
        self.sdk_text = "handler.sa_sigaction = &park_signal;"
        original = "runtime/os/src/host/linux/Os_HostLinux.c"
        source = (self.sdk_text + "\nstruct sigaction handler = {0};\n").encode()
        path = self.root / original
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(source)
        self.files[self.sdk_name] = source
        self.origins[original] = self.sdk_name
        self.files["meta/workbench-ownership.json"] = json.dumps({"files": [{
            "path": self.sdk_name, "owner": "runtime", "producerId": "autosar-generator",
            "sha256": hashlib.sha256(source).hexdigest(),
        }]}).encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(
            self.project / "meta/files.sha256")
        header = {"path": "/usr/include/bits/sigaction.h", "category": "sdk",
                  "target": TARGET, "provider": "libc6-dev", "version": "2.39-0ubuntu8.9",
                  "header": "usr/include/x86_64-linux-gnu/bits/sigaction.h",
                  "sha256": "1" * 64, "algorithm": "md5", "expected": "2" * 32}
        header["key"] = hashlib.sha256(json.dumps(
            {key: value for key, value in header.items() if key not in {"path", "reason", "key"}},
            sort_keys=True).encode()).hexdigest()
        self.sdk_header = header
        definition = {"file": header["path"], "line": 40,
                      "value": "__sigaction_handler.sa_sigaction", "function": False,
                      "use": self.sdk_text, "unresolved": True}
        self.summary["scope"]["translationUnits"].append(self.sdk_name)
        for program in self.summary["programs"]:
            program["translationUnits"].append(self.sdk_name)
        self.summary["scope"].update(
            sdkHeaders=[header],
            sdkTranslationUnits={unit: [header["path"]]
                                 for unit in self.summary["scope"]["translationUnits"]},
            sdkMacroInputs={unit: ({self.sdk_name + ":1": {"sa_sigaction": definition}}
                                  if unit == self.sdk_name else {})
                            for unit in self.summary["scope"]["translationUnits"]})
        self.summary["sourceClassification"] = {
            "complete": True, "unclassifiedDiagnosticKeys": 0}
        self.sdk_record = {"definitions": {
            "sa_sigaction": {"sa_sigaction": {**{key: definition[key] for key in
                                                  ("line", "value", "function")},
                                                **{key: header[key] for key in
                                                   ("header", "provider", "version", "sha256")}}},
            "sigaction": {key: header[key] for key in
                          ("header", "provider", "version", "sha256")}},
            "sites": [{"target": TARGET, "rule": "5.5", "file": self.sdk_name,
                       "line": 1, "symbol": "sa_sigaction", "text": self.sdk_text},
                      {"target": TARGET, "rule": "5.7", "file": self.sdk_name,
                       "line": 2, "symbol": "sigaction",
                       "text": "struct sigaction handler = {0};"}]}
        self.write_sdk_record()
        self.summary["diagnostics"] = [{"id": "misra-c2012-5.5",
            "programs": ["host-batch", "probe"], "adopted": False, "patched": False,
            "locations": [{"file": self.sdk_name, "line": "1", "column": "9"}]}]
        self.bind_sdk()

    def write_sdk_record(self):
        (self.root / c_acceptance.SDK).write_text(
            "## 当前集成义务与类别处置\n<!-- sdk-integration-records-begin -->\n"
            + json.dumps(self.sdk_record) + "\n<!-- sdk-integration-records-end -->\n",
            encoding="utf-8", newline="\n")

    def bind_sdk(self, status="approved"):
        self.bind_groups()
        context = self.context(group="sdk-integration", status=status,
                               identity=c_acceptance.sdk_identity(self.summary, self.project, self.root))
        path = self.root / c_acceptance.SDK
        with path.open("a", encoding="utf-8", newline="\n") as destination:
            destination.write(c_acceptance.BEGIN + "\n" + json.dumps(context)
                              + "\n" + c_acceptance.END + "\n")

    def sdk_octal_inputs(self):
        self.sdk_inputs()
        self.sdk_text = 'descriptor = open("/proc/self/maps", O_RDONLY);'
        source = (self.sdk_text + "\nint other = 01;\n").encode()
        original = next(key for key, value in self.origins.items() if value == self.sdk_name)
        (self.root / original).write_bytes(source)
        self.files[self.sdk_name] = source
        self.files["meta/workbench-ownership.json"] = json.dumps({"files": [{
            "path": self.sdk_name, "owner": "runtime", "producerId": "autosar-generator",
            "sha256": hashlib.sha256(source).hexdigest(),
        }]}).encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        use = c_sdk.source_code(self.sdk_text)
        definition = {"file": self.sdk_header["path"], "line": 43, "value": "00",
                      "function": False, "use": use, "unresolved": False}
        self.summary["scope"]["sdkMacroInputs"][self.sdk_name] = {
            self.sdk_name + ":1": {"O_RDONLY": definition}}
        self.sdk_record = {"definitions": {"O_RDONLY": {"O_RDONLY": {
            **{key: definition[key] for key in ("line", "value", "function")},
            **{key: self.sdk_header[key] for key in ("header", "provider", "version", "sha256")}
        }}}, "sites": [{"target": TARGET, "rule": "7.1", "file": self.sdk_name,
                       "line": 1, "symbol": "O_RDONLY", "text": use}]}
        self.write_sdk_record()
        self.summary["diagnostics"] = [{"id": "misra-c2012-7.1",
            "programs": ["host-batch", "probe"], "adopted": False, "patched": False,
            "locations": [{"file": self.sdk_name, "line": "1",
                           "column": str(self.sdk_text.index("O_RDONLY") + 1)}]}]
        self.bind_sdk()

    def test_sdk_octal_macro_consumes_exact_use_and_zero_diagnostic_obligation(self):
        self.sdk_octal_inputs()
        before = copy.deepcopy(self.summary)
        result = self.evaluate()
        self.assertEqual(result["occurrences"][0]["reason"], "sdk_integration_verified")
        self.assertEqual(result["occurrences"][0]["assessment"], "accepted")
        self.assertEqual(result["sdkObligations"], [{
            "rule": "7.1", "location": {"file": self.sdk_name, "line": 1},
            "entity": "O_RDONLY", "record": c_acceptance.SDK}])
        self.assertEqual(self.summary, before)
        self.summary["diagnostics"] = []
        self.write_sdk_record()
        self.bind_sdk()
        result = self.evaluate()
        self.assertEqual(result["occurrences"], [])
        self.assertEqual(result["sdkObligations"][0]["entity"], "O_RDONLY")
        self.assertFalse(any(row["id"] == "sdk-integration" for row in result["objects"]))

    def test_sdk_octal_outside_macro_position_or_program_remains_blocked(self):
        self.sdk_octal_inputs()
        baseline = copy.deepcopy(self.summary["diagnostics"])
        for change in ({"line": "2", "column": "13"}, {"column": "1"},
                       {"column": str(len('descriptor = open("/proc/self/maps", O_RDONLY);'))},
                       {"file": "other.c"}, {"programs": ["other-program"]}):
            with self.subTest(change=change):
                self.summary["diagnostics"] = copy.deepcopy(baseline)
                diagnostic = self.summary["diagnostics"][0]
                if "programs" in change:
                    diagnostic["programs"] = change["programs"]
                else:
                    diagnostic["locations"][0].update(change)
                self.write_sdk_record()
                self.bind_sdk()
                result = self.evaluate()
                if "programs" in change:
                    self.assertEqual(result["status"], "error")
                    self.assertEqual(result["objects"][-1]["reason"],
                                     "Diagnostic program membership incomplete")
                    metadata = json.loads((self.project / "meta/integration.json").read_text())
                    positions, _ = c_acceptance._sdk_positions(
                        self.summary, self.project, metadata, self.root, None)
                    self.assertEqual(positions, {})
                else:
                    occurrence = result["occurrences"][0]
                    self.assertEqual(occurrence["assessment"], "blocked")
                    self.assertEqual(occurrence["location"], diagnostic["locations"][0])

    def test_sdk_octal_new_use_or_definition_requires_reassessment(self):
        self.sdk_octal_inputs()
        values = self.summary["scope"]["sdkMacroInputs"][self.sdk_name]
        values[self.sdk_name + ":2"] = copy.deepcopy(values[self.sdk_name + ":1"])
        self.write_sdk_record()
        self.bind_sdk()
        result = self.evaluate()
        self.assertIn("unrecorded use: " + self.sdk_name + ":2", result["objects"][-1]["reason"])
        del values[self.sdk_name + ":2"]
        values[self.sdk_name + ":1"]["O_RDONLY"]["value"] = "01"
        self.write_sdk_record()
        self.bind_sdk()
        result = self.evaluate()
        self.assertIn("definition differs: " + self.sdk_name + ":1:O_RDONLY",
                      result["objects"][-1]["reason"])

    def test_sdk_exact_position_consumption_preserves_raw_and_other_groups(self):
        self.sdk_inputs()
        before = copy.deepcopy(self.summary)
        result = self.evaluate()
        self.assertEqual(result.get("sdkInputIdentity"),
                         c_acceptance.sdk_identity(self.summary, self.project, self.root))
        occurrence = result["occurrences"][0]
        self.assertEqual(occurrence["location"],
                         {"file": self.sdk_name, "line": "1", "column": "9"})
        self.assertEqual(occurrence["assessment"], "accepted", result["objects"])
        self.assertEqual(occurrence["reason"], "sdk_integration_verified")
        self.assertEqual(len(result["sdkObligations"]), 2)
        self.assertTrue(any(row["kind"] == "checker-limitation" for row in result["objects"]))
        self.assertEqual(self.summary, before)

    def test_sdk_pending_stale_and_unbound_contexts_block_exact_position(self):
        self.sdk_inputs()
        self.bind_sdk(status="pending")
        result = self.evaluate()
        sdk_identity = c_acceptance.sdk_identity(self.summary, self.project, self.root)
        self.assertEqual(result.get("sdkInputIdentity"), sdk_identity)
        self.assertEqual(result["occurrences"][0]["assessment"], "blocked")
        self.assertEqual(result["occurrences"][0]["reason"],
                         "sdk_integration_assessment_pending")
        self.bind_sdk()
        self.summary["diagnostics"][0]["message"] = "changed report"
        result = self.evaluate()
        self.assertEqual(result.get("sdkInputIdentity"),
                         c_acceptance.sdk_identity(self.summary, self.project, self.root))
        self.assertNotEqual(result.get("sdkInputIdentity"), sdk_identity)
        self.assertEqual(result["occurrences"][0]["location"]["file"], self.sdk_name)
        self.assertEqual(result["occurrences"][0]["reason"],
                         "sdk_integration_context_unbound_or_stale")
        self.bind_groups()
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def test_sdk_other_rule_file_line_and_column_remain_blocked_after_rebinding(self):
        self.sdk_inputs()
        original = copy.deepcopy(self.summary["diagnostics"][0])
        for field, value in (("rule", "7.3"), ("id", "5.5"), ("file", "unknown.c"),
                             ("line", "2"), ("column", "1000")):
            with self.subTest(field=field):
                changed = copy.deepcopy(original)
                if field == "rule":
                    changed["id"] = "misra-c2012-" + value
                elif field == "id":
                    changed["id"] = value
                else:
                    changed["locations"][0][field] = value
                self.summary["diagnostics"] = [changed]
                self.bind_sdk()
                result = self.evaluate()
                self.assertEqual(result["occurrences"][0]["location"], changed["locations"][0])
                self.assertEqual(result["occurrences"][0]["assessment"], "blocked")

    def test_sdk_macro_name_and_definition_are_checked_beyond_context_hash(self):
        self.sdk_inputs()
        values = self.summary["scope"]["sdkMacroInputs"][self.sdk_name][self.sdk_name + ":1"]
        definition = values.pop("sa_sigaction")
        values["OTHER_MACRO"] = definition
        self.bind_sdk()
        result = self.evaluate()
        self.assertEqual(result["status"], "error")
        self.assertIn(self.sdk_name + ":1:sa_sigaction", result["objects"][-1]["reason"])
        values["sa_sigaction"] = values.pop("OTHER_MACRO")
        values["sa_sigaction"]["value"] = "project_defined_member"
        self.bind_sdk()
        self.assertIn("definition differs", self.evaluate()["objects"][-1]["reason"])

    def test_sdk_zero_diagnostics_still_checks_records_and_new_macro_uses(self):
        self.sdk_inputs()
        self.summary["diagnostics"] = []
        self.bind_sdk()
        result = self.evaluate()
        self.assertEqual(len(result["sdkObligations"]), 2)
        self.assertFalse(any(row["id"] == "sdk-integration" for row in result["objects"]))
        values = self.summary["scope"]["sdkMacroInputs"][self.sdk_name]
        values[self.sdk_name + ":3"] = copy.deepcopy(values[self.sdk_name + ":1"])
        self.bind_sdk()
        result = self.evaluate()
        self.assertEqual(result["status"], "error")
        self.assertIn("unrecorded use: " + self.sdk_name + ":3", result["objects"][-1]["reason"])

    def test_sdk_unknown_provider_changed_original_or_missing_dependency_is_rejected(self):
        self.sdk_inputs()
        for field, value in (("provider", "unknown-sdk"), ("sha256", "modified")):
            with self.subTest(field=field):
                old = self.sdk_header[field]
                self.sdk_header[field] = value
                self.bind_sdk()
                self.assertEqual(self.evaluate()["status"], "error")
                self.sdk_header[field] = old
        self.summary["scope"]["sdkTranslationUnits"][self.sdk_name] = []
        self.bind_sdk()
        self.assertIn("definition differs", self.evaluate()["objects"][-1]["reason"])

    def test_sdk_missing_records_or_document_and_mandatory_record_is_rejected(self):
        self.sdk_inputs()
        self.sdk_record["sites"][0]["rule"] = "17.3"
        self.write_sdk_record()
        self.bind_sdk()
        self.assertIn("Invalid SDK integration site", self.evaluate()["objects"][-1]["reason"])
        path = self.root / c_acceptance.SDK
        path.write_text("missing records\n", encoding="utf-8")
        self.assertIn("records missing", self.evaluate()["objects"][-1]["reason"])
        path.unlink()
        self.assertIn("evidence missing", self.evaluate()["objects"][-1]["reason"])

    def test_sdk_windows_unsigned_operand_requires_exact_macro_and_position(self):
        self.sdk_inputs()
        name = "kernel/portable/MSVC-MingW/port.c"
        target = "windows-x64-controlled-v1"
        source = "configASSERT( WaitForSingleObject( xObject, INFINITE ) == 0U );"
        header = {"path": "D:/sdk/include/winbase.h", "category": "sdk", "target": target,
                  "provider": "mingw-w64-x86_64-headers", "version": "fixture",
                  "header": "mingw64/include/winbase.h", "algorithm": "sha256",
                  "sha256": "1" * 64, "expected": "1" * 64}
        header["key"] = hashlib.sha256(json.dumps(
            {key: value for key, value in header.items() if key != "path"},
            sort_keys=True).encode()).hexdigest()
        definition = {"file": header["path"], "line": 518, "value": "0xffffffff",
                      "function": False, "use": source, "unresolved": False,
                      "headerKey": header["key"], "original": "#define INFINITE 0xffffffff"}
        self.summary["scope"].update(
            target=target, translationUnits=[name], sdkHeaders=[header],
            compilerSemanticInputs={"machine": "x86_64-w64-mingw32"},
            sdkTranslationUnits={name: [header["path"]]},
            sdkMacroInputs={name: {name + ":54": {"INFINITE": definition}}})
        self.sdk_record = {"definitions": {"INFINITE": {"INFINITE": {
            **{key: definition[key] for key in ("line", "value", "function")},
            **{key: header[key] for key in ("header", "provider", "version", "sha256")}}}},
            "sites": [{"target": target, "rule": "7.2", "file": name, "line": 54,
                       "symbol": "INFINITE", "text": source}]}
        self.write_sdk_record()
        diagnostic = {"id": "misra-c2012-7.2", "programs": ["host-batch", "probe"],
                      "locations": [{"file": name, "line": "54", "column": "5"}],
                      "sourceOrigins": [{"kind": "unsigned-literal", "macro": "INFINITE",
                                         "category": "sdk", "headerKey": header["key"],
                                         "definitionLine": 518, "value": "0xffffffff"}]}
        metadata = {"originalBswSources": {}}
        kernel = ({name: "\n" * 53 + source + "\n"}, {})
        self.summary["diagnostics"] = [diagnostic]
        positions, obligations = c_acceptance._sdk_positions(
            self.summary, self.project, metadata, self.root, kernel)
        self.assertEqual(set(positions), {(0, 0)})
        self.assertEqual(obligations[0]["location"], {"file": name, "line": 54})
        for change in ("macro", "file", "line", "category", "value", "definitionLine"):
            changed = copy.deepcopy(diagnostic)
            if change in {"macro", "category", "value", "definitionLine"}:
                changed["sourceOrigins"][0][change] = "project"
            else:
                changed["locations"][0][change] = "other.c" if change == "file" else "53"
            self.summary["diagnostics"] = [changed]
            with self.subTest(change=change):
                positions, _ = c_acceptance._sdk_positions(
                    self.summary, self.project, metadata, self.root, kernel)
                self.assertEqual(positions, {})
        definition["file"] = "project.h"
        self.assertRaises(ValueError, c_acceptance._sdk_positions,
                          self.summary, self.project, metadata, self.root, kernel)

    def test_sdk_changed_source_and_owner_are_rejected_after_rebinding(self):
        self.sdk_inputs()
        original = next(key for key, value in self.origins.items() if value == self.sdk_name)
        changed = self.files[self.sdk_name].replace(b"park_signal", b"other_signal")
        (self.root / original).write_bytes(changed)
        self.files[self.sdk_name] = changed
        owners = json.loads(self.files["meta/workbench-ownership.json"])
        owners["files"][0]["sha256"] = hashlib.sha256(changed).hexdigest()
        self.files["meta/workbench-ownership.json"] = json.dumps(owners).encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.bind_sdk()
        self.assertIn("site changed: " + self.sdk_name + ":1",
                      self.evaluate()["objects"][-1]["reason"])
        owners["files"][0]["sha256"] = "0" * 64
        self.files["meta/workbench-ownership.json"] = json.dumps(owners).encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.bind_sdk()
        self.assertIn("source provenance differs", self.evaluate()["objects"][-1]["reason"])

    def test_sdk_changed_package_version_and_record_body_invalidate_evidence(self):
        self.sdk_inputs()
        before = self.identity()
        path = self.root / c_acceptance.SDK
        with path.open("a", encoding="utf-8") as stream:
            stream.write("Changed integration assessment.\n")
        self.assertNotEqual(before, self.identity())
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")
        self.sdk_header["version"] = "new-package"
        self.sdk_header["key"] = hashlib.sha256(json.dumps(
            {key: value for key, value in self.sdk_header.items() if key not in {"path", "key"}},
            sort_keys=True).encode()).hexdigest()
        self.bind_sdk()
        self.assertIn("definition differs", self.evaluate()["objects"][-1]["reason"])

    def test_sdk_mixed_diagnostic_checks_each_physical_location(self):
        self.sdk_inputs()
        other = {"file": "unknown.c", "line": "1", "column": "9"}
        self.summary["diagnostics"][0]["locations"].append(other)
        self.bind_sdk()
        rows = self.evaluate()["occurrences"]
        self.assertEqual([row["assessment"] for row in rows], ["accepted", "blocked"])
        self.assertEqual(rows[1]["location"], other)

    def test_zero_diagnostics_still_requires_each_checker_obligation_and_its_own_group(self):
        self.synthetic_complete_inputs()
        self.summary["diagnostics"] = []
        self.summary["passed"] = True
        for program in self.summary["programs"]:
            program.update(passed=True, exitCode=0)
        self.bind_groups()
        self.assertEqual(self.evaluate()["status"], "accepted")
        text = self.obligation_path.read_text(encoding="utf-8")
        for identifier in ("R7.2", "R7.3", "R12.3", "R1.3"):
            row = next(line for line in text.splitlines()
                       if line.startswith("| " + identifier + " |"))
            line = text.splitlines().index(row) + 1
            for status in ("not_assessed", "missing", "satisfied", "not_applicable"):
                changed = ("" if status == "missing"
                           else row.replace("| satisfied |", f"| {status} |"))
                self.obligation_path.write_text(text.replace(row, changed), encoding="utf-8")
                self.bind_groups()
                before = copy.deepcopy(self.summary)
                result = self.evaluate()
                self.assertEqual(self.summary, before)
                with self.subTest(identifier=identifier, status=status):
                    objects = [item for item in result["objects"]
                               if item["kind"] == "checker-obligation" and item["id"] == identifier]
                    if status in {"satisfied", "not_applicable"}:
                        self.assertEqual(result["status"], "accepted")
                        self.assertEqual(objects, [])
                    else:
                        self.assertEqual(result["status"], "blocked")
                        self.assertEqual(len(objects), 1)
                        self.assertEqual(objects[0]["reason"], "checker-obligation_not_assessed")
                        self.assertEqual(objects[0]["owner"], "#443")
                        self.assertEqual(objects[0]["source"], None if status == "missing" else {
                            "file": self.obligation_path.relative_to(self.root).as_posix(),
                            "line": line})
        self.obligation_path.write_text(text, encoding="utf-8")
        self.bind_groups(groups=tuple(group for group in c_acceptance.GROUPS
                                     if group != "checker-obligations"))
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        self.assertTrue(any(row["kind"] == "checker-obligation" and row["id"] == "R7.2"
                            and row["reason"] == "input_context_unbound_or_stale"
                            for row in result["objects"]))

    def test_checker_obligation_selection_body_and_binding_are_independent_inputs(self):
        self.synthetic_complete_inputs()
        self.summary["diagnostics"] = []
        self.bind_groups()
        identity = self.identity()
        inputs = self.evaluate()["assessmentInputs"]["checkerObligations"]
        name = self.obligation_path.relative_to(self.root).as_posix()
        self.assertEqual(inputs["documents"], {name: c_acceptance.digest(self.obligation_path)})
        self.bind(self.context(group="owned-source", status="approved"))
        self.assertEqual(self.identity(), identity)
        text = self.obligation_path.read_text(encoding="utf-8")
        # bind_groups writes LF; edits and restoration must retain those bytes on Windows.
        for changed in (text + "Changed evidence.\n", text.replace("obligations-", "removed-")):
            with self.subTest(changed=changed[-40:]):
                self.obligation_path.write_text(changed, encoding="utf-8", newline="\n")
                self.assertNotEqual(self.identity(), identity)
                result = self.evaluate()
                self.assertTrue(any(row["kind"] == "context" and row["id"] == "checker-obligations"
                                    and row["reason"] == "input_context_unbound_or_stale"
                                    for row in result["objects"]))
        self.obligation_path.write_text(text, encoding="utf-8", newline="\n")
        self.assertEqual(self.identity(), identity)

    def test_real_checker_records_keep_gaps_and_c99_applicability_separate(self):
        self.owned_inputs()
        self.summary["diagnostics"] = []
        self.summary["passed"] = True
        for program in self.summary["programs"]:
            program.update(passed=True, exitCode=0)
        self.bind_groups()
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        objects = [row for row in result["objects"] if row["kind"] == "checker-obligation"]
        self.assertEqual(len(objects), 124)
        self.assertEqual({row["reason"] for row in objects}, {"checker-obligation_not_assessed"})
        self.assertEqual({row["owner"] for row in objects}, {"#443"})
        located = [row for row in objects if row["source"] is not None]
        self.assertEqual(len(located), 124)
        for row in located:
            self.assertTrue(row["source"]["file"].startswith(
                "docs/standards/types/language-constructs/"))
            text = (self.root / row["source"]["file"]).read_text(encoding="utf-8")
            actual = text.splitlines()[row["source"]["line"] - 1]
            self.assertTrue(actual.startswith("| " + row["id"] + " |"))
            self.assertIn("#443", row["conclusion"])
        records = c_standard.checker_obligations(self.rules, self.root)["records"]
        applicable = [row for row in records if row["status"] == "not_applicable"]
        self.assertEqual({row["id"] for row in applicable}, {"R1.4", "R8.1", "R17.3"})
        for row in applicable:
            filename = "declarations.md" if row["id"] == "R8.1" else "README.md"
            self.assertEqual(row["source"]["file"],
                             "docs/standards/types/language-constructs/" + filename)
        satisfied = {row["id"]: row for row in records if row["status"] == "satisfied"}
        lexical = {"R2.6", "R3.1", "R4.1", "R4.2", "R5.2", "R5.4", "R6.1", "R6.2"}
        declarations = {"R8.10", "R8.11", "R8.12", "R8.14", "R9.4", "R9.5"}
        self.assertEqual(set(satisfied), lexical | declarations | {"R7.1", "R7.3", "R15.5"})
        self.assertIn("Disapplied", satisfied["R15.5"]["conclusion"])
        for guideline in lexical:
            self.assertEqual(satisfied[guideline]["source"]["file"],
                             "docs/standards/types/language-constructs/lexical.md")
        by_id = {row["id"]: row for row in records}
        self.assertEqual(by_id["R7.2"]["status"], "not_assessed")
        self.assertIn("#if／#elif", by_id["R7.2"]["conclusion"])
        self.assertEqual(by_id["R7.1"]["status"], "satisfied")
        self.assertIn("O_RDONLY→00", by_id["R7.1"]["conclusion"])
        self.assertIn("sdk-integration", satisfied["R7.3"]["conclusion"])
        self.assertIn("Dcm_MsgAddInfoType", satisfied["R6.1"]["conclusion"])
        self.assertIn("unsigned char", satisfied["R6.2"]["conclusion"])
        # The real record partition is checked even with zero checker diagnostics.
        batch = lexical | {"R1.2", "R1.3", "R2.1", "R2.2", "R2.3", "R2.4", "R2.5",
                           "R2.7", "R5.3", "R5.6", "R5.8", "R5.9", "R7.1", "R7.2",
                           "R7.3", "R7.4"}
        self.assertEqual(len(batch), 24)
        self.assertEqual(sum(by_id[key]["status"] == "satisfied" for key in batch), 10)
        self.assertEqual(sum(by_id[key]["status"] == "not_assessed" for key in batch), 14)
        blocked = {"R8.2", "R8.3", "R8.4", "R8.5", "R8.6", "R8.7", "R8.9", "R8.13",
                   "R9.1", "R9.2", "R9.3"}
        for guideline in declarations | blocked:
            self.assertEqual(by_id[guideline]["status"],
                             "not_assessed" if guideline in blocked else "satisfied")
            self.assertEqual(by_id[guideline]["source"]["file"],
                             "docs/standards/types/language-constructs/declarations.md")
        self.assertIn("#if／#elif", by_id["R7.1"]["conclusion"])
        self.assertIn("Os_InterruptVectorTable[32]", by_id["R8.11"]["conclusion"])
        self.assertIn("AMD4.47–50", by_id["R9.4"]["conclusion"])
        self.assertIn("[30u]", by_id["R9.5"]["conclusion"])
        sdk_text = (ROOT / c_acceptance.SDK).read_text(encoding="utf-8")
        sdk_record = json.loads(re.search(
            r"<!-- sdk-integration-records-begin -->\s*(.*?)"
            r"\s*<!-- sdk-integration-records-end -->", sdk_text, re.DOTALL)[1])
        self.assertEqual([row for row in sdk_record["sites"] if row["rule"] == "7.1"], [{
            "target": TARGET, "rule": "7.1", "file": "os/src/host/linux/Os_LinuxNative.c",
            "line": 180, "symbol": "O_RDONLY",
            "text": "descriptor = open(                 , O_RDONLY);"}])
        self.assertEqual(sdk_record["definitions"]["O_RDONLY"], {"O_RDONLY": {
            "line": 43, "value": "00", "function": False,
            "header": "usr/include/x86_64-linux-gnu/bits/fcntl-linux.h",
            "provider": "libc6-dev", "version": "2.39-0ubuntu8.9",
            "sha256": "8d50bc7737bcc76f42b1f406244acb287bcf0463d180b6b0566d27d1f23b3e98"}})

    def freertos_kernel(self, original, final, changes):
        name = "third_party/freertos/kernel.h"
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(original.encode())
        manifest = {"files": {"kernel.h": hashlib.sha256(original.encode()).hexdigest()}}
        for filename in ("source-manifest.json", "posix-source-manifest.json"):
            (path.parent / filename).write_text(json.dumps(manifest), encoding="utf-8")
        self.target["kernelPatches"] = []
        for index, change in enumerate(changes, 1):
            patch_name = f"runtime/os/patches/{index:04d}-selection.patch"
            patch_path = self.root / patch_name
            patch_path.parent.mkdir(parents=True, exist_ok=True)
            patch_path.write_bytes(change.encode())
            self.files[patch_name.removeprefix("runtime/")] = change.encode()
            self.target["kernelPatches"].append({"path": patch_name, "includes": ["kernel.h"]})
        self.files["kernel/kernel.h"] = final.encode()
        self.origins[name] = "kernel/kernel.h"
        self.target["kernelSha256"]["kernel.h"] = hashlib.sha256(final.encode()).hexdigest()
        owners = json.loads(self.files["meta/workbench-ownership.json"])["files"]
        owners = [row for row in owners if row["path"] != "kernel/kernel.h"]
        owners.append({
            "path": "kernel/kernel.h", "owner": "adopted", "producerId": "freertos",
            "sha256": hashlib.sha256(original.encode()).hexdigest(),
        })
        self.files["meta/workbench-ownership.json"] = json.dumps({"files": owners}).encode()

    def freertos_inputs(self, *, patched=False, status="approved"):
        self.owned_inputs()
        original = "void upstream(void) {}\n"
        addition = ("/* MR12 RULE 2.5 VIOLATION: retain this fixed selection interface. */\n"
                    "#define KEEP 1u\n")
        change = ("diff --git a/kernel.h b/kernel.h\n--- a/kernel.h\n+++ b/kernel.h\n"
                  "@@ -1 +1,3 @@\n " + original
                  + "".join("+" + line + "\n" for line in addition.splitlines()))
        self.freertos_kernel(original, original + addition, [change])
        adoption = self.root / c_acceptance.ADOPTION
        adoption.write_text(
            "## 当前集成义务与消费\nSynthetic integration evidence.\n"
            "| MISRA C:2012 Rule 2.5 | `kernel/kernel.h` `KEEP` | shape=`#define KEEP 1u` "
            "| 维护者 Synthetic contract choice |\n", encoding="utf-8")
        self.summary["diagnostics"] = [{"id": "misra-c2012-2.5",
            "programs": ["host-batch", "probe"],
            "adopted": not patched, "patched": patched,
            "locations": [{"file": "kernel/kernel.h", "line": 3 if patched else 1, "column": 0}]}]
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.bind_freertos(status=status)

    def bind_freertos(self, *, status="approved"):
        self.bind_groups()
        self.bind(self.context(group="freertos-integration", status=status,
            identity=c_acceptance.freertos_identity(self.summary, self.project, self.root)))

    def test_freertos_original_and_patch_use_current_evidence_and_preserve_raw(self):
        self.freertos_inputs()
        for patched in (False, True):
            with self.subTest(patched=patched):
                diagnostic = self.summary["diagnostics"][0]
                diagnostic.update(adopted=not patched, patched=patched)
                diagnostic["locations"][0]["line"] = 3 if patched else 1
                self.bind_freertos()
                before = copy.deepcopy(self.summary)
                occurrence = self.evaluate()["occurrences"][0]
                self.assertEqual(occurrence["assessment"], "accepted", occurrence)
                self.assertEqual(occurrence["reason"], "freertos_integration_verified")
                self.assertEqual(occurrence["contextId"], "frozen-freertos-integration")
                self.assertEqual(self.summary, before)

    def test_freertos_fixture_bytes_survive_windows_text_newlines(self):
        write_text = Path.write_text

        def windows_write_text(path, data, encoding=None, errors=None, newline=None):
            return write_text(path, data, encoding=encoding, errors=errors,
                              newline="\r\n" if newline is None else newline)

        # Emulate Windows text-mode writes even when this test runs on Linux.
        text_probe = self.directory / "default-text.txt"
        with patch.object(Path, "write_text", windows_write_text):
            text_probe.write_text("default newline\n", encoding="utf-8")
            self.freertos_inputs(patched=True)
        self.assertEqual(text_probe.read_bytes(), b"default newline\r\n")
        original = self.root / "third_party/freertos/kernel.h"
        self.assertEqual(original.read_bytes(), b"void upstream(void) {}\n")
        patch_name = "os/patches/0001-selection.patch"
        self.assertEqual((self.root / "runtime" / patch_name).read_bytes(), self.files[patch_name])
        result = self.evaluate()
        self.assertNotEqual(result["status"], "error", result["objects"])
        self.assertEqual(result["occurrences"][0]["assessment"], "accepted")

    def test_freertos_pending_or_changed_reports_remain_blocked(self):
        self.freertos_inputs(status="pending")
        self.assertEqual(self.evaluate()["occurrences"][0]["reason"],
                         "freertos_integration_assessment_pending")
        self.bind_freertos()
        self.summary["diagnostics"][0]["id"] = "misra-c2012-1.3"
        self.assertEqual(self.evaluate()["occurrences"][0]["reason"],
                         "freertos_integration_context_unbound_or_stale")

    def test_freertos_patch_identity_and_missing_manifest_are_rejected(self):
        self.freertos_inputs()
        manifest = self.root / "third_party/freertos/source-manifest.json"
        saved = manifest.read_bytes()
        manifest.unlink()
        self.assertEqual(self.evaluate()["status"], "error")
        manifest.write_bytes(saved)
        path = self.root / "runtime/os/patches/0001-selection.patch"
        path.write_bytes(path.read_bytes() + b"\n")
        result = self.evaluate()
        self.assertEqual(result["status"], "error")
        self.assertIn("FreeRTOS patch identity differs", result["objects"][-1]["reason"])

    def test_freertos_patch_shape_and_unrecorded_location_are_rejected(self):
        self.freertos_inputs(patched=True)
        adoption = self.root / c_acceptance.ADOPTION
        adoption.write_text(adoption.read_text().replace("#define KEEP 1u", "#define KEEP 2u"))
        self.bind_freertos()
        self.assertEqual(self.evaluate()["occurrences"][0]["reason"], "freertos_record_invalid")
        self.summary["diagnostics"][0]["locations"][0]["line"] = 2
        self.bind_freertos()
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def test_freertos_kernel_bytes_take_precedence_over_hunk_provenance(self):
        from ecu_tools import build

        self.freertos_inputs()
        original = build.require_kernel_patch_applied

        for corrupt in (False, True):
            def after_patch(path, before, *, corrupt=corrupt):
                original(path, before)
                if corrupt:
                    kernel = next(iter(before))
                    kernel.write_bytes(kernel.read_bytes() + b"\n")

            reason = "Kernel patch hunk provenance differs: kernel/kernel.h"
            with self.subTest(corrupt=corrupt), \
                    patch.object(build, "require_kernel_patch_applied", side_effect=after_patch), \
                    patch.object(c_check, "kernel_patched_lines", side_effect=ValueError(reason)):
                result = self.evaluate()
            self.assertEqual(result["status"], "error")
            self.assertEqual(result["objects"][-1]["reason"],
                             "Kernel identity mismatch: kernel/kernel.h" if corrupt else reason)

    def test_freertos_mixed_report_does_not_accept_project_location(self):
        self.freertos_inputs()
        self.summary["diagnostics"][0]["locations"].append(
            {"file": "main.c", "line": 1, "column": 0})
        self.bind_freertos()
        rows = self.evaluate()["occurrences"]
        self.assertEqual(rows[0]["assessment"], "accepted")
        self.assertEqual(rows[1]["assessment"], "blocked")

    def test_freertos_replay_preserves_patch_responsibility_after_reversion(self):
        self.freertos_inputs()
        original = "void upstream(void) {}\n"
        changed = "void upstream(void) { (void)0; }\n"
        patches = []
        for index, (before, after) in enumerate(((original, changed), (changed, original))):
            name = f"runtime/os/patches/000{index + 2}-reversion.patch"
            change = ("diff --git a/kernel.h b/kernel.h\n--- a/kernel.h\n+++ b/kernel.h\n"
                      "@@ -1 +1 @@\n-" + before + "+" + after)
            (self.root / name).write_bytes(change.encode())
            self.files[name.removeprefix("runtime/")] = change.encode()
            patches.append({"path": name, "includes": ["kernel.h"]})
        self.target["kernelPatches"] = patches
        self.target["kernelSha256"]["kernel.h"] = hashlib.sha256(original.encode()).hexdigest()
        self.files["kernel/kernel.h"] = original.encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.bind_freertos()
        occurrence = self.evaluate()["occurrences"][0]
        self.assertEqual(occurrence["assessment"], "blocked", occurrence)
        self.assertNotEqual(occurrence["status"], "freertos_record_matched")

    def assert_required_patch_lines_blocked(self, original, final, changes, lines):
        self.freertos_inputs()
        self.freertos_kernel(original, final, changes)
        locations = [{"file": "kernel/kernel.h", "line": line, "column": 0} for line in lines]
        self.summary["diagnostics"] = [{"id": "misra-c2012-20.7",
            "programs": ["host-batch", "probe"], "patched": True, "adopted": False,
            "sourceCategory": "project-patch", "locations": locations}]
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.bind_freertos()
        result = self.evaluate()
        self.assertNotEqual(result["status"], "error", result["objects"])
        self.assertEqual([row["location"] for row in result["occurrences"]], locations)
        self.assertEqual([row["assessment"] for row in result["occurrences"]],
                         ["blocked"] * len(locations))
        metadata = json.loads((self.project / "meta/integration.json").read_text())
        _, patched = c_acceptance._freertos_sources(self.project, metadata, self.root)
        self.assertEqual(patched["kernel/kernel.h"], lines)

    def test_freertos_duplicate_added_after_original_stays_project_owned(self):
        macro = "#define SCALE(x) x * 2u\n"
        change = "--- a/kernel.h\n+++ b/kernel.h\n@@ -1 +1,2 @@\n " + macro + "+" + macro
        self.assert_required_patch_lines_blocked(macro, macro * 2, [change], [2])

    def test_freertos_deleted_line_readded_elsewhere_stays_project_owned(self):
        macro = "#define SCALE(x) x * 2u\n"
        other = "#define OTHER 0u\n"
        change = ("--- a/kernel.h\n+++ b/kernel.h\n@@ -1,2 +1,2 @@\n-" + macro
                  + " " + other + "+" + macro)
        self.assert_required_patch_lines_blocked(macro + other, other + macro, [change], [2])

    def test_freertos_duplicate_restored_text_keeps_both_project_identities(self):
        macro = "#define SCALE(x) x * 2u\n"
        corrected = "#define SCALE(x) ((x) * 2u)\n"
        first = ("--- a/kernel.h\n+++ b/kernel.h\n@@ -1 +1,2 @@\n-" + macro
                 + "+" + corrected + "+" + macro)
        second = ("--- a/kernel.h\n+++ b/kernel.h\n@@ -1,2 +1,2 @@\n-" + corrected
                  + "+" + macro + " " + macro)
        self.assert_required_patch_lines_blocked(macro, macro * 2, [first, second], [1, 2])

    def test_freertos_patch_cannot_consume_a_new_required_retention(self):
        self.freertos_inputs(patched=True)
        adoption = self.root / c_acceptance.ADOPTION
        adoption.write_text(adoption.read_text().replace("Rule 2.5", "Rule 10.3"))
        for name in ("kernel/kernel.h", "os/patches/0001-selection.patch"):
            self.files[name] = self.files[name].replace(b"RULE 2.5", b"RULE 10.3")
        (self.root / "runtime/os/patches/0001-selection.patch").write_bytes(
            self.files["os/patches/0001-selection.patch"])
        self.target["kernelSha256"]["kernel.h"] = hashlib.sha256(
            self.files["kernel/kernel.h"]).hexdigest()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.summary["diagnostics"][0]["id"] = "misra-c2012-10.3"
        self.bind_freertos()
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def test_freertos_zero_reports_still_requires_integration_evidence(self):
        self.freertos_inputs()
        self.summary["scope"].pop("adoptedSources", None)
        self.summary["diagnostics"] = []
        self.bind_groups()
        result = self.evaluate()
        self.assertIn({"kind": "context", "id": "freertos-integration",
                       "reason": "freertos_integration_context_unbound_or_stale"},
                      result["objects"])

    def test_freertos_zero_reports_rejects_missing_interaction_record(self):
        self.freertos_inputs()
        self.summary["diagnostics"] = []
        adoption = self.root / c_acceptance.ADOPTION
        adoption.write_text(adoption.read_text().replace("## 当前集成义务与消费", ""))
        self.bind_freertos()
        result = self.evaluate()
        self.assertIn({"kind": "context", "id": "freertos-integration",
                       "reason": "freertos_integration_evidence_missing"}, result["objects"])

    def test_freertos_configuration_change_invalidates_integration_evidence(self):
        self.freertos_inputs()
        self.target["compilerFlags"] = ["-DCHANGED_CONFIGURATION=1"]
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        occurrence = self.evaluate()["occurrences"][0]
        self.assertEqual(occurrence["reason"], "freertos_integration_context_unbound_or_stale")

    def test_freertos_report_order_does_not_change_integration_identity(self):
        self.freertos_inputs()
        diagnostic = copy.deepcopy(self.summary["diagnostics"][0])
        diagnostic["id"] = "misra-c2012-8.7"
        self.summary["diagnostics"].append(diagnostic)
        before = c_acceptance.freertos_identity(self.summary, self.project, self.root)
        self.summary["diagnostics"].reverse()
        self.assertEqual(c_acceptance.freertos_identity(self.summary, self.project, self.root),
                         before)

    def owned_inputs(self):
        self.synthetic_complete_inputs()
        shutil.copytree(ROOT / "runtime", self.root / "runtime", dirs_exist_ok=True)
        shutil.copytree(ROOT / "docs/standards", self.root / "docs/standards", dirs_exist_ok=True)
        # This owned-source fixture has no SDK dependency/macro inventory.
        # SDK integration has its own complete contract fixture above.
        (self.root / c_acceptance.SDK).unlink()
        # Real standards supply the checker records; keep their outstanding
        # obligations instead of combining them with synthetic satisfied rows.
        self.supplementary_path.unlink()
        self.obligation_path.unlink()
        application = self.root / c_standard.RETAINED_APPLICATION
        application.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / c_standard.RETAINED_APPLICATION, application)
        for name in c_standard.MANUAL_ASSESSMENT_SOURCES:
            path = self.root / name
            text = path.read_text(encoding="utf-8").replace("| retained |", "| satisfied |")
            path.write_text(text, encoding="utf-8")
        name = "runtime/os/src/host/linux/Os_HostLinux.c"
        delivered = "os/src/host/linux/Os_HostLinux.c"
        self.files[delivered] = (self.root / name).read_bytes()
        self.origins[name] = delivered
        self.plan["runtimeSources"][name] = c_acceptance.digest(self.root / name)
        self.files["meta/workbench-ownership.json"] = json.dumps({"files": [{
            "path": delivered, "sha256": self.plan["runtimeSources"][name],
            "owner": "runtime", "producerId": "runtime",
        }]}).encode()
        source = self.files[delivered].decode()
        offset = source.index("*low = (uintptr_t)thread->mapping;")
        self.summary["diagnostics"] = [{"id": "misra-c2012-11.4",
            "programs": ["host-batch", "probe"],
            "locations": [{"file": delivered, "line": source.count("\n", 0, offset) + 1,
                           "column": offset - source.rfind("\n", 0, offset)}]}]
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.bind_groups(statuses={"checker-limitations": "pending"})

    def test_owned_address_consumes_original_category_with_current_context(self):
        self.owned_inputs()
        context = self.context(group="owned-source", status="approved")
        self.bind(context)
        raw = copy.deepcopy(self.summary)
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked", result["objects"])
        occurrence = result["occurrences"][0]
        self.assertEqual(occurrence["assessment"], "accepted")
        self.assertEqual(occurrence["reason"], "owned_source_approved")
        self.assertEqual(occurrence["record"]["file"],
                         "docs/standards/os/native-facilities/addresses.md")
        self.assertEqual(occurrence["programs"], ["host-batch", "probe"])
        checker_rows = c_standard.supplementary_assessment(
            self.rules, self.summary["coverage"]["knownCheckerLimitations"], self.root,
        )["records"]
        self.assertEqual(len(checker_rows), 11)
        self.assertEqual({row["source"]["file"] for row in checker_rows},
                         {"docs/standards/types.md"})
        self.assertEqual({row["id"] for row in checker_rows if row["status"] == "not_assessed"},
                         set())
        self.assertEqual({row["id"] for row in checker_rows if row["status"] == "retained"},
                         {"R5.5", "R5.7", "R10.1", "R20.1"})
        self.assertEqual(raw, self.summary)
        self.summary["projectAcceptance"] = result
        with self.assertRaisesRegex(ValueError, "formal_assessment_pending"):
            c_acceptance.require_accepted([(self.summary, self.project)], [(TARGET, CASE)],
                                         {"prepare": "success", "analysis": "success"},
                                         root=self.root)

    def test_owned_record_retains_formal_and_stale_context_blockers(self):
        self.owned_inputs()
        self.bind(self.context(group="owned-source", status="pending"))
        occurrence = self.evaluate()["occurrences"][0]
        self.assertEqual(occurrence["reason"], "formal_deviation_pending")
        self.assertEqual(occurrence["status"], "owned_record_matched")
        self.assertEqual(occurrence["assessment"], "blocked")

    def test_owned_source_and_exact_position_are_required(self):
        self.owned_inputs()
        self.bind(self.context(group="owned-source", status="approved"))
        self.summary["diagnostics"][0]["locations"][0]["line"] += 1
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")
        self.summary["diagnostics"][0]["locations"][0]["line"] -= 1
        name = "runtime/os/src/host/linux/Os_HostLinux.c"
        path = self.root / name
        path.write_bytes(path.read_bytes() + b"/* changed producer */\n")
        self.bind_groups()
        self.bind(self.context(group="owned-source", status="approved"))
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def test_owned_record_document_is_part_of_current_evidence(self):
        self.owned_inputs()
        self.bind(self.context(group="owned-source", status="approved"))
        path = self.root / "docs/standards/os/native-facilities/addresses.md"
        path.write_text(path.read_text(encoding="utf-8") + "\nChanged category evidence.\n",
                        encoding="utf-8")
        self.assertEqual(self.evaluate()["occurrences"][0]["reason"],
                         "owned_source_context_unbound_or_stale")

    def test_owned_record_does_not_override_sdk_source(self):
        self.owned_inputs()
        self.summary["scope"]["sdkHeaders"] = [{
            "path": self.summary["diagnostics"][0]["locations"][0]["file"],
            "category": "sdk", "key": "synthetic-header-identity",
        }]
        self.bind(self.context(group="owned-source", status="approved"))
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def select_owned(self, source, rule, expression, *, programs=None, delivered=None):
        delivered = delivered or source.removeprefix("runtime/")
        data = (self.root / source).read_bytes()
        self.files[delivered] = data
        self.origins[source] = delivered
        self.files["meta/workbench-ownership.json"] = json.dumps({"files": [{
            "path": delivered, "sha256": hashlib.sha256(data).hexdigest(),
            "owner": "runtime", "producerId": "runtime",
        }]}).encode()
        text = data.decode()
        offset = text.index(expression)
        self.summary["diagnostics"] = [{"id": "misra-c2012-" + rule,
            "programs": programs or ["host-batch", "probe"], "locations": [{
                "file": delivered, "line": text.count("\n", 0, offset) + 1,
                "column": offset - text.rfind("\n", 0, offset),
            }]}]
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.bind_groups()
        self.bind(self.context(group="owned-source", status="approved"))

    def test_owned_exact_cohort_shape_and_program_scope(self):
        self.owned_inputs()
        source = "runtime/os/include/Os.h"
        self.select_owned(source, "20.10", "#define OS_TASK_ENTRY(identifier)")
        occurrence = self.evaluate()["occurrences"][0]
        self.assertEqual(occurrence["assessment"], "accepted")
        self.assertEqual(occurrence["entity"], "OS_TASK_ENTRY")
        original = (self.root / source).read_bytes()
        (self.root / source).write_bytes(
            original.replace(b"Os_TaskEntry_##identifier", b"Other_##identifier"))
        self.select_owned(source, "20.10", "#define OS_TASK_ENTRY(identifier)")
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")
        (self.root / source).write_bytes(original)
        self.select_owned("runtime/ecu/src/Ecu_HostBatch.c", "8.7", "StatusType Ecu_HostBatchParse",
                          programs=["probe"])
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "accepted")
        self.select_owned("runtime/ecu/src/Ecu_HostBatch.c", "8.7", "StatusType Ecu_HostBatchParse",
                          programs=["host-batch"])
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def test_owned_required_risk_record_keeps_formal_obligation(self):
        self.owned_inputs()
        self.select_owned("runtime/os/src/Os_Error.c", "18.6", "context->parameters = parameters;")
        occurrence = self.evaluate()["occurrences"][0]
        self.assertEqual(occurrence["status"], "owned_record_matched")
        self.assertEqual(occurrence["entity"], "Os_ErrorContextSave")
        self.assertEqual(occurrence["reason"], "formal_deviation_pending")

    def test_owned_group_cannot_approve_a_generated_retention_record(self):
        self.owned_inputs()
        self.select_owned("runtime/os/src/Os_Arti.c", "11.4", "const uint64 native_address",
                          delivered="generated/src/Os_Arti.c")
        self.bind_groups(statuses={"generated-retention": "pending"})
        self.bind(self.context(group="owned-source", status="approved"))
        occurrence = self.evaluate()["occurrences"][0]
        self.assertEqual(occurrence["assessment"], "blocked")
        self.assertEqual(occurrence["status"], "record_matched")
        self.assertEqual(occurrence["reason"], "formal_deviation_pending")

    def test_owned_position_cannot_overflow_into_a_record_on_the_next_line(self):
        self.owned_inputs()
        source = "runtime/os/include/Os.h"
        self.select_owned(source, "20.10", "#define OS_TASK_ENTRY(identifier)")
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "accepted")
        location = self.summary["diagnostics"][0]["locations"][0]
        lines = (self.root / source).read_text(encoding="utf-8").splitlines(keepends=True)
        location["line"] -= 1
        location["column"] += len(lines[location["line"] - 1])
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def test_owned_both_scope_does_not_authorize_a_new_program(self):
        self.owned_inputs()
        self.summary["scope"]["entries"]["other"] = self.summary["scope"]["entries"].pop("probe")
        self.summary["programs"][1]["name"] = "other"
        self.target["entries"] = self.summary["scope"]["entries"]
        self.select_owned("runtime/os/include/Os.h", "20.10", "#define OS_TASK_ENTRY(identifier)",
                          programs=["other"])
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def test_owned_all_scope_does_not_authorize_a_new_profile(self):
        self.owned_inputs()
        self.bind_groups(cases={group: "unreviewed-profile" for group in c_acceptance.GROUPS})
        self.bind(self.context(group="owned-source", status="approved", case="unreviewed-profile"))
        occurrence = self.evaluate()["occurrences"][0]
        self.assertEqual(occurrence["assessment"], "blocked")
        self.assertEqual(occurrence["reason"], "owned_source_case_outside_record")

    def test_owned_memmap_consumption_rejects_unrelated_undef(self):
        self.owned_inputs()
        source = "runtime/bsw/include/Can_MemMap.h"
        self.select_owned(source, "20.5", "#undef CAN_STOP_SEC_CODE")
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "accepted")
        path = self.root / source
        path.write_bytes(path.read_bytes().replace(
            b"#undef CAN_STOP_SEC_CODE", b"#undef UNRELATED"))
        self.select_owned(source, "20.5", "#undef UNRELATED")
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def test_owned_unrecorded_memmap_undef_waits_for_category_instance_consumer(self):
        self.owned_inputs()
        self.select_owned("runtime/os/include/Os_MemMap.h", "20.5", "#undef OS_MEMMAP\n")
        occurrence = self.evaluate()["occurrences"][0]
        self.assertEqual(occurrence["assessment"], "blocked")
        self.assertEqual(occurrence["reason"], "no_record")

    def test_owned_memmap_requires_the_complete_protocol(self):
        self.owned_inputs()
        source = "runtime/bsw/src/Can.c"
        self.select_owned(source, "20.1", '#include "Can_MemMap.h"')
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "accepted")
        path = self.root / source
        path.write_bytes(path.read_bytes().replace(b"CAN_STOP_SEC_CODE", b"CAN_STOP_SEC_OTHER"))
        self.select_owned(source, "20.1", '#include "Can_MemMap.h"')
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

    def test_owned_c90_ruling_uses_the_effective_language(self):
        self.owned_inputs()
        source = "runtime/bsw/src/Can.c"
        self.select_owned(source, "17.3", "Can_InternalRxIndication(")
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "accepted")
        self.summary["scope"]["compilerFlags"].append("-std=c90")
        self.select_owned(source, "17.3", "Can_InternalRxIndication(")
        self.assertEqual(self.evaluate()["occurrences"][0]["assessment"], "blocked")

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
                self.identity()  # A real c-check summary always carries the TU closure.
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
        groups = {row["records"]["kind"] for row in summary["projectAcceptance"]["contexts"]}
        self.assertEqual(groups,
                         {"generated-retention", "manual-assessment", "checker-limitations",
                          "checker-obligations"})
        self.assertEqual(len(summary["coverage"]["rules"]), 221)
        self.assertEqual(summary["coverage"]["manualAssessment"]["checkerUncovered"]["total"], 66)
        supplementary = c_standard.supplementary_assessment(self.rules, summary["coverage"]["knownCheckerLimitations"], self.root)
        self.assertEqual(len(supplementary["records"]), 11)
        self.assertEqual(summary["projectAcceptance"]["occurrences"][0]["entity"], self.retained["name"])
        self.require(summary)
        self.assertFalse(summary["passed"])
        self.assertEqual(summary["programs"][0]["exitCode"], 1)

    def grp_inputs(self):
        self.synthetic_complete_inputs()
        own = b"int owned(int x) {\n    if (x) { return 1; }\n    return 0;\n}\n"
        upstream = own.replace(b"owned", b"upstream")
        patched = upstream.replace(
            b"    return 0;", b"    if (x == 2) { return 2; }\n    return 0;")
        original = self.root / "third_party/freertos/kernel.h"
        original.parent.mkdir(parents=True)
        original.write_bytes(upstream)
        producer = self.root / "runtime/producer.c"
        producer.write_bytes(own)
        self.plan["runtimeSources"]["runtime/producer.c"] = c_acceptance.digest(producer)
        self.origins = {"runtime/producer.c": "bsw/Owned.c",
                        "third_party/freertos/kernel.h": "kernel/kernel.h"}
        self.files.update({"generated/Owned.c": own, "bsw/Owned.c": own,
                           "kernel/kernel.h": patched, "unknown.c": own})
        self.target["kernelSha256"]["kernel.h"] = hashlib.sha256(patched).hexdigest()
        owners = [("generated/Owned.c", "generated", "autosar-generator", own),
                  ("bsw/Owned.c", "product", "autosar-product", own),
                  ("kernel/kernel.h", "product", "autosar-product", upstream)]
        self.files["meta/workbench-ownership.json"] = json.dumps({"files": [
            {"path": name, "owner": owner, "producerId": producer,
             "sha256": hashlib.sha256(data).hexdigest()}
            for name, owner, producer, data in owners]}).encode()
        change = ("--- a/kernel.h\n+++ b/kernel.h\n@@ -1,4 +1,5 @@\n"
                  " int upstream(int x) {\n     if (x) { return 1; }\n"
                  "+    if (x == 2) { return 2; }\n     return 0;\n }\n")
        self.freertos_kernel(upstream.decode(), patched.decode(), [change])
        (self.root / c_acceptance.ADOPTION).write_text("## 当前集成义务与消费\n", encoding="utf-8")
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.summary["scope"].update(sdkHeaders=[], sdkMacroInputs={}, patchedLines={})

    def grp_diagnostic(self, name, line=2, *, rule="15.5", **details):
        text = self.files[name].decode().splitlines()[line - 1]
        return {"id": "misra-c2012-" + rule, "programs": ["host-batch", "probe"],
                "locations": [{"file": name, "line": str(line),
                               "column": str(text.index("return") + 1)}],
                "patched": False, "adopted": False, "sourceCategory": "project",
                "sourceOrigins": [], **details}

    def test_grp_consumes_owned_returns_and_patch_without_changing_raw_results(self):
        self.grp_inputs()
        self.summary["diagnostics"].extend([
            self.grp_diagnostic("generated/Owned.c"), self.grp_diagnostic("bsw/Owned.c"),
            self.grp_diagnostic("kernel/kernel.h", 3, adopted=True,
                                sourceCategory="adopted-freertos")])
        self.bind_groups()
        self.bind(self.context(group="freertos-integration", status="approved",
            identity=c_acceptance.freertos_identity(self.summary, self.project, self.root)))
        before = copy.deepcopy(self.summary)
        result = self.evaluate()
        self.assertEqual(result["status"], "accepted", result["objects"])
        for row in result["occurrences"][1:]:
            self.assertEqual(row["reason"], "grp_disapplied")
            self.assertEqual(row["record"], "docs/adr/0025-misra-layered-scope.md#grp指南重分类计划")
            self.assertEqual(row["programs"], ["host-batch", "probe"])
        self.assertEqual(self.summary, before)
        self.summary["projectAcceptance"] = result
        self.require(self.summary)

    def test_grp_rejects_forged_source_labels_unknown_sources_and_other_rules(self):
        self.grp_inputs()
        self.summary["diagnostics"] = [
            self.grp_diagnostic("kernel/kernel.h"), self.grp_diagnostic("unknown.c"),
            self.grp_diagnostic("generated/Owned.c", rule="15.4")]
        self.summary["scope"]["patchedLines"] = {"kernel/kernel.h": [2]}
        self.summary["scope"]["sdkMacroInputs"] = {"main.c": {"kernel/kernel.h:2": {
            "PROJECT_CONFIG": {"file": "generated/Owned.c", "line": 1, "value": "1"}}}}
        self.bind_groups()
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        self.assertEqual(len(result["occurrences"]), 3)
        self.assertTrue(all(row["assessment"] == "blocked" for row in result["occurrences"]))

    def test_grp_uses_hunk_identity_for_duplicate_kernel_returns(self):
        self.grp_inputs()
        original = (self.root / "third_party/freertos/kernel.h").read_text()
        statement = "    if (x) { return 1; }\n"
        final = original.replace(statement, statement * 2)
        change = ("--- a/kernel.h\n+++ b/kernel.h\n@@ -1,4 +1,5 @@\n"
                  " int upstream(int x) {\n " + statement + "+" + statement
                  + "     return 0;\n }\n")
        self.freertos_kernel(original, final, [change])
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        self.summary["diagnostics"] = [self.grp_diagnostic("kernel/kernel.h", line)
                                        for line in (2, 3)]
        self.bind_freertos()
        rows = self.evaluate()["occurrences"]
        self.assertEqual([row["location"]["line"] for row in rows], ["2", "3"])
        self.assertEqual([row["status"] for row in rows],
                         ["freertos_record_matched", "grp_disapplied"])
        self.assertEqual([row["assessment"] for row in rows], ["accepted", "accepted"])

    def test_grp_keeps_untraceable_return_blocked(self):
        self.grp_inputs()
        diagnostic = self.grp_diagnostic("generated/Owned.c")
        for change in ("column", "line-directive"):
            with self.subTest(change=change):
                self.summary["diagnostics"] = [copy.deepcopy(diagnostic)]
                if change == "column":
                    self.summary["diagnostics"][0]["locations"][0]["column"] = "1"
                elif change == "line-directive":
                    self.summary["scope"]["sdkLineDirectives"] = True
                self.bind_groups()
                result = self.evaluate()
                self.assertEqual(result["status"], "blocked")
                self.assertEqual(result["occurrences"][0]["assessment"], "blocked")

    def test_grp_decides_each_location_without_hiding_the_remaining_diagnostic(self):
        self.grp_inputs()
        diagnostic = self.grp_diagnostic("generated/Owned.c")
        diagnostic["locations"].extend(self.grp_diagnostic("unknown.c")["locations"])
        self.summary["diagnostics"] = [diagnostic]
        self.bind_freertos()
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        self.assertEqual([row["assessment"] for row in result["occurrences"]],
                         ["accepted", "blocked"])
        self.assertEqual([row["locationIndex"] for row in result["occurrences"]], [0, 1])
        self.assertEqual(result["objects"][0]["location"]["file"], "unknown.c")

    def test_grp_rejects_sdk_return_macro_and_sdk_header(self):
        self.grp_inputs()
        diagnostic = self.grp_diagnostic("generated/Owned.c")
        self.files["generated/Owned.c"] = self.files["generated/Owned.c"].replace(
            b"return 1", b"FOREIGN_RETURN")
        ledger = json.loads(self.files["meta/workbench-ownership.json"])
        ledger["files"][0]["sha256"] = hashlib.sha256(self.files["generated/Owned.c"]).hexdigest()
        self.files["meta/workbench-ownership.json"] = json.dumps(ledger).encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        header = "/provider/foreign.h"
        self.summary["scope"].update(
            sdkHeaders=[{"path": header, "category": "sdk", "key": "s" * 64}],
            sdkMacroInputs={"main.c": {"generated/Owned.c:2": {
                "FOREIGN_RETURN": {"file": header, "line": 1, "value": "return 1"}}}})
        foreign = copy.deepcopy(diagnostic)
        foreign["locations"][0]["file"] = header
        self.summary["diagnostics"] = [diagnostic, foreign]
        self.bind_groups()
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        self.assertTrue(all(row["assessment"] == "blocked" for row in result["occurrences"]))

    def test_grp_owned_return_with_sdk_value_keeps_macro_provenance(self):
        self.grp_inputs()
        diagnostic = self.grp_diagnostic("generated/Owned.c")
        self.files["generated/Owned.c"] = self.files["generated/Owned.c"].replace(
            b"return 1", b"return FOREIGN_VALUE")
        ledger = json.loads(self.files["meta/workbench-ownership.json"])
        ledger["files"][0]["sha256"] = hashlib.sha256(self.files["generated/Owned.c"]).hexdigest()
        self.files["meta/workbench-ownership.json"] = json.dumps(ledger).encode()
        self.seal()
        self.summary["sourceSealSha256"] = c_acceptance.digest(self.project / "meta/files.sha256")
        header = "/provider/foreign.h"
        self.summary["scope"].update(
            sdkHeaders=[{"path": header, "category": "sdk", "key": "s" * 64}],
            sdkMacroInputs={"main.c": {"generated/Owned.c:2": {
                "FOREIGN_VALUE": {"file": header, "line": 1, "value": "1"}}}})
        c_sdk.classify([diagnostic], self.summary["scope"], [], self.project)
        self.assertTrue(diagnostic["sourceOrigins"])
        self.summary["diagnostics"] = [diagnostic]
        self.bind_freertos()
        before = copy.deepcopy(self.summary)
        result = self.evaluate()
        self.assertEqual(result["status"], "accepted", result["objects"])
        self.assertEqual(result["occurrences"][0]["reason"], "grp_disapplied")
        self.assertEqual(self.summary, before)

    def test_grp_requires_ownership_and_current_original_source_bytes(self):
        self.grp_inputs()
        self.summary["diagnostics"] = [self.grp_diagnostic("bsw/Owned.c")]
        producer = self.root / "runtime/producer.c"
        producer.write_bytes(producer.read_bytes() + b"/* changed original */\n")
        self.bind_groups()
        result = self.evaluate()
        self.assertEqual(result["occurrences"][0]["assessment"], "blocked")
        self.assertIn("producer_provenance_unverified",
                      [row["reason"] for row in result["objects"]])
        self.summary["diagnostics"] = [self.grp_diagnostic("generated/Owned.c")]
        (self.project / "meta/workbench-ownership.json").unlink()
        result = self.evaluate()
        self.assertEqual(result["status"], "error")

    def test_grp_preserves_other_assessment_gates_and_binds_policy_bytes(self):
        self.grp_inputs()
        self.summary["diagnostics"] = [self.grp_diagnostic("generated/Owned.c")]
        self.bind_groups()
        before = self.identity()
        policy = self.root / "docs/adr/0025-misra-layered-scope.md"
        policy.write_bytes(policy.read_bytes() + b"\nPolicy changed.\n")
        self.assertNotEqual(self.identity(), before)
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        self.assertIn("input_context_unbound_or_stale",
                      [row["reason"] for row in result["objects"]])
        self.bind_groups(status="pending")
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        self.assertIn("formal_assessment_pending", [row["reason"] for row in result["objects"]])

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

        checker_obligations = c_standard.checker_obligations

        def old_obligations(rules, root):
            checked = copy.deepcopy(rules)
            next(row for row in checked if row["id"] == "D1.1")["automaticChecker"] = False
            return checker_obligations(checked, root)

        # Reproduce the legacy attack with both independent mapping guards
        # bypassed, then restore them for the real strict-consumer rejection.
        with patch.object(c_acceptance, "_catalog", side_effect=old_mapping), \
                patch.object(c_standard, "checker_obligations", side_effect=old_obligations):
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
        self.sdk_inputs()
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
        header.write_bytes(b"\n" * 39 + b"#define sa_sigaction __sigaction_handler.sa_sigaction\n")
        package = {"provider": "libc6-dev", "version": "fixture",
                   "header": self.sdk_header["header"], "algorithm": "md5",
                   "expected": hashlib.md5(header.read_bytes(), usedforsecurity=False).hexdigest()}
        packages = {str(header): package}
        record = c_sdk.header_identity(header, self.project, TARGET, "x86_64-linux-gnu", packages)
        self.assertEqual(record["category"], "sdk")
        scope = self.summary["scope"]
        scope.update(compilerSemanticInputs={"machine": "x86_64-linux-gnu", "release": "12"},
                     compilerDiscoveredDefines={"main.c": []}, platformModel=None, platformLibrary=None,
                     sdkHeaders=[record], sdkTranslationUnits={
                         unit: [record["path"]] for unit in scope["translationUnits"]})
        definition = scope["sdkMacroInputs"][self.sdk_name][self.sdk_name + ":1"]["sa_sigaction"]
        definition["file"] = record["path"]
        for values in (self.sdk_record["definitions"]["sa_sigaction"]["sa_sigaction"],
                       self.sdk_record["definitions"]["sigaction"]):
            values.update({key: record[key] for key in ("header", "provider", "version", "sha256")})
        self.write_sdk_record()
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
            self.bind_sdk()
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

    def test_current_checker_records_preserve_their_specific_outstanding_obligations(self):
        self.synthetic_complete_inputs()
        self.supplementary_path.unlink()
        path = self.root / "docs/standards/types.md"
        path.write_bytes((ROOT / "docs/standards/types.md").read_bytes())
        self.bind_groups(status="pending")
        before = copy.deepcopy(self.summary)
        result = self.evaluate()
        self.assertEqual(result["status"], "blocked")
        self.assertEqual(self.summary, before)
        rows = c_standard.supplementary_assessment(
            self.rules, self.summary["coverage"]["knownCheckerLimitations"], self.root,
        )["records"]
        self.assertEqual({row["id"] for row in rows}, c_standard.CHECKER_LIMITATION_GUIDELINES)
        outstanding = set()
        self.assertEqual(
            {row["id"] for row in rows if row["status"] == "not_assessed"}, outstanding,
        )
        self.assertEqual({row["id"] for row in rows if row["status"] == "retained"},
                         {"R5.5", "R5.7", "R10.1", "R20.1"})
        for row in rows:
            reason = ("checker-limitation_not_assessed" if row["id"] in outstanding
                      else "formal_assessment_pending")
            self.assertTrue(any(obj["kind"] == "checker-limitation" and obj["id"] == row["id"]
                                and obj["reason"] == reason and obj["source"] == row["source"]
                                for obj in result["objects"]), row)
            self.assertEqual(row["source"]["file"], "docs/standards/types.md")
            line = path.read_text(encoding="utf-8").splitlines()[row["source"]["line"] - 1]
            self.assertTrue(line.startswith("| " + row["id"] + " |"))

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
        self.bind_groups()
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
                    self.bind(self.context(
                        identity=(c_acceptance.evidence_identity(self.summary, self.project, group, self.root)
                                  if group == "generated-retention" else identity),
                        status="approved", group=group))
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
