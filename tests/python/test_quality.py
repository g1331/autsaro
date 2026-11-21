"""Protect the incremental format gate from hiding changed-line violations."""

import json
import os
import shlex
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import quality


class ContributionTests(unittest.TestCase):
    def test_branch_prefix_is_case_insensitive_and_checks_token_boundaries(self):
        for branch in (
            "t3code/naming", "T3-Code.namer", "CLAUDE-code/refs", "codex", "codex_fix",
            "cursor-issue", "antigravity.topic", "copilot/task", "gemini-task", "windsurf/task",
        ):
            with self.subTest(branch=branch):
                errors = quality.branch_name_errors(branch)
                self.assertEqual(len(errors), 1)
                self.assertIn(repr(branch), errors[0])
                self.assertIn("git branch -m feat/describe-change", errors[0])
        for branch in (
            "feat/enforce-pr-branch-hygiene", "fix/codex-import", "cursorial-motion",
            "t3codec", "master", "main", "release", "release/1.2",
        ):
            with self.subTest(branch=branch):
                self.assertEqual(quality.branch_name_errors(branch), [])

    def test_body_accepts_plain_reference_paragraphs(self):
        for body in (
            "Closes #1",
            "## Change\n\nRefs #509\n\nImplemented the checks.",
            "Refs #509\r\nCloses #504  \r\n",
            "<!-- Explain the change here -->\n\nrEfS\t#509",
            "```\nCloses #12\n```\n\nRefs #509",
        ):
            with self.subTest(body=body):
                self.assertEqual(quality.pull_request_body_errors(body), [])

    def test_body_rejects_missing_malformed_and_example_references(self):
        for body in (
            "", "Fixes #509", "Refs #0", "Refs #-12", "Refs #1a", "Refs #12.5", "Refs #N",
            "Refs owner/repo#12", "Refs \\#509", "Refs &#35;509",
            "<!--\nCloses #509\n-->", "Example <!--\nRefs #509\n-->",
            "````markdown\n```\nCloses #509\n````", "~~~\nRefs #509",
            "`Closes #509`", "``an example\nRefs #509\n``",
            "    Refs #509", "> Example\nRefs #509", "- Example\n\n  Refs #509",
            "# Refs #509", "Refs #509\n---------",
            "| Directive |\n| --- |\n| Refs #509 |", "[Refs #509](https://example.com)",
            "Use Refs #509 in the PR description.",
        ):
            with self.subTest(body=body):
                errors = quality.pull_request_body_errors(body)
                self.assertEqual(len(errors), 1)
                self.assertIn("PR body is missing an issue reference", errors[0])
                self.assertIn("Closes #123", errors[0])
                self.assertIn("Refs #123", errors[0])

    def test_local_branch_and_detached_head_use_real_git_refs(self):
        with tempfile.TemporaryDirectory() as temporary, patch.dict(os.environ):
            os.environ.pop("GITHUB_EVENT_NAME", None)
            root = Path(temporary)

            def git(*args):
                subprocess.run(
                    ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
                    cwd=root, check=True, capture_output=True,
                )

            git("init", "--quiet", "-b", "codex/example")
            git("commit", "--allow-empty", "-m", "test: seed checkout")
            # An identically named tag must not hide the branch behind Git's short-ref disambiguation.
            git("tag", "codex/example")
            with patch.object(quality, "ROOT", root):
                self.assertIn("codex/example", quality.contribution_errors()[0])
                for event in ("push", "workflow_dispatch", "release"):
                    with patch.dict(os.environ, {"GITHUB_EVENT_NAME": event}):
                        self.assertEqual(quality.contribution_errors(), [])
                git("switch", "--detach", "HEAD")
                self.assertEqual(quality.contribution_errors(), [])

    def test_pr_event_uses_source_branch_and_body_instead_of_checkout(self):
        with tempfile.TemporaryDirectory() as temporary:
            event = Path(temporary) / "event.json"
            environment = {"GITHUB_EVENT_NAME": "pull_request", "GITHUB_EVENT_PATH": str(event)}
            with patch.dict(os.environ, environment), patch.object(quality, "run") as git:
                for branch, body, expected in (
                    ("t3code/example", None, ("t3code/example", "PR body is missing")),
                    ("t3code/example", "Refs #509", ("t3code/example",)),
                    ("feat/check-pr", "<!-- Refs #509 -->", ("PR body is missing",)),
                    ("feat/check-pr", "Refs #509", ()),
                ):
                    with self.subTest(branch=branch, body=body):
                        event.write_text(json.dumps({
                            "pull_request": {
                                "head": {"ref": branch}, "base": {"ref": "master"}, "body": body,
                            },
                        }), encoding="utf-8")
                        errors = quality.contribution_errors()
                        self.assertEqual(len(errors), len(expected))
                        for message, error in zip(expected, errors):
                            self.assertIn(message, error)
                event.write_text('{"pull_request": {}}', encoding="utf-8")
                with self.assertRaisesRegex(RuntimeError, "Invalid GitHub pull_request event"):
                    quality.contribution_errors()
                git.assert_not_called()

    def test_pr_body_edits_trigger_the_same_required_quality_job(self):
        import yaml

        workflow = yaml.load(
            (quality.ROOT / ".github/workflows/checks.yml").read_text(), Loader=yaml.BaseLoader,
        )
        self.assertIn("edited", workflow["on"]["pull_request"]["types"])
        self.assertIn("quality", workflow["jobs"]["required"]["needs"])
        self.assertTrue(any(
            "autosar_tooling quality --scope all" in step.get("run", "")
            for step in workflow["jobs"]["quality"]["steps"]
        ))

    def test_pr_body_edit_checks_current_text_instead_of_previous_reference(self):
        with tempfile.TemporaryDirectory() as temporary:
            event = Path(temporary) / "event.json"
            environment = {"GITHUB_EVENT_NAME": "pull_request", "GITHUB_EVENT_PATH": str(event)}
            payload = {
                "action": "edited",
                "changes": {"body": {"from": "Closes #631"}},
                "pull_request": {"head": {"ref": "fix/ci-cancelled-required-status"}, "body": ""},
            }
            with patch.dict(os.environ, environment):
                event.write_text(json.dumps(payload), encoding="utf-8")
                self.assertIn("PR body is missing an issue reference", quality.contribution_errors()[0])
                payload["pull_request"]["body"] = "Refs #509"
                event.write_text(json.dumps(payload), encoding="utf-8")
                self.assertEqual(quality.contribution_errors(), [])


