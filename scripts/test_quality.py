"""Protect the incremental format gate from hiding changed-line violations."""

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import quality


class QualityTests(unittest.TestCase):
    def test_changed_line_formatting_fails_without_rejecting_legacy_lines(self):
        path = Path("core/src/example.rs")
        source = "fn old( ){ }\nfn new( ){ }\n"
        formatted = "fn old() {}\nfn new() {}\n"
        self.assertEqual(
            quality.changed_format_errors(path, source, formatted, {1}),
            ["core/src/example.rs:1: format differs from configured formatter"],
        )
        self.assertEqual(
            quality.changed_format_errors(path, source, formatted, {3}), []
        )

    def test_hygiene_rejects_trailing_space_and_python_syntax(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path("scripts/broken.py")
            target = Path(temporary) / path
            target.parent.mkdir()
            target.write_bytes(b"if True:  \n")
            with patch.object(quality, "ROOT", Path(temporary)):
                errors = quality.hygiene_errors(path)
        self.assertTrue(any("trailing whitespace" in error for error in errors))
        self.assertTrue(any("Python syntax error" in error for error in errors))

    def test_diff_hunk_counts_added_lines(self):
        diff = "@@ -2,0 +3,2 @@\n+a\n+b\n@@ -8 +10,0 @@\n-old\n"
        lines = set()
        for match in quality.HUNK.finditer(diff):
            start = int(match.group(1))
            count = int(match.group(2) or "1")
            lines.update(range(start, start + count))
        self.assertEqual(lines, {3, 4})


if __name__ == "__main__":
    unittest.main()
