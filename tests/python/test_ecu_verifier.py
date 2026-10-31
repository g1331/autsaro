"""Independent HostBatch transcripts protect the delivered verifier's exact oracle."""
from __future__ import annotations

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from ecu_tools.verify import verify


class VerifierTests(unittest.TestCase):
    def replay(self, period: int, mutation=None):
        # Fixed public CAN/UDS vectors, not output read from a built ECU or the oracle.
        rx = "RX 800 4 78563412"
        read = "RX 1792 8 0322123400000000"
        two = "RX 1792 8 05221234f1860000"
        cts = "RX 1792 8 3000000000000000"
        value = "78563412"
        sf = "0762123412345678"
        ff = "100a621234123456"
        cf = "2178f18601000000"
        deadline = 201
        recovery = next(tick for tick in range(deadline + 1, deadline + period + 1) if tick % period == 0)
        # period=1 crosses the verifier's bounded-batch split at129.
        wait = [(129, [], "00000000", [], False)] if period == 1 else []
        scenarios = {
            "echo": [
                (period, [rx], value, [], False),
                (period + 1, [rx, read], value, [sf], False),
                (period + 2, [rx, two], value, [ff], False),
                (period + 3, [rx, cts], value, [cf], False),
                (period + 4, [rx, "RX 1792 8 07221234f1861234"], value, ["037f221300000000"], False),
            ],
            "arbitration": [
                (period, [rx], value, [], False),
                (3 * period - 1, [rx, two], value, [ff], False),
                (3 * period, [rx, cts], value, [cf], False),
                (4 * period, [rx, two], value, [ff], False),
                (4 * period, [cts], value, [cf], False),
                (5 * period, [rx, read], value, [sf], False),
            ],
            "ncr-recovery": [
                (1, ["RX 1792 8 1009221234123412"], "00000000", ["3000000000000000"], False),
                *wait,
                (deadline, [], "00000000", [], True),
                (recovery, [rx], value, [], False),
                (recovery + 1, [rx, read], value, [sf], False),
            ],
            "malformed": [],
        }

        def query(command, logs, label, *, stdin_file=None, timeout):
            if label == "probe":
                self.assertIsNone(stdin_file)
                return "ecu_probe completed=20\nlifecycle=Closed state=Ready reason=0 "
            name = label.removeprefix("protocol-")
            commands = []
            records = ["READY HostBatchV1"]
            previous = 0
            for batch, (at, messages, tx, diagnostics, error) in enumerate(scenarios[name], 1):
                commands.extend([f"BEGIN {at}", *messages, "COMMIT"])
                # Independent tick-by-tick schedule: input flush, application/Com,
                # Com flush, Dcm. All these vectors release the HOH between writers.
                outputs = []
                for tick in range(previous + 1, at + 1):
                    if tick % period == 0:
                        outputs.append((tick, 801, 4, tx))
                outputs.extend((at, 1800, 8, data) for data in diagnostics)
                if mutation:
                    outputs = mutation(name, batch, outputs)
                for sequence, (epoch, can_id, dlc, data) in enumerate(outputs, 1):
                    records.append(f"OUT epoch={epoch} sequence={sequence} ticket={sequence} "
                                   f"pdu=0 id={can_id} dlc={dlc} data={data}")
                records.append(f"COMMIT_{'ERROR' if error else 'OK'} batch={batch} epoch={at} "
                               f"sequence=0 inputs={len(messages)} status=0 input_status=0 "
                               f"transport_status={10 if error else 0} "
                               f"transport_epoch={at if error else 0} transport_count={1 if error else 0}")
                previous = at
            if name == "malformed":
                commands.append("BEGIN -1")
                records.append("REJECT epoch=0 sequence=0")
            self.assertEqual(stdin_file.read_text(encoding="ascii"), "\n".join(commands) + "\n")
            for role in "STCD":
                records.append(f"STACK role={role} tid=1 low=100 high=300 reserve=200 guard=10 "
                               "sp=200 buffer_low=0 buffer_high=0 "
                               "context=linux-x86_64-ucontext altstack=1")
            records.append("lifecycle=Closed state=Ready reason=0 ")
            return "\n".join(records)

        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            (project / "meta").mkdir()
            (project / "logs").mkdir()
            inputs = {"format": "autosar-ecu-test-inputs-v1", "periodMs": period,
                      "receiveCanId": 800, "transmitCanId": 801, "requestCanId": 1792,
                      "responseCanId": 1800, "did": 0x1234, "initialReceiveValue": 0,
                      "receiveTimeoutMs": 200}
            (project / "meta/verification-inputs.json").write_text(json.dumps(inputs), encoding="utf-8")
            with (patch("ecu_tools.verify.sealed_sources", return_value=([], {
                    "profile": "ecu", "target": "linux-x64-controlled-v1"})),
                  patch("ecu_tools.verify.build", return_value=project / "ecu_host_batch"),
                  patch("ecu_tools.verify.query", side_effect=query) as observed,
                  contextlib.redirect_stdout(io.StringIO()) as output):
                verify(project, project / "build")
                self.assertEqual(observed.call_count, 5)
                self.assertIn("probe=normal PASS ticks=20", output.getvalue())
                self.assertIn("protocol=arbitration PASS batches=6", output.getvalue())
                self.assertIn("ECU_HANDOFF_VERIFY PASS:", output.getvalue())

    def test_exact_output_sets_with_short_and_default_periods(self):
        for period in [1, 2, 3, 10]:
            with self.subTest(period=period):
                self.replay(period)

    def test_flow_control_does_not_wait_for_long_application_period(self):
        self.replay(250)

    def test_missing_periodic_or_diagnostic_frames_are_refused(self):
        for period in [1, 2, 3, 10]:
            for scenario, batch in [("echo", 4), ("arbitration", 3), ("arbitration", 4),
                                    ("arbitration", 5), ("ncr-recovery", 1)]:
                for can_id in [801, 1800]:
                    if can_id == 801 and (batch == 5 or (scenario == "ncr-recovery" and period != 1)
                                          or (scenario == "echo" and (period + 3) % period)):
                        continue
                    def remove(name, number, outputs, *, can_id=can_id, scenario=scenario, batch=batch):
                        return [frame for frame in outputs if frame[1] != can_id] if (
                            name, number) == (scenario, batch) else outputs
                    with (self.subTest(period=period, scenario=scenario, batch=batch, can_id=can_id),
                          self.assertRaisesRegex(AssertionError, "Independent complete output set differs")):
                        self.replay(period, remove)

    def test_extra_duplicate_or_corrupt_frames_are_refused(self):
        for kind in ["extra", "duplicate", "epoch", "id", "dlc", "data", "repeat-periodic"]:
            def change(name, batch, outputs, *, kind=kind):
                if name != "arbitration" or batch != 5:
                    return outputs
                epoch, can_id, dlc, data = outputs[0]
                variants = {
                    "extra": [*outputs, (epoch, 123, 4, "00000000")],
                    "duplicate": [*outputs, outputs[0]],
                    "epoch": [(epoch + 1, can_id, dlc, data)],
                    "id": [(epoch, 123, dlc, data)],
                    "dlc": [(epoch, can_id, 7, data[:14])],
                    "data": [(epoch, can_id, dlc, "00" * dlc)],
                    "repeat-periodic": [*outputs, (epoch, 801, 4, "78563412")],
                }
                return variants[kind]
            with (self.subTest(kind=kind),
                  self.assertRaisesRegex(AssertionError, "Independent complete output set differs")):
                self.replay(3, change)


if __name__ == "__main__":
    unittest.main()
