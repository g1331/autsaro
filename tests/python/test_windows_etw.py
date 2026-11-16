"""ETW XML decoding and exact process identity joins, without a Windows dependency."""

import json
import tempfile
import unittest
from pathlib import Path

from autosar_tooling.windows_etw import correlate, events, filetime


class WindowsEtwTests(unittest.TestCase):
    def test_filetime_preserves_the_seventh_fractional_digit(self):
        self.assertEqual(filetime("1601-01-01T00:00:00.0000001Z"), 1)
        self.assertEqual(filetime("0x10"), 16)
        self.assertEqual(filetime("134390529764705859"), 134390529764705859)

    def test_tracerpt_preserves_named_fields_and_header_time(self):
        xml = """<Events xmlns="http://schemas.microsoft.com/win/2004/08/events/event">
        <Event><System><Provider Name="Microsoft-Windows-Kernel-Process"/>
        <EventID>1</EventID><TimeCreated SystemTime="2026-11-17T00:00:00.0000001Z"/>
        </System><EventData><Data Name="ProcessID">42</Data>
        <Data Name="ImageName">python.exe</Data></EventData></Event></Events>"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "events.xml"
            path.write_text(xml)
            decoded = list(events(path))
        self.assertEqual(decoded[0]["fields"]["ProcessID"], "42")
        self.assertEqual(decoded[0]["fields"]["ImageName"], "python.exe")
        self.assertEqual(
            decoded[0]["timestamp"]["SystemTime"], "2026-11-17T00:00:00.0000001Z"
        )

    def test_join_excludes_a_reused_pid_and_missing_start_fails_control(self):
        def event(identifier, created, **fields):
            return {
                "event_id": str(identifier),
                "timestamp": {"SystemTime": str(created)},
                "fields": {
                    "ProcessID": "42",
                    "CreateTime": str(created),
                    "ImageName": "python.exe",
                    "ParentProcessID": "7",
                    **fields,
                },
            }

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            folder = root / "etw"
            folder.mkdir()
            evidence = root / "job-evidence"
            evidence.mkdir()
            member = {"pid": 42, "created_100ns": 100}
            (evidence / "rust-control.jsonl").write_text(
                json.dumps({"phase": "before_stop", "members": [member]}) + "\n"
            )
            (evidence / "control-real.json").write_text(json.dumps(member))
            data = [
                event(1, 100),
                event(2, 100),
                event(1, 200),
                event(2, 200),
                event(0, 101, CommandLine="old child"),
                event(0, 201, CommandLine="new child"),
            ]
            path = folder / "lifecycle.jsonl"
            path.write_text("".join(json.dumps(value) + "\n" for value in data))
            correlate(folder)
            joined = json.loads((folder / "members-etw.jsonl").read_text())
            self.assertEqual(joined["etw"]["created_100ns"], 100)
            commands = joined["etw"]["command_events"]
            self.assertEqual(
                [value["fields"]["CommandLine"] for value in commands], ["old child"]
            )
            # Removing the exact start while leaving a same-PID event cannot satisfy evidence.
            path.write_text("".join(json.dumps(value) + "\n" for value in data[1:]))
            with self.assertRaises(KeyError):
                correlate(folder)
