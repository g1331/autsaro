"""Protect the incremental format gate from hiding changed-line violations."""

import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import quality


class QualityTests(unittest.TestCase):
    def test_first_commit_baseline_checks_every_added_line(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(['git', 'init', '--quiet', str(root)], check=True)
            baseline = subprocess.check_output(
                ['git', 'hash-object', '-w', '-t', 'tree', '--stdin'],
                cwd=root, input=b'',
            ).decode().strip()
            path = Path('scripts/new.py')
            (root / path).parent.mkdir()
            (root / path).write_text('first = 1\nsecond = 2\n', encoding='utf-8')
            subprocess.run(['git', 'add', path.as_posix()], cwd=root, check=True)
            with patch.object(quality, 'ROOT', root):
                self.assertEqual(quality.changed_lines(path, baseline, set()), {1, 2})

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
