"""ETW XML decoding and exact process identity joins, without a Windows dependency."""

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling.windows_etw import (
    PROFILE,
    correlate,
    events,
    filetime,
    start,
    stop,
)


class WindowsEtwTests(unittest.TestCase):
    def test_filetime_preserves_the_seventh_fractional_digit(self):
        self.assertEqual(filetime("1601-01-01T00:00:00.0000001Z"), 1)
        self.assertEqual(filetime("1601-01-01T00:00:00.000000100+00:00"), 1)
        self.assertEqual(filetime("  134390529764705859"), 134390529764705859)
        self.assertEqual(filetime("0x10"), 16)
        self.assertEqual(filetime("134390529764705859"), 134390529764705859)

    def test_tracerpt_preserves_named_fields_and_header_time(self):
        xml = """<Events xmlns="http://schemas.microsoft.com/win/2004/08/events/event">
        <Event><System><Provider Name="Microsoft-Windows-Kernel-Process"/>
        <EventID>1</EventID><TimeCreated SystemTime="2026-11-17T00:00:00.0000001Z"/>
        </System><ExtendedTracingInfo>
        <EventGuid>{3d6fa8d0-fe05-11d0-9dda-00c04fd7ba7c}</EventGuid>
        </ExtendedTracingInfo><EventData><Data Name="ProcessID">42</Data>
        <Data Name="ImageName">python.exe</Data></EventData></Event></Events>"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "events.xml"
            path.write_text(xml)
            decoded = list(events(path))
        self.assertEqual(
            decoded[0]["event_guid"], "{3d6fa8d0-fe05-11d0-9dda-00c04fd7ba7c}"
        )
        self.assertEqual(decoded[0]["fields"]["ProcessID"], "42")
        self.assertEqual(decoded[0]["fields"]["ImageName"], "python.exe")
        self.assertEqual(
            decoded[0]["timestamp"]["SystemTime"], "2026-11-17T00:00:00.0000001Z"
        )

    def test_join_excludes_a_reused_pid_and_missing_start_fails_control(self):
        def event(identifier, created, **fields):
            if identifier == 2:
                fields["ExitTime"] = str(created + 10)
            return {
                "event_id": str(identifier),
                "opcode": "1",
                "provider": {"Guid": "{22fb2cd6-0e7b-422b-a0c7-2fad1fd0e716}"},
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
            data[4]["provider"] = {"Guid": "{9e814aad-3204-11d2-9a82-006008a86939}"}
            data[4]["event_guid"] = "{3d6fa8d0-fe05-11d0-9dda-00c04fd7ba7c}"
            data[4]["timestamp"]["SystemTime"] = "1601-01-01T00:00:00.000010100+00:00"
            path = folder / "lifecycle.jsonl"
            path.write_text("".join(json.dumps(value) + "\n" for value in data))
            correlate(folder)
            joined = json.loads((folder / "members-etw.jsonl").read_text())
            self.assertEqual(joined["etw"]["created_100ns"], 100)
            commands = joined["etw"]["command_events"]
            self.assertEqual(
                [value["fields"]["CommandLine"] for value in commands], ["old child"]
            )
            # Losing the reused PID's manifest start must not attach its later command.
            path.write_text(
                "".join(
                    json.dumps(value) + "\n"
                    for index, value in enumerate(data)
                    if index != 2
                )
            )
            correlate(folder)
            joined = json.loads((folder / "members-etw.jsonl").read_text())
            self.assertEqual(len(joined["etw"]["command_events"]), 1)
            rundown = event(0, 102, CommandLine="rundown")
            rundown["opcode"] = "3"
            unrelated = event(0, 103, CommandLine="unrelated")
            unrelated["provider"] = {"Guid": "unrelated"}
            path.write_text(
                "".join(
                    json.dumps(value) + "\n" for value in data + [rundown, unrelated]
                )
            )
            correlate(folder)
            joined = json.loads((folder / "members-etw.jsonl").read_text())
            self.assertEqual(len(joined["etw"]["command_events"]), 1)
            # Removing the exact start while leaving a same-PID event cannot satisfy evidence.
            path.write_text("".join(json.dumps(value) + "\n" for value in data[1:]))
            with self.assertRaises(KeyError):
                correlate(folder)

    def test_wpr_profile_has_both_modes_and_resolvable_provider_references(self):
        import xml.etree.ElementTree as ET

        root = ET.fromstring(PROFILE)
        ids = {
            element.get("Id"): element for element in root.iter() if element.get("Id")
        }
        profiles = root.findall("./Profiles/Profile")
        self.assertEqual(
            {profile.get("LoggingMode") for profile in profiles}, {"File", "Memory"}
        )
        for profile in profiles:
            self.assertEqual(
                profile.get("Id"),
                ".".join(
                    profile.get(name) for name in ("Name", "DetailLevel", "LoggingMode")
                ),
            )
            for reference in profile.iter():
                if reference.tag in {
                    "SystemCollectorId",
                    "SystemProviderId",
                    "EventCollectorId",
                    "EventProviderId",
                }:
                    target = ids[reference.get("Value")]
                    self.assertEqual(target.tag, reference.tag.removesuffix("Id"))
        self.assertEqual(
            ids["JobProcessProvider"].get("Name"), "Microsoft-Windows-Kernel-Process"
        )
        self.assertEqual(ids["JobProcessProvider"].get("NonPagedMemory"), "true")

    def test_collector_failure_degrades_evidence_without_blocking_tests(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            with patch(
                "autosar_tooling.windows_etw.command",
                side_effect=RuntimeError("WPR rejected profile"),
            ):
                self.assertFalse(start(folder))
            state = json.loads((folder / "collector-state.json").read_text())
            self.assertEqual(state["phase"], "start")
            self.assertIn("WPR rejected profile", state["error"])
            self.assertFalse(stop(folder))
            state = json.loads((folder / "collector-state.json").read_text())
            self.assertFalse(state["complete"])

    def test_successful_system_only_capture_is_marked_incomplete(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / "started").touch()
            (folder / "events.xml").write_text(
                '<Events><Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event">'
                '<System><Provider Guid="{9e814aad-3204-11d2-9a82-006008a86939}"/>'
                "<EventID>0</EventID><Opcode>0</Opcode></System>"
                '<EventData><Data Name="EventsLost"> 0</Data>'
                '<Data Name="BuffersLost"> 0</Data></EventData></Event></Events>'
            )
            with patch("autosar_tooling.windows_etw.command"):
                self.assertFalse(stop(folder))
            state = json.loads((folder / "collector-state.json").read_text())
            self.assertIn("no Kernel-Process lifecycle events", state["error"])
            health = json.loads((folder / "trace-health.json").read_text())
            self.assertEqual(health["decoded_events"], 1)
            self.assertEqual(health["process_events"], 0)
