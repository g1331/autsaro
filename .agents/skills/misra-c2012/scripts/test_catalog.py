"""Exercise inventory mutations and the read-only catalog command contract."""

import json
import re
import shutil
import subprocess
import sys
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
            p.write_text(re.sub(r"^\| \[D3\.1\].*\n", "", p.read_text(encoding="utf-8"), flags=re.MULTILINE), encoding="utf-8")
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
            s = re.sub(r"(^\| \[R17\.5\].*?)\| Required \|", r"\1| Advisory |", s, flags=re.MULTILINE)
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

    def run_catalog(self, *arguments):
        return subprocess.run(
            [sys.executable, str(catalog.ROOT / "scripts/catalog.py"), *arguments],
            capture_output=True, text=True, encoding="utf-8", check=False,
        )

    def test_query_filters_exact_identifiers(self):
        result = self.run_catalog("query", "--id", "R17.3", "--id", "R21.4")
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = json.loads(result.stdout)
        self.assertEqual({row["id"] for row in rows}, {"R17.3", "R21.4"})
        self.assertTrue(all(row["condition"] and row["action"] and row["check"] for row in rows))

    def test_query_intersects_group_and_identifier(self):
        result = self.run_catalog("query", "--group", "libraries", "--id", "R21.6", "--id", "R17.3")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([row["id"] for row in json.loads(result.stdout)], ["R21.6"])

    def test_query_rejects_unknown_identifier_and_empty_selection(self):
        for arguments in (("--id", "R99.1"), ("--group", "missing")):
            with self.subTest(arguments=arguments):
                result = self.run_catalog("query", *arguments)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertIn("ERROR:", result.stderr)

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
