"""Exercise command-local Ubuntu sources, package preservation and apt failures."""

import json
import os
import shlex
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import yaml
from autosar_tooling.config import ROOT


class CoreBuildDependenciesTests(unittest.TestCase):
    def test_linux_preparation_preserves_packages_sources_and_failures(self):
        action = yaml.safe_load((ROOT / '.github/actions/core-build-dependencies/action.yml').read_text())
        step = action['runs']['steps'][0]
        self.assertEqual(step['if'], "runner.os == 'Linux'")
        self.assertEqual(step['env']['LINUX_PACKAGES'], '${{ inputs.linux-packages }}')
        desktop = yaml.safe_load((ROOT / '.github/workflows/desktop.yml').read_text())
        consumer = next(item for item in desktop['jobs']['build']['steps']
                        if item.get('uses') == './.github/actions/core-build-dependencies')
        # Windows' system bash launches WSL; Actions uses Git Bash for shell: bash.
        bash = str(Path(os.environ['ProgramFiles']) / 'Git/bin/bash.exe') if os.name == 'nt' else 'bash'
        for mirror in ('mirror+file:/etc/apt/apt-mirrors.txt', 'http://azure.archive.ubuntu.com/ubuntu/'):
            for packages, failure in (('', ''), (consumer['with']['linux-packages'], ''),
                                      ('build-essential cmake', 'update'), ('pkg-config', 'install')):
                with self.subTest(mirror=mirror, packages=packages, failure=failure), tempfile.TemporaryDirectory(prefix='apt test ') as temporary:
                    directory = Path(temporary)
                    source = directory / 'ubuntu.sources'
                    original = (f'Types: deb\nURIs: {mirror}\n'
                                'Suites: noble noble-updates noble-backports noble-security\n'
                                'Components: main restricted universe multiverse\n'
                                'Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n')
                    source.write_text(original)
                    command = directory / 'sudo.py'
                    command.write_text('''import json, os, sys
from pathlib import Path
args = sys.argv[1:]
assert args[0] == 'apt-get', args
assert args[1] == '-o' and args[2].startswith('Dir::Etc::sourcelist='), args
assert args[3:5] == ['-o', 'Dir::Etc::sourceparts=-'], args
source = Path(args[2].split('=', 1)[1])
with open(os.environ['APT_LOG'], 'a') as output:
    output.write(json.dumps({'args': args, 'source': str(source), 'text': source.read_text()}) + '\\n')
sys.exit(100 if args[5] == os.environ['FAIL_APT_PHASE'] else 0)
''', newline='\n')
                    log = directory / 'apt.jsonl'
                    script = step['run'].replace('/etc/apt/sources.list.d/ubuntu.sources', shlex.quote(source.as_posix()))
                    script = (f'sudo() {{ {shlex.quote(Path(sys.executable).as_posix())} '
                              f'{shlex.quote(command.as_posix())} "$@"; }}\n' + script)
                    result = subprocess.run(
                        [bash, '--noprofile', '--norc', '-eo', 'pipefail', '-c', script],
                        env={**os.environ, 'RUNNER_TEMP': directory.as_posix(), 'LINUX_PACKAGES': packages,
                             'APT_LOG': str(log), 'FAIL_APT_PHASE': failure},
                        capture_output=True, text=True, check=False,
                    )
                    self.assertEqual(result.returncode, 100 if failure else 0, result.stdout + result.stderr)
                    calls = [json.loads(line) for line in log.read_text().splitlines()]
                    self.assertEqual([call['args'][5] for call in calls], ['update'] if failure == 'update' else ['update', 'install'])
                    if failure != 'update':
                        self.assertEqual(calls[1]['args'][6:], ['-y', 'libxml2-dev', 'libclang-dev', 'clang', 'pkg-config', *packages.split()])
                    for call in calls:
                        self.assertEqual(call['text'], original.replace(mirror, 'https://archive.ubuntu.com/ubuntu/'))
                        self.assertFalse(Path(call['source']).exists(), 'temporary source must be cleaned up')
                    self.assertEqual(source.read_text(), original)
