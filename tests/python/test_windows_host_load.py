"""Host counter sampling for Windows CI jobs: argv, CSV reading and sampler shutdown."""

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from autosar_tooling import windows_etw
from autosar_tooling import windows_host_load as load


def csv_text(rows, partial=""):
    """typeperf CSV: quoted cells, counters prefixed with the computer name."""
    names = ['"(PDH-CSV 4.0)"'] + [
        f'"\\\\RUNNER{name}"' for name in load.COUNTERS.values()
    ]
    lines = [",".join(names)]
    for stamp, values in rows:
        cells = [f'"{stamp}"'] + [f'"{value}"' for value in values]
        lines.append(",".join(cells))
    return "\n".join(lines) + "\n" + partial


def row(index, busy, queue, latency_s):
    # The first sample of a rate counter is blank; later rows carry every counter.
    values = {key: " " if busy == " " else 1 for key in load.COUNTERS}
    values.update(
        cpu_busy_pct=busy,
        run_queue_length=queue,
        disk_latency_ms=latency_s,
        memory_available_mb=12000,
    )
    return (f"11/19/2026 15:53:{index:02d}.000", list(values.values()))


class HostLoadTests(unittest.TestCase):
    def write(self, folder, text, running=False):
        (folder / "samples.csv").write_text(text)
        (folder / "typeperf.log").write_text("")
        (folder / "sampler.json").write_text(json.dumps({"pid": 4321}))
        return patch.object(load, "terminate", return_value=running)

    def test_sampler_command_reads_counters_at_a_low_fixed_rate(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory) / "host-load"
            with patch.object(load.subprocess, "Popen") as popen:
                popen.return_value.pid = 4321
                load.start(folder)
            argv = popen.call_args.args[0]
            self.assertEqual(argv[0], "typeperf")
            self.assertEqual(
                argv[1 : 1 + len(load.COUNTERS)], list(load.COUNTERS.values())
            )
            self.assertEqual(argv[argv.index("-si") + 1], str(load.INTERVAL_S))
            self.assertEqual(argv[argv.index("-sc") + 1], str(load.MAX_SAMPLES))
            self.assertEqual(Path(argv[argv.index("-o") + 1]), folder / "samples.csv")
            self.assertEqual(
                json.loads((folder / "sampler.json").read_text())["pid"], 4321
            )

    def test_summary_skips_blank_first_samples_and_partial_last_row(self):
        rows = [row(0, " ", " ", " "), row(5, 20, 0, 0.001), row(10, 90, 6, 0.004)]
        rows.append(row(15, 100, 2, 0.010))
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            with self.write(
                folder, csv_text(rows, '"11/19/2026 15:53:20.000","9'), True
            ):
                text = load.stop(folder)
            summary = json.loads((folder / "summary.json").read_text())
            self.assertEqual(text, (folder / "summary.md").read_text())
        self.assertEqual(summary["samples"], 3)
        self.assertTrue(summary["sampler_running_at_stop"])
        busy = summary["counters"]["cpu_busy_pct"]
        self.assertEqual((busy["median"], busy["p95"], busy["max"]), (90, 100, 100))
        self.assertEqual(summary["counters"]["run_queue_length"]["max"], 6)
        # Avg. Disk sec/Transfer is seconds; the report is in milliseconds.
        self.assertAlmostEqual(summary["counters"]["disk_latency_ms"]["max"], 10)
        self.assertIn("| cpu_busy_pct | 90.00 | 100.00 | 100.00 |", text)
        self.assertIn("sampler running at stop: True", text)

    def test_missing_samples_report_what_typeperf_said(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / "sampler.json").write_text(json.dumps({"pid": 4321}))
            (folder / "typeperf.log").write_text("Error: No valid counters.")
            with (
                patch.object(load, "terminate", return_value=False),
                self.assertRaisesRegex(RuntimeError, "No valid counters"),
            ):
                load.stop(folder)

    def test_only_a_running_typeperf_with_the_recorded_pid_is_killed(self):
        def tasklist(image):
            return type("Result", (), {"stdout": f'"{image}","4321","Console"'})()

        with patch.object(load.subprocess, "run") as run:
            run.return_value = tasklist("typeperf.exe")
            self.assertTrue(load.terminate(4321))
            self.assertEqual(run.call_args.args[0], ["taskkill", "/PID", "4321", "/F"])
        with patch.object(load.subprocess, "run") as run:
            run.return_value = tasklist("python.exe")
            self.assertFalse(load.terminate(4321))
            self.assertEqual(run.call_count, 1)

    def test_sampler_failures_never_stop_the_etw_steps(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            with patch.object(load, "start", side_effect=FileNotFoundError("typeperf")):
                windows_etw.start_host_load(folder)
            with patch.object(load, "stop", side_effect=RuntimeError("no samples")):
                windows_etw.stop_host_load(folder)
            with (
                patch.object(
                    load, "stop", return_value="### Windows host load\n"
                ) as stop,
                patch.object(windows_etw.windows_process_timing, "publish") as publish,
            ):
                windows_etw.stop_host_load(folder)
            stop.assert_called_once_with(folder / "host-load")
            publish.assert_called_once_with("### Windows host load\n")


if __name__ == "__main__":
    unittest.main()
