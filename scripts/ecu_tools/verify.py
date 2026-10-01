"""Independent CAN/DID and refusal oracle for the real production HostBatch."""
from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

from ecu_tools.build import build, query, sealed_sources


@dataclass(frozen=True)
class ExpectedBatch:
    epoch: int
    outputs: tuple[tuple[int, int, int, str], ...]
    transport_error: bool = False


def verify(project: Path, build_directory: Path) -> None:
    _, target = sealed_sources(project)
    project = project.resolve(strict=True)
    if target.get("profile") != "ecu":
        raise ValueError("Integrated HostBatch verification requires the ECU profile")
    inputs = json.loads((project / "verification/inputs.json").read_text(encoding="utf-8"))
    if inputs.get("format") != "autosar-ecu-test-inputs-v1":
        raise ValueError("Unsupported independent verification input format")
    required = ("periodMs", "receiveCanId", "transmitCanId", "requestCanId", "responseCanId", "did", "initialReceiveValue", "receiveTimeoutMs")
    if any(type(inputs.get(name)) is not int for name in required):
        raise ValueError("Actual endpoint/configuration inputs are incomplete")
    period = inputs["periodMs"]
    if period <= 0 or inputs["receiveTimeoutMs"] <= 0 or not 0 <= inputs["did"] <= 65535:
        raise ValueError("Independent verification timing or DID input is invalid")
    binary = build(project, build_directory, "host-batch")
    logs = binary.parent / "logs"
    did = f"{inputs['did']:04x}"
    request = f"0322{did}00000000"
    response = f"0762{did}12345678"
    initial = inputs["initialReceiveValue"].to_bytes(4, "little").hex()

    def actor(label: str, batches: list[tuple[int, list[str], str, list[str], bool]], *, malformed: bool = False) -> None:
        commands = []
        expected = []
        previous = 0
        current_tx = initial
        step = min(1000, period * 128)

        def append(at: int, messages: list[str], tx: str, diagnostics: list[str], error: bool) -> None:
            nonlocal previous
            commands.append(f"BEGIN {at}")
            commands.extend(messages)
            commands.append("COMMIT")
            outputs = [(epoch, inputs["transmitCanId"], 4, tx)
                       for epoch in range((previous // period + 1) * period, at + 1, period)]
            outputs.extend((at, inputs["responseCanId"], 8, data) for data in diagnostics)
            expected.append(ExpectedBatch(at, tuple(sorted(outputs)), error))
            previous = at

        for at, messages, tx, diagnostics, error in batches:
            while at - previous > step:
                append(previous + step, [], current_tx, [], False)
            append(at, messages, tx, diagnostics, error)
            current_tx = tx
        if malformed:
            commands.append("BEGIN -1")
        stdin = logs / f"{label}.stdin"
        stdin.write_text("\n".join(commands) + "\n", encoding="ascii", newline="\n")
        observed = query([str(binary)], logs, f"protocol-{label}", stdin_file=stdin, timeout=60)
        lines = observed.splitlines()
        if not lines or lines[0] != "READY HostBatchV1":
            raise AssertionError(f"Actual HostBatch did not become ready: {observed}")
        parsed = []
        receipts = []
        outputs = []
        rejection = []
        closed = False
        stack_roles = set()
        for line in lines[1:]:
            match = re.fullmatch(r"OUT epoch=(\d+) sequence=(\d+) ticket=(\d+) pdu=(\d+) id=(\d+) dlc=(\d+) data=([0-9a-f]+)", line)
            if match:
                if len(match[7]) != 2 * int(match[6]):
                    raise AssertionError(f"Output payload/DLC differs: {line}")
                outputs.append((int(match[1]), int(match[5]), int(match[6]), match[7]))
                continue
            if line.startswith(("COMMIT_OK ", "COMMIT_ERROR ")):
                fields = dict(re.findall(r"([a-z_]+)=(\d+)", line))
                if not all(name in fields for name in ("batch", "epoch", "sequence", "inputs", "status", "input_status", "transport_status", "transport_epoch", "transport_count")):
                    raise AssertionError(f"Malformed batch receipt: {line}")
                receipts.append((line.startswith("COMMIT_ERROR "), fields))
                parsed.append(tuple(sorted(outputs)))
                outputs = []
            elif line.startswith("REJECT "):
                rejection.append(line)
            elif line.startswith("STACK "):
                role = re.search(r"role=([A-Z])\b", line)
                fields = {name: int(value) for name, value in re.findall(r"([a-z_]+)=(\d+)", line)}
                if role is None or not all(name in fields for name in ("tid", "low", "high", "reserve", "guard", "sp", "buffer_low", "buffer_high")):
                    raise AssertionError(f"Malformed physical stack record: {line}")
                if (fields["tid"] == 0 or fields["guard"] == 0
                        or not fields["low"] + fields["guard"] < fields["sp"] < fields["high"]
                        or fields["high"] - fields["low"] != fields["reserve"]):
                    raise AssertionError(f"Actual native stack/guard invariant differs: {line}")
                if fields["buffer_high"] and not (
                        fields["buffer_low"] < fields["buffer_high"]
                        and (fields["buffer_high"] <= fields["low"] or fields["buffer_low"] >= fields["high"])):
                    raise AssertionError(f"Kernel metadata aliases the native stack: {line}")
                if target["target"] == "linux-x64-controlled-v1" and (
                        "context=linux-x86_64-ucontext" not in line or fields.get("altstack", 0) == 0):
                    raise AssertionError(f"Linux context/alternate stack observation differs: {line}")
                stack_roles.add(role[1])
            elif line.startswith("lifecycle=Closed "):
                if "state=Ready reason=0 " not in line:
                    raise AssertionError(f"Actual ECU closed with a failure: {line}")
                closed = True
            elif not line.startswith("lifecycle="):
                raise AssertionError(f"Unexpected actual protocol record: {line}")
        if not closed or not {"S", "T", "C", "D"} <= stack_roles or outputs or len(receipts) != len(expected):
            raise AssertionError(f"Native output/receipt/lifecycle closure differs: {observed}")
        for want, got, (error, fields) in zip(expected, parsed, receipts, strict=True):
            if got != want.outputs or int(fields["epoch"]) != want.epoch or error != want.transport_error:
                raise AssertionError(f"Independent complete output set differs: expected={want}; observed={got}; receipt={fields}")
            if want.transport_error:
                if (fields["transport_status"], fields["transport_epoch"], fields["transport_count"]) != ("10", str(want.epoch), "1"):
                    raise AssertionError(f"Actual N_Cr timeout is absent: {fields}")
            elif (fields["status"], fields["input_status"], fields["transport_status"]) != ("0", "0", "0"):
                raise AssertionError(f"Production batch unexpectedly failed: {fields}")
        if malformed:
            if len(rejection) != 1 or "epoch=0 sequence=0" not in rejection[0]:
                raise AssertionError(f"Malformed epoch was not refused before automotive work: {rejection}")
        elif rejection:
            raise AssertionError(f"Valid production input was rejected: {rejection}")
        print(f"protocol={label} PASS batches={len(expected)}")

    rx = f"RX {inputs['receiveCanId']} 4 78563412"
    diagnostic = lambda data: f"RX {inputs['requestCanId']} 8 {data}"
    actor("echo", [
        (period, [rx], "78563412", [], False),
        (period + 1, [rx, diagnostic(request)], "78563412", [response], False),
        (period + 2, [rx, diagnostic(f"0522{did}f1860000")], "78563412", [f"100a62{did}123456"], False),
        (period + 3, [rx, diagnostic("3000000000000000")], "78563412", ["2178f18601000000"], False),
        (period + 4, [rx, diagnostic(f"0722{did}f186{did}")], "78563412", ["037f221300000000"], False),
    ])
    deadline = 1 + inputs["receiveTimeoutMs"]
    recovery = (deadline // period + 1) * period
    actor("ncr-recovery", [
        (1, [diagnostic(f"100922{did}{did}12")], initial, ["3000000000000000"], False),
        (deadline, [], initial, [], True),
        (recovery, [rx], "78563412", [], False),
        (recovery + 1, [rx, diagnostic(request)], "78563412", [response], False),
    ])
    actor("malformed", [], malformed=True)
    sealed_sources(project)
    print("ECU_HANDOFF_VERIFY PASS: CAN/DID echo, two-DID boundary, real N_Cr timeout/recovery, malformed admission; host behavior only.")
