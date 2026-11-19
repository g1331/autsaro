"""ETW XML decoding and exact process identity joins, without a Windows dependency."""

import datetime
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import windows_etw
from autosar_tooling.windows_etw import (
    PROFILE,
    ControlProofError,
    correlate,
    events,
    filetime,
    start,
    stop,
)

EPOCH = datetime.datetime(1601, 1, 1, tzinfo=datetime.UTC)


def iso(ticks, zone):
    """FILETIME ticks in the spelling tracerpt uses: nine fractional digits."""
    seconds, fraction = divmod(ticks, 10_000_000)
    base = EPOCH + datetime.timedelta(seconds=seconds)
    return f"{base:%Y-%m-%dT%H:%M:%S}.{fraction * 100:09d}{zone}"


def process_events(pid, created, lifetime=10_000, skew=0, command="python probe.py"):
    """Creation, classic command line and exit of one process, as tracerpt decodes them.

    CreateTime and ExitTime are kernel FILETIME payload; event headers carry the trace
    clock, which sits `skew` ticks away from it. The classic command-line event follows
    the manifest start event by 7 ticks, as in run 44481920869 (3732).
    """

    def header(ticks):
        return {"SystemTime": iso(ticks + skew, "+00:00")}

    manifest = {
        "provider": {
            "Name": "Microsoft-Windows-Kernel-Process",
            "Guid": "{22fb2cd6-0e7b-422b-a0c7-2fad1fd0e716}",
        },
        "event_guid": None,
    }
    image = (
        r"\Device\HarddiskVolume3\hostedtoolcache\windows\Python\3.12.10\x64\python.exe"
    )
    identity = {"ProcessID": f"{pid:8}", "CreateTime": iso(created, "Z")}
    return [
        {
            **manifest,
            "event_id": "1",
            "opcode": "1",
            "timestamp": header(created),
            "fields": {**identity, "ParentProcessID": "    6128", "ImageName": image},
        },
        {
            "provider": {"Guid": "{9e814aad-3204-11d2-9a82-006008a86939}"},
            "event_guid": "{3d6fa8d0-fe05-11d0-9dda-00c04fd7ba7c}",
            "event_id": "0",
            "opcode": "1",
            "timestamp": header(created + 7),
            "fields": {
                "ProcessId": f"0x{pid:X}",
                "ImageFileName": "python.exe",
                "CommandLine": command,
            },
        },
        {
            **manifest,
            "event_id": "2",
            "opcode": "2",
            "timestamp": header(created + lifetime),
            "fields": {
                **identity,
                "ExitTime": iso(created + lifetime, "Z"),
                "CPUCycleCount": "244754516",
                "ImageName": "python.exe",
            },
        },
    ]


