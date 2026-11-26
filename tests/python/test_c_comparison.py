"""CI baselines retain source occurrences and refuse incompatible evidence."""

import hashlib
import json
import os
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import c_comparison, c_sdk
from autosar_tooling.c_comparison import compare
from autosar_tooling.c_standard import MANUAL_ASSESSMENT_SOURCES
from ecu_tools.build import digest
from link_fixtures import make_link


def inventory(files):
    """A search-space inventory of provider files, given as {path: sha256}."""
    return {"format": "sdk-search-inventory-v1", "links": {}, "files": {
        name: {"header": name, "provider": "libc6-dev", "algorithm": "md5",
               "expected": "e" * 32, "sha256": sha} for name, sha in files.items()}}


class CComparisonTests(unittest.TestCase):
    def test_host_scope_uses_target_dependency_union_and_patch_landing_lines(self):
        scope = {"translationUnits": ["bsw/src/Can.c", "integration/src/host.c",
                                      "os/src/host/linux/port.c", "kernel/tasks.c"],
                 "projectTranslationUnits": {
                     "bsw/src/Can.c": ["integration/include/Shared.h"],
                     "integration/src/host.c": ["integration/include/Private.h"],
                     "os/src/host/linux/port.c": ["integration/include/Port.h"],
                     "kernel/tasks.c": ["kernel/portable/MSVC-MingW/portmacro.h"]},
                 "patchedLines": {"kernel/tasks.c": [7],
                                  "kernel/portable/MSVC-MingW/port.c": [7]}}
        expected = {
            "integration/include/Shared.h": "in-scope",
            "integration/include/Private.h": "host-integration",
            "integration/include/Port.h": "host-integration",
            "integration/src/host.c": "host-integration",
            "os/src/host/windows/Os_Stack.c": "host-platform",
            "kernel/portable/MSVC-MingW/port.c": "host-platform",
            "kernel/portable/MSVC-MingW/portmacro.h": "host-platform",
            "kernel/tasks.c": "in-scope", "bsw/src/Can.c": "in-scope"}
        for name, category in expected.items():
            with self.subTest(name=name):
                self.assertEqual(c_comparison.location_scope(
                    scope, {"file": name, "line": "7"}), category)
        self.assertEqual(c_comparison.location_scope(
            scope, {"file": "kernel/tasks.c", "line": "8"}), "adopted-kernel")
        other = {**scope, "projectTranslationUnits": {
            **scope["projectTranslationUnits"],
            "bsw/src/Can.c": ["integration/include/Private.h"]}}
        union = c_comparison.target_headers([scope, other])
        self.assertEqual(c_comparison.location_scope(scope, {
            "file": "integration/include/Private.h", "line": "1"}, headers=union), "in-scope")
        del scope["projectTranslationUnits"]["kernel/tasks.c"]
        with self.assertRaisesRegex(ValueError, "complete GCC"):
            c_comparison.location_scope(scope, {
                "file": "integration/include/Private.h", "line": "1"})

    def test_source_tree_layout_keeps_configuration_checks_with_new_producer_digests(self):
        with tempfile.TemporaryDirectory() as temporary:
            summary, project, output, baseline = self.fixture(Path(temporary))
            old_target = baseline / "project/target.json"
            old_target.write_text(json.dumps({**json.loads(old_target.read_text()),
                                              "includePaths": ["."]}))
            for root in (project, output / "project"):
                target = json.loads((root / "target.json").read_text())
                target.update(definitionFingerprint="new definitions", inputSha256="new input",
                              includePaths=[".", "os/src/host/common"])
                (root / "target.json").write_text(json.dumps(target))
            summary["scope"]["includePaths"].append("os/src/host/common")
            self.assertFalse(compare(summary, project, output, baseline, source_tree=True)["new"])
            with self.assertRaisesRegex(ValueError, "configuration differs"):
                compare(summary, project, output, baseline)
            target["compilerFlags"] = ["-std=c11"]
            (project / "target.json").write_text(json.dumps(target))
            with self.assertRaisesRegex(ValueError, "configuration differs"):
                compare(summary, project, output, baseline, source_tree=True)

    def test_source_tree_compares_v4_and_v5_handoffs_with_the_same_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.fixture(Path(temporary))

            def write_target(root, version, snapshots):
                path = root / "target.json"
                target = json.loads(path.read_bytes())
                target["nativeDelivery"] = {
                    "format": f"autosar-workbench-handoff-v{version}",
                    "inputSnapshots": snapshots,
                }
                path.write_text(json.dumps(target))

            inputs = [{"kind": "arxml", "logicalPath": "config.arxml"}]
            write_target(baseline / "project", 4, inputs)
            write_target(project, 5, inputs)
            self.assertFalse(compare(current, project, output, baseline, source_tree=True)["new"])
            with self.assertRaisesRegex(ValueError, "Target/sample configuration differs"):
                compare(current, project, output, baseline)

            changed_inputs = [*inputs, {"kind": "inactive-application", "logicalPath": "user.c"}]
            for version, snapshots in ((5, changed_inputs), (6, inputs)):
                with self.subTest(version=version, snapshots=snapshots):
                    write_target(project, version, snapshots)
                    with self.assertRaisesRegex(ValueError, "Target/sample configuration differs"):
                        compare(current, project, output, baseline, source_tree=True)

    def test_source_tree_checks_each_patched_kernel_before_comparing_its_file_set(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.fixture(Path(temporary))
            previous = deepcopy(current)
            patch_chain = [{"path": "runtime/os/patches/change.patch", "includes": ["tasks.c"]}]

            def kernel(root, name, data, patches):
                path = root / "kernel" / name
                path.parent.mkdir(exist_ok=True)
                path.write_bytes(data)
                target = root / "target.json"
                value = json.loads(target.read_text())
                value.update(kernelPatches=patches, kernelSha256={name: digest(path)})
                target.write_text(json.dumps(value))
                return path

            old_root = baseline / "project"
            old = kernel(old_root, "tasks.c", b"old kernel", patch_chain)
            previous["scope"]["patchedKernel"] = patch_chain
            self.write(baseline, previous)
            new = kernel(project, "tasks.c", b"new kernel", [])
            self.assertFalse(compare(current, project, output, baseline, source_tree=True)["new"])
            with self.assertRaisesRegex(ValueError, "Target/sample configuration differs"):
                compare(current, project, output, baseline)
            for path, original in ((old, b"old kernel"), (new, b"new kernel")):
                with self.subTest(path=path):
                    path.write_bytes(b"modified after analysis")
                    with self.assertRaisesRegex(ValueError, "Kernel identity mismatch: kernel/tasks.c"):
                        compare(current, project, output, baseline, source_tree=True)
                    path.write_bytes(original)
            current["scope"]["patchedKernel"] = patch_chain
            with self.assertRaisesRegex(ValueError, "Kernel patch receipt differs"):
                compare(current, project, output, baseline, source_tree=True)
            current["scope"]["patchedKernel"] = []
            new.unlink()
            kernel(project, "other.c", b"new kernel", [])
            with self.assertRaisesRegex(ValueError, "Target/sample configuration differs"):
                compare(current, project, output, baseline, source_tree=True)

    def test_matrix_receipts_bind_all_raw_inputs_and_ignore_derived_annotations(self):
        with tempfile.TemporaryDirectory() as temporary:
            summary, *_ = self.fixture(Path(temporary))
            summary["scope"]["target"] = c_comparison.MATRIX_TARGETS[0]
            receipts = []
            for target in c_comparison.MATRIX_TARGETS:
                for sample in c_comparison.MATRIX_SAMPLES:
                    other = deepcopy(summary)
                    other["scope"]["target"] = target
                    receipts.append(c_comparison.scope_receipt(other, sample))
            matrix = c_comparison.scope_matrix(receipts)
            before = c_comparison.raw_identity(summary)
            c_comparison.annotate_scope(summary, set(matrix["targetHeaders"]))
            summary["projectAcceptance"] = {"scopeMatrix": matrix}
            self.assertEqual(c_comparison.raw_identity(summary), before)
            self.assertEqual(c_comparison.matrix_headers(summary, matrix), {"main.c"})
            for broken in (receipts[:-1], receipts + [receipts[0]]):
                with self.assertRaisesRegex(ValueError, "incomplete|duplicated"):
                    c_comparison.scope_matrix(broken)
            changed = deepcopy(matrix)
            changed["targetHeaders"].append("integration/forged.h")
            with self.assertRaisesRegex(ValueError, "union differs"):
                c_comparison.matrix_headers(summary, changed)
            summary["scope"]["projectTranslationUnits"]["main.c"].append("other.h")
            with self.assertRaisesRegex(ValueError, "bind this raw analysis"):
                c_comparison.matrix_headers(summary, matrix)

    def fixture(self, root):
        baseline = root / "baseline/sample"
        output = root / "current"
        project = root / "sample"
        for path in (project, baseline / "project", output / "project"):
            (path / "meta").mkdir(parents=True)
            (path / "meta/files.sha256").write_text("seal\n")
            (path / "meta/integration.json").write_text('{"plan":{"configuration":"same"}}')
            (path / "target.json").write_text('{"target":"linux","sources":["main.c"]}')
            (path / "main.c").write_text("same();\nold();\nsame();\n")
        summary = {
            "error": None, "passed": False,
            "tool": {"version": "2.21.0", "expectedSourceRevision": "revision", "addons": {"misra.py": "hash"}, "binarySha256": "host-specific"},
            "coverage": {"rules": ["R1"], "manualAssessment": {
                "format": "misra-manual-assessment-v1",
                "inputIdentity": {
                    "parserSha256": "e" * 64,
                    "selectionSha256": hashlib.sha256("\n".join(MANUAL_ASSESSMENT_SOURCES).encode()).hexdigest(),
                    "documents": {name: "a" * 64 for name in MANUAL_ASSESSMENT_SOURCES},
                }}},
            "sourceSealSha256": digest(baseline / "project/meta/files.sha256"),
            "scope": {"target": "linux", "profile": "ecu", "entries": {"target": "main.c"}, "compilerFlags": ["-std=c99"],
                      "includePaths": ["."], "translationUnits": ["main.c"],
                      "projectTranslationUnits": {"main.c": ["main.c"]},
                      "supplemental": [], "patchedKernel": [],
                      "sdkInventory": inventory({"usr/include/limits.h": "a" * 64})},
            "programs": [{"name": "target", "entry": "main.c", "translationUnits": ["main.c"], "exitCode": 1}],
            "diagnostics": [self.diagnostic("rule", 1), self.diagnostic("rule", 2)],
        }
        summary["semanticIdentity"] = {
            "version": 4,
            "sdk": hashlib.sha256(json.dumps(c_sdk.semantic_inputs(
                summary["scope"]), sort_keys=True).encode()).hexdigest(),
            "compiler": {"machine": "x86_64-linux-gnu", "release": "13.3.0",
                         "predefines": ["#define __GNUC__ 13"], "builtins": ["-D__GNUC__=13"],
                         "includeSearch": {"quote": [], "system": ["usr/include"],
                                           "environment": {"CPATH": None, "C_INCLUDE_PATH": None}}},
            "defines": {"main.c": []},
            "models": {"platformModel": None, "platformLibrary": None},
            "libraries": {name: "a" * 64 for name in ("std", "posix", "implicitStd")},
            "addonRuntime": {"runaddon.py": "d" * 64, "cppcheck.py": "d" * 64,
                             "cppcheckdata.py": "d" * 64},
            "analysisCode": {name: "b" * 64 for name in
                             ("c_check.py", "c_addon.py", "c_comparison.py", "c_sdk.py", "c_standard.py", "c_retention.py", "c_acceptance.py", "c_preprocessor.py", "build.py", "process.py", "owner.py", "launch.py", "windows_job.py")},
        }
        self.write(baseline, summary)
        return summary, project, output, baseline

    def diagnostic(self, rule, line):
        return {"id": rule, "locations": [{"file": "main.c", "line": str(line)}], "programs": ["target"]}

    def write(self, baseline, summary):
        (baseline / "summary.json").write_text(json.dumps(summary))

    def seal(self, root, metadata):
        (root / "meta/integration.json").write_text(json.dumps(metadata))
        names = sorted(p.relative_to(root).as_posix() for p in root.rglob("*")
                       if p.is_file() and p.relative_to(root).as_posix()
                       not in {"meta/files.list", "meta/files.sha256"})
        (root / "meta/files.list").write_text("".join(name + "\n" for name in names))
        (root / "meta/files.sha256").write_text("".join(
            f"{digest(root / name)}  {name}\n" for name in sorted([*names, "meta/files.list"])))

    def runtime_fixture(self, root):
        current, project, output, baseline = self.fixture(root)
        for path in (project, output / "project", baseline / "project"):
            self.seal(path, {
                "format": "autosar-ecu-integration-v3",
                "originalBswSources": {"runtime/bsw/src/Example.c": "main.c"},
                "plan": {"configuration": "same", "runtimeSources": {
                    "runtime/bsw/src/Example.c": digest(path / "main.c"),
                    "core/src/integration/example.rs": "c" * 64,
                }},
            })
        current["sourceSealSha256"] = digest(project / "meta/files.sha256")
        self.write(baseline, current)
        return current, project, output, baseline

    def test_r11_analysis_projects_compare_only_the_same_can_selection(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.runtime_fixture(Path(temporary))
            old_root = baseline / "project"
            old_path = old_root / "meta/integration.json"
            metadata = json.loads(old_path.read_text())
            metadata["format"] = "autosar-ecu-integration-v2"
            metadata["plan"].update(
                formatVersion=2, communicationRuntime={"can": {}},
                signals=[{"trigger": "/Network/Channel/Signal", "canId": 801}],
            )
            metadata["plan"]["composition"] = {"networkEndpoints": [
                {"transport": metadata["plan"]["signals"][0].copy()},
            ]}
            old_target = json.loads((old_root / "target.json").read_text())
            old_target.update(format="autosar-build-target-v3",
                              nativeDelivery={"format": "autosar-workbench-handoff-v4"})
            (old_root / "target.json").write_text(json.dumps(old_target))
            self.seal(old_root, metadata)
            previous = deepcopy(current)
            previous["sourceSealSha256"] = digest(old_root / "meta/files.sha256")
            self.write(baseline, previous)

            metadata["format"] = "autosar-ecu-integration-v3"
            metadata["plan"].update(
                formatVersion=3, communicationRuntime={"can": {
                    "physicalChannel": "/Network/Channel",
                    "receiveIdType": "STANDARD", "transmitIdType": "STANDARD",
                }},
                signals=[{"trigger": "/Network/Channel/Signal", "canId": 801,
                          "extended": False, "acceptsFd": False}],
            )
            transport = metadata["plan"]["composition"]["networkEndpoints"][0]["transport"]
            transport.update(extended=False, acceptsFd=False)
            for root in (project, output / "project"):
                target = {**old_target, "format": "autosar-build-target-v4",
                          "nativeDelivery": {"format": "autosar-workbench-handoff-v5"},
                          "hostBatchProtocol": "HostBatchV2"}
                (root / "target.json").write_text(json.dumps(target))
                self.seal(root, metadata)
            current["sourceSealSha256"] = digest(project / "meta/files.sha256")
            self.assertFalse(compare(current, project, output, baseline, source_tree=True)["new"])

            # The layered-workspace baseline already uses handoff v5 before HostBatchV2.
            old_target["nativeDelivery"]["format"] = "autosar-workbench-handoff-v5"
            (old_root / "target.json").write_text(json.dumps(old_target))
            self.seal(old_root, json.loads(old_path.read_text()))
            previous["sourceSealSha256"] = digest(old_root / "meta/files.sha256")
            self.write(baseline, previous)
            self.assertFalse(compare(current, project, output, baseline, source_tree=True)["new"])

            for record in (metadata["plan"]["signals"][0], transport):
                with self.subTest(record=record):
                    record["extended"] = True
                    for root in (project, output / "project"):
                        self.seal(root, metadata)
                    current["sourceSealSha256"] = digest(project / "meta/files.sha256")
                    with self.assertRaisesRegex(ValueError, "Sample generation configuration differs"):
                        compare(current, project, output, baseline, source_tree=True)
                    record["extended"] = False

    def test_classical_baseline_compares_with_fd_capable_tools(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.runtime_fixture(Path(temporary))
            old_root = baseline / "project"
            metadata = json.loads((old_root / "meta/integration.json").read_text())
            metadata["plan"].update(
                communicationRuntime={"can": {"controller": "/Can/Controller"}},
                signals=[{"canId": 801, "acceptsFd": True}],
                composition={"networkEndpoints": [{"transport": {"canId": 801, "acceptsFd": True}}]},
                configuration=[
                    {"path": "/Can/Controller", "definition": "/Defs/Controller",
                     "references": {"/Defs/Controller/CanControllerDefaultBaudrate": ["/Can/Controller/Rate"]}},
                    {"path": "/Can/Controller/Rate", "definition": "/Defs/Rate",
                     "parameters": {"/Defs/Rate/CanControllerBaudRate": ["500"]}},
                ],
            )
            self.seal(old_root, metadata)
            previous = deepcopy(current)
            previous["sourceSealSha256"] = digest(old_root / "meta/files.sha256")
            self.write(baseline, previous)
            metadata["plan"]["communicationRuntime"]["can"].update(
                nominalBitrate=500_000, dataBitrate=None, brs=False, fdPadding=0,
            )
            metadata["plan"]["signals"][0]["fd"] = False
            metadata["plan"]["composition"]["networkEndpoints"][0]["transport"]["fd"] = False
            for root in (project, output / "project"):
                target = root / "target.json"
                target.write_text(json.dumps({**json.loads(target.read_text()),
                                              "canFrameProfile": "classical-and-fd"}))
                self.seal(root, metadata)
            current["sourceSealSha256"] = digest(project / "meta/files.sha256")
            self.assertFalse(compare(current, project, output, baseline, source_tree=True)["new"])

            metadata["plan"]["communicationRuntime"]["can"]["dataBitrate"] = 2_000_000
            for root in (project, output / "project"):
                self.seal(root, metadata)
            current["sourceSealSha256"] = digest(project / "meta/files.sha256")
            with self.assertRaisesRegex(ValueError, "Sample generation configuration differs"):
                compare(current, project, output, baseline, source_tree=True)

    def test_sealed_runtime_source_changes_reach_diagnostic_comparison(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.runtime_fixture(Path(temporary))
            before = (baseline / "summary.json").read_bytes()
            for root in (project, output / "project"):
                (root / "main.c").write_text("new();\nold();\nsame();\n")
                metadata = json.loads((root / "meta/integration.json").read_text())
                metadata["plan"]["runtimeSources"]["runtime/bsw/src/Example.c"] = digest(root / "main.c")
                self.seal(root, metadata)
            current["sourceSealSha256"] = digest(project / "meta/files.sha256")
            self.assertEqual(compare(current, project, output, baseline), {
                "new": [{"rule": "rule", "file": "main.c", "source": "new();", "count": 1}],
                "removed": [{"rule": "rule", "file": "main.c", "source": "same();", "count": 1}],
            })
            self.assertFalse(current["passed"])
            self.assertEqual((baseline / "summary.json").read_bytes(), before)

    def test_runtime_source_bytes_provenance_configuration_and_generator_remain_bound(self):
        for mutation in ("input-bytes", "output-bytes", "baseline-bytes", "unsealed-metadata", "claimed-hash",
                         "configuration", "generator", "copied-path", "extra-source", "missing-origin", "missing-sources",
                         "origin-type", "source-type"):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.runtime_fixture(Path(temporary))
                if mutation == "unsealed-metadata":
                    metadata = json.loads((project / "meta/integration.json").read_text())
                    metadata["owner"] = "unsealed"
                    (project / "meta/integration.json").write_text(json.dumps(metadata))
                elif mutation.endswith("-bytes"):
                    root = {"input-bytes": project, "output-bytes": output / "project",
                            "baseline-bytes": baseline / "project"}[mutation]
                    (root / "main.c").write_text("unsealed();\nold();\nsame();\n")
                else:
                    for root in (project, output / "project"):
                        metadata = json.loads((root / "meta/integration.json").read_text())
                        sources = metadata["plan"]["runtimeSources"]
                        if mutation == "claimed-hash":
                            sources["runtime/bsw/src/Example.c"] = "0" * 64
                        elif mutation == "configuration":
                            metadata["plan"]["configuration"] = "different"
                        elif mutation == "generator":
                            sources["core/src/integration/example.rs"] = "d" * 64
                        elif mutation == "copied-path":
                            (root / "copy.c").write_bytes((root / "main.c").read_bytes())
                            metadata["originalBswSources"]["runtime/bsw/src/Example.c"] = "copy.c"
                        elif mutation == "extra-source":
                            (root / "copy.c").write_bytes((root / "main.c").read_bytes())
                            metadata["originalBswSources"]["runtime/bsw/src/Other.c"] = "copy.c"
                            sources["runtime/bsw/src/Other.c"] = digest(root / "copy.c")
                        elif mutation == "missing-origin":
                            metadata["originalBswSources"].clear()
                        elif mutation == "missing-sources":
                            del metadata["plan"]["runtimeSources"]
                        elif mutation == "origin-type":
                            metadata["originalBswSources"] = []
                        elif mutation == "source-type":
                            metadata["plan"]["runtimeSources"] = []
                        self.seal(root, metadata)
                    current["sourceSealSha256"] = digest(project / "meta/files.sha256")
                message = "C baseline refused:"
                if mutation in ("origin-type", "source-type"):
                    message += " Runtime source provenance invalid"
                with self.assertRaisesRegex(ValueError, message):
                    compare(current, project, output, baseline)

    def test_origin_mapping_cannot_reclassify_generator_code_as_runtime_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.runtime_fixture(Path(temporary))
            original = "core/src/integration/example.rs"
            for root in (project, output / "project", baseline / "project"):
                (root / "generator.rs").write_text("old generator\n")
                metadata = json.loads((root / "meta/integration.json").read_text())
                metadata["originalBswSources"][original] = "generator.rs"
                metadata["plan"]["runtimeSources"][original] = digest(root / "generator.rs")
                self.seal(root, metadata)
            current["sourceSealSha256"] = digest(project / "meta/files.sha256")
            self.write(baseline, current)
            for root in (project, output / "project"):
                (root / "generator.rs").write_text("new generator\n")
                metadata = json.loads((root / "meta/integration.json").read_text())
                metadata["plan"]["runtimeSources"][original] = digest(root / "generator.rs")
                self.seal(root, metadata)
            current["sourceSealSha256"] = digest(project / "meta/files.sha256")
            with self.assertRaisesRegex(ValueError, "Sample generation configuration differs"):
                compare(current, project, output, baseline)

    def test_counts_new_and_removed_separately_even_when_net_is_zero(self):
        # Mutation: set subtraction/net count loses the second identical occurrence.
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.fixture(Path(temporary))
            current["diagnostics"] = [self.diagnostic("rule", 1), self.diagnostic("rule", 3)]
            result = compare(current, project, output, baseline)
            self.assertEqual(result, {
                "new": [{"rule": "rule", "file": "main.c", "source": "same();", "count": 1}],
                "removed": [{"rule": "rule", "file": "main.c", "source": "old();", "count": 1}],
            })

    def test_diagnostics_cannot_cancel_across_programs(self):
        for old_programs, new_programs in ((["host-batch"], ["probe"]),
                                          (["host-batch", "probe"], ["probe", "host-batch"])):
            with self.subTest(old=old_programs, new=new_programs), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.fixture(Path(temporary))
                current["programs"] = [
                    {"name": name, "entry": "main.c", "translationUnits": ["main.c"], "exitCode": 1}
                    for name in ("host-batch", "probe")
                ]
                current["diagnostics"] = [{**self.diagnostic("rule", 1), "programs": old_programs}]
                self.write(baseline, current)
                current["diagnostics"][0]["programs"] = new_programs
                result = compare(current, project, output, baseline)
                if set(old_programs) == set(new_programs):
                    self.assertEqual(result, {"new": [], "removed": []})
                else:
                    self.assertEqual(result, {
                        "new": [{"rule": "rule", "file": "main.c", "source": "same();", "count": 1}],
                        "removed": [{"rule": "rule", "file": "main.c", "source": "same();", "count": 1}],
                    })

    def test_program_changes_at_identical_source_lines_do_not_offset(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.fixture(Path(temporary))
            current["programs"] = [
                {"name": name, "entry": "main.c", "translationUnits": ["main.c"], "exitCode": 1}
                for name in ("host-batch", "probe")
            ]
            current["diagnostics"] = [
                {**self.diagnostic("rule", 1), "programs": ["host-batch", "probe"]},
                {**self.diagnostic("rule", 3), "programs": ["host-batch"]},
            ]
            self.write(baseline, current)
            current["diagnostics"][1]["programs"] = ["probe"]
            result = compare(current, project, output, baseline)
            self.assertEqual(result["new"], [{"rule": "rule", "file": "main.c", "source": "same();", "count": 1}])
            self.assertEqual(result["removed"], result["new"])

    def test_line_moves_do_not_change_counts_but_rule_file_text_and_program_occurrences_do(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.fixture(Path(temporary))
            (output / "project/main.c").write_text("\nsame();\nold();\n")
            current["diagnostics"] = [self.diagnostic("rule", 2), self.diagnostic("rule", 3)]
            self.assertEqual(compare(current, project, output, baseline), {"new": [], "removed": []})
            current["diagnostics"][0]["programs"].append("probe")
            self.assertEqual(compare(current, project, output, baseline)["new"][0]["count"], 1)
            current["diagnostics"][0]["id"] = "different-rule"
            self.assertEqual(compare(current, project, output, baseline)["new"][0]["rule"], "different-rule")

    def test_each_tool_target_sample_and_completeness_mismatch_refuses_with_local_fallback(self):
        # Mutation: removing any identity comparison must admit one of these mismatches.
        changes = [
            lambda s: s["tool"].update(version="other"),
            lambda s: s["tool"].update(expectedSourceRevision="other"),
            lambda s: s["tool"].update(addons={}),
            lambda s: s.update(coverage={}),
            lambda s: s.update(error="timeout"),
            lambda s: s["programs"][0].update(exitCode=2),
            lambda s: s["programs"][0].update(entry="other.c"),
            lambda s: s["diagnostics"][0].update(id="syntaxError"),
        ]
        for field in ("target", "profile", "entries", "compilerFlags", "includePaths", "translationUnits", "supplemental", "patchedKernel"):
            changes.append(lambda s, field=field: s["scope"].update({field: "different"}))
        for change in changes:
            with self.subTest(change=change), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.fixture(Path(temporary))
                change(current)
                with self.assertRaisesRegex(ValueError, "baseline refused:.*compute the matching baseline locally"):
                    compare(current, project, output, baseline)

    def test_missing_sources_changed_configuration_or_sample_and_bad_seal_refuse(self):
        for name in ("source", "seal", "current-seal", "output-seal", "target", "plan"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.fixture(Path(temporary))
                if name == "source":
                    (baseline / "project/main.c").unlink()
                elif name == "seal":
                    (baseline / "project/meta/files.sha256").write_text("changed")
                elif name == "current-seal":
                    (project / "meta/files.sha256").write_text("changed")
                elif name == "output-seal":
                    (output / "project/meta/files.sha256").write_text("changed")
                elif name == "target":
                    (project / "target.json").write_text('{}')
                elif name == "plan":
                    (project / "meta/integration.json").write_text('{"plan":{}}')
                with self.assertRaisesRegex(ValueError, "C baseline refused:"):
                    compare(current, project, output, baseline)

    def test_sample_identity_uses_configuration_rather_than_result_directory_name(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.fixture(Path(temporary))
            baseline = baseline.rename(baseline.with_name("downloaded-analysis"))
            project = project.rename(project.with_name("local-project"))
            self.assertEqual(compare(current, project, output, baseline), {"new": [], "removed": []})

    def test_different_binary_build_is_allowed_and_passed_false_is_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            current, project, output, baseline = self.fixture(Path(temporary))
            previous = deepcopy(current)
            current["tool"]["binarySha256"] = "another-build"
            self.assertEqual(compare(current, project, output, baseline), {"new": [], "removed": []})
            self.assertFalse(current["passed"])
            self.assertEqual(json.loads((baseline / "summary.json").read_text()), previous)

    def test_semantic_components_refuse_independently(self):
        changes = {
            "compiler": lambda x: x["compiler"].update(release="14.1.0"),
            "defines": lambda x: x["defines"]["main.c"].append("-DCHAR_BIT=16"),
            "models": lambda x: x["models"].update(platformModel="c" * 64),
            "libraries": lambda x: x["libraries"].update(posix="c" * 64),
            "addonRuntime": lambda x: x["addonRuntime"].update({"runaddon.py": "c" * 64}),
            "sdk": lambda x: x.update(sdk="e" * 64),
            "analysisCode": lambda x: x["analysisCode"].update({"c_addon.py": "c" * 64}),
        }
        for field, change in changes.items():
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.fixture(Path(temporary))
                change(current["semanticIdentity"])
                with self.assertRaisesRegex(ValueError, "Semantic identity differs: " + field):
                    compare(current, project, output, baseline)

    def test_source_tree_sdk_usage_changes_preserve_shared_header_checks(self):
        for changed in ("usage", "header", "provider", "define", "macro", "receipt"):
            with self.subTest(changed=changed), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.fixture(Path(temporary))
                header = {"path": "/sdk/limits.h", "header": "include/limits.h",
                          "provider": "libc6-dev", "version": "1", "sha256": "a" * 64,
                          "category": "sdk", "target": "linux", "algorithm": "md5",
                          "expected": "a" * 32, "key": "a" * 64}
                current["scope"].update(sdkHeaders=[header], sdkTranslationUnits={
                    "main.c": [header["path"]]}, sdkMacroInputs={"main.c": {}})
                current["semanticIdentity"]["defines"]["main.c"] = ["-DCHAR_BIT=8"]

                def bind(summary):
                    for row in summary["scope"]["sdkHeaders"]:
                        row["key"] = c_sdk.header_key(row)
                    # The inventory keeps provider "libc6-dev", so a changed provider is
                    # left to the per-header comparison.
                    summary["scope"]["sdkInventory"] = inventory({
                        "usr/include/limits.h": "a" * 64,
                        **{row["header"]: row["sha256"] for row in summary["scope"]["sdkHeaders"]}})
                    sdk = c_sdk.semantic_inputs(summary["scope"])
                    summary["semanticIdentity"]["sdk"] = hashlib.sha256(
                        json.dumps(sdk, sort_keys=True).encode()).hexdigest()

                bind(current)
                previous = deepcopy(current)
                self.write(baseline, previous)
                if changed == "usage":
                    current["semanticIdentity"]["defines"]["main.c"].append("-DINT_MAX=2147483647")
                elif changed in {"header", "provider"}:
                    header[{"header": "sha256", "provider": "provider"}[changed]] = "b" * 64
                elif changed == "define":
                    current["semanticIdentity"]["defines"]["main.c"] = ["-DCHAR_BIT=16"]
                elif changed == "macro":
                    for summary, value in ((previous, "8"), (current, "16")):
                        summary["scope"]["sdkMacroInputs"]["main.c"] = {"main.c:1": {
                            "CHAR_BIT": {"file": header["path"], "line": 1,
                                         "value": value, "function": False}}}
                        bind(summary)
                    self.write(baseline, previous)
                bind(current)
                if changed == "receipt":
                    current["semanticIdentity"]["sdk"] = "e" * 64
                if changed == "usage":
                    self.assertEqual(compare(current, project, output, baseline, source_tree=True),
                                     {"new": [], "removed": []})
                    with self.assertRaisesRegex(ValueError, "Semantic identity differs: defines"):
                        compare(current, project, output, baseline)
                else:
                    with self.assertRaisesRegex(ValueError, "Semantic identity differs: sdk|"
                                               "Semantic identity differs: defines"):
                        compare(current, project, output, baseline, source_tree=True)

    def sdk_summary(self, template, project, root, side, *, opened, version="1", installed=None,
                    search=("inc",), cpath=None, links=None, relative_links=False,
                    foreign=None, providers=None):
        """A real producer record on a miniature provider tree.

        `opened` files are consumed, `installed` ones only present. Files reachable through
        the `search` directories are in the include search space; others (such as lib/) are
        not. `links` maps a path below the provider root to the directory it points to.
        """
        provider = root / side
        packages = {}
        for file, text in {**opened, **(installed or {})}.items():
            path = provider / file
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(text.encode())
            packages[str(path)] = {
                "provider": next((name for prefix, name in (providers or {}).items()
                                  if file.startswith(prefix)), "libc6-dev"),
                "version": version, "header": file, "algorithm": "md5",
                "expected": hashlib.md5(path.read_bytes(), usedforsecurity=False).hexdigest()}
        for file, text in (foreign or {}).items():
            (provider / file).parent.mkdir(parents=True, exist_ok=True)
            (provider / file).write_bytes(text.encode())
        for name, target in (links or {}).items():
            link = provider / name
            destination = (os.path.relpath(provider / target, link.parent)
                           if relative_links else provider / target)
            make_link(self, link, destination, directory=(provider / target).is_dir())
        summary = deepcopy(template)
        paths = [provider / file for file in opened]
        compiler = summary["semanticIdentity"]["compiler"]
        compiler["includeSearch"] = {
            "quote": [], "system": list(search),
            "environment": {"CPATH": cpath, "C_INCLUDE_PATH": None}}
        summary["scope"].update(
            sdkHeaders=[c_sdk.header_identity(path, project, "linux", "x86_64-linux-gnu", packages)
                        for path in paths],
            sdkTranslationUnits={"main.c": [path.as_posix() for path in paths]},
            sdkMacroInputs={"main.c": {}}, compilerSemanticInputs=deepcopy(compiler),
            diagnosticCompiler={"path": "/selected/bin/gcc"})
        with patch.object(c_sdk, "debian_headers", return_value=packages), \
                patch.object(c_sdk, "provider_root", return_value=provider):
            summary["semanticIdentity"]["sdk"] = c_sdk.semantic(summary["scope"])
        return summary

    def sdk_pair(self, root, baseline_side, current_side):
        template, project, output, baseline = self.fixture(root)
        template.update(diagnostics=[], passed=True)
        for program in template["programs"]:
            program["exitCode"] = 0
        before = self.sdk_summary(template, project, root, "baseline", **baseline_side)
        now = self.sdk_summary(template, project, root, "current", **current_side)
        self.write(baseline, before)
        return before, now, project, output, baseline

    def test_sdk_identity_binds_the_search_space_and_ignores_package_revisions(self):
        common = {"inc/common.h": "#define COMMON_VALUE 1\n"}
        added = {**common, "inc/added.h": "#define ADDED_VALUE 1\n"}
        library = {"lib/libc.a": "archive 1\n"}
        installed_added = {"inc/added.h": "#define ADDED_VALUE 1\n"}
        other = {"inc/added.h": "#define ADDED_VALUE 2\n"}
        # (name, baseline side, current side, source-tree error, exact-source error)
        cases = [
            ("package revision", {"opened": common}, {"opened": common, "version": "2"},
             None, None),
            ("file outside the search space differs", {"opened": common, "installed": library},
             {"opened": common, "installed": {"lib/libc.a": "archive 2\n"}}, None, None),
            ("file outside the search space added", {"opened": common},
             {"opened": common, "installed": library}, None, None),
            ("unopened header in the search space differs",
             {"opened": common, "installed": installed_added},
             {"opened": common, "installed": other}, "sdk inventory: inc/added.h", "sdk"),
            ("unopened header in the search space added", {"opened": common},
             {"opened": common, "installed": installed_added},
             "sdk inventory: inc/added.h", "sdk"),
            ("header opened by one side only, other bytes on the other",
             {"opened": added}, {"opened": common, "installed": other},
             "sdk inventory: inc/added.h", "sdk"),
            ("opened header differs", {"opened": common},
             {"opened": {"inc/common.h": "#define COMMON_VALUE 2\n"}},
             "sdk inventory: inc/common.h", "sdk"),
            ("one more header opened", {"opened": common, "installed": installed_added},
             {"opened": added}, None, "sdk"),
            ("one less header opened", {"opened": added},
             {"opened": common, "installed": installed_added}, None, "sdk"),
        ]
        for name, previous, now, tree_error, exact_error in cases:
            for source_tree, error in ((True, tree_error), (False, exact_error)):
                with self.subTest(case=name, source_tree=source_tree), \
                        tempfile.TemporaryDirectory() as temporary:
                    _before, current, project, output, baseline = self.sdk_pair(
                        Path(temporary).resolve(), previous, now)
                    if error is None:
                        self.assertEqual(compare(current, project, output, baseline,
                                                 source_tree=source_tree),
                                         {"new": [], "removed": []})
                    else:
                        with self.assertRaisesRegex(ValueError,
                                                    "Semantic identity differs: " + error):
                            compare(current, project, output, baseline, source_tree=source_tree)

    def test_links_below_a_search_directory_make_provider_files_reachable(self):
        text = {"outside/branch/optional.h": "#define FEATURE_TOKEN 0\n"}
        flipped = {"outside/branch/optional.h": "#define FEATURE_TOKEN 1\n"}
        common = {"inc/common.h": "#define COMMON_VALUE 1\n"}
        reached = {"inc/branch": "outside/branch"}
        other_name = {"other/branch/optional.h": "#define FEATURE_TOKEN 1\n"}
        same_bytes = {"copy/branch/optional.h": text["outside/branch/optional.h"]}
        # (name, baseline side, current side, double-tree error, same-source error)
        cases = [
            ("same links on both sides", {"opened": common, "installed": text, "links": reached},
             {"opened": common, "installed": text, "links": reached}, None, None),
            # The reviewed escape: identical sources and search sequence, but only the
            # baseline reaches the header (and then opens it) through the link.
            ("link retargeted so the header is no longer reachable",
             {"opened": {**common, **text}, "links": reached},
             {"opened": common, "installed": flipped, "links": {"inc/branch": "inc"}},
             "sdk inventory: inc/branch/optional.h", "sdk"),
            ("link retargeted to the same name with other bytes",
             {"opened": common, "installed": {**text, **other_name}, "links": reached},
             {"opened": common, "installed": {**text, **other_name},
              "links": {"inc/branch": "other/branch"}},
             "sdk inventory: inc/branch/optional.h", "sdk"),
            ("link retargeted to the same bytes under another path",
             {"opened": common, "installed": {**text, **same_bytes}, "links": reached},
             {"opened": common, "installed": {**text, **same_bytes},
              "links": {"inc/branch": "copy/branch"}},
             "sdk inventory: inc/branch/optional.h", "sdk"),
            ("bytes behind the link differ",
             {"opened": common, "installed": text, "links": reached},
             {"opened": common, "installed": flipped, "links": reached},
             "sdk inventory: inc/branch/optional.h", "sdk"),
            ("link that leads back to its own directory",
             {"opened": common, "links": {"inc/loop": "inc"}},
             {"opened": common, "links": {"inc/loop": "inc"}}, None, None),
        ]
        for name, previous, now, tree_error, exact_error in cases:
            for source_tree, error in ((True, tree_error), (False, exact_error)):
                with self.subTest(case=name, source_tree=source_tree), \
                        tempfile.TemporaryDirectory() as temporary:
                    _before, current, project, output, baseline = self.sdk_pair(
                        Path(temporary).resolve(), previous, now)
                    if error is None:
                        self.assertEqual(compare(current, project, output, baseline,
                                                 source_tree=source_tree),
                                         {"new": [], "removed": []})
                    else:
                        with self.assertRaisesRegex(ValueError,
                                                    "Semantic identity differs: " + error):
                            compare(current, project, output, baseline, source_tree=source_tree)

    def test_every_link_met_below_a_search_directory_decides_the_identity(self):
        common = {"inc/common.h": "#define COMMON_VALUE 1\n"}
        d1 = {"d1/h.h": "#define FEATURE_TOKEN 0\n"}
        d2 = {"d2/h.h": "#define FEATURE_TOKEN 1\n"}
        both = {**d1, **d2}
        base = {"opened": common, "installed": both, "search": ("d1", "d2", "inc")}
        ancestor = {"opened": {"a/x.h": "#define X 1\n"},
                    "installed": {"a/inc/y.h": "#define Y 1\n"}, "search": ("a",)}
        outside = {"lib/real.h": "#define REAL 1\n", "lib/real2.h": "#define REAL 1\n"}
        link = "sdk inventory: link "
        # (name, baseline side, current side, double-tree error, same-source error)
        cases = [
            # Both targets are search directories entered before the link is met: the
            # file entries are identical and only the link target says which one the
            # compiler reaches through branch/h.h.
            ("link retargeted between visited directories",
             {**base, "links": {"inc/branch": "d1"}}, {**base, "links": {"inc/branch": "d2"}},
             link + "inc/branch", "sdk"),
            # The reviewed escape: an ancestor link retargeted to a different ancestor.
            ("ancestor link retargeted to another ancestor",
             {**ancestor, "links": {"a/inc/branch": "a"}},
             {**ancestor, "links": {"a/inc/branch": "."}}, link + "a/inc/branch", "sdk"),
            ("same links on both sides", {**base, "links": {"inc/branch": "d1"}},
             {**base, "links": {"inc/branch": "d1"}}, None, None),
            ("relative and equivalent absolute link", {**base, "links": {"inc/branch": "d1"}},
             {**base, "links": {"inc/branch": "d1"}, "relative_links": True}, None, None),
            ("middle of a link chain retargeted",
             {**base, "links": {"mid/b": "d1", "inc/a": "mid/b"}},
             {**base, "links": {"mid/b": "d2", "inc/a": "mid/b"}}, link + "inc/a", "sdk"),
            ("file link retargeted to another provider file with the same bytes",
             {"opened": common, "installed": outside, "links": {"inc/alias.h": "lib/real.h"}},
             {"opened": common, "installed": outside, "links": {"inc/alias.h": "lib/real2.h"}},
             "sdk inventory: inc/alias.h", "sdk"),
            ("file link to a provider file outside the search space, unchanged",
             {"opened": common, "installed": outside, "links": {"inc/alias.h": "lib/real.h"}},
             {"opened": common, "installed": outside, "links": {"inc/alias.h": "lib/real.h"}},
             None, None),
            ("link to a directory of another provider",
             {**base, "links": {"inc/branch": "d1"}},
             {**base, "installed": {**d1, **d2, "d3/h.h": d1["d1/h.h"]},
              "providers": {"d3": "linux-libc-dev"}, "links": {"inc/branch": "d3"}},
             "sdk inventory: inc/branch/h.h", "sdk"),
            # Files that belong to no provider are not part of the identity (opening one is
            # refused by the source provenance validation), and neither are the links that
            # lead to them: a runner's clang or ncurses links come and go with the image.
            ("link to a foreign file, bytes differ",
             {**base, "foreign": {"out/x.h": "one\n"}, "links": {"inc/f.h": "out/x.h"}},
             {**base, "foreign": {"out/x.h": "two\n"}, "links": {"inc/f.h": "out/x.h"}},
             None, None),
            ("link to a foreign file, target differs",
             {**base, "foreign": {"out/x.h": "one\n", "out/y.h": "one\n"},
              "links": {"inc/f.h": "out/x.h"}},
             {**base, "foreign": {"out/x.h": "one\n", "out/y.h": "one\n"},
              "links": {"inc/f.h": "out/y.h"}}, None, None),
            ("package-external links added on one side",
             {**base, "foreign": {"llvm/c.h": "one\n"}},
             {**base, "foreign": {"llvm/c.h": "one\n"},
              "links": {"inc/clang": "llvm", "inc/curses.h": "llvm/c.h"}}, None, None),
            ("package-external link retargeted between foreign directories",
             {**base, "foreign": {"llvm16/c.h": "one\n", "llvm17/c.h": "one\n"},
              "links": {"inc/clang": "llvm16"}},
             {**base, "foreign": {"llvm16/c.h": "one\n", "llvm17/c.h": "one\n"},
              "links": {"inc/clang": "llvm17"}}, None, None),
            ("dangling link, other target",
             {**base, "links": {"inc/d.h": "out/gone.h"}},
             {**base, "links": {"inc/d.h": "out/other.h"}}, None, None),
            # Where the filter must not hide anything: one side reaches provider files.
            ("provider directory link retargeted to a foreign directory",
             {**base, "foreign": {"out/x.h": "one\n"}, "links": {"inc/branch": "d1"}},
             {**base, "foreign": {"out/x.h": "one\n"}, "links": {"inc/branch": "out"}},
             link + "inc/branch", "sdk"),
            ("dangling link that now leads to a provider directory",
             {**base, "links": {"inc/d": "out/gone"}}, {**base, "links": {"inc/d": "d1"}},
             link + "inc/d", "sdk"),
        ]
        # A header reached by an escaping relative include (#include "../lib/esc.h" from a
        # header in inc/) is opened outside every search directory: it is compared as opened.
        escaped = {"opened": {**common, "lib/esc.h": "#define ESC 1\n"}}
        cases += [
            ("escaping include, same bytes", escaped, escaped, None, None),
            ("escaping include, other bytes", escaped,
             {"opened": {**common, "lib/esc.h": "#define ESC 2\n"}},
             "sdk inventory: lib/esc.h", "sdk"),
        ]
        for name, previous, now, tree_error, exact_error in cases:
            for source_tree, error in ((True, tree_error), (False, exact_error)):
                with self.subTest(case=name, source_tree=source_tree), \
                        tempfile.TemporaryDirectory() as temporary:
                    _before, current, project, output, baseline = self.sdk_pair(
                        Path(temporary).resolve(), previous, now)
                    if error is None:
                        self.assertEqual(compare(current, project, output, baseline,
                                                 source_tree=source_tree),
                                         {"new": [], "removed": []})
                    else:
                        with self.assertRaisesRegex(ValueError,
                                                    "Semantic identity differs: " + error):
                            compare(current, project, output, baseline, source_tree=source_tree)

    def test_a_missing_or_incomplete_inventory_is_never_comparable(self):
        common = {"opened": {"inc/common.h": "#define COMMON_VALUE 1\n"}}
        broken = {
            "missing": lambda scope: scope.pop("sdkInventory"),
            "empty": lambda scope: scope["sdkInventory"].update(files={}),
            "format": lambda scope: scope["sdkInventory"].update(format="invalid"),
            "opened header absent": lambda scope: scope["sdkInventory"]["files"].clear()
            or scope["sdkInventory"]["files"].update(
                {"inc/other.h": next(iter(scope["sdkHeaders"]))["sha256"] and {
                    "header": "usr/inc/other.h", "provider": "libc6-dev", "algorithm": "md5",
                    "expected": "e" * 32, "sha256": "f" * 64}}),
            "record shape": lambda scope: next(iter(scope["sdkInventory"]["files"].values())).pop(
                "expected"),
        }
        for sides in ("current", "baseline", "both"):
            for name, damage in broken.items():
                for source_tree in (True, False):
                    with self.subTest(sides=sides, damage=name, source_tree=source_tree), \
                            tempfile.TemporaryDirectory() as temporary:
                        before, current, project, output, baseline = self.sdk_pair(
                            Path(temporary).resolve(), common, common)
                        for side, summary in (("baseline", before), ("current", current)):
                            if sides in (side, "both"):
                                damage(summary["scope"])
                                # Recompute the receipt, so only the inventory is wrong.
                                sdk = (c_sdk.semantic_inputs(summary["scope"])
                                       if "sdkInventory" in summary["scope"] else None)
                                if sdk is not None:
                                    summary["semanticIdentity"]["sdk"] = hashlib.sha256(
                                        json.dumps(sdk, sort_keys=True).encode()).hexdigest()
                        self.write(baseline, before)
                        with self.assertRaisesRegex(
                                ValueError, "SDK search inventory missing, empty or incomplete"):
                            compare(current, project, output, baseline, source_tree=source_tree)

    def test_a_tampered_receipt_is_refused_in_both_modes(self):
        common = {"opened": {"inc/common.h": "#define COMMON_VALUE 1\n"},
                  "installed": {"inc/extra.h": "#define EXTRA_VALUE 1\n"}}
        for source_tree in (True, False):
            with self.subTest(source_tree=source_tree), \
                    tempfile.TemporaryDirectory() as temporary:
                _before, current, project, output, baseline = self.sdk_pair(
                    Path(temporary).resolve(), common, common)
                current["scope"]["sdkInventory"]["files"]["inc/extra.h"]["sha256"] = "0" * 64
                with self.assertRaisesRegex(ValueError, "Semantic identity differs: sdk receipt"):
                    compare(current, project, output, baseline, source_tree=source_tree)

    def test_include_search_sequence_and_variables_are_compiler_identity(self):
        shadow = {"inc/limits.h": "#define FEATURE 0\n", "alt/limits.h": "#define FEATURE 1\n"}
        both = {"opened": {"inc/limits.h": shadow["inc/limits.h"]},
                "installed": {"alt/limits.h": shadow["alt/limits.h"]}, "search": ("inc", "alt")}
        swapped = {"opened": {"alt/limits.h": shadow["alt/limits.h"]},
                   "installed": {"inc/limits.h": shadow["inc/limits.h"]}, "search": ("alt", "inc")}
        dropped = {**both, "search": ("inc",)}
        variable = {**both, "cpath": "/extra"}
        # A dropped directory also shrinks the inventory; the double tree reports that first.
        cases = [("order swapped, so another file is found first", both, swapped, "compiler"),
                 ("search directory dropped", both, dropped, "(?:compiler|sdk inventory)"),
                 ("CPATH set", both, variable, "compiler")]
        for name, previous, now, error in cases:
            for source_tree in (True, False):
                with self.subTest(case=name, source_tree=source_tree), \
                        tempfile.TemporaryDirectory() as temporary:
                    _before, current, project, output, baseline = self.sdk_pair(
                        Path(temporary).resolve(), previous, now)
                    with self.assertRaisesRegex(ValueError, "Semantic identity differs: " + error):
                        compare(current, project, output, baseline, source_tree=source_tree)

    def test_sdk_environment_is_recorded_but_not_compared(self):
        common = {"inc/common.h": "#define COMMON_VALUE 1\n"}
        with tempfile.TemporaryDirectory() as temporary:
            before, current, project, output, baseline = self.sdk_pair(
                Path(temporary).resolve(), {"opened": common},
                {"opened": common, "version": "2", "installed": {"lib/libc.a": "archive\n"}})
            self.assertEqual(before["scope"]["sdkEnvironment"]["files"], 1)
            self.assertEqual(current["scope"]["sdkEnvironment"]["files"], 2)
            self.assertNotEqual(before["scope"]["sdkEnvironment"]["sha256"],
                                current["scope"]["sdkEnvironment"]["sha256"])
            self.assertEqual(before["semanticIdentity"]["sdk"], current["semanticIdentity"]["sdk"])
            self.assertEqual(compare(current, project, output, baseline),
                             {"new": [], "removed": []})

    def test_compiler_identity_still_refuses_with_matching_sdk_headers(self):
        common = {"opened": {"inc/common.h": "#define COMMON_VALUE 1\n"}}
        changes = {"release": lambda x: x.update(release="13.3.1"),
                   "machine": lambda x: x.update(machine="x86_64-pc-linux-gnu"),
                   "predefines": lambda x: x["predefines"].append("#define __SSE2__ 1"),
                   "builtins": lambda x: x["builtins"].append("-D__SSE2__=1")}
        for field, change in changes.items():
            for source_tree in (True, False):
                with self.subTest(field=field, source_tree=source_tree), \
                        tempfile.TemporaryDirectory() as temporary:
                    _before, current, project, output, baseline = self.sdk_pair(
                        Path(temporary).resolve(), common, common)
                    change(current["semanticIdentity"]["compiler"])
                    with self.assertRaisesRegex(ValueError, "Semantic identity differs: compiler"):
                        compare(current, project, output, baseline, source_tree=source_tree)

    def test_superseded_semantic_identity_refuses_on_either_side_with_a_clear_reason(self):
        for version in (2, 3):
            for previous in (False, True):
                for source_tree in (True, False):
                    with self.subTest(version=version, previous=previous, source_tree=source_tree), \
                            tempfile.TemporaryDirectory() as temporary:
                        current, project, output, baseline = self.fixture(Path(temporary))
                        old = deepcopy(current)
                        old["semanticIdentity"]["version"] = version
                        if previous:
                            self.write(baseline, old)
                        else:
                            current = old
                        with self.assertRaisesRegex(ValueError, f"version {version} is superseded by version 4"):
                            compare(current, project, output, baseline, source_tree=source_tree)

    def test_missing_old_and_incomplete_identity_refuse_on_either_side(self):
        changes = [lambda x: x.pop("semanticIdentity"),
                   lambda x: x.update(semanticIdentity=None),
                   lambda x: x["semanticIdentity"].update(version=1)]
        for field in ("compiler", "defines", "models", "libraries", "analysisCode", "addonRuntime"):
            changes.append(lambda x, field=field: x["semanticIdentity"].update({field: {}}))
        changes.append(lambda x: x["semanticIdentity"]["models"].update(platformModel="broken"))
        for previous in (False, True):
            for change in changes:
                with self.subTest(previous=previous, change=change), tempfile.TemporaryDirectory() as temporary:
                    current, project, output, baseline = self.fixture(Path(temporary))
                    if previous:
                        old = deepcopy(current)
                        change(old)
                        self.write(baseline, old)
                    else:
                        change(current)
                    with self.assertRaisesRegex(ValueError, "Semantic identity missing, unsupported or incomplete"):
                        compare(current, project, output, baseline)


    def test_manual_record_parser_and_selection_changes_refuse_without_changing_raw_evidence(self):
        for field in ("parserSha256", "selectionSha256", "documents"):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.fixture(Path(temporary))
                before = (baseline / "summary.json").read_bytes()
                identity = current["coverage"]["manualAssessment"]["inputIdentity"]
                if field == "documents":
                    identity[field][MANUAL_ASSESSMENT_SOURCES[0]] = "f" * 64
                else:
                    identity[field] = "f" * 64
                with self.assertRaisesRegex(ValueError, "Checker coverage differs"):
                    compare(current, project, output, baseline)
                self.assertFalse(current["passed"])
                self.assertEqual((baseline / "summary.json").read_bytes(), before)

    def test_matching_incomplete_manual_identities_still_refuse(self):
        for field in ("format", "parserSha256", "selectionSha256", "documents"):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.fixture(Path(temporary))
                assessment = current["coverage"]["manualAssessment"]
                if field == "format":
                    assessment[field] = "unknown"
                else:
                    assessment["inputIdentity"][field] = {} if field == "documents" else "invalid"
                self.write(baseline, current)
                with self.assertRaisesRegex(ValueError, "Semantic identity missing, unsupported or incomplete"):
                    compare(current, project, output, baseline)
