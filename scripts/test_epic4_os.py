"""Dependency rejection checks for the independent Epic 4 verifier."""

import shutil
import copy
import json
import re
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import epic4_os


class DependencyTests(unittest.TestCase):
    def test_activation_gate_rejects_swapped_fifo_and_missing_observer(self):
        evidence = json.loads(
            (
                epic4_os.ROOT / "docs/assurance/evidence/epic4/activation-fifo.json"
            ).read_text(encoding="utf-8")
        )
        original = evidence["observations"]
        with patch("epic4_os.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(epic4_os.check_activation(Path("unused.exe"))), len(original)
            )
        for mutation in ["order", "observer"]:
            with self.subTest(mutation=mutation):
                records = copy.deepcopy(original)
                result = records[0]
                if mutation == "order":
                    result["stdout"] = result["stdout"].replace(
                        "trace=ISRLAHaCBMZ", "trace=ISRLAHaBCMZ"
                    )
                else:
                    result["stdout"] = re.sub(
                        r"observer=\d+", "observer=0", result["stdout"]
                    )
                with (
                    patch("epic4_os.execute", side_effect=records),
                    self.assertRaises(AssertionError),
                ):
                    epic4_os.check_activation(Path("unused.exe"))

    def test_finish_gate_rejects_entry_order_return_and_missing_observer(self):
        original = json.loads(
            (
                epic4_os.ROOT
                / "docs/assurance/evidence/epic4/returned-task-finish-regression-4-18.json"
            ).read_text(encoding="utf-8")
        )["observations"]
        with patch("epic4_os.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(epic4_os.check_finish(Path("unused.exe"))), len(original)
            )
        for mutation in ["order", "return", "observer"]:
            with self.subTest(mutation=mutation):
                records = copy.deepcopy(original)
                if mutation == "observer":
                    records[0]["stdout"] = re.sub(
                        r"observer=\d+", "observer=0", records[0]["stdout"]
                    )
                else:
                    records[0]["stdout"] = records[0]["stdout"].replace(
                        "trace=ISRLACMZ",
                        "trace=ISRLCAMZ" if mutation == "order" else "trace=ISRLACXMZ",
                    )
                self.assertNotEqual(records[0]["stdout"], original[0]["stdout"])
                with (
                    patch("epic4_os.execute", side_effect=records),
                    self.assertRaises(AssertionError),
                ):
                    epic4_os.check_finish(Path("unused.exe"))

        # Reject the former whole-OS close instead of accepting a returned Task
        # which failed to finish its activation and run the pending monitor.
        records = copy.deepcopy(original)
        returned = next(row for row in records if row["scenario"] == "missing-end")
        returned["exit"] = 7
        returned["stdout"] = returned["stdout"].replace("trace=ISRLAMZ", "trace=ISRLAZ")
        with (
            patch("epic4_os.execute", side_effect=records),
            self.assertRaises(AssertionError),
        ):
            epic4_os.check_finish(Path("unused.exe"))

    def test_missing_and_modified_kernel_rejected(self):
        with tempfile.TemporaryDirectory(prefix="epic4-dependency-") as directory:
            kernel = Path(directory) / "kernel"
            shutil.copytree(epic4_os.KERNEL, kernel)
            self.assertEqual(epic4_os.verify_sources(kernel)["license"], "MIT")
            source = kernel / "tasks.c"
            source.write_bytes(source.read_bytes() + b"\n/* modified */\n")
            with self.assertRaisesRegex(ValueError, "kernel digest mismatch: tasks.c"):
                epic4_os.verify_sources(kernel)
            source.unlink()
            with self.assertRaises(FileNotFoundError):
                epic4_os.verify_sources(kernel)

    def test_resource_gate_rejects_preemption_wait_and_mask_order(self):
        original = json.loads(
            (
                epic4_os.ROOT
                / "docs/assurance/evidence/epic4/configured-resource-regression-4-18.json"
            ).read_text(encoding="utf-8")
        )["observations"]
        with patch("epic4_os.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(epic4_os.check_resources(Path("unused.exe"))), len(original)
            )
        for index, before, after in [
            (1, "trace=ISRAnHaMZ", "trace=ISRAHnaMZ"),
            (4, "trace=ISRABaMZ", "trace=ISRAaBMZ"),
            (10, "trace=ISRApJaMZ", "trace=ISRAJpaMZ"),
            (17, "trace=ISRAaMZ", "trace=ISRAJrHaMZ"),
        ]:
            with self.subTest(index=index):
                records = copy.deepcopy(original)
                records[index]["stdout"] = records[index]["stdout"].replace(
                    before, after
                )
                self.assertNotEqual(records[index]["stdout"], original[index]["stdout"])
                with (
                    patch("epic4_os.execute", side_effect=records),
                    self.assertRaises(AssertionError),
                ):
                    epic4_os.check_resources(Path("unused.exe"))

    def test_compiler_identity_rejected(self):
        class Result:
            stdout = "unexpected compiler"

        with (
            patch("epic4_os.subprocess.run", return_value=Result()),
            self.assertRaisesRegex(ValueError, "requires GCC"),
        ):
            epic4_os.compiler()

    def test_event_gate_rejects_lost_record_ticket_and_missing_restart(self):
        original = json.loads(
            (
                epic4_os.ROOT / "docs/assurance/evidence/epic4/event-wakeup.json"
            ).read_text(encoding="utf-8")
        )["observations"]
        with patch("epic4_os.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(epic4_os.check_events(Path("unused.exe"))), len(original)
            )
        for scenario, before, after in [
            ("mailbox-between", "records=2", "records=1"),
            ("mailbox-full", "ticket=256", "ticket=255"),
            ("new-instance", "entries=2", "entries=1"),
            ("category1", "rejected=12", "rejected=11"),
        ]:
            records = copy.deepcopy(original)
            record = next(row for row in records if row["scenario"] == scenario)
            record["stdout"] = record["stdout"].replace(before, after)
            self.assertNotEqual(
                record["stdout"],
                next(row["stdout"] for row in original if row["scenario"] == scenario),
            )
            with (
                patch("epic4_os.execute", side_effect=records),
                self.assertRaises(AssertionError),
            ):
                epic4_os.check_events(Path("unused.exe"))

    def test_time_gate_rejects_lost_tick_wrap_and_action_error(self):
        original = json.loads(
            (
                epic4_os.ROOT / "docs/assurance/evidence/epic4/calling-time-regression-4-18.json"
            ).read_text(encoding="utf-8")
        )["observations"]
        with patch("epic4_os.execute", side_effect=copy.deepcopy(original)):
            self.assertEqual(
                len(epic4_os.check_time(Path("unused.exe"))), len(original)
            )
        for scenario, before, after in [
            ("thousand", "ticks=1000", "ticks=999"),
            ("wrap", "kernel=0 counter=1", "kernel=1 counter=1"),
            ("absolute-cycle", "counter=6", "counter=5"),
            ("action-error", "errors=1 last=4", "errors=0 last=0"),
            ("tick-mask-all", "observed=1 kernel=0 pending=1", "observed=1 kernel=1 pending=1"),
            ("tick-mask-os", "observed=1 kernel=0 pending=1", "observed=1 kernel=0 pending=0"),
            ("tick-owner-all", "phase=0 waiting_commit=1", "phase=1 waiting_commit=1"),
            ("tick-owner-os", "phase=0 waiting_commit=1", "phase=0 waiting_commit=0"),
        ]:
            records = copy.deepcopy(original)
            record = next(row for row in records if row["scenario"] == scenario)
            record["stdout"] = record["stdout"].replace(before, after)
            self.assertNotEqual(
                record["stdout"],
                next(row["stdout"] for row in original if row["scenario"] == scenario),
            )
            with (
                patch("epic4_os.execute", side_effect=records),
                self.assertRaises(AssertionError),
            ):
                epic4_os.check_time(Path("unused.exe"))


if __name__ == "__main__":
    unittest.main()
