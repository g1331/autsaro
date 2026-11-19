"""Per-image process timing and the runner-versus-product verdict, without Windows."""

import datetime
import json
import tempfile
import unittest
from pathlib import Path

from autosar_tooling import windows_process_timing as timing

KERNEL_PROCESS = {
    "Name": "Microsoft-Windows-Kernel-Process",
    "Guid": timing.KERNEL_PROCESS,
}
MHZ = 2000
START = datetime.datetime(2026, 11, 19, 15, 53, tzinfo=datetime.UTC)


def stamp(seconds):
    moment = START + datetime.timedelta(seconds=seconds)
    return f"{moment:%Y-%m-%dT%H:%M:%S.%f}00Z"


def header():
    return {
        "provider": {"Guid": "{9e814aad-3204-11d2-9a82-006008a86939}"},
        "event_id": "0",
        "opcode": "0",
        "fields": {"CPUSpeed": f"    {MHZ}", "NumberOfProcessors": "       4"},
    }


def process(pid, image, begin, lifetime, cpu_seconds):
    """Manifest start and exit events as tracerpt decodes them (padded numbers)."""
    identity = {"ProcessID": f"{pid:8}", "CreateTime": stamp(begin)}
    path = rf"\Device\HarddiskVolume3\tools\{image}"
    return [
        {
            "provider": KERNEL_PROCESS,
            "event_id": "1",
            "opcode": "1",
            "fields": {**identity, "ParentProcessID": "     100", "ImageName": path},
        },
        {
            "provider": KERNEL_PROCESS,
            "event_id": "2",
            "opcode": "2",
            "fields": {
                **identity,
                "ExitTime": stamp(begin + lifetime),
                "CPUCycleCount": str(int(cpu_seconds * MHZ * 1_000_000)),
                "ImageName": image,
            },
        },
    ]


def write_trace(folder, processes, start_probe=0.015, end_probe=None):
    events = [header()]
    for pid, (image, lifetime, cpu) in enumerate(processes, start=1000):
        events += process(pid, image, pid, lifetime, cpu)
    (folder / "lifecycle.jsonl").write_text(
        "".join(json.dumps(event) + "\n" for event in events)
    )
    (folder / "overhead.json").write_text(
        json.dumps({"without_etw_s": [start_probe] * 12})
    )
    if end_probe is not None:
        (folder / "reference-end.json").write_text(
            json.dumps({"samples_s": [end_probe] * 12})
        )


def workload(slowdown=1.0):
    """The tool mix of a build, `slowdown` times slower, plus one root that outlives tests."""
    mix = [("GIT.EXE", 0.04, 0.015)] * 29 + [("GIT.EXE", 0.04, 0.035)]
    mix += [("cc1.exe", 0.5, 0.45)] * 25
    mix += [("python.exe", 300.0, 0.05)] + [("ld.exe", 0.1, 0.07)] * 20
    return [(image, life * slowdown, cpu * slowdown) for image, life, cpu in mix]


BASELINE = {
    "labels": {
        "native-protocol": {
            "probe_ms": 15.0,
            "images": {
                "git.exe": {"wall_ms": 40.0, "cpu_ms": 15.0},
                "cc1.exe": {"wall_ms": 500.0, "cpu_ms": 450.0},
                "ld.exe": {"wall_ms": 100.0, "cpu_ms": 70.0},
            },
        },
        "native-rest": {"probe_ms": 25.0, "images": {}},
    }
}


