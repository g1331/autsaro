"""Selected kernel patches must work inside an enclosing developer checkout."""
from __future__ import annotations

import hashlib
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from ecu_tools.build import (
    kernel_patch_environment,
    kernel_patch_targets,
    require_kernel_identity,
    require_kernel_patch_applied,
)


class KernelPatchTests(unittest.TestCase):
    def test_selected_files_are_patched_inside_and_outside_a_checkout(self):
        git = shutil.which("git")
        self.assertIsNotNone(git, "Kernel patch preparation requires Git")
        with tempfile.TemporaryDirectory(prefix="autosar-kernel-patch-") as temporary:
            root = Path(temporary).resolve(strict=True)
            checkout = root / "checkout"
            checkout.mkdir()
            subprocess.run([git, "init", "--quiet", str(checkout)], check=True, capture_output=True)
            patch = root / "change.patch"
            patch.write_bytes(b"--- a/selected.h\n+++ b/selected.h\n@@ -1 +1 @@\n-before\n+after\n"
                              b"--- a/excluded.h\n+++ b/excluded.h\n@@ -1 +1 @@\n-before\n+after\n")
            for parent in (checkout, root):
                with self.subTest(enclosing_checkout=parent == checkout):
                    kernel = parent / "output" / "kernel"
                    kernel.mkdir(parents=True)
                    for name in ("selected.h", "excluded.h"):
                        (kernel / name).write_bytes(b"before\n")
                    env = kernel_patch_environment(kernel)
                    for check in (True, False):
                        result = subprocess.run([git, "apply", "--include=selected.h",
                                                 *(["--check"] if check else []), str(patch)],
                                                cwd=kernel, env=env, capture_output=True, text=True, check=False)
                        self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual((kernel / "selected.h").read_text(), "after\n")
                    self.assertEqual((kernel / "excluded.h").read_bytes(), b"before\n")
                    expected = {"selected.h": hashlib.sha256(b"after\n").hexdigest(),
                                "excluded.h": hashlib.sha256(b"before\n").hexdigest()}
                    require_kernel_identity(kernel, expected)
                    # A one-byte corruption after a successful real Git application.
                    (kernel / "selected.h").write_bytes(b"After\n")
                    with self.assertRaisesRegex(ValueError, "^Kernel identity mismatch: kernel/selected.h$"):
                        require_kernel_identity(kernel, expected)

    def test_a_patch_that_changes_no_selected_file_is_refused(self):
        with tempfile.TemporaryDirectory(prefix="autosar-kernel-patch-") as temporary:
            kernel = Path(temporary)
            patch = kernel / "change.patch"
            patch.write_bytes(b"--- a/selected.h\r\n+++ b/selected.h\r\n@@ -1 +1 @@\r\n-before\r\n+after\r\n")
            (kernel / "selected.h").write_bytes(b"before\n")
            with self.assertRaisesRegex(ValueError, "selects no file"):
                kernel_patch_targets(kernel, patch, ("other.h",))
            before = kernel_patch_targets(kernel, patch, ("selected.h",))
            with self.assertRaisesRegex(RuntimeError, "left selected.h unchanged"):
                require_kernel_patch_applied(patch, before)
            (kernel / "selected.h").write_bytes(b"after\n")
            require_kernel_patch_applied(patch, before)

    def test_identity_closure_and_records_are_required(self):
        with tempfile.TemporaryDirectory(prefix="autosar-kernel-patch-") as temporary:
            kernel = Path(temporary)
            (kernel / "include").mkdir()
            (kernel / "include/task.h").write_bytes(b"header")
            identity = hashlib.sha256(b"header").hexdigest()
            require_kernel_identity(kernel, {"include/task.h": identity})
            for expected, message in (
                (None, "Missing patched kernel identities"),
                ({}, "Missing patched kernel identities"),
                ({"../outside": identity}, "Invalid kernel identity: kernel/../outside"),
                ({"include/task.h": "bad"}, "Invalid kernel identity: kernel/include/task.h"),
                ({"missing.c": identity}, "Kernel file closure mismatch: kernel/include/task.h"),
                ({"include/task.h": identity, "missing.c": identity}, "Kernel file closure mismatch: kernel/missing.c"),
            ):
                with self.subTest(expected=expected), self.assertRaises(ValueError) as caught:
                    require_kernel_identity(kernel, expected)
                self.assertEqual(str(caught.exception), message)
