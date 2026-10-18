"""Documentation gates reject independent negative and boundary fixtures."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import documentation as docs
from autosar_tooling import quality


class DocumentationTests(unittest.TestCase):
    def fixture(self, files):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for name, text in files.items():
            target = root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text, encoding="utf-8")
        return root, [Path(name) for name in files]

    def test_links_check_targets_anchors_references_images_and_html(self):
        root, paths = self.fixture(
            {
                "README.md": '[title](docs/page.md#中文-code--value)\n[again][ref]\n[ref]: docs/page.md#same-1\n![asset](a%20b.png)\n<a href="docs/page.md#manual">x</a>\n[dir](docs/)\n',
                "docs/page.md": '# 中文 `Code` & Value\n# Same\n# Same\n<a id="manual"></a>\n',
                "a b.png": "",
            }
        )
        self.assertEqual(docs.link_errors(root, paths), [])
        (root / "README.md").write_text(
            '[bad](missing.md)\n[bad](docs/page.md#absent)\n[x][undefined]\n<img src="missing.png">\n',
            encoding="utf-8",
        )
        errors = docs.link_errors(root, paths)
        self.assertEqual(len(errors), 4, errors)
        for number, error in enumerate(errors, 1):
            self.assertTrue(error.startswith(f"README.md:{number}:"), errors)
        self.assertIn("missing link anchor", errors[1])

    def test_code_external_and_excluded_snapshots_do_not_create_links(self):
        root, paths = self.fixture(
            {
                "README.md": "```md\n[example](missing.md)\n```\n`[example](missing.md)`\n<!-- [example](missing.md) -->\n[web](https://example.com/no) [mail](mailto:x@y)\n",
                "third_party/upstream.md": "[old](missing.md)",
                "docs/project/archive/old.md": "[old](missing.md)",
            }
        )
        self.assertEqual(docs.link_errors(root, paths), [])

    def test_deleted_untracked_ignored_and_outside_targets(self):
        root, paths = self.fixture(
            {
                "README.md": "[gone](gone.md)\n[out](../../outside)\n",
                "gone.md": "# Was here",
            }
        )
        (root / "gone.md").unlink()
        errors = docs.link_errors(root, paths)
        self.assertEqual(len(errors), 2)
        self.assertIn("missing link target", errors[0])
        self.assertIn("leaves repository", errors[1])
        (root / "gone.md").write_text("# Local ignored copy")
        self.assertEqual(len(docs.link_errors(root, [Path("README.md")])), 2)

    def test_duplicate_setext_explicit_and_unicode_anchors(self):
        self.assertEqual(
            docs.anchors('# A\n# A\n# A-1\n标题\n====\n<a name="manual"></a>'),
            {"a", "a-1", "a-1-1", "标题", "manual"},
        )

    def test_bilingual_only_existing_root_pairs_and_all_change_kinds(self):
        english, chinese = Path("README.md"), Path("README.zh-CN.md")
        known = {english, chinese, Path("CHANGELOG.md"), Path("docs/a.md")}
        self.assertEqual(docs.bilingual_errors({english, chinese}, known), [])
        self.assertEqual(
            docs.bilingual_errors({Path("CHANGELOG.md"), Path("docs/a.md")}, known), []
        )
        for changed, partner in ((english, chinese), (chinese, english)):
            self.assertEqual(
                docs.bilingual_errors({changed}, known),
                [f"{changed}: bilingual document must change together with {partner}"],
            )

    def test_changelog_added_lines_only_groups_suffix_and_continuations(self):
        source = "# Changelog\n## Unreleased\n- legacy\n### Added\n- New tool (#174)\n  continued\n### Other\n- Bad group (#2)\n### Fixed\n- No number\n## 1.0\n- No group (#3)\n"
        self.assertEqual(docs.changelog_errors(source, {5}), [])
        errors = docs.changelog_errors(source, {5, 6, 8, 10, 12})
        self.assertEqual([int(error.split(":")[1]) for error in errors], [6, 8, 10, 12])
        for group in ("Breaking", "Added", "Changed", "Fixed"):
            self.assertEqual(
                docs.changelog_errors(f"### {group}\n- Entry (#123)\n", {1, 2}), []
            )
        self.assertTrue(docs.changelog_errors("### Added\n- Entry (#1).\n", {2}))

    def test_official_manifest_names_patterns_and_non_explanations(self):
        root, paths = self.fixture(
            {
                "core/resources/official.json": json.dumps(
                    [{"path": "downloads/custom.zip"}]
                ),
                "elsewhere/custom.zip": "",
                "elsewhere/AUTOSAR_CP_SWS_Test.pdf": "",
                "AUTOSAR_FO_test.ZIP": "",
                "AUTOSAR_AP_Test.pdf": "",
                "docs/official/cache.xml": "",
                "docs/official/README.md": "# Instructions",
                "runtime/test.pdf": "",
            }
        )
        errors = docs.official_errors(root, paths)
        self.assertEqual(len(errors), 5, errors)
        self.assertTrue(
            all("must remain outside version control" in error for error in errors)
        )

    def test_real_git_baseline_includes_committed_staged_pending_deleted_and_untracked(
        self,
    ):
        root, _ = self.fixture(
            {
                "README.md": "# Hello\n",
                "README.zh-CN.md": "# 你好\n",
                "CHANGELOG.md": "# Changelog\n- old\n",
                "core/resources/official.json": "[]",
            }
        )

        def git(*args):
            return subprocess.check_output(
                ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
                cwd=root,
                text=True,
            ).strip()

        git("init", "--quiet")
        git("add", ".")
        git("commit", "--quiet", "-m", "base")
        base = git("rev-parse", "HEAD")
        (root / "README.md").write_text("# Updated\n")
        git("add", "README.md")
        git("commit", "--quiet", "-m", "topic")
        with patch.object(quality, "ROOT", root):
            self.assertEqual(len(docs.check(base)), 1)
            (root / "README.zh-CN.md").write_text("# 更新\n")
            self.assertEqual(docs.check(base), [])
            (root / "CHANGELOG.md").write_text(
                "# Changelog\n- old\n### Fixed\n- Good (#174)\n"
            )
            self.assertEqual(docs.check(base), [])
            (root / "AUTOSAR_CP_Test.pdf").write_text("not committed yet")
            self.assertEqual(len(docs.check(base)), 1)
            (root / "AUTOSAR_CP_Test.pdf").unlink()
            (root / "README.zh-CN.md").unlink()
            self.assertEqual(docs.check(base), [])
            (root / "README.md").write_text("# Hello\n")
            # Deleting only the Chinese half cannot erase the known baseline pair.
            self.assertEqual(len(docs.check(base)), 1)


if __name__ == "__main__":
    unittest.main()