class ProcessTimingTests(unittest.TestCase):
    def summary(self, slowdown=1.0, probe=0.015, end=None, label="native-protocol"):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            write_trace(folder, workload(slowdown), probe, end)
            text = timing.write_summary(folder, label, BASELINE)
            data = json.loads((folder / "process-timing.json").read_text())
            self.assertEqual(text, (folder / "process-timing.md").read_text())
        return text, data

    def test_per_image_wall_and_cpu_statistics_from_start_and_exit_events(self):
        _, data = self.summary()
        git = data["images"]["git.exe"]
        self.assertEqual(git["count"], 30)
        self.assertAlmostEqual(git["wall_median_ms"], 40, places=3)
        self.assertAlmostEqual(git["cpu_median_ms"], 15, places=3)
        self.assertAlmostEqual(git["cpu_total_s"], 0.47)
        self.assertEqual(data["cpu_mhz"], MHZ)
        # The root outlives the tests: it is counted, with its own lifetime and maximum.
        python = data["images"]["python.exe"]
        self.assertEqual((python["count"], python["wall_max_s"]), (1, 300.0))

    def test_unfinished_processes_are_not_timed(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            write_trace(folder, workload())
            start, _ = process(5, "stuck.exe", 5, 1, 0)
            with (folder / "lifecycle.jsonl").open("a") as output:
                output.write(json.dumps(start) + "\n")
            records, _ = timing.process_records(folder / "lifecycle.jsonl")
        self.assertNotIn("stuck.exe", {image for image, _, _ in records})

    def test_slow_probe_blames_the_runner(self):
        text, data = self.summary(slowdown=3.0, probe=0.045)
        self.assertEqual(data["comparison"]["verdict"], "runner-slow")
        self.assertAlmostEqual(data["comparison"]["probe_ratio"], 3.0)
        self.assertIn("runner slow: the reference probe is 3.0x the baseline", text)
        self.assertIn("| cc1.exe | 25 | 1500 | 1500 | 1.5 | 1350 | 33.8 | 500 | 3.0x |", text)

    def test_the_slower_of_start_and_end_probe_counts(self):
        _, data = self.summary(probe=0.015, end=0.04)
        self.assertEqual(data["comparison"]["verdict"], "runner-slow")
        self.assertAlmostEqual(data["comparison"]["probe_ratio"], 40 / 15)

    def test_slow_processes_with_a_normal_probe_blame_the_product(self):
        text, data = self.summary(slowdown=3.0, probe=0.016, end=0.014)
        self.assertEqual(data["comparison"]["verdict"], "processes-slow-probe-normal")
        self.assertIn("the product or fixture does more or heavier work", text)

    def test_healthy_run_is_normal_and_the_threshold_is_two_times(self):
        self.assertEqual(self.summary()[1]["comparison"]["verdict"], "normal")
        just_below = self.summary(slowdown=1.9, probe=0.015 * 1.9)[1]
        self.assertEqual(just_below["comparison"]["verdict"], "normal")
        at_threshold = self.summary(slowdown=2.0, probe=0.015 * 2.0)[1]
        self.assertEqual(at_threshold["comparison"]["verdict"], "runner-slow")

    def test_images_below_the_sample_floor_are_not_compared(self):
        few = [("cc1.exe", 5.0, 4_000_000_000)] * (timing.MIN_SAMPLES - 1)
        comparison = timing.compare(
            timing.image_statistics(few, MHZ),
            {"start_ms": 15.0, "end_ms": None},
            "native-protocol",
            BASELINE,
        )
        self.assertEqual(comparison["image_ratios"], {})
        self.assertEqual(comparison["verdict"], "normal")

    def test_jobs_without_a_label_use_the_median_probe_baseline(self):
        text, data = self.summary(slowdown=1.0, probe=0.05, label=None)
        # Median of the recorded shards (15 and 25 ms) is 20 ms; 50 ms is 2.5x.
        self.assertEqual(data["comparison"]["verdict"], "runner-slow")
        self.assertAlmostEqual(data["comparison"]["probe_ratio"], 2.5)
        self.assertIn("### Windows process timing (no label)", text)

    def test_no_baseline_at_all_reports_absolute_figures(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            write_trace(folder, workload())
            text = timing.write_summary(folder, None, {})
        self.assertIn("no baseline for this job", text)

    def test_recorded_baseline_keeps_short_lived_tools_only(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            folders = []
            for index, slowdown in enumerate((1.0, 1.2, 1.1)):
                folder = root / str(index)
                folder.mkdir()
                write_trace(folder, workload(slowdown), 0.015 * slowdown)
                folders.append(folder)
            target = root / "baseline.json"
            timing.record_baseline("native-protocol", "three runs", folders, target)
            recorded = json.loads(target.read_text())["labels"]["native-protocol"]
        self.assertEqual(recorded["source"], "three runs")
        self.assertEqual(recorded["probe_ms"], 16.5)
        self.assertEqual(set(recorded["images"]), {"git.exe", "cc1.exe", "ld.exe"})
        self.assertEqual(recorded["images"]["cc1.exe"]["wall_ms"], 550.0)
        self.assertEqual(recorded["images"]["cc1.exe"]["cpu_ms"], 495.0)

    def test_committed_baseline_covers_every_windows_native_shard(self):
        labels = timing.read_baseline()["labels"]
        for shard in ("protocol", "projects", "integration", "rest"):
            entry = labels[f"native-{shard}"]
            self.assertGreater(entry["probe_ms"], 0)
            self.assertGreater(len(entry["images"]), 3)


if __name__ == "__main__":
    unittest.main()
