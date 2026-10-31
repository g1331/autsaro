"""Normal delivered query consumes and persists stderr without altering stdout."""
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

from ecu_tools.build import query
from ecu_tools.process import ProcessResult


class ComToolTests(unittest.TestCase):
    def invoke(self, stderr: str, *, success=True):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        logs = Path(temporary.name)
        out, err = logs / "stdout", logs / "stderr"
        out.write_text("READY HostBatchV1\n")
        err.write_text(stderr)
        result = ProcessResult("test", 1, None, 0 if success else 5, "exited", out, err)
        owner = Mock()
        owner.wait.return_value = result
        with patch("ecu_tools.build.OwnedProcess", return_value=owner):
            text = query([str(logs / "ecu_probe")], logs, "observe")
        return text, json.loads((logs / "observe-com-rejections.json").read_text())

    def test_normal_query_preserves_exact_records_and_stdout(self):
        text, diagnostics = self.invoke("COM_TX_REJECT pdu=19 result=7\nCOM_TX_REJECT_END count=1\n")
        self.assertEqual(text, "READY HostBatchV1\n")
        self.assertEqual(diagnostics, {"records": [{"pdu": 19, "result": 7}], "complete": True})

    def test_incomplete_missing_end_and_abnormal_exit_refuse_complete_success(self):
        for stream, success in [("COM_TX_REJECT_INCOMPLETE\nCOM_TX_REJECT_END count=0\n", True),
                                ("COM_TX_REJECT pdu=19 result=7\n", True),
                                ("COM_TX_REJECT_END count=0\n", False)]:
            with self.subTest(stream=stream, success=success), self.assertRaises(RuntimeError):
                self.invoke(stream, success=success)
