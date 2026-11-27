"""Exercise the post-merge alert step with GitHub API responses."""

import json
import os
import runpy
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import yaml
from autosar_tooling.config import ROOT

REPO = 'g1331/autsaro'
SHA = 'a' * 40
RUN_URL = f'https://github.com/{REPO}/actions/runs/42/attempts/2'
ISSUES = 'issues?state=open&labels=area%3Aquality-ci&per_page=100'
JOBS = 'actions/runs/42/attempts/2/jobs?per_page=100'


class MasterCiAlertTests(unittest.TestCase):
    def run_alert(self, result, issues, master=SHA):
        workflow = yaml.safe_load((ROOT / '.github/workflows/checks.yml').read_text(encoding='utf-8'))
        step = workflow['jobs']['master-result']['steps'][0]['run']
        failure = {
            'name': 'extended / fuzz (cantp_receive)',
            'html_url': f'{RUN_URL}/jobs/49',
            'status': 'completed',
            'conclusion': 'failure',
        }

        def github(command):
            endpoint = command[-1].removeprefix(f'repos/{REPO}/')
            match endpoint:
                case 'git/ref/heads/master':
                    payload = {'object': {'sha': master}}
                case _ if endpoint == ISSUES:
                    self.assertEqual(command[2:-1], ['--paginate', '--slurp'])
                    payload = issues
                case _ if endpoint == JOBS:
                    self.assertEqual(command[2:-1], ['--paginate', '--slurp'])
                    payload = [{'jobs': [{**failure, 'name': 'Skipped job', 'conclusion': 'skipped'}]},
                               {'jobs': [failure]}]
                case _:
                    self.fail(f'Unexpected API request: {command}')
            return json.dumps(payload).encode()

        with tempfile.TemporaryDirectory() as temporary:
            summary = Path(temporary) / 'summary'
            reporter = Path(temporary) / 'master-result.py'
            reporter.write_text(step, encoding='utf-8')
            env = {
                'GITHUB_REPOSITORY': REPO,
                'GITHUB_SERVER_URL': 'https://github.com',
                'GITHUB_SHA': SHA,
                'GITHUB_RUN_ID': '42',
                'GITHUB_RUN_ATTEMPT': '2',
                'GITHUB_STEP_SUMMARY': str(summary),
                'RESULTS': json.dumps({'required': {'result': 'success'}, 'extended': {'result': result}}),
            }
            with (mock.patch.dict(os.environ, env),
                  mock.patch.object(subprocess, 'check_output', side_effect=github),
                  mock.patch.object(subprocess, 'run') as write,
                  self.assertRaises(SystemExit) as exit_status):
                runpy.run_path(str(reporter), run_name='__main__')
            self.assertEqual(exit_status.exception.code, int(result != 'success'))
            self.assertIn(f'`extended`: {result}', summary.read_text(encoding='utf-8'))
            return write.call_args_list

    def test_failure_creates_one_rolling_alert_and_later_failure_comments_on_it(self):
        other_issues = [{'number': 1, 'title': 'Master CI failed (run 41)'},
                        {'number': 2, 'title': 'Master CI failed', 'pull_request': {}}]
        rolling_issue = {'number': 3, 'title': 'Master CI failed'}
        for pages, action in (([other_issues], ['create', '--title', rolling_issue['title'],
                                                '--label', 'area:quality-ci']),
                              ([other_issues, [rolling_issue]], ['comment', '3'])):
            with self.subTest(action=action):
                (write,) = self.run_alert('failure', pages)
                self.assertEqual(write.args[0], ['gh', 'issue', *action, '--repo', REPO, '--body-file', '-'])
                body = write.kwargs['input']
                self.assertIn(SHA, body)
                self.assertIn(RUN_URL, body)
                self.assertIn(f'[`extended / fuzz (cantp_receive)`]({RUN_URL}/jobs/49): failure', body)
                self.assertNotIn('Skipped job', body)

    def test_recovery_comments_and_closes_only_the_rolling_alert(self):
        issues = [[{'number': 3, 'title': 'Master CI failed'}]]
        (write,) = self.run_alert('success', issues)
        self.assertEqual(write.args[0][:-1], ['gh', 'issue', 'close', '3', '--repo', REPO,
                                            '--reason', 'completed', '--comment'])
        self.assertIn(SHA, write.args[0][-1])
        self.assertIn(RUN_URL, write.args[0][-1])
        self.assertEqual(self.run_alert('success', [[]]), [])

    def test_stale_runs_keep_their_result_without_mutating_issues(self):
        for result in ('success', 'failure'):
            with self.subTest(result=result):
                self.assertEqual(self.run_alert(result, [[{'number': 3, 'title': 'Master CI failed'}]],
                                                master='b' * 40), [])
