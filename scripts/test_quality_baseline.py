"""Check that the audit fails closed on unclassified C and missing evidence."""

import hashlib
import io
import shutil
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

import quality_baseline
from quality_baseline import c_scope_errors, generated_input_errors, spec_evidence_gaps


class BaselineTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("doxygen"), "Doxygen is not installed")
    def test_doxygen_rejects_missing_public_api_docs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            include = root / "runtime" / "include"
            include.mkdir(parents=True)
            (root / "runtime" / "Doxyfile").write_text(
                (quality_baseline.ROOT / "runtime/Doxyfile").read_text(
                    encoding="utf-8"
                ),
                encoding="utf-8",
            )
            header = include / "Example.h"
            header.write_text("int Example(int value);\n", encoding="utf-8")
            with (
                patch.object(quality_baseline, "ROOT", root),
                redirect_stdout(io.StringIO()),
            ):
                self.assertFalse(quality_baseline.doxygen_check())
                header.write_text(
                    "/** @file\n * @brief Example API.\n */\nint Example(int value);\n",
                    encoding="utf-8",
                )
                self.assertFalse(quality_baseline.doxygen_check())
                header.write_text(
                    "/** @file\n * @brief Example API.\n */\n"
                    "/** @brief Returns the input.\n * @param wrong Invalid name.\n"
                    " * @return The input.\n */\nint Example(int value);\n",
                    encoding="utf-8",
                )
                self.assertFalse(quality_baseline.doxygen_check())
                header.write_text(
                    "/** @file\n * @brief Example API.\n */\n"
                    "/** @brief Returns the input.\n * @param value Input value.\n"
                    " * @return The input.\n */\nint Example(int value);\n",
                    encoding="utf-8",
                )
                self.assertTrue(quality_baseline.doxygen_check())

    def test_new_c_source_needs_an_explicit_role(self):
        with tempfile.TemporaryDirectory() as temporary:
            source_dir = Path(temporary) / "runtime" / "src"
            source_dir.mkdir(parents=True)
            for name in (
                "Can.c",
                "CanIf.c",
                "CanTp.c",
                "Com.c",
                "Dcm.c",
                "Dem.c",
                "LSduR.c",
                "NvM.c",
                "Os.c",
                "PduR.c",
                "Rte.c",
                "ecu_host_main.c",
                "Ecu_Runtime.c",
                "Ecu_Status.c",
                "Security.c",
                "NewModule.c",
            ):
                (source_dir / name).touch()
            self.assertEqual(
                c_scope_errors(Path(temporary)),
                ["Unclassified runtime C source: NewModule.c"],
            )

    def test_unreviewed_spec_evidence_cannot_pass(self):
        state = {
            "capabilities": [
                {"id": "A", "gates": {"spec_obligations": {"status": "not_run"}}},
                {"id": "B", "gates": {"spec_obligations": {"status": "passed"}}},
            ]
        }
        self.assertEqual(spec_evidence_gaps(state), ["A: spec_obligations=not_run"])

    def test_generated_scan_requires_actual_listed_configuration(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "files.list").write_bytes(
                b"Dcm_Externals.h\nEcu_Config.c\ninclude/Ecu_Config.h\n"
            )
            (directory / "Ecu_Config.c").touch()
            (directory / "Dcm_Externals.h").touch()
            self.assertEqual(
                generated_input_errors(directory),
                ["Generated output is missing include/Ecu_Config.h"],
            )
            (directory / "include").mkdir()
            (directory / "include/Ecu_Config.h").touch()
            listing = (directory / "files.list").read_bytes()
            names = listing.decode().splitlines()
            record = [
                f"{hashlib.sha256((directory / name).read_bytes()).hexdigest()}  {name}"
                for name in names
            ]
            record.append(f"{hashlib.sha256(listing).hexdigest()}  files.list")
            (directory / "files.sha256").write_bytes(
                ("\n".join(record) + "\n").encode()
            )
            self.assertEqual(generated_input_errors(directory), [])
            (directory / "Ecu_Config.c").write_text("changed")
            self.assertEqual(
                generated_input_errors(directory),
                ["Generated files.sha256 does not match output bytes"],
            )
            (directory / "files.list").write_bytes(b"../outside.c\n")
            self.assertEqual(
                generated_input_errors(directory),
                ["Generated files.list has unsafe or duplicate paths"],
            )


if __name__ == "__main__":
    unittest.main()
