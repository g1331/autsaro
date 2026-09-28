"""Dependency rejection checks for the independent Epic 4 verifier."""

import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import epic4_os


class DependencyTests(unittest.TestCase):
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
