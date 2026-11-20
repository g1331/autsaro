from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import c_fingerprint, c_gate, ci_reporting

HEADER = "#define VALUE 1\ntypedef unsigned int Count;\n"
SOURCE = (
    '#include "types.h"\n'
    "const char *source_path = __FILE__;\n"
    "Count read_count(void) { return VALUE + 2; }\n"
)


class FingerprintTests(unittest.TestCase):
    def scan(self, output: Path, *, header: str = HEADER, source: str = SOURCE,
             other: str = "int other(void) { return 0; }\n",
             flags: tuple[str, ...] = ("-std=c99",), revision: str = "pinned",
             duplicate: bool = False, rule: str = "misra-c2012-10.3") -> dict:
        compiler = os.environ.get("AUTOSAR_CC") or shutil.which("gcc")
        if compiler is None:
            self.skipTest("target GCC unavailable")
        project = output / "project"
        project.mkdir(parents=True, exist_ok=True)
        units = ["source.c", "other.c"]
        dependencies = {"source.c": ["source.c", "types.h"], "other.c": ["other.c"]}
        for name, text in (("source.c", source), ("types.h", header), ("other.c", other)):
            (project / name).write_text(text, encoding="utf-8")
        for index, unit in enumerate(units):
            output_text = subprocess.check_output(
                [compiler, *flags, "-E", "-dD", (project / unit).as_posix()], encoding="utf-8",
            )
            (output / f"compiler-origins-{index}.stdout.log").write_text(
                output_text, encoding="utf-8",
            )
        line = next(index for index, text in enumerate(source.splitlines(), 1) if "return" in text)
        diagnostic = {
            "id": rule, "severity": "style", "programs": ["host-batch", "probe"],
            "message": "test diagnostic",
            "locations": [{"file": "source.c", "line": str(line)}],
        }
        summary = {
            "passed": False, "error": None, "coverage": {"completeCompliance": False},
            "scope": {
                "translationUnits": units, "target": "test-target",
                "projectTranslationUnits": dependencies,
                # The target compiler reads a type header outside the analyzer's inputs.
                "cppcheckInputs": {unit: {"project": [unit], "sdk": []} for unit in units},
                "sdkTranslationUnits": {unit: [] for unit in units}, "sdkHeaders": [],
                "compilerFlags": list(flags), "includePaths": [],
            },
            "tool": {"version": "test", "expectedSourceRevision": revision, "addons": {}},
            "semanticIdentity": {
                "compiler": {
                    "machine": "test", "release": "test", "predefines": [], "builtins": [],
                },
                "models": {}, "libraries": {}, "addonRuntime": {},
                "analysisCode": {"c_addon.py": "test", "c_check.py": "test"},
                "defines": {unit: [] for unit in units},
            },
            "programs": [{"name": program, "translationUnits": units}
                         for program in diagnostic["programs"]],
            "diagnostics": [diagnostic] * (2 if duplicate else 1),
        }
        (output / "summary.json").write_text(json.dumps(summary), encoding="utf-8")
        return summary

    def test_target_tokens_and_configuration_invalidate_only_relevant_fingerprints(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            output = root / "baseline"
            summary = self.scan(output)
            before = c_fingerprint.inventory(summary, output)
            approved = {row["fingerprint"] for row in before}
            self.assertEqual(len(approved), 2)
            self.assertNotIn(None, approved)
            cases = (
                ("nearby", {"source": SOURCE.replace("+ 2", "+ 3")}, "blocked"),
                ("macro", {"header": HEADER.replace("VALUE 1", "VALUE 3")}, "blocked"),
                ("typedef", {"header": HEADER.replace("unsigned int", "unsigned long")}, "blocked"),
                ("options", {"flags": ("-std=c99", "-fno-strict-aliasing")}, "blocked"),
                ("version", {"revision": "changed"}, "blocked"),
                ("unrelated", {"other": "int other(void) { return 55; }\n"}, "passed"),
                ("moved", {"source": "\n\n" + SOURCE}, "passed"),
            )
            for name, changes, status in cases:
                with self.subTest(name=name):
                    candidate = root / name
                    current = c_fingerprint.inventory(self.scan(candidate, **changes), candidate)
                    result = c_fingerprint.verdict(current, before, approved)
                    self.assertEqual(result["status"], status)
                    if status == "blocked":
                        self.assertEqual(set(result["staleDispositions"]), approved)
                        self.assertEqual(len(result["unacceptedNew"]), 2)

    def test_ctu_checker_includes_other_program_units(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            previous, current = root / "previous", root / "current"
            before = self.scan(previous, rule="misra-c2012-8.7")
            after = self.scan(current, rule="misra-c2012-8.7",
                              other="int other(void) { return 12; }\n")
            result = c_fingerprint.verdict(c_fingerprint.inventory(after, current),
                                           c_fingerprint.inventory(before, previous), set())
            self.assertEqual(result["status"], "blocked")
            self.assertEqual(len(result["new"]), 2)

    def test_duplicate_occurrences_never_inherit_a_disposition(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            rows = c_fingerprint.inventory(self.scan(root, duplicate=True), root)
            self.assertEqual(len(rows), 4)
            self.assertTrue(all(row["fingerprint"] is None and row["reason"] == "ambiguous_context"
                                for row in rows))
            self.assertEqual(c_fingerprint.verdict(rows, [], set())["status"], "incomplete")

    def test_sdk_anchor_requires_the_analyzed_header_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            header = root / "sdk.h"
            header.write_bytes(b"#define EXTERNAL 1\n")
            summary = self.scan(root, flags=("-std=c99", "-include", header.as_posix()))
            scope = summary["scope"]
            scope["sdkHeaders"] = [{
                "path": header.as_posix(), "key": "provider-header",
                "sha256": hashlib.sha256(header.read_bytes()).hexdigest(),
            }]
            scope["sdkTranslationUnits"] = {unit: [header.as_posix()]
                                            for unit in scope["translationUnits"]}
            summary["diagnostics"][0]["locations"] = [{"file": header.as_posix(), "line": "1"}]
            bound = c_fingerprint.inventory(summary, root)
            self.assertTrue(all(row["fingerprint"] is not None for row in bound))
            header.write_bytes(b"#define EXTERNAL 2\n")
            refused = c_fingerprint.inventory(summary, root)
            self.assertTrue(all(row["reason"] == "missing_context" for row in refused))

    def test_transferred_receipt_requires_exact_summary_and_occurrence_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            summary = self.scan(root)
            inventory = c_fingerprint.inventory(summary, root)
            receipt_sha = c_fingerprint.write_receipt(root, inventory)
            path = root / c_fingerprint.RECEIPT
            receipt = json.loads(path.read_text(encoding="utf-8"))
            rows = c_fingerprint.read_receipt(root, summary, receipt_sha)
            self.assertEqual(len(rows), 2)
            path.write_text(json.dumps({**receipt, "occurrences": rows[:-1]}), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "differs from the raw analysis manifest"):
                c_fingerprint.read_receipt(root, summary, receipt_sha)
            with self.assertRaisesRegex(ValueError, "coverage differs"):
                changed_sha = hashlib.sha256(path.read_bytes()).hexdigest()
                c_fingerprint.read_receipt(root, summary, changed_sha)
            receipt_sha = c_fingerprint.write_receipt(root, inventory)
            with (root / "summary.json").open("a", encoding="utf-8") as stream:
                stream.write("\n")
            with self.assertRaisesRegex(ValueError, "does not bind"):
                c_fingerprint.read_receipt(root, summary, receipt_sha)

    def test_explicit_disposition_and_removal(self):
        row = {"fingerprint": "a" * 64}
        approval = {row["fingerprint"]}
        self.assertEqual(c_fingerprint.verdict([row], [], set())["status"], "blocked")
        self.assertEqual(c_fingerprint.verdict([row], [], approval)["status"], "passed")
        result = c_fingerprint.verdict([], [row], approval)
        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["staleDispositions"], sorted(approval))
        self.assertEqual(c_fingerprint.verdict([], [row], set())["status"], "passed")

    def test_preview_uses_scope_and_excludes_mandatory_retention(self):
        summary = {
            "scope": {},
            "coverage": {"rules": [{"id": "R17.3", "category": "Mandatory"}]},
            "diagnostics": [
                {"id": "misra-c2012-17.3", "locations": [{"file": "source.c"}]},
                {"id": "misra-c2012-10.4", "locations": [{"file": "source.c"}]},
                {"id": "misra-c2012-10.4", "locations": [{"file": "integration/host.c"}]},
            ],
            "projectAcceptance": {"occurrences": [
                {"diagnostic": index, "locationIndex": 0, "assessment": "accepted"}
                for index in range(3)
            ]},
        }
        current = [
            {"diagnostic": index, "locationIndex": 0, "fingerprint": c_fingerprint.sha(index)}
            for index in range(3)
        ]
        with patch.object(c_gate.c_comparison, "matrix_headers", return_value=set()):
            preview = c_gate.fingerprint_comparison(
                {"current": summary, "baseline": summary},
                {"current": current, "baseline": []}, {"current": {}, "baseline": {}},
            )
        self.assertEqual(preview["status"], "blocked")
        self.assertEqual(preview["candidateDispositions"], [current[1]["fingerprint"]])
        self.assertEqual(preview["unacceptedNew"], [current[0]["fingerprint"]])

    def test_sarif_uses_receipt_identities_per_program(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            output = root / "results" / "sample"
            summary = self.scan(output)
            rows = c_fingerprint.inventory(summary, output)
            (output / "gate.json").write_text(json.dumps({
                "diagnosticFingerprints": {
                    "format": c_fingerprint.FORMAT, "current": rows, "baseline": rows[:1],
                },
            }), encoding="utf-8")
            result = ci_reporting.sarif(ci_reporting.cppcheck(output.parent, root), "Cppcheck")
            reports = result["runs"][0]["results"]
            self.assertEqual([item["baselineState"] for item in reports], ["unchanged", "new"])
            self.assertEqual([item["partialFingerprints"] for item in reports],
                             [{c_fingerprint.SARIF_KEY: row["fingerprint"]} for row in rows])
            mixed = ci_reporting.cppcheck(output.parent, root)
            mixed[1].pop("baselineState")
            reports = ci_reporting.sarif(mixed, "Cppcheck")["runs"][0]["results"]
            self.assertTrue(all("baselineState" not in item for item in reports))
            self.assertTrue(all("partialFingerprints" in item for item in reports))
