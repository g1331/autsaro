"""Exercise the shared CI package installer with stubbed apt and sleep."""

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
        action_path = ROOT / '.github/actions/core-build-dependencies'
        installer = (action_path / 'install-ubuntu-packages.sh').read_text()
        action = yaml.safe_load((action_path / 'action.yml').read_text())
        step = action['runs']['steps'][0]
        self.assertEqual(step['if'], "runner.os == 'Linux'")
        self.assertEqual(step['env']['LINUX_PACKAGES'], '${{ inputs.linux-packages }}')
        desktop = yaml.safe_load((ROOT / '.github/workflows/desktop.yml').read_text())
        consumer = next(item for item in desktop['jobs']['build']['steps']
                        if item.get('uses') == './.github/actions/core-build-dependencies')
        desktop_runtime = next(item for item in desktop['jobs']['desktop']['steps']
                               if item.get('name') == 'Linux desktop runtime dependencies')
        self.assertEqual(desktop_runtime['if'], "runner.os == 'Linux'")
        # Windows' system bash launches WSL; Actions uses Git Bash for shell: bash.
        bash = str(Path(os.environ['ProgramFiles']) / 'Git/bin/bash.exe') if os.name == 'nt' else 'bash'
        for mirror in ('mirror+file:/etc/apt/apt-mirrors.txt', 'http://azure.archive.ubuntu.com/ubuntu/'):
            for packages, failure in (('', ''), (consumer['with']['linux-packages'], ''), (None, 'warn-once'),
                                      ('build-essential\ncmake', 'warn-once'), ('clang', 'warn-always'),
                                      ('clang', 'update'), ('pkg-config', 'install')):
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
is_update = args[:3] == ['env', 'LC_ALL=C', 'apt-get']
args = args[2:] if is_update else args
assert args[0] == 'apt-get', args
assert args[1] == '-o' and args[2].startswith('Dir::Etc::sourcelist='), args
assert args[3:5] == ['-o', 'Dir::Etc::sourceparts=-'], args
assert args[5:11] == [
    '-o', 'Acquire::Retries=5',
    '-o', 'Acquire::http::Timeout=15',
    '-o', 'Acquire::https::Timeout=15',
], args
phase = args[11]
assert is_update == (phase == 'update'), args
source = Path(args[2].split('=', 1)[1])
log = Path(os.environ['APT_LOG'])
first_call = not log.exists()
with log.open('a') as output:
    output.write(json.dumps({'args': args[11:], 'source': str(source), 'text': source.read_text()}) + '\\n')
failure = os.environ['FAIL_APT_PHASE']
if is_update:
    assert args[12:] == ['--error-on=any'], args
    if failure == 'warn-once' and first_call:
        print('W: Failed to fetch https://archive.ubuntu.com/ubuntu/dists/noble/InRelease', file=sys.stderr)
    if failure == 'warn-always':
        print('W: Some index files failed to download.', file=sys.stderr)
    if failure == 'update':
        print('E: File has unexpected size. Mirror sync in progress?', file=sys.stderr)
if phase == failure:
    sys.exit(100)
''', newline='\n')
                    log = directory / 'apt.jsonl'
                    test_installer = directory / 'install-ubuntu-packages.sh'
                    test_installer.write_text(
                        installer.replace('/etc/apt/sources.list.d/ubuntu.sources', shlex.quote(source.as_posix())),
                        newline='\n',
                    )
                    stubs = directory / 'stubs.sh'
                    stubs.write_text(
                        f'sudo() {{ {shlex.quote(Path(sys.executable).as_posix())} '
                        f'{shlex.quote(command.as_posix())} "$@"; }}\n'
                        'sleep() { echo "sleep $1"; }\n',
                        newline='\n',
                    )
                    run = step['run']
                    expected_packages = ['libxml2-dev', 'libclang-dev', 'clang', 'pkg-config', *(packages or '').split()]
                    if packages is None:
                        run = desktop_runtime['run'].replace(
                            '.github/actions/core-build-dependencies/install-ubuntu-packages.sh',
                            shlex.quote(test_installer.as_posix()),
                        )
                        expected_packages = [
                            'libwebkit2gtk-4.1-0', 'libxml2', 'webkit2gtk-driver', 'xvfb', 'xdotool',
                            'xclip', 'xsettingsd', 'tcpdump', 'bpftrace', 'binutils', 'gdb',
                        ]
                    result = subprocess.run(
                        [bash, '--noprofile', '--norc', '-eo', 'pipefail', '-c', run],
                        env={**os.environ, 'RUNNER_TEMP': directory.as_posix(), 'LINUX_PACKAGES': packages or '',
                             'ACTION_PATH': directory.as_posix(), 'BASH_ENV': stubs.as_posix(),
                             'APT_LOG': str(log), 'FAIL_APT_PHASE': failure},
                        capture_output=True, text=True, check=False,
                    )
                    update_failed = failure in ('update', 'warn-always')
                    self.assertEqual(result.returncode, 100 if update_failed or failure == 'install' else 0,
                                     result.stdout + result.stderr)
                    calls = [json.loads(line) for line in log.read_text().splitlines()]
                    update_count = 3 if update_failed else 2 if failure == 'warn-once' else 1
                    expected_phases = ['update'] * update_count + ([] if update_failed else ['install'])
                    self.assertEqual([call['args'][0] for call in calls], expected_phases)
                    self.assertEqual([line for line in result.stdout.splitlines() if line.startswith('sleep ')],
                                     ['sleep 5', 'sleep 10'][:update_count - 1])
                    if not update_failed:
                        self.assertEqual(calls[-1]['args'][1:], ['-y', *expected_packages])
                    for call in calls:
                        self.assertEqual(call['text'], original.replace(mirror, 'https://archive.ubuntu.com/ubuntu/'))
                        self.assertFalse(Path(call['source']).exists(), 'temporary source must be cleaned up')
                    self.assertEqual(source.read_text(), original)
