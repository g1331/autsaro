"""CI baselines retain source occurrences and refuse incompatible evidence."""

import json
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path

from autosar_tooling.c_comparison import compare
from ecu_tools.build import digest


class CComparisonTests(unittest.TestCase):
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
            "coverage": {"rules": ["R1"]},
            "sourceSealSha256": digest(baseline / "project/meta/files.sha256"),
            "scope": {"target": "linux", "profile": "ecu", "entries": {"target": "main.c"}, "compilerFlags": ["-std=c99"],
                      "includePaths": ["."], "translationUnits": ["main.c"], "supplemental": [], "patchedKernel": []},
            "programs": [{"name": "target", "entry": "main.c", "translationUnits": ["main.c"], "exitCode": 1}],
            "diagnostics": [self.diagnostic("rule", 1), self.diagnostic("rule", 2)],
        }
        summary["semanticIdentity"] = {
            "version": 1,
            "compiler": {"machine": "x86_64-linux-gnu", "release": "13.3.0",
                         "predefines": ["#define __GNUC__ 13"], "builtins": ["-D__GNUC__=13"]},
            "defines": {"main.c": []},
            "models": {"platformModel": None, "platformLibrary": None},
            "libraries": {name: "a" * 64 for name in ("std", "posix")},
            "analysisCode": {name: "b" * 64 for name in
                             ("c_check.py", "c_addon.py", "c_comparison.py", "build.py", "process.py", "owner.py")},
        }
        self.write(baseline, summary)
        return summary, project, output, baseline

    def diagnostic(self, rule, line):
        return {"id": rule, "locations": [{"file": "main.c", "line": str(line)}], "programs": ["target"]}

    def write(self, baseline, summary):
        (baseline / "summary.json").write_text(json.dumps(summary))

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
        for name in ("source", "seal", "target", "plan"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.fixture(Path(temporary))
                if name == "source":
                    (baseline / "project/main.c").unlink()
                elif name == "seal":
                    (baseline / "project/meta/files.sha256").write_text("changed")
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
            "analysisCode": lambda x: x["analysisCode"].update({"c_addon.py": "c" * 64}),
        }
        for field, change in changes.items():
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temporary:
                current, project, output, baseline = self.fixture(Path(temporary))
                change(current["semanticIdentity"])
                with self.assertRaisesRegex(ValueError, "Semantic identity differs: " + field):
                    compare(current, project, output, baseline)

    def test_missing_old_and_incomplete_identity_refuse_on_either_side(self):
        changes = [lambda x: x.pop("semanticIdentity"),
                   lambda x: x.update(semanticIdentity=None),
                   lambda x: x["semanticIdentity"].update(version=0)]
        for field in ("compiler", "defines", "models", "libraries", "analysisCode"):
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
