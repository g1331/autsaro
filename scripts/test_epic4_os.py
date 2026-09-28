"""Dependency rejection checks for the independent Epic 4 verifier."""

import shutil
import copy
import json
import re
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import epic4_os


class DependencyTests(unittest.TestCase):
    def test_activation_gate_rejects_swapped_fifo_and_missing_observer(self):
        evidence = json.loads(
            (
                epic4_os.ROOT / "docs/assurance/evidence/epic4/activation-fifo.json"
            ).read_text(encoding="utf-8")
        )
        original = evidence["observations"][0]
        for mutation in ["order", "observer"]:
            with self.subTest(mutation=mutation):
                result = copy.deepcopy(original)
                if mutation == "order":
                    result["stdout"] = result["stdout"].replace(
                        "trace=ISRLAHaCBMZ", "trace=ISRLAHaBCMZ"
                    )
                else:
                    result["stdout"] = re.sub(
                        r"observer=\d+", "observer=0", result["stdout"]
                    )
                with (
                    patch("epic4_os.execute", return_value=result),
                    self.assertRaises(AssertionError),
                ):
                    epic4_os.check_activation(Path("unused.exe"))

    def test_missing_and_modified_kernel_rejected(self):
        with tempfile.TemporaryDirectory(prefix="epic4-dependency-") as directory:
            kernel = Path(directory) / "kernel"
            shutil.copytree(epic4_os.KERNEL, kernel)
            self.assertEqual(epic4_os.verify_sources(kernel)["license"], "MIT")
            source = kernel / "tasks.c"
            source.write_bytes(source.read_bytes() + b"\n/* modified */\n")
            with self.assertRaisesRegex(ValueError, "kernel digest mismatch: tasks.c"):
                epic4_os.verify_sources(kernel)
            source.unlink()
            with self.assertRaises(FileNotFoundError):
                epic4_os.verify_sources(kernel)

    def test_compiler_identity_rejected(self):
        class Result:
            stdout = "unexpected compiler"

        with (
            patch("epic4_os.subprocess.run", return_value=Result()),
            self.assertRaisesRegex(ValueError, "requires GCC"),
        ):
            epic4_os.compiler()


if __name__ == "__main__":
    unittest.main()
