"""Run the generated reference package as a receiver with a selected Python."""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from autosar_tooling.config import ROOT, archive_path, archives, environment


class OfflineDeliveryTests(unittest.TestCase):
    def test_moved_ecu_handoff_builds_and_verifies_with_receiver_python(self):
        env = environment()
        receiver = env.get('AUTOSAR_RECEIVER_PYTHON', sys.executable)
        target = 'windows-x64-controlled-v1' if os.name == 'nt' else 'linux-x64-controlled-v1'
        with tempfile.TemporaryDirectory(prefix='autosar-handoff-receiver-') as temporary:
            root = Path(temporary)
            package = root / 'generated'
            command = ['cargo', 'run', '--locked', '--manifest-path', str(ROOT / 'core/Cargo.toml'), '--bin', 'generate_epic4_ecu', '--', '--handoff', '--target', target, '--output', str(package)]
            for name, flag in [('R24-11 XSD', '--xsd-archive'), ('R24-11 MOD', '--mod-archive')]:
                variable, relative, _ = archives()[name]
                command += [flag, str(archive_path(variable, relative))]
            for source in sorted((ROOT / 'core/tests/fixtures/epic4/positive').glob('*.arxml')):
                command += ['--input', str(source)]
            preview = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True, encoding='utf-8', timeout=300, check=False)
            self.assertEqual(preview.returncode, 0, preview.stdout + preview.stderr)
            revision = json.loads(preview.stdout)['revision']
            generated = subprocess.run([*command, '--write', '--revision', revision], cwd=ROOT, env=env, capture_output=True, text=True, encoding='utf-8', timeout=300, check=False)
            self.assertEqual(generated.returncode, 0, generated.stdout + generated.stderr)
            moved = root / 'receiver' / 'package'
            moved.parent.mkdir()
            shutil.move(str(package), moved)
            before = {p.relative_to(moved).as_posix(): p.read_bytes() for p in moved.rglob('*') if p.is_file()}
            verified = subprocess.run([receiver, '-I', '-S', str(moved / 'tools/ecu-tool.py'), 'verify', '--project', str(moved), '--build-directory', str(root / 'build')], cwd=moved.parent, env=env, capture_output=True, text=True, encoding='utf-8', timeout=300, check=False)
            self.assertEqual(verified.returncode, 0, verified.stdout + verified.stderr)
            self.assertTrue((root / 'build').is_dir())
            self.assertEqual({p.relative_to(moved).as_posix(): p.read_bytes() for p in moved.rglob('*') if p.is_file()}, before)

    def test_moved_reference_package_builds_and_verifies_without_checkout_imports(self):
        env = environment()
        receiver = env.get("AUTOSAR_RECEIVER_PYTHON", sys.executable)
        target = "windows-x64-controlled-v1" if os.name == "nt" else "linux-x64-controlled-v1"
        variable, relative, _ = archives()["R24-11 XSD"]
        with tempfile.TemporaryDirectory(prefix="autosar-receiver-") as temporary:
            root = Path(temporary)
            package = root / "generated"
            generated = subprocess.run(
                ["cargo", "run", "--locked", "--manifest-path", str(ROOT / "core/Cargo.toml"), "--bin", "package_host_reference", "--", str(package), "--target", target, "--xsd-archive", str(archive_path(variable, relative))],
                cwd=ROOT, env=env, capture_output=True, text=True, encoding="utf-8", timeout=300, check=False,
            )
            self.assertEqual(generated.returncode, 0, generated.stdout + generated.stderr)
            moved = root / "receiver" / "package"
            moved.parent.mkdir()
            shutil.move(str(package), moved)
            before = {p.relative_to(moved).as_posix(): p.read_bytes() for p in moved.rglob("*") if p.is_file()}
            verified = subprocess.run(
                [receiver, "-I", "-S", str(moved / "tools/ecu-tool.py"), "verify", "--project", str(moved), "--build-directory", str(root / "build"), "--report-path", str(root / "result.json")],
                cwd=moved.parent, env=env, capture_output=True, text=True, encoding="utf-8", timeout=300, check=False,
            )
            self.assertEqual(verified.returncode, 0, verified.stdout + verified.stderr)
            self.assertTrue((root / "result.json").is_file())
            self.assertEqual({p.relative_to(moved).as_posix(): p.read_bytes() for p in moved.rglob("*") if p.is_file()}, before)
