"""Dependency identity checks must not mistake an existing ZIP for the pinned one."""

from __future__ import annotations

import hashlib
import io
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from autosar_tooling.doctor import _archive


class ArchiveIdentityTests(unittest.TestCase):
    def test_wrong_archive_is_rejected_without_mutating_it(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "schema.zip"
            archive.write_bytes(b"different legal-looking ZIP")
            original = archive.read_bytes()
            output = io.StringIO()
            with (
                patch.dict("os.environ", {"TEST_SCHEMA_ARCHIVE": str(archive)}),
                redirect_stdout(output),
            ):
                self.assertFalse(
                    _archive(
                        "schema",
                        "TEST_SCHEMA_ARCHIVE",
                        "unused",
                        hashlib.sha256(b"expected").hexdigest(),
                    )
                )
            self.assertIn("version_mismatch", output.getvalue())
            self.assertEqual(archive.read_bytes(), original)

    def test_missing_archive_reports_missing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = io.StringIO()
            with (
                patch.dict(
                    "os.environ",
                    {"TEST_SCHEMA_ARCHIVE": str(Path(directory) / "absent.zip")},
                ),
                redirect_stdout(output),
            ):
                self.assertFalse(
                    _archive("schema", "TEST_SCHEMA_ARCHIVE", "unused", "0" * 64)
                )
            self.assertIn("missing", output.getvalue())


if __name__ == "__main__":
    unittest.main()
