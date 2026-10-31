"""Independent bounded COM publication, concurrent writers and output-failure controls."""
from __future__ import annotations

import os
import shutil
import tempfile
import unittest
from pathlib import Path

from autosar_tooling.config import ROOT
from ecu_tools.build import com_rejections, query


class ComRejectionTests(unittest.TestCase):
    def test_publication_capacity_preemption_multiwriter_and_output_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            binary = directory / ("com-rejections.exe" if os.name == "nt" else "com-rejections")
            compiler = Path(os.environ.get("AUTOSAR_CC") or shutil.which("gcc")).resolve()
            includes = ["runtime/bsw/include", "runtime/os/include", "runtime/ecu/include",
                        "runtime/os/src/host/linux", "core/tests/fixtures/bsw-contracts"]
            flags = ["-std=c99", "-Wall", "-Wextra", "-Werror", "-O1"]
            if os.name != "nt":
                flags.append("-pthread")
            for include in includes:
                flags.extend(["-I", str(ROOT / include)])
            query([str(compiler), *flags, str(ROOT / "runtime/ecu/src/Ecu_ComRejection.c"),
                   str(ROOT / "runtime/os/src/Os_HostIo.c"),
                   str(ROOT / "tests/fixtures/com_rejections.c"), "-o", str(binary)], directory, "compile")
            for case in ("preempt", "pending", "capacity", "multiwriter", "output", "end-output"):
                with self.subTest(case=case):
                    self.assertEqual(query([str(binary), case], directory, case), "")

    def test_parser_requires_normal_end_and_preserves_exact_records(self):
        stream = "COM_TX_REJECT pdu=17 result=3\nCOM_TX_REJECT pdu=17 result=3\nCOM_TX_REJECT_END count=2\n"
        self.assertEqual(com_rejections(stream, success=True), {
            "records": [{"pdu": 17, "result": 3}, {"pdu": 17, "result": 3}], "complete": True})
        self.assertFalse(com_rejections(stream, success=False)["complete"])
        self.assertFalse(com_rejections(stream.split("COM_TX_REJECT_END")[0], success=True)["complete"])
        self.assertFalse(com_rejections("COM_TX_REJECT_INCOMPLETE\nCOM_TX_REJECT_END count=0\n", success=True)["complete"])
        for bad in (stream.replace("count=2", "count=1"), stream + "COM_TX_REJECT pdu=0 result=1\n",
                    stream.replace("result=3", "result=0"), "COM_TX_REJECT pdu=broken\n"):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                com_rejections(bad, success=True)
