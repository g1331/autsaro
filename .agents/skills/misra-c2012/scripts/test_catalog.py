"""Exercise inventory mutations and prevent false matrix completion."""

import json
import re
import shutil
import tempfile
import unittest
from pathlib import Path

import catalog


class CatalogTests(unittest.TestCase):
    def setUp(self):
        self.index, self.rows = catalog.load_catalog()

    def test_current_catalog_matches_entire_baseline(self):
        self.assertEqual(len(self.rows), 221)
        self.assertEqual(sum(row["id"].startswith("R") for row in self.rows), 200)
        self.assertEqual(sum(row["id"].startswith("D") for row in self.rows), 21)
        categories = {row["id"]: row["category"] for row in self.rows}
        self.assertEqual(categories["R13.6"], "Required")
        self.assertEqual(categories["R17.5"], "Required")
        self.assertEqual(categories["R21.11"], "Advisory")
        self.assertEqual(categories["R21.12"], "Required")

    def assert_catalog_mutation_rejected(self, mutate):
        with tempfile.TemporaryDirectory(prefix="misra-catalog-test-") as directory:
            root = Path(directory)
            shutil.copytree(catalog.ROOT / "references", root / "references")
            mutate(root)
            with self.assertRaises(ValueError):
                catalog.load_catalog(root)

    def test_missing_guidance_is_rejected(self):
        def mutate(root):
            p = root / "references/directives.md"
            p.write_text(re.sub(r"^\| \[D3\.1\].*\n", "", p.read_text(encoding="utf-8"), flags=re.M), encoding="utf-8")
        self.assert_catalog_mutation_rejected(mutate)

    def test_duplicate_guidance_is_rejected(self):
        def mutate(root):
            p = root / "references/directives.md"
            s = p.read_text(encoding="utf-8")
            row = next(line for line in s.splitlines() if line.startswith("| [D3.1]"))
            p.write_text(s + row + "\n", encoding="utf-8")
        self.assert_catalog_mutation_rejected(mutate)

    def test_wrong_category_is_rejected(self):
        def mutate(root):
            p = root / "references/control-flow.md"
            s = p.read_text(encoding="utf-8")
            s = re.sub(r"(^\| \[R17\.5\].*?)\| Required \|", r"\1| Advisory |", s, flags=re.M)
            p.write_text(s, encoding="utf-8")
        self.assert_catalog_mutation_rejected(mutate)

    def test_baseline_identifier_tampering_is_rejected(self):
        def mutate(root):
            p = root / "references/baseline-index.json"
            data = json.loads(p.read_text(encoding="utf-8"))
            data["guideline_metadata"][0]["id"] = "D99.1"
            p.write_text(json.dumps(data), encoding="utf-8")
        self.assert_catalog_mutation_rejected(mutate)

    def test_empty_guidance_is_rejected(self):
        def mutate(root):
            p = root / "references/directives.md"
            lines = p.read_text(encoding="utf-8").splitlines()
            for i, line in enumerate(lines):
                if line.startswith("| [D3.1]"):
                    cells = line.split("|")
                    cells[5] = " "
                    lines[i] = "|".join(cells)
            p.write_text("\n".join(lines) + "\n", encoding="utf-8")
        self.assert_catalog_mutation_rejected(mutate)

    def test_revision_swap_is_rejected_even_if_counts_are_unchanged(self):
        def mutate(root):
            p = root / "references/baseline-index.json"
            data = json.loads(p.read_text(encoding="utf-8"))
            rows = data["guideline_metadata"]
            rows[0]["introduced"], rows[16]["introduced"] = rows[16]["introduced"], rows[0]["introduced"]
            p.write_text(json.dumps(data), encoding="utf-8")
        self.assert_catalog_mutation_rejected(mutate)

    def test_matrix_starts_unassessed_and_is_not_complete(self):
        matrix = catalog.create_matrix(self.rows)
        self.assertEqual(len(matrix["entries"]), 221)
        self.assertTrue(all(entry["result"] == "not_assessed" for entry in matrix["entries"]))
        self.assertEqual(len(catalog.check_matrix(matrix, self.rows)), 222)

    def test_matrix_missing_entry_is_rejected(self):
        matrix = catalog.create_matrix(self.rows)
        matrix["entries"].pop()
        with self.assertRaises(ValueError):
            catalog.check_matrix(matrix, self.rows)

    def test_mandatory_deviation_is_rejected(self):
        matrix = catalog.create_matrix(self.rows)
        entry = next(row for row in matrix["entries"] if row["id"] == "R9.1")
        entry.update(applicability="applicable", result="deviation_approved", approval="some approval")
        with self.assertRaisesRegex(ValueError, "Mandatory"):
            catalog.check_matrix(matrix, self.rows)

    def test_required_cannot_be_disapplied(self):
        matrix = catalog.create_matrix(self.rows)
        matrix["entries"][0]["effective_category"] = "Disapplied"
        with self.assertRaisesRegex(ValueError, "classification"):
            catalog.check_matrix(matrix, self.rows)

    def test_not_applicable_needs_reason_and_evidence(self):
        matrix = catalog.create_matrix(self.rows)
        entry = matrix["entries"][0]
        entry.update(applicability="not_applicable", result="not_applicable")
        issues = catalog.check_matrix(matrix, self.rows)
        self.assertIn("D1.1: reason missing", issues)
        self.assertIn("D1.1: normative basis or evidence missing", issues)

    def test_approved_deviation_needs_approval_record(self):
        matrix = catalog.create_matrix(self.rows)
        entry = matrix["entries"][0]
        entry.update(applicability="applicable", result="deviation_approved", reason="bounded requirement", normative_basis="licensed source section", evidence=["review record"])
        self.assertIn("D1.1: approval evidence missing", catalog.check_matrix(matrix, self.rows))

    def test_bookkeeping_accepts_accounted_entries_without_certifying_evidence(self):
        matrix = catalog.create_matrix(self.rows)
        matrix.update(scope="hypothetical test", target_and_configuration="hypothetical C99")
        for entry in matrix["entries"]:
            entry.update(applicability="applicable", result="pass", normative_basis="synthetic test source", evidence=["synthetic test evidence"])
        self.assertEqual(catalog.check_matrix(matrix, self.rows), [])

    def test_official_comparison_rejects_a_duplicate(self):
        contents = "\n".join(f"{'Dir' if r['id'].startswith('D') else 'Rule'} {r['id'][1:]}\t{r['category']}" for r in self.rows)
        with tempfile.TemporaryDirectory(prefix="misra-source-test-") as directory:
            p = Path(directory) / "synthetic-metadata.txt"
            p.write_text(contents, encoding="utf-8")
            catalog.compare_official(p, self.rows)
            p.write_text(contents + "\nDir 1.1\tRequired\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                catalog.compare_official(p, self.rows)


if __name__ == "__main__":
    unittest.main()
