"""Regression checks for the independently found fixture integrity gaps."""

import contextlib
import io
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import input_oracles


class InputIntegrityTests(unittest.TestCase):
    def test_unlisted_xsd_invalid_input_cannot_enter_a_passing_baseline(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Path(directory) / "fixture"
            shutil.copytree(input_oracles.FIXTURE, fixture)
            with patch.object(input_oracles, "FIXTURE", fixture):
                with contextlib.redirect_stdout(io.StringIO()):
                    input_oracles.main()
                (fixture / "positive/extra.arxml").write_text(
                    '<AUTOSAR xmlns="http://autosar.org/schema/r4.0"><NOT-AUTOSAR/></AUTOSAR>',
                    encoding="utf-8",
                )
                with self.assertRaisesRegex(ValueError, "file set differs"):
                    input_oracles.main()

    def test_local_object_cannot_shadow_pinned_official_definition(self):
        xml = (
            b'<AUTOSAR xmlns="http://autosar.org/schema/r4.0"><AR-PACKAGES><AR-PACKAGE>'
            b"<SHORT-NAME>AUTOSAR</SHORT-NAME><AR-PACKAGES><AR-PACKAGE>"
            b"<SHORT-NAME>EcucDefs</SHORT-NAME><ELEMENTS><ECUC-MODULE-DEF>"
            b"<SHORT-NAME>Os</SHORT-NAME></ECUC-MODULE-DEF></ELEMENTS>"
            b"</AR-PACKAGE></AR-PACKAGES></AR-PACKAGE></AR-PACKAGES></AUTOSAR>"
        )
        external = input_oracles.objects(input_oracles.load_xml(xml), "external MOD")
        with self.assertRaisesRegex(ValueError, "shadows pinned MOD definition"):
            input_oracles.audit(
                {"shadow.arxml": xml},
                external,
                {"checks": [], "required_current_entries": [], "bsw_signatures": {}},
            )


if __name__ == "__main__":
    unittest.main()
