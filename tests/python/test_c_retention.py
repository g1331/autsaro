"""Fixed retention records are references, never analyzer success or deviations."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import c_check, c_retention

TARGET = "linux-x64-controlled-v1"


class RetentionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.project = Path(self.temporary.name)
        self.rows = c_retention.records()
        self.row = next(row for row in self.rows if row["rule"] == "2.5")

    def write(self, row, text=None):
        source = text or f"/* MR12 RULE {row['rule']} VIOLATION: retain exact entity. */\n{row['shape']}\n"
        if row["rule"] == "11.4" and text is None:
            source = "void Os_ArtiAlarmBaseReturn(void *address) {\n" + source + "}\n"
        path = self.project / row["delivered"]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")
        return source

    def diagnostic(self, row=None, line=2):
        row = row or self.row
        return {"id": "misra-c2012-" + row["rule"], "severity": "style", "message": "raw finding",
                "locations": [{"file": row["delivered"], "line": str(line), "column": "1"}],
                "programs": ["host-batch", "probe"]}

    def occurrence(self, diagnostic=None, target=TARGET):
        return c_retention.analyze([diagnostic or self.diagnostic()], self.project, target)["occurrences"][0]

    def test_fixed_cohort_and_every_rule_match_without_formal_acceptance(self):
        self.assertEqual(len(self.rows), 160)
        for rule in ("2.5", "2.3", "8.7", "11.4"):
            row = next(row for row in self.rows if row["rule"] == rule)
            self.write(row)
            raw = self.diagnostic(row, 3 if rule == "11.4" else 2)
            before = copy.deepcopy(raw)
            result = c_retention.analyze([raw], self.project, TARGET)
            item = result["occurrences"][0]
            with self.subTest(rule=rule):
                self.assertEqual(item["status"], "record_matched")
                self.assertEqual(item["entity"], row["name"])
                self.assertEqual(item["record"], row["record"])
                self.assertEqual(item["producer"], row["producer"])
                self.assertEqual(item["programs"], ["host-batch", "probe"])
                self.assertEqual(item["evidenceClosure"], "not_established")
                self.assertEqual(result["profileVerification"], "not_established")
                self.assertEqual(result["status"], "partial")
                self.assertEqual(raw, before)
        altered = copy.deepcopy(self.rows)
        altered[0]["shape"] += " 1"
        with self.assertRaisesRegex(ValueError, "cohort identity"):
            c_retention.check_cohort(altered)

    def test_instance_handle_requires_its_own_retention_record(self):
        row = next(row for row in self.rows if row["name"] == "Rte_Inst_EchoApplication")
        self.write(row)
        raw = self.diagnostic(row)
        self.assertEqual(self.occurrence(raw)["status"], "record_matched")
        remaining = [item for item in self.rows if item is not row]
        with self.assertRaisesRegex(ValueError, "cohort identity"):
            c_retention.check_cohort(remaining)
        with patch.object(c_retention, "records", return_value=remaining):
            occurrence = self.occurrence(raw)
        self.assertEqual(occurrence["status"], "unmatched")
        self.assertEqual(occurrence["reason"], "no_record")
        self.assertIsNone(occurrence["entity"])

    def test_shared_project_check_preserves_complete_cohort_and_extra_marker_rejection(self):
        sources = {}
        for row in self.rows:
            source = f"/* MR12 RULE {row['rule']} VIOLATION: retain exact entity. */\n{row['shape']}\n"
            sources.setdefault(row["delivered"], []).append(source)
        for name, fragments in sources.items():
            path = self.project / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("\n".join(fragments), encoding="utf-8")
        self.assertEqual(c_retention.check_project(self.project, self.rows), [])
        path = self.project / self.row["delivered"]
        extra = (path.read_text() + "\n/* MR12 RULE 2.5 VIOLATION: retain extra. */\n"
                 "#define Unapproved_Retention 7u\n")
        self.assertIn((self.row["delivered"], "Unapproved_Retention", "cohort"),
                      c_retention.check_project(self.project, self.rows, {self.row["delivered"]: extra}))

    def test_shape_marker_entity_rule_program_and_target_are_not_widened(self):
        original = self.write(self.row)
        for source, change, reason in (
            (original.replace(self.row["shape"], self.row["shape"] + " + 1"), {}, "shape_mismatch"),
            (original.replace("VIOLATION:", "OTHER:"), {}, "marker_mismatch"),
            (original.replace(self.row["name"], "UnknownEntity"), {}, "entity_location_unmatched"),
            (original, {"id": "misra-c2012-10.4"}, "no_record"),
            (original, {"programs": ["host-batch", "unknown"]}, "program_outside_record"),
            (original, {"programs": []}, "program_outside_record"),
        ):
            self.write(self.row, source)
            with self.subTest(reason=reason):
                item = self.occurrence({**self.diagnostic(), **change})
                self.assertEqual(item["status"], "unmatched")
                self.assertEqual(item["reason"], reason)
        self.write(self.row, original)
        self.assertEqual(self.occurrence(target="unknown")["reason"], "target_outside_record")

    def test_multiple_locations_and_absent_locations_remain_visible(self):
        self.write(self.row)
        raw = self.diagnostic()
        raw["locations"].append({"file": "generated/unknown.h", "line": "2", "column": "1"})
        result = c_retention.analyze([raw, {**raw, "locations": []}], self.project, TARGET)
        self.assertEqual([item["status"] for item in result["occurrences"]],
                         ["record_matched", "unmatched", "unmatched"])
        self.assertEqual([item["diagnostic"] for item in result["occurrences"]], [0, 0, 1])
        self.assertIsNone(result["occurrences"][-1]["location"])

    def test_position_and_complete_multiline_replacement_are_required(self):
        original = self.write(self.row)
        self.assertEqual(self.occurrence(self.diagnostic(line=1))["status"], "unmatched")
        wrong_column = self.diagnostic()
        wrong_column["locations"][0]["column"] = "10000"
        self.assertEqual(self.occurrence(wrong_column)["status"], "unmatched")
        self.write(self.row, original.rstrip() + " \\\n+ + 1\n")
        self.assertEqual(self.occurrence()["reason"], "shape_mismatch")
        self.write(self.row, original + original)
        self.assertEqual(self.occurrence()["status"], "unmatched")

    def test_local_address_name_cannot_match_another_function(self):
        row = next(row for row in self.rows if row["rule"] == "11.4")
        source = self.write(row)
        self.write(row, source.replace("Os_ArtiAlarmBaseReturn", "OtherFunction"))
        self.assertEqual(self.occurrence(self.diagnostic(row, 3))["reason"], "function_mismatch")

    def test_malformed_records_name_the_source_line_and_field(self):
        directory = self.project / c_retention.RECORD_DIRECTORY
        directory.mkdir(parents=True)
        path = directory / "records.md"
        fields = {"delivered": "delivered=`generated/include/X.h`", "shape": "shape=`#define Name 1U`",
                  "profiles": "profiles=S", "producer": "producer=x"}
        for missing in (*fields, "source/entity"):
            source = "`source` `Name`" if missing != "source/entity" else "`Name`"
            details = "; ".join(value for key, value in fields.items() if key != missing)
            path.write_text(f"heading\n| MISRA C:2012 Rule 2.5 | {source} | {details} | decision |\n")
            with self.subTest(field=missing), patch.object(c_retention, "ROOT", self.project), \
                    self.assertRaisesRegex(ValueError, f"Invalid retention {missing}: .*records.md:2"):
                c_retention.records()

    def test_normal_check_consumes_xml_and_preserves_raw_failure(self):
        self.check_xml()

    def test_normal_check_reports_record_parse_failure(self):
        directory = self.project / c_retention.RECORD_DIRECTORY
        directory.mkdir(parents=True)
        (directory / "records.md").write_text(
            "| MISRA C:2012 Rule 2.5 | `source` `Name` | delivered=`generated/include/X.h`; profiles=S; producer=x | decision |\n"
        )
        with patch.object(c_retention, "ROOT", self.project):
            self.check_xml("Invalid retention shape:")

    def check_xml(self, expected_error=None):
        source = self.project / "sealed"
        (source / "meta").mkdir(parents=True)
        (source / "meta/files.sha256").write_text("seal", encoding="utf-8")
        output = self.project / "analysis"
        root = output / "project"
        self.project = root
        units = ["common.c", "batch.c", "probe.c"]
        database = [{"file": str(root / name)} for name in units]
        scope = {"profile": "ecu", "entries": {"host-batch": "batch.c", "probe": "probe.c"},
                 "cppcheckLibraries": {}}

        def prepare(*args):
            self.write(self.row)
            return root, units, database, scope

        def run(argv, directory, stage):
            commands = json.loads((directory / "compile_commands.json").read_text())
            (directory / "cache/files.txt").write_text("".join("dump:::" + row["file"] + "\n" for row in commands))
            receipts = directory / "addon-receipts"
            receipts.mkdir()
            for index, row in enumerate(commands):
                (receipts / f"{index}.json").write_text(json.dumps({"units": [row["file"]], "configurations": 1}))
            return 1, "", (f'<results><errors><error id="misra-c2012-2.5" severity="style" msg="raw finding">'
                           f'<location file="{self.row["delivered"]}" line="2" column="1"/>'
                           '</error><error id="unknown" severity="warning" msg="unknown finding"/></errors></results>')

        with patch.object(c_check, "tool_identity", return_value=(root / "cppcheck", root, {})), \
                patch.object(c_check, "coverage", return_value={"completeCompliance": False}), \
                patch.object(c_check, "semantic_identity", return_value={}), \
                patch.object(c_check, "prepare", side_effect=prepare), \
                patch.object(c_check, "run", side_effect=run), patch.object(c_check, "sealed_sources"):
            self.assertEqual(c_check.check(source, TARGET, output), 1)
        summary = json.loads((output / "summary.json").read_text())
        self.assertFalse(summary["passed"])
        self.assertFalse(summary["coverage"]["completeCompliance"])
        self.assertEqual([p["exitCode"] for p in summary["programs"]], [1, 1])
        self.assertEqual([d["id"] for d in summary["diagnostics"]], ["misra-c2012-2.5", "unknown"])
        self.assertEqual(summary["diagnostics"][0]["programs"], ["host-batch", "probe"])
        if expected_error:
            self.assertIn(expected_error, summary["error"])
            self.assertIn("records.md:1", summary["error"])
            self.assertNotIn("generatedRetention", summary)
            return
        self.assertIsNone(summary["error"])
        self.assertEqual([r["status"] for r in summary["generatedRetention"]["occurrences"]],
                         ["record_matched", "unmatched"])
        self.assertIn('id="unknown"', (output / "probe/diagnostics.xml").read_text())


if __name__ == "__main__":
    unittest.main()
