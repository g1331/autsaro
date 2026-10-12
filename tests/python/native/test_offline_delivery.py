"""Run a generated mainline package as a receiver with a selected Python."""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from autosar_tooling.config import environment


class OfflineDeliveryTests(unittest.TestCase):
    def test_moved_ecu_package_builds_and_verifies_with_receiver_python(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        if not samples:
            self.fail(
                "Generate c_analysis samples and set AUTOSAR_C_ANALYSIS_SAMPLES first"
            )
        env = environment()
        receiver = env.get("AUTOSAR_RECEIVER_PYTHON", sys.executable)
        with tempfile.TemporaryDirectory(
            prefix="autosar-handoff-receiver-"
        ) as temporary:
            root = Path(temporary)
            moved = root / "receiver" / "package"
            moved.parent.mkdir()
            shutil.copytree(Path(samples) / "standard-ecu", moved)

            def snapshot():
                return {
                    p.relative_to(moved).as_posix(): p.read_bytes()
                    for p in moved.rglob("*")
                    if p.is_file()
                }

            before = snapshot()
            verified = subprocess.run(
                [
                    receiver,
                    "-I",
                    "-S",
                    str(moved / "tools/ecu-tool.py"),
                    "verify",
                    "--project",
                    str(moved),
                    "--build-directory",
                    str(root / "build"),
                ],
                cwd=moved.parent,
                env=env,
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=300,
                check=False,
            )
            self.assertEqual(verified.returncode, 0, verified.stdout + verified.stderr)
            self.assertTrue((root / "build").is_dir())
            self.assertEqual(snapshot(), before)
