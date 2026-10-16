"""Protect the incremental format gate from hiding changed-line violations."""

import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import quality


class QualityTests(unittest.TestCase):
    def test_clang_format_and_clang_tidy_must_share_one_version(self):
        def lock(**versions):
            body = "".join(f'[[package]]\nname = "{name}"\nversion = "{version}"\n\n' for name, version in versions.items())
            path = Path(temporary) / "uv.lock"
            path.write_text("version = 1\n\n" + body, encoding="utf-8")
            return path
        with tempfile.TemporaryDirectory() as temporary:
            self.assertEqual(quality.paired_tool_errors(lock(**{"clang-format": "22.1.8", "clang-tidy": "22.1.8", "ruff": "0.16.9"})), [])
            self.assertEqual(
                quality.paired_tool_errors(lock(**{"clang-format": "22.1.9", "clang-tidy": "22.1.8"})),
                ["uv.lock: clang-format 22.1.9, clang-tidy 22.1.8; upgrade clang-format and clang-tidy together to the same LLVM release"],
            )
            self.assertEqual(len(quality.paired_tool_errors(lock(**{"clang-format": "22.1.8"}))), 1)
        self.assertEqual(quality.paired_tool_errors(), [])

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

    def test_merge_base_compares_from_the_fork_not_the_advanced_branch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            git = ['git', '-c', 'user.name=t', '-c', 'user.email=t@t']
            subprocess.run(['git', 'init', '--quiet', '-b', 'trunk', str(root)], check=True)
            subprocess.run([*git, 'commit', '--quiet', '--allow-empty', '-m', 'fork'], cwd=root, check=True)
            fork = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
            subprocess.run(['git', 'switch', '--quiet', '-c', 'topic'], cwd=root, check=True)
            subprocess.run([*git, 'commit', '--quiet', '--allow-empty', '-m', 'topic'], cwd=root, check=True)
            subprocess.run(['git', 'switch', '--quiet', 'trunk'], cwd=root, check=True)
            subprocess.run([*git, 'commit', '--quiet', '--allow-empty', '-m', 'advanced'], cwd=root, check=True)
            subprocess.run(['git', 'switch', '--quiet', 'topic'], cwd=root, check=True)
            with patch.object(quality, 'ROOT', root), patch.object(quality, 'check', return_value=[]) as check:
                self.assertEqual(quality.main(['--merge-base', 'trunk']), 0)
                self.assertEqual(check.call_args.args[0], fork)
                self.assertEqual(quality.main(['--merge-base', 'missing']), 2)

    def test_renamed_file_counts_only_its_edited_lines(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            git = ['git', '-c', 'user.name=t', '-c', 'user.email=t@t']
            subprocess.run(['git', 'init', '--quiet', str(root)], check=True)
            (root / 'old.py').write_text('first = 1\nsecond = 2\nthird = 3\n', encoding='utf-8')
            subprocess.run(['git', 'add', '.'], cwd=root, check=True)
            subprocess.run([*git, 'commit', '--quiet', '-m', 'base'], cwd=root, check=True)
            subprocess.run(['git', 'mv', 'old.py', 'new.py'], cwd=root, check=True)
            (root / 'new.py').write_text('first = 1\nsecond = 20\nthird = 3\n', encoding='utf-8')
            with patch.object(quality, 'ROOT', root):
                renamed = quality.renamed_sources('HEAD')
                self.assertEqual(renamed, {Path('new.py'): Path('old.py')})
                self.assertEqual(quality.changed_lines(Path('new.py'), 'HEAD', set(), renamed), {2})

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

    def test_plan_numbers_in_names_and_text_fail_outside_exempt_records(self):
        # Assembled so this test file does not contain the numbers it rejects.
        epic, story = "ep" + "ic", "st" + "ory"
        files = {
            f"core/tests/fixtures/{epic}4/input.arxml": "<A/>\n",
            "core/src/clean.rs": f"// {epic.title()} 9 milestone\nfn main() {{}}\n",
            "tests/python/notes.py": f"KEY = '{story}-4.11'\n",
            "core/src/retired.rs": f'const P: &str = "{epic}4-win64-sr-cs-v1";\n',
            "docs/adr/0001-record.md": f"Source: architecture/{epic}-4/SPINE.md\n",
            "docs/images/picture.png": f"\0{epic}4".encode(),
            "core/src/composition.rs": "// epicycle and storyline are words\n",
        }
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, content in files.items():
                (root / name).parent.mkdir(parents=True, exist_ok=True)
                data = content if isinstance(content, bytes) else content.encode()
                (root / name).write_bytes(data)
            with patch.object(quality, "ROOT", root):
                errors = quality.plan_number_errors([Path(name) for name in files])
        self.assertEqual(
            errors,
            [
                f"core/tests/fixtures/{epic}4/input.arxml: development plan number '{epic}4' in path",
                f"core/src/clean.rs:1: development plan number '{epic.title()} 9'",
                f"tests/python/notes.py:1: development plan number '{story}-4'",
            ],
        )

    def test_oversized_documents_fail_outside_exempt_records(self):
        limit = quality.DOCUMENT_LIMIT
        files = {
            "docs/large.md": b"x" * (limit + 1),
            "docs/limit.md": b"x" * limit,
            # CRLF line ends of a Windows checkout do not count.
            "docs/crlf.md": b"x\r\n" * (limit // 2),
            "docs/adr/0001-record.md": b"x" * (limit + 1),
            "CHANGELOG.md": b"x" * (limit + 1),
            "docs/large.txt": b"x" * (limit + 1),
        }
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, content in files.items():
                (root / name).parent.mkdir(parents=True, exist_ok=True)
                (root / name).write_bytes(content)
            with patch.object(quality, "ROOT", root):
                errors = quality.document_size_errors([Path(name) for name in files])
        self.assertEqual(len(errors), 1)
        self.assertTrue(errors[0].startswith(f"docs/large.md: {limit + 1} bytes exceeds"), errors)

    def test_runtime_c_files_open_with_the_file_description(self):
        files = {
            "runtime/one_line.c": b"/** @file Owns the state. */\nint x;\n",
            "runtime/block.h": b"/** @file\r\n * @brief Declares it.\r\n */\r\n",
            "runtime/missing.c": b"#include <stdint.h>\n/** @file Too late. */\n",
            "runtime/named.c": b"/** @filed */\n",
        }
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, content in files.items():
                (root / name).parent.mkdir(parents=True, exist_ok=True)
                (root / name).write_bytes(content)
            with patch.object(quality, "ROOT", root):
                failing = [name for name in files if quality.file_header_errors(Path(name))]
        self.assertEqual(failing, ["runtime/missing.c", "runtime/named.c"])

    def test_clang_tidy_comparison_counts_each_finding_against_the_baseline(self):
        from collections import Counter

        clone = ("bugprone-x", "runtime/a.c", "} else {")
        size = ("readability-function-size", "runtime/a.c", "void f(void)")
        fixed = ("bugprone-y", "runtime/b.c", "x = y;")
        # A second identical line is a new finding even though the text is already listed.
        errors = quality.clang_tidy_comparison(
            Counter({clone: 2, size: 1}), Counter({clone: 1, size: 1, fixed: 1})
        )
        self.assertEqual(len(errors), 2, errors)
        self.assertIn("runtime/a.c: new clang-tidy finding [bugprone-x]: } else {", errors)
        self.assertTrue(errors[1].endswith("fixed finding, remove: bugprone-y runtime/b.c x = y;"))

    def test_committed_clang_tidy_baseline_parses(self):
        baseline = quality.read_clang_tidy_baseline()
        self.assertTrue(baseline)
        self.assertTrue(all(name.startswith(("runtime/", "generated/")) for _, name, _ in baseline))

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
