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
COMMIT_STATUS = (
    " status=0 input_status=0 transport_status=0 transport_epoch=0 transport_count=0\n"
)
COMMIT = "COMMIT_OK batch=1 epoch=1 sequence=1 input_first=0 inputs=0" + COMMIT_STATUS
CLOSED = "lifecycle=Closed state=Ready reason=0 trace=I\n"
PIPELINE_OUTPUT = [
    "OUT epoch=1 sequence=4 ticket=1 pdu=1 id=1800 dlc=8 data=0762123400000000\n",
    "OUT epoch=10 sequence=16 ticket=2 pdu=0 id=801 dlc=4 data=78563412\n",
    "OUT epoch=10 sequence=17 ticket=3 pdu=1 id=1800 dlc=8 data=0762123412345678\n",
    "OUT epoch=20 sequence=31 ticket=4 pdu=0 id=801 dlc=4 data=78563412\n",
    "OUT epoch=20 sequence=32 ticket=5 pdu=1 id=1800 dlc=8 data=0762123412345678\n",
]


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

    def test_pipeline_frames_belong_to_the_batch_that_precedes_each_receipt(self):
        receipts = [
            f"COMMIT_OK batch={number} epoch={epoch} sequence={sequence} input_first={first} "
            f"inputs={count}{COMMIT_STATUS}"
            for number, (epoch, sequence, first, count) in enumerate(
                [(0, 3, 1, 2), (1, 6, 0, 0), (10, 19, 15, 1), (10, 20, 0, 0), (20, 34, 30, 1)],
                1,
            )
        ]
        first, second, third, fourth, fifth = receipts
        before_third = first + PIPELINE_OUTPUT[0] + second
        epoch_ten = "".join(PIPELINE_OUTPUT[1:3])
        epoch_twenty = "".join(PIPELINE_OUTPUT[3:])
        body = before_third + epoch_ten + third + fourth + epoch_twenty + fifth
        check = RECEIVER["check_batch"]
        check(READY + body + CLOSED, pipeline=True)
        wrong_batch = before_third + third + epoch_ten + fourth + epoch_twenty + fifth
        with self.assertRaisesRegex(ValueError, "frames for batch 3"):
            check(READY + wrong_batch + CLOSED, pipeline=True)
        with self.assertRaisesRegex(ValueError, "frames for batch 2"):
            check(READY + "".join(receipts + PIPELINE_OUTPUT) + CLOSED, pipeline=True)
        with self.assertRaisesRegex(ValueError, "frame after the final receipt"):
            check(READY + body + PIPELINE_OUTPUT[0] + CLOSED, pipeline=True)
        with self.assertRaisesRegex(ValueError, "closed outside the end"):
            check(READY + CLOSED + body, pipeline=True)

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
