"""Protect the desktop matrix's exact-once scenario coverage and failure exit."""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

import yaml
from autosar_tooling.config import ROOT


class DesktopWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = yaml.safe_load((ROOT / '.github/workflows/desktop.yml').read_text(encoding='utf-8'))
        cls.job = cls.workflow['jobs']['desktop']
        cls.step = next(step for step in cls.job['steps'] if step.get('name') == 'Desktop scenarios')

    def test_every_file_runs_in_exactly_one_shard_on_each_platform(self):
        matrix = self.job['strategy']['matrix']
        self.assertEqual(matrix['os'], ['ubuntu-24.04', 'windows-latest'])
        assigned = [name for shard in matrix['shard'] for name in shard['files'].split()]
        expected = [file.name.removesuffix('.test.mjs') for file in (ROOT / 'tests/desktop').glob('*.test.mjs')]
        self.assertCountEqual(assigned, expected)
        self.assertFalse(self.job['strategy']['fail-fast'])
        self.assertEqual(self.job['needs'], 'build')
        extended = yaml.safe_load((ROOT / '.github/workflows/extended.yml').read_text(encoding='utf-8'))
        self.assertIn('desktop', extended['jobs']['result']['needs'])

    def test_selected_files_run_once_and_failure_reaches_the_job(self):
        script = self.step['run'].split("<<'JS'\n", 1)[1].rsplit('\nJS', 1)[0]
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            tests = directory / 'tests/desktop'
            tests.mkdir(parents=True)
            (tests / 'pass.test.mjs').write_text("import test from 'node:test'; test('selected pass', () => {});", encoding='utf-8')
            (tests / 'fail.test.mjs').write_text("import test from 'node:test'; test('selected fail', () => { throw new Error('shard failure'); });", encoding='utf-8')
            for assigned, selected, status in (
                ('pass', 'tests/desktop/*.test.mjs', 0),
                ('fail', 'tests/desktop/*.test.mjs', 1),
                ('fail', 'tests/desktop/pass.test.mjs', 0),
                ('pass', 'tests/desktop/missing.test.mjs', 1),
                ('windows-paths', 'tests/desktop/*.test.mjs', 0),
            ):
                with self.subTest(assigned=assigned, selected=selected):
                    windows = assigned == 'windows-paths'
                    invocation = script.replace(
                        "import { globSync } from 'node:fs';",
                        r"const globSync = () => ['tests\\desktop\\pass.test.mjs'];",
                    ) if windows else script
                    result = subprocess.run(
                        ['node', '--input-type=module', '-e', invocation], cwd=directory,
                        env={**os.environ, 'SHARD_FILES': 'pass' if windows else assigned, 'SCENARIOS': selected, 'TEST_CONCURRENCY': '1'},
                        capture_output=True, text=True, check=False,
                    )
                    self.assertEqual(result.returncode, status, result.stdout + result.stderr)
                    if (assigned == 'pass' or windows) and status == 0:
                        self.assertIn('selected pass', result.stdout)
                        self.assertNotIn('selected fail', result.stdout)