def write_run(root, data, controls=(), members=None):
    """Lay out one job's ETW folder and evidence, then correlate it."""
    folder = root / "etw"
    folder.mkdir()
    evidence = root / "job-evidence"
    evidence.mkdir()
    (folder / "lifecycle.jsonl").write_text(
        "".join(json.dumps(value) + "\n" for value in data)
    )
    for pid, created in controls:
        (evidence / f"control-{pid}.json").write_text(
            json.dumps({"pid": pid, "created_100ns": created})
        )
    members = members if members is not None else [control for control in controls]
    records = [
        {"phase": "before_stop", "members": [{"pid": pid, "created_100ns": created}]}
        for pid, created in members
    ]
    (evidence / "rust-control.jsonl").write_text(
        "".join(json.dumps(record) + "\n" for record in records)
    )
    correlate(folder)
    return folder


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

    def test_control_command_line_joins_whatever_the_trace_clock_offset(self):
        created = 134395479825996909  # CreateTime of control 3732 in run 44481920869
        # Header time minus CreateTime was -46 ticks for this control, -36439 at worst in
        # that run and up to +33848; the join must hold at every offset.
        for skew in (-46, -36439, 0, 33848):
            with self.subTest(skew=skew), tempfile.TemporaryDirectory() as directory:
                folder = write_run(
                    Path(directory),
                    process_events(
                        3732, created, skew=skew, command="probe descendant"
                    ),
                    controls=[(3732, created)],
                )
                joined = json.loads((folder / "members-etw.jsonl").read_text())
                commands = joined["etw"]["command_events"]
                self.assertEqual(
                    [value["fields"]["CommandLine"] for value in commands],
                    ["probe descendant"],
                )

    def test_join_excludes_a_reused_pid_and_missing_start_fails_control(self):
        first = 134395479825996909
        second = first + 20_000
        # The trace clock runs 3000 ticks ahead of the kernel clock, so the second
        # process's start header precedes the first one's CreateTime + lifetime.
        old = process_events(42, first, skew=-3000, command="old child")
        new = process_events(42, second, skew=-3000, command="new child")
        rundown = process_events(42, first, skew=-3000, command="rundown")[1]
        rundown["opcode"] = "3"
        unrelated = process_events(42, first, skew=-3000, command="unrelated")[1]
        unrelated["provider"] = {"Guid": "unrelated"}
        unrelated["event_guid"] = None

        def commands(folder):
            joined = [
                json.loads(line)
                for line in (folder / "members-etw.jsonl").read_text().splitlines()
            ]
            return {
                member["member"]["created_100ns"]: [
                    value["fields"]["CommandLine"]
                    for value in member["etw"]["command_events"]
                ]
                for member in joined
            }

        with tempfile.TemporaryDirectory() as directory:
            folder = write_run(
                Path(directory),
                old + new + [rundown, unrelated],
                controls=[(42, first)],
                members=[(42, first), (42, second)],
            )
            self.assertEqual(
                commands(folder), {first: ["old child"], second: ["new child"]}
            )
        with tempfile.TemporaryDirectory() as directory:
            # Losing the first exit event leaves the next creation as the only bound.
            lost_exit = [*old[:2], *new]
            folder = write_run(
                Path(directory),
                lost_exit,
                members=[(42, first), (42, second)],
            )
            self.assertEqual(
                commands(folder), {first: ["old child"], second: ["new child"]}
            )
        with tempfile.TemporaryDirectory() as directory:
            # Losing the reused PID's manifest start must not attach its later command.
            folder = write_run(
                Path(directory),
                old + new[1:],
                controls=[(42, first)],
                members=[(42, first)],
            )
            self.assertEqual(commands(folder), {first: ["old child"]})
        # Removing the exact start while leaving a same-PID event cannot satisfy evidence.
        with (
            tempfile.TemporaryDirectory() as directory,
            self.assertRaisesRegex(ControlProofError, "no process start"),
        ):
            write_run(Path(directory), old[1:], controls=[(42, first)])

    def test_unreadable_control_proofs_fail_instead_of_escaping(self):
        created = 134395479825996909
        start, classic, stop = process_events(3732, created)
        nameless = {**start, "fields": {**start["fields"]}}
        del nameless["fields"]["ImageName"]
        for name, data, proof in (
            (
                "image field",
                [nameless, classic, stop],
                '{"pid": 3732, "created_100ns": %d}',
            ),
            ("truncated proof", [start, classic, stop], "{"),
            ("missing pid", [start, classic, stop], '{"created_100ns": %d}'),
        ):
            with (
                self.subTest(name),
                tempfile.TemporaryDirectory() as directory,
                self.assertRaisesRegex(ControlProofError, "unreadable proof"),
            ):
                root = Path(directory)
                (root / "job-evidence").mkdir()
                (root / "job-evidence" / "control-1.json").write_text(
                    proof.replace("%d", str(created))
                )
                (root / "etw").mkdir()
                (root / "etw" / "lifecycle.jsonl").write_text(
                    "".join(json.dumps(value) + "\n" for value in data)
                )
                correlate(root / "etw")

    def test_each_missing_control_fact_fails_the_proof(self):
        created = 134395479825996909
        start, classic, stop = process_events(3732, created)

        def with_field(event, **fields):
            return {**event, "fields": {**event["fields"], **fields}}

        cases = {
            "command line": [start, stop],
            "exit": [start, classic],
            "parent": [with_field(start, ParentProcessID="0"), classic, stop],
            "image": [with_field(start, ImageName="cmd.exe"), classic, stop],
        }
        for missing, data in cases.items():
            with (
                self.subTest(missing=missing),
                tempfile.TemporaryDirectory() as directory,
                self.assertRaisesRegex(ControlProofError, missing),
            ):
                write_run(Path(directory), data, controls=[(3732, created)])

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

    def test_stop_times_the_quiet_probe_and_publishes_the_labelled_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / "started").touch()
            (folder / "events.xml").write_text(
                '<Events><Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event">'
                '<System><Provider Name="Microsoft-Windows-Kernel-Process" '
                'Guid="{22fb2cd6-0e7b-422b-a0c7-2fad1fd0e716}"/>'
                "<EventID>1</EventID><Opcode>1</Opcode>"
                '<TimeCreated SystemTime="2026-11-19T15:53:06.688326100+00:00"/></System>'
                '<EventData><Data Name="ProcessID"> 5</Data></EventData></Event></Events>'
            )
            timing = "autosar_tooling.windows_etw.windows_process_timing"
            with (
                patch("autosar_tooling.windows_etw.command"),
                patch("autosar_tooling.windows_etw.stop_host_load"),
                patch(
                    "autosar_tooling.windows_etw.benchmark", return_value=[0.02] * 12
                ),
                patch(f"{timing}.write_summary", return_value="TEXT") as summary,
                patch(f"{timing}.publish") as publish,
                patch.dict(os.environ, {"AUTOSAR_ETW_LABEL": "native-protocol"}),
            ):
                self.assertTrue(stop(folder))
            summary.assert_called_once_with(folder, "native-protocol")
            publish.assert_called_once_with("TEXT")
            end = json.loads((folder / "reference-end.json").read_text())
            self.assertEqual(end, {"samples_s": [0.02] * 12})

    def test_a_timing_summary_failure_is_reported_without_raising(self):
        data = process_events(3732, 134395479825996909)
        del data[2]["fields"]["CPUCycleCount"]
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / "lifecycle.jsonl").write_text(
                "".join(json.dumps(value) + "\n" for value in data)
            )
            with patch("sys.stderr") as stderr:
                windows_etw.publish_process_timing(folder)
            self.assertFalse((folder / "process-timing.md").exists())
        written = "".join(call.args[0] for call in stderr.write.call_args_list)
        self.assertIn("Process timing summary unavailable", written)
        self.assertIn("CPUCycleCount", written)

    def test_required_controls_fail_the_stop_step_when_the_proof_fails(self):
        created = 134395479825996909
        start, classic, stop_event = process_events(3732, created)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            folder = root / "etw"
            folder.mkdir()
            (folder / "started").touch()
            evidence = root / "job-evidence"
            evidence.mkdir()
            (evidence / "control-3732.json").write_text(
                json.dumps({"pid": 3732, "created_100ns": created})
            )

            def run(data, required):
                environment = {"AUTOSAR_ETW_REQUIRE_CONTROLS": required}
                with (
                    patch.dict(os.environ, environment),
                    patch("autosar_tooling.windows_etw.command"),
                    patch("autosar_tooling.windows_etw.events", return_value=data),
                ):
                    return stop(folder)

            # The failing run's shape: creation and exit are traced, the command line is not.
            self.assertFalse(run([start, stop_event], "false"))
            state = json.loads((folder / "collector-state.json").read_text())
            self.assertFalse(state["complete"])
            self.assertIn("command line", state["error"])
            with self.assertRaisesRegex(ControlProofError, "command line"):
                run([start, stop_event], "true")
            state = json.loads((folder / "collector-state.json").read_text())
            self.assertFalse(state["complete"])
            self.assertTrue(run([start, classic, stop_event], "true"))
            state = json.loads((folder / "collector-state.json").read_text())
            self.assertTrue(state["complete"])
            (evidence / "control-3732.json").unlink()
            with self.assertRaisesRegex(ControlProofError, "no identity proof"):
                run([start, classic, stop_event], "true")
            self.assertTrue(run([start, classic, stop_event], "false"))

    def test_required_controls_fail_the_stop_step_on_any_collection_failure(self):
        created = 134395479825996909
        data = process_events(3732, created)

        def prepare(directory, started=True):
            folder = Path(directory) / "etw"
            folder.mkdir()
            if started:
                (folder / "started").touch()
            evidence = Path(directory) / "job-evidence"
            evidence.mkdir()
            (evidence / "control-3732.json").write_text(
                json.dumps({"pid": 3732, "created_100ns": created})
            )
            return folder

        def run(folder, events_, required):
            environment = {"AUTOSAR_ETW_REQUIRE_CONTROLS": required}
            with (
                patch.dict(os.environ, environment),
                patch("autosar_tooling.windows_etw.command"),
                patch("autosar_tooling.windows_etw.events", return_value=events_),
            ):
                return stop(folder)

        loss = {
            "provider": {"Guid": "{9e814aad-3204-11d2-9a82-006008a86939}"},
            "event_id": "0",
            "opcode": "0",
            "event_guid": None,
            "timestamp": {},
            "fields": {"EventsLost": "3", "BuffersLost": "0"},
        }
        cases = {
            "never started": (False, data, "never started"),
            "no process events": (True, [loss], "no Kernel-Process lifecycle"),
            "lost events": (True, [*data, loss], "lost events"),
        }
        for name, (started, events_, message) in cases.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                folder = prepare(directory, started)
                with self.assertRaisesRegex(RuntimeError, message):
                    run(folder, events_, "true")
                state = json.loads((folder / "collector-state.json").read_text())
                self.assertFalse(state["complete"])
                self.assertFalse(run(folder, events_, "false"))
