"""Exercise the standalone receiver without building a native program."""

import os
import runpy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

RECEIVER = runpy.run_path(
    str(Path(__file__).resolve().parents[2] / "scripts/rehearse_ecu_handoff.py")
)
READY = "READY HostBatchV1\n"
COMMIT = (
    "COMMIT_OK batch=1 epoch=1 sequence=1 input_first=0 inputs=0"
    " status=0 input_status=0 transport_status=0 transport_epoch=0 transport_count=0\n"
)
CLOSED = "lifecycle=Closed state=Ready reason=0 trace=I\n"


class ReceiverTests(unittest.TestCase):
    def test_receipts_require_ready_and_reject_failed_legal_batches(self):
        check = RECEIVER["check_batch"]
        check(READY + COMMIT + CLOSED, pipeline=False)
        with self.assertRaisesRegex(ValueError, "did not become ready"):
            check("REJECT epoch=1 sequence=0\nCOMMIT_OK batch=1 epoch=1\n", pipeline=False)
        for failure in ("REJECT epoch=1 sequence=0\n", "COMMIT_ERROR batch=1 epoch=1\n"):
            with (
                self.subTest(failure=failure),
                self.assertRaisesRegex(ValueError, "rejected a legal batch"),
            ):
                check(READY + failure + COMMIT + CLOSED, pipeline=False)
        with self.assertRaisesRegex(ValueError, "unexpected HostBatch receipt"):
            check(READY + COMMIT.replace("batch=1", "batch=2") + CLOSED, pipeline=False)

    def test_sender_inventory_rejects_missing_paths_and_a_substituted_seal(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for sample in RECEIVER["SAMPLES"]:
                seal = root / sample / RECEIVER["SEAL"]
                seal.parent.mkdir(parents=True)
                seal.write_bytes(sample.encode())
            trusted = RECEIVER["regression_seals"](root)
            check = RECEIVER["check_seals"]
            check(RECEIVER["regression_seals"](root), trusted)
            with self.assertRaisesRegex(ValueError, "four fixed paths"):
                check(trusted, {name: trusted[name] for name in list(trusted)[:-1]})
            seal.write_bytes(b"valid seal from another build")
            with self.assertRaisesRegex(ValueError, "no-diagnostic/meta/files.sha256"):
                check(RECEIVER["regression_seals"](root), trusted)
            seal.unlink()
            with self.assertRaises(FileNotFoundError):
                RECEIVER["regression_seals"](root)

    def test_environment_excludes_ambient_path_and_build_overrides(self):
        python = Path(sys.executable).absolute()
        variables = {
            "AUTOSAR_CC": str(python),
            "AUTOSAR_OBJDUMP": str(python),
            "AUTOSAR_GIT": str(python),
            "CPATH": "untrusted",
            "PYTHONPATH": "untrusted",
            "PATH": "untrusted",
        }
        with patch.dict(os.environ, variables):
            env = RECEIVER["receiver_environment"](python)
        self.assertEqual(env["AUTOSAR_CC"], str(python.resolve(strict=True)))
        self.assertIn(str(python.parent), env["PATH"].split(os.pathsep))
        self.assertNotIn("untrusted", env["PATH"])
        self.assertNotIn("CPATH", env)
        self.assertNotIn("PYTHONPATH", env)