class CommitMessageTests(unittest.TestCase):
    def test_source_quality_propagates_contribution_and_commit_failures(self):
        from contextlib import ExitStack

        from autosar_tooling import c_standard, documentation

        with ExitStack() as stack:
            for name in (
                'source_paths', 'scoped_paths', 'plan_number_errors',
                'document_size_errors', 'paired_tool_errors',
            ):
                stack.enter_context(patch.object(quality, name, return_value=[]))
            stack.enter_context(patch.object(quality, 'renamed_sources', return_value={}))
            for name in ('runtime_findings', 'deviation_findings', 'baseline_errors'):
                stack.enter_context(patch.object(c_standard, name, return_value=set()))
            stack.enter_context(patch.object(documentation, 'check', return_value=[]))
            stack.enter_context(patch.object(
                quality, 'contribution_errors', return_value=['PR body is missing an issue reference'],
            ))
            commits = stack.enter_context(patch.object(
                quality, 'commit_range_errors', return_value=['commit abc12345: invalid title'],
            ))
            self.assertEqual(quality.check('base', commit_head='topic'), [
                'PR body is missing an issue reference', 'commit abc12345: invalid title',
            ])
            commits.assert_called_once_with('base', 'topic')

    def test_conventional_titles_and_breaking_footer(self):
        for message in (
            "feat: add support", "fix(dcm): handle timeout", "ci(workflows): check messages",
            "feat(rte)!: change contract\n\nBREAKING CHANGE: regenerate clients\n",
            "docs: describe usage\n\nRefs #12\n", "chore: update tools",
        ):
            with self.subTest(message=message):
                self.assertEqual(quality.commit_message_errors(message, "abc12345"), [])
        for message in (
            "add support", "Feat: add support", "style: tidy", "fix(): handle timeout",
            "fix(bad scope): handle timeout", "fix: ", "feat!: change contract",
        ):
            with self.subTest(message=message):
                errors = quality.commit_message_errors(message, "abc12345")
                self.assertTrue(errors)
                self.assertIn("abc12345", errors[0])
                self.assertIn(repr(message), errors[0])

    def test_all_closing_keywords_and_reference_forms_in_title_and_body(self):
        for keyword in (
            "close", "closes", "closed", "fix", "fixes", "fixed",
            "resolve", "resolves", "resolved",
        ):
            for reference in ("#12", "owner/repo#12", "https://github.com/owner/repo/issues/12"):
                for text in (
                    f"{keyword} {reference}", f"{keyword.upper()}: {reference}",
                    f"{keyword}:{reference}",
                ):
                    for message in (f"fix: {text}", f"fix: handle timeout\n\n{text}\n"):
                        with self.subTest(message=message):
                            errors = quality.commit_message_errors(message, "abc12345")
                            self.assertEqual(len(errors), 1, errors)
                            self.assertIn("closing keyword", errors[0])
                            self.assertIn(text, errors[0])
        for text in ("Refs #12", "prefix #12", "fix timeout", "closes /tmp/issues/12"):
            self.assertEqual(
                quality.commit_message_errors(f"fix: handle timeout\n\n{text}", "a"), [],
            )

    def test_merge_and_generated_revert_exemptions(self):
        self.assertEqual(quality.commit_message_errors(
            "Merge branch 'topic'\nCloses #12", "a", merge=True,
        ), [])
        revert = 'Revert "fix: handle timeout"\n\nThis reverts commit ' + 'a' * 40 + '.\n'
        self.assertEqual(quality.commit_message_errors(revert, "a"), [])
        self.assertEqual(quality.commit_message_errors(
            revert + '\nSigned-off-by: Test <test@example.com>\n', 'a',
        ), [])
        self.assertTrue(quality.commit_message_errors('Revert "arbitrary bypass"', "a"))
        self.assertTrue(quality.commit_message_errors("Merge branch 'fake'", "a"))

    def test_real_pr_range_excludes_target_commits_and_checks_every_topic_commit(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)

            def git(*args):
                return subprocess.check_output(
                    ['git', '-c', 'user.name=t', '-c', 'user.email=t@t', *args],
                    cwd=root, text=True, stderr=subprocess.STDOUT, input='',
                ).strip()

            git('init', '--quiet', '-b', 'master')
            git('commit', '--quiet', '--allow-empty', '-m', 'legacy title')
            base = git('rev-parse', 'HEAD')
            git('switch', '--quiet', '-c', 'topic')
            (root / 'topic-file').write_text('topic\n')
            git('add', 'topic-file')
            git('commit', '--quiet', '--allow-empty', '-m', 'fix: fix #12')
            invalid = git('rev-parse', 'HEAD')
            git('commit', '--quiet', '--allow-empty', '-m', 'test: cover messages')
            head = git('rev-parse', 'HEAD')
            git('switch', '--quiet', 'master')
            (root / 'target-file').write_text('target\n')
            git('add', 'target-file')
            git('commit', '--quiet', '--allow-empty', '-m', 'unrelated legacy target')
            target = git('rev-parse', 'HEAD')
            git('merge', '--quiet', '--no-ff', 'topic', '-m', 'Merge PR\n\nCloses #12')
            with patch.object(quality, 'ROOT', root):
                for start, end in ((base, head), (target, head), (target, 'HEAD')):
                    errors = quality.commit_range_errors(start, end)
                    self.assertEqual(len(errors), 1, errors)
                    self.assertIn(invalid[:8], errors[0])
                    self.assertIn('fix: fix #12', errors[0])
                self.assertEqual(quality.commit_range_errors(head, head), [])
                empty = git('hash-object', '-w', '-t', 'tree', '--stdin')
                self.assertEqual(len(quality.commit_range_errors(empty, base)), 1)
                with self.assertRaises(RuntimeError):
                    quality.commit_range_errors('missing', head)
            merge = git('rev-parse', 'HEAD')
            git('revert', '--no-edit', '-m', '1', 'HEAD')
            with patch.object(quality, 'ROOT', root):
                self.assertEqual(quality.commit_range_errors(merge), [])
            (root / 'file').write_text('content\n')
            git('add', 'file')
            git('commit', '--quiet', '-m', 'test: add file')
            before_revert = git('rev-parse', 'HEAD')
            git('revert', '--no-edit', 'HEAD')
            with patch.object(quality, 'ROOT', root):
                self.assertEqual(quality.commit_range_errors(before_revert), [])

    def test_required_ci_passes_the_real_pr_head_with_full_history(self):
        import yaml

        workflow = yaml.safe_load((quality.ROOT / '.github/workflows/checks.yml').read_text())
        job = workflow['jobs']['quality']
        checkout = next(
            s for s in job['steps'] if s.get('uses', '').startswith('actions/checkout@')
        )
        self.assertEqual(checkout['with']['fetch-depth'], 0)
        step = next(s for s in job['steps'] if '--commit-head' in s.get('run', ''))
        self.assertIn('--base "$CHECK_BASE"', step['run'])
        self.assertIn('--commit-head "$COMMIT_HEAD"', step['run'])
        self.assertEqual(
            step['env']['COMMIT_HEAD'], '${{ github.event.pull_request.head.sha || github.sha }}',
        )
        self.assertIn('quality', workflow['jobs']['required']['needs'])

    def test_installed_commit_msg_hook_uses_quality_cli(self):
        import yaml

        config = yaml.safe_load((quality.ROOT / '.pre-commit-config.yaml').read_text())
        hook = next(h for h in config['repos'][0]['hooks'] if h['id'] == 'commit-message')
        self.assertEqual(
            hook['entry'], 'uv run --locked python -m autosar_tooling quality --commit-msg',
        )
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(['git', 'init', '--quiet', str(root)], check=True)
            hook['entry'] = shlex.join([
                sys.executable, '-m', 'autosar_tooling', 'quality', '--commit-msg',
            ])
            config['repos'][0]['hooks'] = [hook]
            (root / '.pre-commit-config.yaml').write_text(json.dumps(config), encoding='utf-8')
            environment = {
                **os.environ, 'PRE_COMMIT_HOME': str(root / 'hook-cache'),
                'PYTHONPATH': str(quality.ROOT / 'tools/python/src'),
            }
            subprocess.run(
                [sys.executable, '-m', 'pre_commit', 'install', '--hook-type', 'commit-msg'],
                cwd=root, env=environment, check=True, capture_output=True,
            )
            for message, valid in (
                ('fix: handle timeout\n\nRefs #12', True), ('missing type', False),
                ('fix: handle timeout\n\nCloses #12', False),
            ):
                result = subprocess.run(
                    ['git', '-c', 'user.name=t', '-c', 'user.email=t@t',
                     'commit', '--allow-empty', '-m', message],
                    cwd=root, env=environment, capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode == 0, valid, result.stdout + result.stderr)
                if not valid:
                    self.assertIn('COMMIT_EDITMSG', result.stdout + result.stderr)
            merge_path = root / '.git/MERGE_HEAD'
            merge_path.write_text('a' * 40 + '\n')
            message_file = root / '.git/COMMIT_EDITMSG'
            message_file.write_text("Merge branch 'topic'\n\nCloses #12\n")
            with patch.object(quality, 'ROOT', root):
                self.assertEqual(quality.main(['--commit-msg', str(message_file)]), 0)


