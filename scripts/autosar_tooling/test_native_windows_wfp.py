"""Actual Windows private-executable scope rejection without filter activation."""

import ctypes
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path


@unittest.skipUnless(
    os.name == "nt" and ctypes.sizeof(ctypes.c_void_p) == 8,
    "Actual native Win64 WFP API is required",
)
class PrivateWfpScopeTests(unittest.TestCase):
    def test_prepare_rejects_traversal_to_existing_outside_executable(self):
        from autosar_tooling.native_windows_wfp import prepare

        with tempfile.TemporaryDirectory(prefix="autosar-wfp-regression-") as directory:
            root = Path(directory)
            allowed = root / "allowed"
            allowed.mkdir()
            inside = allowed / "inside.exe"
            shutil.copyfile(sys.executable, inside)
            shutil.copyfile(sys.executable, root / "outside.exe")
            with self.assertRaisesRegex(RuntimeError, "traversal"):
                prepare([inside, allowed / ".." / "outside.exe"], allowed)


if __name__ == "__main__":
    unittest.main()
