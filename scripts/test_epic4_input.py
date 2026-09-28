"""Regression checks for the independently found fixture integrity gaps."""

import contextlib
import io
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import epic4_input


class InputIntegrityTests(unittest.TestCase):
    def test_unlisted_xsd_invalid_input_cannot_enter_a_passing_baseline(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Path(directory) / "fixture"
            shutil.copytree(epic4_input.FIXTURE, fixture)
            with patch.object(epic4_input, "FIXTURE", fixture):
                with contextlib.redirect_stdout(io.StringIO()):
                    epic4_input.main()
                (fixture / "positive/extra.arxml").write_text(
                    '<AUTOSAR xmlns="http://autosar.org/schema/r4.0"><NOT-AUTOSAR/></AUTOSAR>',
                    encoding="utf-8",
                )
                with self.assertRaisesRegex(ValueError, "file set differs"):
                    epic4_input.main()

    def test_local_object_cannot_shadow_pinned_official_definition(self):
        xml = (
            '<AUTOSAR xmlns="http://autosar.org/schema/r4.0"><AR-PACKAGES><AR-PACKAGE>'
            "<SHORT-NAME>AUTOSAR</SHORT-NAME><AR-PACKAGES><AR-PACKAGE>"
            "<SHORT-NAME>EcucDefs</SHORT-NAME><ELEMENTS><ECUC-MODULE-DEF>"
            "<SHORT-NAME>Os</SHORT-NAME></ECUC-MODULE-DEF></ELEMENTS>"
            "</AR-PACKAGE></AR-PACKAGES></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"
        ).encode()
        external = epic4_input.objects(epic4_input.load_xml(xml), "external MOD")
        with self.assertRaisesRegex(ValueError, "shadows pinned MOD definition"):
            epic4_input.audit(
                {"shadow.arxml": xml},
                external,
                {"checks": [], "required_current_entries": [], "bsw_signatures": {}},
            )


if __name__ == "__main__":
    unittest.main()