class QualityTests(unittest.TestCase):
    def test_command_failure_keeps_stage_command_raw_exit_and_both_streams(self):
        with patch.object(quality.subprocess, "run", return_value=subprocess.CompletedProcess(
            ["git"], 23, b"producer stdout", b"producer stderr",
        )):
            result = quality.run(["git", "diff", "missing"])
        self.assertEqual(result.returncode, 23)
        for text in ("quality:", "argv=['git', 'diff', 'missing']", "exit_code=23", "producer stdout", "producer stderr"):
            self.assertIn(text, result.stderr)
        with patch.object(quality.subprocess, "run", side_effect=subprocess.TimeoutExpired(
            ["git"], 60, output=b"before hang", stderr=b"hung producer",
        )), self.assertRaises(RuntimeError) as failure:
            quality.run(["git", "diff"])
        for text in ("quality:", "argv=", "status=timeout", "exit_code=None", "before hang", "hung producer"):
            self.assertIn(text, str(failure.exception))

    def test_clang_tidy_failure_prints_command_and_exit_once(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary) / "sample"
            (project / "kernel").mkdir(parents=True)
            (project / "meta").mkdir()
            (project / "target.json").write_text(json.dumps({
                "target": "linux-test", "kernelPatches": [], "includePaths": ["."],
                "sources": ["unit.c"], "entries": {}, "compilerFlags": ["-std=c99"],
            }))
            (project / "meta/integration.json").write_text('{"originalBswSources":{}}')
            with patch.object(quality.subprocess, "run", return_value=subprocess.CompletedProcess(
                ["clang-tidy"], 23, b"producer stdout", b"producer stderr",
            )), self.assertRaises(RuntimeError) as failure:
                quality.clang_tidy_project(project, "clang-tidy")
            message = str(failure.exception)
            self.assertIn("sample: clang-tidy could not analyze unit.c", message)
            self.assertEqual(message.count("argv="), 1)
            self.assertEqual(message.count("exit_code=23"), 1)
            self.assertIn("producer stderr", message)

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

    def test_real_git_worktree_content_and_installed_pre_push_wrapper(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "source"
            remote = Path(temporary) / "remote.git"
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            subprocess.run(["git", "init", "--quiet", "--bare", str(remote)], check=True)

            def git(*args, check=True):
                return subprocess.run(
                    ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
                    cwd=root, check=check, capture_output=True, text=True,
                )

            path = Path("scripts/source.py")
            target = root / path
            target.parent.mkdir()
            clean, invalid = b"value = 1\n", b"value = 1 \n"
            target.write_bytes(clean)
            # Exercise the installed pre-push wrapper with the real source-selection,
            # byte-reading and hygiene chain, without running unrelated C checks.
            probe = root / ".git" / "quality_probe.py"
            probe.write_text(
                "import json\nfrom pathlib import Path\n"
                f"import sys\nsys.path.insert(0, {str(quality.ROOT / 'tools/python/src')!r})\n"
                "from autosar_tooling import quality\nquality.ROOT = Path.cwd()\n"
                "errors = [e for p in quality.scoped_paths('all') for e in quality.hygiene_errors(p)]\n"
                "print('HYGIENE=' + json.dumps(errors))\nraise SystemExit(bool(errors))\n",
                encoding="utf-8",
            )
            config = {
                "repos": [{"repo": "local", "hooks": [{
                    "id": "quality", "name": "quality", "language": "system",
                    "entry": shlex.join([sys.executable, str(probe)]),
                    "pass_filenames": False, "always_run": True,
                    "verbose": True, "stages": ["pre-push"],
                }]}],
            }
            (root / ".pre-commit-config.yaml").write_text(json.dumps(config), encoding="utf-8")
            (root / ".gitattributes").write_text("*.py text eol=lf\n", encoding="utf-8")
            git("add", ".")
            git("commit", "--quiet", "-m", "clean source")
            clean_head = git("rev-parse", "HEAD").stdout.strip()
            git("remote", "add", "origin", str(remote))
            environment = {**os.environ, "PRE_COMMIT_HOME": str(Path(temporary) / "hooks")}
            subprocess.run(
                [sys.executable, "-m", "pre_commit", "install", "--hook-type", "pre-push"],
                cwd=root, env=environment, check=True, capture_output=True,
            )
            expected = ["scripts/source.py:1: trailing whitespace"]
            cases = (
                ("clean", clean, clean, [], []),
                ("unstaged", clean, invalid, expected, []),
                ("staged_masked", invalid, clean, [], expected),
                ("committed_invalid", invalid, invalid, expected, expected),
                ("staged_fix", clean, invalid, expected, []),
                ("untracked", clean, clean, ["scripts/local.py:1: trailing whitespace"],
                 ["scripts/local.py:1: trailing whitespace"]),
            )
            for name, indexed, working, direct_errors, hook_errors in cases:
                with self.subTest(name=name):
                    git("reset", "--hard", clean_head)
                    git("commit", "--quiet", "--allow-empty", "-m", name)
                    if name in {"committed_invalid", "staged_fix"}:
                        target.write_bytes(invalid)
                        git("add", path.as_posix())
                        git("commit", "--quiet", "-m", "invalid committed source")
                    target.write_bytes(indexed)
                    git("add", path.as_posix())
                    target.write_bytes(working)
                    local = root / "scripts/local.py"
                    if name == "untracked":
                        local.write_bytes(invalid)
                    with patch.object(quality, "ROOT", root):
                        paths = quality.scoped_paths("all")
                        self.assertEqual(paths, [Path("scripts/local.py"), path] if local.exists() else [path])
                        self.assertEqual(
                            [e for p in paths for e in quality.hygiene_errors(p)], direct_errors,
                        )
                        if name == "unstaged":
                            self.assertEqual(quality.changed_lines(path, "HEAD", set()), {1})
                        if name == "untracked":
                            self.assertEqual(quality.source_paths(untracked_only=True), [Path("scripts/local.py")])
                    before = git("status", "--porcelain").stdout
                    result = subprocess.run(
                        ["git", "push", "origin", f"HEAD:refs/heads/{name}"],
                        cwd=root, env=environment, capture_output=True, text=True, check=False,
                    )
                    self.assertEqual(result.returncode, 1 if hook_errors else 0, result.stdout + result.stderr)
                    self.assertIn("HYGIENE=" + json.dumps(hook_errors), result.stdout + result.stderr)
                    self.assertEqual(git("status", "--porcelain").stdout, before)
                    self.assertEqual(target.read_bytes(), working)
                    self.assertEqual(git("show", f":{path.as_posix()}").stdout.encode(), indexed)

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
            "AGENTS.md": b"x" * (limit + 1),
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
        self.assertEqual(len(errors), 2, errors)
        self.assertTrue(errors[0].startswith(f"docs/large.md: {limit + 1} bytes exceeds"), errors)
        self.assertTrue(errors[1].startswith("AGENTS.md: "), errors)

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

    def test_clang_tidy_trailing_documentation_preserves_code_and_counts(self):
        from collections import Counter

        def entry(text, count=1):
            return Counter({("readability-identifier-naming", "runtime/os/include/Test.h", text): count})

        original = entry("uint8 StandardName;")
        documented = entry("uint8 StandardName; /**< Owned by the calling actor. */")
        self.assertEqual(quality.clang_tidy_comparison(documented, original), [])
        self.assertEqual(quality.clang_tidy_comparison(original, documented), [])
        self.assertEqual(quality.clang_tidy_comparison(
            entry("#define SetEvent Os_SetEvent /**< Standard service alias. */"),
            entry("#define SetEvent Os_SetEvent"),
        ), [])
        self.assertTrue(quality.clang_tidy_comparison(
            entry('#define TEXT "/**< changed literal */"'), entry('#define TEXT "original"'),
        ))
        for changed in (
            entry("uint8 OtherName; /**< Owned by the calling actor. */"),
            entry("uint8 StandardName; /* ordinary comment */"),
            entry("uint8 StandardName; /**< Owned by the calling actor. */", 2),
            entry('const char *name = "/**< changed literal */";'),
            entry("uint8 /**< embedded documentation */ StandardName;"),
        ):
            self.assertTrue(quality.clang_tidy_comparison(changed, original))

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


class ClangTidyReportTests(unittest.TestCase):
    def test_optional_report_preserves_raw_existing_findings_and_failure(self):
        import collections
        import os

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'sample').mkdir()
            (root / 'sample/target.json').write_text('{}')
            destination = root / 'reports/tidy.json'
            raw = {'rule': 'readability-test', 'file': 'runtime/test.h', 'line': 7, 'message': 'raw message'}
            def analyze(project, tidy, *, report=None):
                if report is not None:
                    report.append(raw)
                return collections.Counter({('readability-test', 'runtime/test.h', 'int x;'): 1})
            baseline = collections.Counter({('readability-test', 'runtime/test.h', 'int x;'): 1})
            with patch.dict(os.environ, {'AUTOSAR_CLANG_TIDY_REPORT': str(destination)}), patch.object(quality.shutil, 'which', return_value='tidy'), patch.object(quality, 'clang_tidy_project', side_effect=analyze), patch.object(quality, 'read_clang_tidy_baseline', return_value=baseline):
                self.assertEqual(quality.clang_tidy_errors(root), [])
                self.assertEqual(json.loads(destination.read_text()), [raw])
            def fail(project, tidy, *, report=None):
                report.append(raw)
                raise RuntimeError('analysis failed')
            with patch.dict(os.environ, {'AUTOSAR_CLANG_TIDY_REPORT': str(destination)}), patch.object(quality.shutil, 'which', return_value='tidy'), patch.object(quality, 'clang_tidy_project', side_effect=fail), self.assertRaisesRegex(RuntimeError, 'analysis failed'):
                quality.clang_tidy_errors(root)
            self.assertEqual(json.loads(destination.read_text()), [raw])

    def test_raw_report_parser_includes_baseline_excluded_rules_and_sample(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary) / 'sample'
            (project / 'kernel').mkdir(parents=True)
            (project / 'meta').mkdir()
            (project / 'bsw').mkdir()
            (project / 'bsw/Test.h').write_text('int generated;\n')
            (project / 'unit.c').write_text('int runtime;\n')
            (project / 'target.json').write_text(json.dumps({
                'target': 'linux-test', 'kernelPatches': [], 'includePaths': ['.'],
                'sources': ['unit.c'], 'entries': {}, 'compilerFlags': ['-std=c99'],
            }))
            (project / 'meta/integration.json').write_text(json.dumps({'originalBswSources': {'runtime/Test.c': 'unit.c'}}))
            output = f'{project}/unit.c:1:2: warning: Runtime message [check-one]\n{project}/bsw/Test.h:1:1: warning: Generated message [readability-identifier-naming]\n'
            output += f'{project}/unit.c:1:3: warning: Other column [check-one]\n{project}/unit.c:1:2: warning: Runtime message [check-one]\n'
            report = []
            with patch.object(quality, 'run', return_value=subprocess.CompletedProcess(['tidy'], 0, output, '')):
                findings = quality.clang_tidy_project(project, 'tidy', report=report)
            self.assertEqual(list(findings), [('check-one', 'runtime/Test.c', 'int runtime;')])
            self.assertEqual(findings[('check-one', 'runtime/Test.c', 'int runtime;')], 2)
            self.assertEqual(report, [
                {'rule': 'check-one', 'file': 'runtime/Test.c', 'line': 1, 'column': 2, 'message': 'Runtime message', 'sample': 'sample'},
                {'rule': 'readability-identifier-naming', 'file': 'bsw/Test.h', 'line': 1, 'column': 1, 'message': 'Generated message', 'sample': 'sample'},
                {'rule': 'check-one', 'file': 'runtime/Test.c', 'line': 1, 'column': 3, 'message': 'Other column', 'sample': 'sample'},
                {'rule': 'check-one', 'file': 'runtime/Test.c', 'line': 1, 'column': 2, 'message': 'Runtime message', 'sample': 'sample'},
            ])
