"""Independent protocol and diagnostic consumers of the two normal native entries."""
from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

from ecu_tools.build import build, query, sealed_sources


@dataclass(frozen=True)
class ExpectedBatch:
    epoch: int
    outputs: tuple[tuple[int, int, int, str, int, int, int, str], ...]
    transport_error: bool = False


def verify(project: Path, build_directory: Path) -> None:
    _, target = sealed_sources(project)
    project = project.resolve(strict=True)
    if target.get("profile") != "ecu":
        raise ValueError("Integrated HostBatch verification requires the ECU profile")
    inputs = json.loads((project / "meta/verification-inputs.json").read_text(encoding="utf-8"))
    if inputs.get("format") != "autosar-ecu-test-inputs-v2":
        raise ValueError("Unsupported independent verification input format")
    if "configuredEndpoints" in inputs and "endpoints" not in inputs:
        raise ValueError("This project supplies configured endpoints for independent scenarios. The reference echo oracle does not apply.")
    required = ("periodMs", "did", "initialReceiveValue", "receiveTimeoutMs")
    if any(type(inputs.get(name)) is not int for name in required):
        raise ValueError("Actual endpoint/configuration inputs are incomplete")
    period = inputs["periodMs"]
    if period <= 0 or inputs["receiveTimeoutMs"] <= 0 or not 0 <= inputs["did"] <= 65535:
        raise ValueError("Independent verification timing or DID input is invalid")
    can_fd = inputs.get("canFd")
    if type(can_fd) is not bool:
        raise ValueError("Verification input lacks its CAN frame capability. Regenerate from the original ARXML.")
    channels = inputs.get("channels")
    endpoints = inputs.get("endpoints")
    roles = {"receive": ("RX", 4), "transmit": ("TX", 4),
             "request": ("RX", 8), "response": ("TX", 8)}
    if (not isinstance(channels, dict) or not 1 <= len(channels) <= 256
            or set(channels) != {str(index) for index in range(len(channels))}
            or any(not isinstance(path, str) or not path.startswith("/") for path in channels.values())
            or len(set(channels.values())) != len(channels)
            or not isinstance(endpoints, dict) or endpoints.keys() != roles.keys()):
        raise ValueError("Actual endpoint/configuration inputs are incomplete")
    fd_timing = inputs.get("fdTimingByChannel")
    if fd_timing is None and len(channels) == 1:
        fd_timing = {"0": inputs.get("fdTiming")} if can_fd else {}
    if (not isinstance(fd_timing, dict) or bool(fd_timing) != can_fd
            or any(channel not in channels or not isinstance(timing, dict)
                   or type(timing.get("brs")) is not bool
                   or any(type(timing.get(name)) is not int or timing[name] <= 0
                          for name in ("nominalBitrate", "dataBitrate"))
                   for channel, timing in fd_timing.items())):
        raise ValueError("Configured FD controller timing is incomplete")
    for role, (direction, length) in roles.items():
        endpoint = endpoints[role]
        if not isinstance(endpoint, dict) or type(endpoint.get("channel")) is not int:
            raise ValueError(f"Invalid configured endpoint: {role}")
        channel = str(endpoint["channel"])
        policies = {"CLASSICAL"}
        if role == "receive":
            policies.add("CLASSICAL_OR_FD")
        if channel in fd_timing and role in ("receive", "transmit"):
            policies.add("FD")
        if (endpoint.get("module") != "CanIf"
                or endpoint.get("direction") != direction or channel not in channels
                or endpoint.get("fdPolicy") not in policies
                or endpoint.get("format") not in ("STANDARD", "EXTENDED")
                or type(endpoint.get("id")) is not int
                or not 0 <= endpoint["id"] <= (0x7FF if endpoint["format"] == "STANDARD" else 0x1FFFFFFF)
                or type(endpoint.get("handle")) is not int or not 0 <= endpoint["handle"] <= 255
                or endpoint.get("logicalLength") != length or endpoint.get("wireLength") != length):
            raise ValueError(f"Invalid configured endpoint: {role}")

    def receive_frame(role: str, data: str, fd: int | None = None) -> str:
        ep = endpoints[role]
        if fd is None:
            fd = int(ep["fdPolicy"] == "FD" or (str(ep["channel"]) in fd_timing and ep["fdPolicy"] == "CLASSICAL_OR_FD"))
        return f"RX {ep['channel']} {ep['format']} {fd} {ep['id']} {ep['wireLength']} {data}"

    def expected_frame(at: int, role: str, data: str) -> tuple[int, int, int, str, int, int, int, str]:
        ep = endpoints[role]
        return at, ep["handle"], ep["channel"], ep["format"], int(ep["fdPolicy"] == "FD"), ep["id"], ep["wireLength"], data

    binary = build(project, build_directory, "host-batch")
    logs = binary.parent / "logs"
    did = f"{inputs['did']:04x}"
    request = f"0322{did}00000000"
    response = f"0762{did}12345678"
    initial = inputs["initialReceiveValue"].to_bytes(4, "little").hex()

    def actor(label: str, batches: list[tuple[int, list[str], str, list[str], bool]], *, malformed: bool = False) -> None:
        commands = ["PROTOCOL HostBatchV2"]
        expected = []
        previous = 0
        current_tx = initial
        step = min(1000, period * 128)

        def append(at: int, messages: list[str], tx: str, diagnostics: list[str], error: bool) -> None:
            nonlocal previous
            commands.append(f"BEGIN {at}")
            commands.extend(messages)
            commands.append("COMMIT")
            # Input-phase FC/first CF flush before periodic work; Com flushes
            # before Dcm. Equal epochs alone do not contend for the single HOH.
            outputs = [expected_frame(epoch, "transmit", tx)
                       for epoch in range((previous // period + 1) * period, at + 1, period)]
            outputs.extend(expected_frame(at, "response", data) for data in diagnostics)
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
        if lines[:2] != ["READY HostBatchV2", "PROTOCOL_OK HostBatchV2"]:
            raise AssertionError(f"Actual HostBatch did not become ready: {observed}")
        parsed = []
        receipts = []
        outputs = []
        rejection = []
        closed = False
        stack_roles = set()
        for line in lines[2:]:
            match = re.fullmatch(
                r"OUT epoch=(\d+) sequence=\d+ ticket=\d+ pdu=(\d+) channel=(\d+) "
                r"format=(STANDARD|EXTENDED) fd=([01]) id=(\d+) length=(\d+) data=([0-9a-f]+|-)", line,
            )
            if match:
                epoch, handle, channel, fmt, fd, can_id, length, data = match.groups()
                length = int(length)
                if (length == 0 and data != "-") or (length != 0 and len(data) != 2 * length):
                    raise AssertionError(f"Output payload length differs: {line}")
                outputs.append((int(epoch), int(handle), int(channel), fmt, int(fd), int(can_id), length, data))
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

    rx = receive_frame("receive", "78563412")
    def diagnostic(data: str) -> str:
        return receive_frame("request", data)

    actor("echo", [
        (period, [rx], "78563412", [], False),
        (period + 1, [rx, diagnostic(request)], "78563412", [response], False),
        (period + 2, [rx, diagnostic(f"0522{did}f1860000")], "78563412", [f"100a62{did}123456"], False),
        (period + 3, [rx, diagnostic("3000000000000000")], "78563412", ["2178f18601000000"], False),
        (period + 4, [rx, diagnostic(f"0722{did}f186{did}")], "78563412", ["037f221300000000"], False),
    ])
    actor("arbitration", [
        (period, [rx], "78563412", [], False),
        # Keep FF-to-CTS at one tick even when the application period exceeds N_Bs.
        (3 * period - 1, [rx, diagnostic(f"0522{did}f1860000")], "78563412", [f"100a62{did}123456"], False),
        (3 * period, [rx, diagnostic("3000000000000000")], "78563412", ["2178f18601000000"], False),
        (4 * period, [rx, diagnostic(f"0522{did}f1860000")], "78563412", [f"100a62{did}123456"], False),
        # An already-ticked epoch admits the CF without repeating periodic work.
        (4 * period, [diagnostic("3000000000000000")], "78563412", ["2178f18601000000"], False),
        (5 * period, [rx, diagnostic(request)], "78563412", [response], False),
    ])
    deadline = 1 + inputs["receiveTimeoutMs"]
    recovery = (deadline // period + 1) * period
    actor("ncr-recovery", [
        (1, [diagnostic(f"100922{did}{did}12")], initial, ["3000000000000000"], False),
        (deadline, [], initial, [], True),
        (recovery, [rx], "78563412", [], False),
        (recovery + 1, [rx, diagnostic(request)], "78563412", [response], False),
    ])
    if str(endpoints["receive"]["channel"]) in fd_timing:
        policy = endpoints["receive"]["fdPolicy"]
        actor("can-fd-policy", [
            (period, [receive_frame("receive", "78563412", 0)],
             initial if policy == "FD" else "78563412", [], False),
            (2 * period, [receive_frame("receive", "2a000000", 1)],
             "78563412" if policy == "CLASSICAL" else "2a000000", [], False),
        ])
    actor("malformed", [], malformed=True)
    probe = build(project, build_directory / "probe", "probe")
    transmit = endpoints["transmit"]
    ticks = max(20, period) if transmit["fdPolicy"] == "FD" else 20
    probe_command = [str(probe)]
    if ticks != 20:
        probe_command.extend(["--ticks", str(ticks)])
    probe_output = query(probe_command, logs, "probe", timeout=60)
    probe_lines = probe_output.splitlines()
    if (f"ecu_probe completed={ticks}" not in probe_lines
            or not any(line.startswith("lifecycle=Closed state=Ready reason=0 ") for line in probe_lines)):
        raise AssertionError(f"Normal probe did not complete: {probe_output}")
    if transmit["fdPolicy"] == "FD":
        timing = fd_timing[str(transmit["channel"])]
        brs = timing["brs"]
        rate = timing["dataBitrate" if brs else "nominalBitrate"]
        can_id = transmit["id"] | 0x40000000
        if transmit["format"] == "EXTENDED":
            can_id |= 0x80000000
        prefix = f" pdu={transmit['handle']} id={can_id} dlc={transmit['wireLength']}"
        observations = [line for line in probe_lines
                        if line.startswith("ecu_output ") and prefix + " " in line
                        and f" channel={transmit['channel']} " in line]
        if (len(observations) != ticks // period
                or any(not line.endswith(f" brs={int(brs)} data_bitrate={rate}") for line in observations)):
            raise AssertionError(f"FD output bitrate switch or data phase rate differs: {probe_output}")
        print(f"probe=can-fd-timing PASS brs={int(brs)} data_bitrate={rate}")
    print(f"probe=normal PASS ticks={ticks} COM diagnostics parsed")
    sealed_sources(project)
    print("ECU_HANDOFF_VERIFY PASS: CAN/DID echo, single-HOH arbitration, two-DID boundary, real N_Cr timeout/recovery, malformed admission; host behavior only.")
