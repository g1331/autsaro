"""Build and independently verify the fixed offline Epic 4 OS target."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KERNEL = ROOT / "third_party/freertos"
TARGET = ROOT / "runtime/os"


def verify_sources(kernel: Path = KERNEL) -> dict:
    manifest = json.loads((kernel / "source-manifest.json").read_text())
    if manifest["commit"] != "054e14f3397023aa83813a65aa065fc4597d481b":
        raise ValueError("fixed kernel identity mismatch")
    for name, expected in manifest["files"].items():
        if hashlib.sha256((kernel / name).read_bytes()).hexdigest() != expected:
            raise ValueError(f"kernel digest mismatch: {name}")
    if manifest["license"] != "MIT":
        raise ValueError("kernel license mismatch")
    return manifest


def compiler() -> str:
    cc = os.environ.get("AUTOSAR_CC", "gcc")
    version = subprocess.run(
        [cc, "--version"], capture_output=True, text=True, check=True
    )
    machine = subprocess.run(
        [cc, "-dumpmachine"], capture_output=True, text=True, check=True
    )
    pinned = json.loads(
        (TARGET / "toolchain.json").read_text(encoding="utf-8")
    )
    executable = Path(shutil.which(cc) or cc)
    if (
        compiler_description(version.stdout.splitlines()[0])
        != compiler_description(pinned["identity"])
        or machine.stdout.strip() != pinned["target"]
        or hashlib.sha256(executable.read_bytes()).hexdigest()
        != pinned["executable_sha256"]
    ):
        raise ValueError("target requires GCC 16.1.0 x86_64-w64-mingw32")
    return cc


def compiler_description(identity: str) -> str:
    # argv[0] spelling/case is not compiler identity; vendor/release and the
    # actual executable digest are independently pinned.
    return identity.partition(" ")[2]


def verify_vector_section(binary: Path, tool: Path) -> None:
    sections = subprocess.run(
        [str(tool), "-h", str(binary)], capture_output=True, text=True, check=True
    ).stdout
    matches = list(re.finditer(
        r"^\s*(\d+)\s+\.os_vec\s+([0-9a-fA-F]+)[^\n]*\n([^\n]+)",
        sections, re.MULTILINE,
    ))
    if len(matches) != 1 or int(matches[0][2], 16) != 32 * 8:
        raise ValueError("native interrupt vector section is missing or has the wrong size")
    flags = matches[0][3]
    if "DATA" not in flags or "READONLY" in flags or "ALLOC" not in flags:
        raise ValueError("native interrupt vector section must be writable allocated data")
    symbols = subprocess.run(
        [str(tool), "-t", str(binary)], capture_output=True, text=True, check=True
    ).stdout
    tables = list(re.finditer(
        r"\(sec\s+(\d+)\)[^\n]*\s0x([0-9a-fA-F]+)\s+Os_InterruptVectorTable$",
        symbols, re.MULTILINE,
    ))
    if (len(tables) != 1 or int(tables[0][1]) != int(matches[0][1]) + 1
            or int(tables[0][2], 16) != 0):
        raise ValueError("the actual interrupt vector table is not in its dedicated section")


def build(directory: Path, harness: str = "lifecycle.c", defines: tuple[str, ...] = (),
          target: Path | None = None) -> tuple[Path, dict]:
    target = TARGET if target is None else target
    verify_sources()
    cc = compiler()
    copied = directory / "kernel"
    shutil.copytree(KERNEL, copied)
    patches = sorted((target / "patches").glob("*.patch"))
    for patch in patches:
        subprocess.run(
            ["git", "apply", "--ignore-space-change", "--check", str(patch)],
            cwd=copied,
            check=True,
        )
        subprocess.run(
            ["git", "apply", "--ignore-space-change", str(patch)],
            cwd=copied,
            check=True,
        )
    binary = directory / "os_harness.exe"
    command = [
        cc,
        "-std=c99",
        "-O1",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-I",
        str(target),
        "-I",
        str(target / "include"),
        "-I",
        str(target / "src"),
        "-I",
        str(ROOT / "runtime/include"),
        "-I",
        str(copied / "include"),
        "-I",
        str(copied / "portable/MSVC-MingW"),
        str(target / "src/Os.c"),
        str(target / "src/Os_Error.c"),
        str(target / "src/Os_Interrupt.c"),
        str(target / "src/Os_Backend.c"),
        str(target / "src/Os_Vector.c"),
        str(target / "src/Os_Stack.c"),
        str(target / "src/Os_HostEvent.c"),
        str(target / "src/Os_Mailbox.c"),
        str(target / "src/Os_Time.c"),
        str(target / "src/Os_Schedule.c"),
        str(target / "tests" / harness),
        str(copied / "tasks.c"),
        str(copied / "list.c"),
        str(copied / "queue.c"),
        str(copied / "portable/MSVC-MingW/port.c"),
        "-lwinmm",
        "-o",
        str(binary),
    ]
    command[1:1] = ["-D" + value for value in defines]
    if harness == "native_stack.c":
        command.insert(1, "-DOS_STACK_TESTS")
    if harness == "activation.c":
        command.insert(1, "-DOS_ACTIVATION_TESTS")
    if harness == "finish_chain.c":
        command.insert(1, "-DOS_FINISH_TESTS")
    if harness == "resource_preemption.c":
        command.insert(1, "-DOS_RESOURCE_TESTS")
    if harness == "event_wakeup.c":
        command.insert(1, "-DOS_EVENT_TESTS")
    if harness == "controlled_time.c":
        command.insert(1, "-DOS_TIME_TESTS")
    if harness == "idle_state.c":
        command.insert(1, "-DOS_IDLE_TESTS")
    compiled = subprocess.run(command, capture_output=True, text=True, check=False)
    if compiled.returncode:
        raise RuntimeError(f"OS C99 build failed:\n{compiled.stdout}{compiled.stderr}")
    tool = Path(shutil.which(cc) or cc).parent / "objdump.exe"
    symbols = subprocess.run(
        [str(tool), "-t", str(binary)], capture_output=True, text=True, check=True
    ).stdout
    if "__emutls" in symbols:
        raise ValueError(
            "native exception path requires PE TLS without emutls allocation"
        )
    verify_vector_section(binary, tool)
    return binary, {"command": command}


def execute(
    binary: Path, scenario: str, failure: int | str = 0, invalid_stack: str = ""
) -> dict:
    env = dict(os.environ)
    env.pop("AUTOSAR_OS_FAIL_RESOURCE", None)
    env.pop("AUTOSAR_OS_BAD_GUARANTEE", None)
    if invalid_stack:
        env["AUTOSAR_OS_BAD_GUARANTEE"] = invalid_stack
    if failure:
        env["AUTOSAR_OS_FAIL_RESOURCE"] = str(failure)
    result = subprocess.run(
        [str(binary), scenario],
        env=env,
        timeout=10,
        capture_output=True,
        text=True,
        check=False,
    )
    return {
        "scenario": scenario,
        "failure": failure,
        "invalid_stack": invalid_stack,
        "exit": result.returncode,
        "stdout": result.stdout.strip(),
        "stderr": result.stderr.strip(),
    }


def require(condition: bool, observation: dict) -> None:
    if not condition:
        raise AssertionError(observation)


def check_sc1_timing(binary: Path) -> list[dict]:
    paired = [("A", 0, 2), ("E", 0, 4), ("A", 0, 7), ("E", 0, 9)]
    cases = {
        "capacity": [("A", 0, 3), ("E", 0, 5), ("B", 1, 2)],
        "eight-tables": [("A", i, 2) for i in range(8)],
        "wrap": [],
        "absolute": [("A", 0, 15), ("E", 0, 1)],
        "relative-max": [("A", 0, 15), ("E", 0, 1)],
        "relative-zero": [("A", 0, 1)],
        "same-point": [("E", 0, 1)],
        "repeat": paired + [("A", 0, 1)],
        "link": paired,
        "link-backward": paired,
        "link-zero-final": [("A", 0, 2), ("E", 0, 4), ("A", 0, 4)],
        "replace-next": paired,
        "stop-next": paired[:2],
        "autostart": paired[:2],
        "autostart-mode2": paired[:2],
        "autostart-absolute": [("A", 0, 1), ("E", 0, 3)],
        "counter-chain": [],
        "action-error": [("E", 0, 4)],
        "rejects": paired,
        "category1": [],
        "category2": [],
    }
    records = []
    for name, expected in cases.items():
        result = execute(binary, name)
        require(result["exit"] == 0 and not result["stderr"], result)
        actual = [(kind, int(counter), int(value)) for kind, counter, value in re.findall(
            r"timing_action (\w) counter=(\d+) value=(\d+)", result["stdout"]
        )]
        require(actual == expected, result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        require("status=0" in result["stdout"], result)
        require(("errors=1 last_error=4" if name == "action-error" else "errors=0 last_error=0") in result["stdout"], result)
        if name == "rejects":
            require("rejects=19" in result["stdout"], result)
        if name == "category1":
            require("rejects=5" in result["stdout"], result)
        result["independent_expected_actions"] = expected
        records.append(result)
    for name in [
        "cycle", "counter-count", "table-count", "null-tables", "duplicate-id", "counter",
        "zero-points", "many-points", "null-points", "duplicate-offset", "duration",
        "empty-action", "null-action", "many-actions", "task", "event", "repeat-final",
        "sync", "autostart", "mincycle",
    ]:
        result = execute(binary, "bad-" + name)
        status = 3 if name in {"duplicate-id", "counter", "task", "event"} else 8
        require(result["exit"] == 0 and not result["stderr"], result)
        require(result["stdout"] == f"timing_prepare case=bad-{name} status={status} ready=0", result)
        result["independent_expected_status"] = status
        records.append(result)
    return records


def check_time(binary: Path) -> list[dict]:
    records = []
    for case, ticks, work, app, rejected, expected in [
        ("thousand", 1000, 1000, 100, 2, [(1, 1, 1, 1), (2, 2, 2, 1), (3, 3, 3, 1)]),
        (
            "wrap",
            3,
            3,
            0,
            2,
            [
                (65535, 4294967294, 65535, 1),
                (65536, 4294967295, 0, 1),
                (65537, 0, 1, 1),
            ],
        ),
        ("epoch-max", 1, 1, 0, 2, [(18446744073709551615, 1, 65535, 1)]),
        ("inflight", 1, 1, 0, 4, [(1, 1, 1, 1)]),
        ("tick-mask-all", 1, 1, 0, 2, [(1, 1, 1, 1)]),
        ("tick-mask-os", 1, 1, 0, 2, [(1, 1, 1, 1)]),
        ("tick-owner-all", 1, 1, 0, 2, [(1, 1, 1, 1)]),
        ("tick-owner-os", 1, 1, 0, 2, [(1, 1, 1, 1)]),
    ]:
        result = execute(binary, case)
        require(result["exit"] == 0 and not result["stderr"], result)
        require(
            f"time ticks={ticks} work={work} app={app} rejects={rejected} error=0"
            in result["stdout"],
            result,
        )
        require("trace=ISREMZ " in result["stdout"], result)
        require("context rejected=7" in result["stdout"], result)
        if case in ["tick-mask-all", "tick-mask-os", "tick-owner-all", "tick-owner-os"]:
            require("tick_masked observed=1 kernel=0 pending=1" in result["stdout"], result)
        if case in ["tick-owner-all", "tick-owner-os"]:
            require("dispatch_release phase=0 waiting_commit=1" in result["stdout"], result)
        actual = [
            tuple(map(int, row))
            for row in re.findall(
                r"TICK epoch=(\d+) kernel=(\d+) counter=(\d+) actions=(\d+)",
                result["stdout"],
            )
        ]
        require(actual == expected, result)
        records.append(result)
    for case, trace, entries, callbacks, errors, last, rejected, counter in [
        ("relative-wrap", "ISREHrMZ", 1, 0, 0, 0, 3, 1),
        ("absolute-cycle", "ISRErMZ", 0, 0, 0, 0, 2, 6),
        ("absolute-current", "ISREBrMZ", 0, 1, 0, 0, 0, 14),
        ("absolute-wide", "ISRErMZ", 0, 0, 0, 0, 0, 0),
        ("action-error", "ISRErMZ", 0, 0, 1, 4, 0, 1),
        ("callback", "ISREBBBrMZ", 0, 3, 0, 0, 0, 7),
        ("counter-errors", "ISRErMZ", 0, 0, 0, 0, 15, 0),
        ("category1-time", "ISREJrMZ", 0, 0, 0, 0, 8, 0),
        ("category2-time", "ISREJrMZ", 0, 0, 0, 0, 1, 1),
    ]:
        result = execute(binary, case)
        require(result["exit"] == 0 and not result["stderr"], result)
        require(f"trace={trace} " in result["stdout"], result)
        require(
            "time ticks=0 work=0 app=0 rejects=0 error=0" in result["stdout"], result
        )
        require("context rejected=7" in result["stdout"], result)
        require(
            f"alarm entries={entries} callbacks={callbacks} errors={errors} last={last} rejected={rejected} counter={counter}"
            in result["stdout"],
            result,
        )
        records.append(result)
    for case, status in [
        ("config-counter-null", 8),
        ("config-counter-capacity", 8),
        ("config-counter-duplicate", 3),
        ("config-counter-maximum", 8),
        ("config-counter-base", 8),
        ("config-counter-cycle", 8),
        ("config-system-missing", 3),
        ("config-system-hardware", 8),
        ("config-alarm-null", 8),
        ("config-alarm-capacity", 8),
        ("config-alarm-duplicate", 3),
        ("config-alarm-counter", 3),
        ("config-alarm-action", 8),
        ("config-alarm-task", 8),
        ("config-alarm-basic", 8),
        ("config-alarm-callback", 8),
        ("config-alarm-cycle", 8),
        ("config-owner-basic", 8),
        ("config-owner-inactive", 8),
        ("config-wake", 8),
        ("config-tick-category1", 8),
    ]:
        result = execute(binary, case)
        require(result["exit"] == 0 and result["stdout"] == f"prepare={status}", result)
        records.append(result)
    for case, work in [("reset-failure", 0), ("signal-failure", 1)]:
        result = execute(binary, case)
        require(result["exit"] == 7 and "trace=ISREZ " in result["stdout"], result)
        require(f"time ticks=0 work={work} app=0 " in result["stdout"], result)
        require("time_signal_failed=1" in result["stdout"], result)
        records.append(result)
    for case, trace in [
        ("replace-tick-startup", "ISZ"),
        ("replace-tick-running", "ISREZ"),
    ]:
        result = execute(binary, case)
        require(result["exit"] == 1 and f"trace={trace} " in result["stdout"], result)
        require("time ticks=0 work=0 app=0 " in result["stdout"], result)
        records.append(result)
    result = execute(binary, "close-before-pending")
    require(result["exit"] == 7 and "trace=ISREZ " in result["stdout"], result)
    require("time ticks=0 work=0 app=0 rejects=1 error=7" in result["stdout"], result)
    require("time_signal_failed=0" in result["stdout"], result)
    records.append(result)
    result = execute(binary, "thousand", failure=1)
    require(result["exit"] == 7 and "trace=IZ " in result["stdout"], result)
    require(
        "threads=0 events=0 mutexes=0 resource_calls=1 " in result["stdout"], result
    )
    require("time ticks=0 work=0 app=0 " in result["stdout"], result)
    records.append(result)
    return records


def check_events(binary: Path) -> list[dict]:
    cases = [
        ("already", "ISREaMZ", 2, 0, 0, 1),
        ("wait", "ISREPwpMZ", 2, 0, 0, 1),
        ("oracle-wait", "ISREPwpMZ", 2, 0, 0, 1),
        ("ownership", "ISREPwpMZ", 2, 0, 0, 1),
        ("new-instance", "ISREEnMZ", 2, 0, 0, 2),
        ("errors", "ISRErMZ", 10, 0, 0, 1),
        ("category1", "ISREJiMZ", 12, 0, 0, 1),
        ("category1-ceiling", "ISREJiMZ", 12, 0, 0, 1),
        ("category2", "ISREJiMZ", 6, 0, 0, 1),
        ("mailbox-before", "ISREqMZ", 2, 2, 2, 1),
        ("mailbox-between", "ISREqMZ", 2, 2, 2, 1),
        ("mailbox-after-clear", "ISREqMZ", 2, 2, 2, 1),
        ("mailbox-empty-wait", "ISREqMZ", 2, 2, 2, 1),
        ("mailbox-wait", "ISREqMZ", 2, 1, 1, 1),
        ("mailbox-full", "ISREqMZ", 2, 256, 256, 1),
        ("mailbox-wrap", "ISREqMZ", 2, 522, 522, 1),
        ("mailbox-exhaustion", "ISREqMZ", 2, 1, 18446744073709551615, 1),
        ("mailbox-api", "ISREPqpMZ", 4, 2, 2, 1),
    ]
    observations = []
    for scenario, trace, rejected, records, ticket, entries in cases:
        result = execute(binary, scenario)
        summary = re.search(
            r"event checks=(\d+) rejected=(\d+) records=(\d+) ticket=(\d+) entries=(\d+) error=(\d+) bridge_rejected=(\d+)",
            result["stdout"],
        )
        require(result["exit"] == 0 and summary is not None, result)
        values = tuple(map(int, summary.groups()))
        bridge_rejected = (
            9
            if scenario == "mailbox-api"
            else 1
            if scenario in {"mailbox-full", "mailbox-exhaustion"}
            else 0
        )
        require(
            values[0] > 15
            and values[1:] == (rejected, records, ticket, entries, 0, bridge_rejected),
            result,
        )
        require(f"trace={trace} " in result["stdout"] and not result["stderr"], result)
        observations.append(result)
    for scenario, status in [
        ("input-invalid-task", 3),
        ("input-basic", 8),
        ("input-inactive", 8),
        ("input-category1", 8),
        ("category1-resource", 8),
    ]:
        result = execute(binary, scenario)
        require(result["exit"] == 0 and result["stdout"] == f"prepare={status}", result)
        observations.append(result)
    mode = execute(binary, "mailbox-mode")
    require(mode["exit"] == 8 and "trace=IZ " in mode["stdout"], mode)
    require("threads=0 events=0 mutexes=0 " in mode["stdout"], mode)
    observations.append(mode)
    for scenario, trace in [
        ("mailbox-replace-startup", "ISZ"),
        ("mailbox-replace-running", "ISREZ"),
    ]:
        result = execute(binary, scenario)
        require(result["exit"] == 1 and f"trace={trace} " in result["stdout"], result)
        require("records=0 ticket=0 " in result["stdout"], result)
        observations.append(result)
    return observations


def check_activation(binary: Path) -> list[dict]:
    import re

    literal_oracles = json.loads(
        (ROOT / "core/tests/fixtures/epic4_oracles/os.json").read_text(encoding="utf-8")
    )
    fifo = {case["id"]: case for case in literal_oracles["cases"]}
    cases = [
        ("aab", "ISRLAHaCBMZ", (2, 1, 0, 1), 0),
        ("aba", "ISRLAHaBCMZ", (2, 1, 0, 1), 0),
        ("aabb", "ISRLAHaCBDMZ", (2, 2, 0, 1), 0),
        ("abab", "ISRLAHaBCDMZ", (2, 2, 0, 1), 0),
        ("limits", "ISRLAHaCBMZ", (2, 1, 0, 1), 2),
        ("invalid", "ISRLAHaCBMZ", (2, 1, 0, 1), 1),
        ("extended", "ISRLEMZ", (0, 0, 1, 0), 2),
        ("overflow", "ISRLABMZ", (1, 1, 0, 0), 0),
        ("rebase-fifo", "ISRLABCDMZ", (2, 2, 0, 0), 0),
        ("autostart", "ISRABMZ", (1, 1, 0, 0), 0),
        ("capacity", "ISRLA" + "C" * 31 + "MZ", (32, 0, 0, 0), 1),
        ("wrap", "ISRLA" + "C" * 47 + "MZ", (48, 0, 0, 0), 1),
        ("isr", "ISRLJAlBMZ", (1, 1, 0, 0), 3),
    ]
    observations = []
    for scenario, expected_trace, counts, rejected in cases:
        result = execute(binary, scenario)
        summary = re.search(
            r"activation a=(\d+) b=(\d+) e=(\d+) h=(\d+) observer=(\d+) rejected=(\d+) error=(\d+)",
            result["stdout"],
        )
        trace = re.search(r"trace=(\S+)", result["stdout"])
        require(
            result["exit"] == 0 and summary is not None and trace is not None, result
        )
        require(tuple(map(int, summary.groups()[:4])) == counts, result)
        require(
            int(summary[5]) > 0
            and int(summary[6]) == rejected
            and int(summary[7]) == 0,
            result,
        )
        require(
            trace[1] == expected_trace
            and "input_closed=1 tick_closed=1" in result["stdout"],
            result,
        )
        if scenario in {"aabb", "abab"}:
            entry_names = {"A": "A#1", "C": "A#2", "B": "B#1", "D": "B#2"}
            actual_entries = [
                entry_names[marker] for marker in trace[1] if marker in entry_names
            ]
            require(
                actual_entries == fifo["fifo-" + scenario]["expected_entry_order"],
                result,
            )
        result["independent_expected_trace"] = expected_trace
        observations.append(result)
    for scenario in ["zero-limit", "bad-limit", "extended-multiple", "bad-kind"]:
        result = execute(binary, scenario)
        require(result["exit"] == 0 and result["stdout"] == "prepare=8", result)
        observations.append(result)
    for scenario, pending in [
        ("close-admission", 0),
        ("close-ready", 0),
        ("close-accepted", 1),
    ]:
        close = execute(binary, scenario)
        require(close["exit"] == 0 and "trace=ISRLQZ " in close["stdout"], close)
        require(
            f"close_pending={pending}\n" in close["stdout"]
            and "input_closed=1 tick_closed=1" in close["stdout"],
            close,
        )
        observations.append(close)
    return observations


def check_finish(binary: Path) -> list[dict]:
    observations = []
    cases = [
        ("terminate-pending", "ISRLACMZ", (2, 0, 0, 0, 0), 0, 0),
        ("self", "ISRLABCMZ", (2, 1, 0, 0, 0), 0, 0),
        ("self-pending", "ISRLACBFMZ", (3, 1, 0, 0, 0), 0, 0),
        ("high", "ISRLAHMZ", (1, 0, 1, 0, 0), 0, 0),
        ("same", "ISRLABMZ", (1, 1, 0, 0, 0), 0, 0),
        ("same-fifo", "ISRLAHBbMZ", (1, 2, 1, 0, 0), 0, 0),
        ("low", "ISRLACDMZ", (2, 0, 0, 1, 0), 0, 0),
        ("invalid", "ISRLABMZ", (1, 1, 0, 0, 0), 1, 0),
        ("limit", "ISRLAEMZ", (1, 0, 0, 0, 1), 1, 0),
        ("resource", "ISRLAMZ", (1, 0, 0, 0, 0), 2, 0),
        ("extended-self", "ISRLEeMZ", (0, 0, 0, 0, 2), 0, 0),
        ("extended-target", "ISRLEAeMZ", (1, 0, 0, 0, 2), 0, 0),
        ("isr", "ISRLAJaCMZ", (2, 0, 0, 0, 0), 2, 0),
        ("boundary-pre", "ISRLAJBCMZ", (2, 1, 0, 0, 0), 2, 0),
        ("boundary-post", "ISRLAJBCMZ", (2, 1, 0, 0, 0), 2, 0),
        ("missing-end", "ISRLAMZ", (1, 0, 0, 0, 0), 0, 0),
    ]
    for scenario, expected, counts, rejected, code in cases:
        result = execute(binary, scenario)
        summary = re.search(
            r"finish a=(\d+) b=(\d+) h=(\d+) d=(\d+) e=(\d+) observer=(\d+) rejected=(\d+) error=(\d+)",
            result["stdout"],
        )
        require(
            result["exit"] == code
            and f"trace={expected} " in result["stdout"]
            and summary is not None,
            result,
        )
        require(tuple(map(int, summary.groups()[:5])) == counts, result)
        require(
            int(summary[6]) > 0
            and int(summary[7]) == rejected
            and int(summary[8]) == code,
            result,
        )
        result["independent_expected_trace"] = expected
        observations.append(result)
    for scenario, code in [
        ("resource-null", 8),
        ("resource-capacity", 8),
        ("resource-access", 8),
        ("resource-duplicate", 3),
        ("resource-ceiling-low", 8),
        ("resource-ceiling-high", 8),
    ]:
        result = execute(binary, scenario)
        require(result["exit"] == 0 and result["stdout"] == f"prepare={code}", result)
        observations.append(result)
    return observations


def check_resources(binary: Path) -> list[dict]:
    observations = []
    for scenario, trace, counts in [
        ("full", "ISRAHaMZ", (1, 0, 0, 0)),
        ("non", "ISRAnHaMZ", (1, 0, 0, 0)),
        ("internal", "ISRAgHaMZ", (1, 0, 0, 0)),
        ("internal-preempt", "ISRADgHaMZ", (1, 0, 0, 0)),
        ("wait", "ISRABaMZ", (0, 1, 0, 0)),
        ("already", "ISRAaBMZ", (0, 1, 0, 0)),
        ("nested", "ISRAHaMZ", (1, 0, 0, 5)),
        ("resource-wait", "ISRAHaMZ", (1, 0, 0, 6)),
        ("restore", "ISRArHaBMZ", (1, 1, 0, 0)),
        ("nonowner", "ISRADaMZ", (0, 0, 0, 0)),
        ("masked-isr", "ISRApJaMZ", (0, 0, 1, 4)),
        ("external-isr", "ISRAJaMZ", (0, 0, 1, 4)),
        ("oracle-non", "ISRAnHaMZ", (1, 0, 0, 0)),
        ("oracle-resource", "ISRArHaMZ", (1, 0, 0, 0)),
        ("internal-resource", "ISRAaMZ", (0, 0, 0, 3)),
        ("access", "ISRAaBMZ", (0, 1, 0, 2)),
        ("scheduler", "ISRAJrHaMZ", (1, 0, 1, 4)),
        ("scheduler-unconfigured", "ISRAaMZ", (0, 0, 0, 0)),
    ]:
        result = execute(binary, scenario)
        summary = re.search(
            r"resources h=(\d+) b=(\d+) irq=(\d+) observer=(\d+) rejected=(\d+) error=(\d+)",
            result["stdout"],
        )
        require(
            result["exit"] == 0
            and f"trace={trace} " in result["stdout"]
            and summary is not None,
            result,
        )
        require(
            (int(summary[1]), int(summary[2]), int(summary[3]), int(summary[5]))
            == counts
            and int(summary[4]) > 0
            and int(summary[6]) == 0,
            result,
        )
        result["independent_expected_trace"] = trace
        observations.append(result)
    for scenario, code in [
        ("bad-schedule", 8),
        ("internal-null", 8),
        ("internal-capacity", 8),
        ("internal-duplicate", 3),
        ("internal-missing", 3),
        ("internal-low", 8),
        ("internal-high", 8),
        ("non-internal-low", 8),
    ]:
        result = execute(binary, scenario)
        require(result["exit"] == 0 and result["stdout"] == f"prepare={code}", result)
        observations.append(result)
    return observations


def check_lifecycle(binary: Path) -> list[dict]:
    observations = []
    for scenario, trace in [
        ("normal", "ISRAD"),
        ("mode2", "ISRBD"),
        ("priority", "ISRBD"),
    ]:
        result = execute(binary, scenario)
        require(result["exit"] == 0 and f"trace={trace} " in result["stdout"], result)
        require("input_closed=1 tick_closed=1" in result["stdout"], result)
        require("FORBIDDEN" not in result["stdout"], result)
        observations.append(result)
    for scenario, code in [
        ("zero-count", 8),
        ("capacity", 8),
        ("duplicate", 3),
        ("bad-id", 3),
        ("zero-priority", 8),
        ("bad-priority", 8),
        ("null-entry", 8),
        ("bad-stack", 8),
        ("small-stack", 8),
        ("large-stack", 8),
        ("null-tasks", 8),
        ("bad-autostart", 8),
    ]:
        result = execute(binary, scenario)
        require(result["exit"] == code, result)
        require("rejected_before_threads=1" in result["stdout"], result)
        observations.append(result)
    for scenario, trace, reason in [
        ("bad-mode", "ID", 8),
        ("startup-failure", "ISD", 7),
        ("repeat-start", "ISRD", 7),
    ]:
        result = execute(binary, scenario)
        require(
            result["exit"] == reason and f"trace={trace} " in result["stdout"], result
        )
        require("state=Failed" in result["stdout"], result)
        observations.append(result)
    for failure in ["invalid", "-1", "429496729600", "0"]:
        result = execute(binary, "normal", failure)
        require(result["exit"] == 8 and "trace=ID " in result["stdout"], result)
        observations.append(result)
    normal = observations[0]["stdout"]
    calls = int(normal.split("resource_calls=")[1].split()[0])
    # Every actual native resource acquisition, including bootstrap and idle.
    for failure in range(1, calls + 1):
        result = execute(binary, "normal", failure)
        require(result["exit"] == 7 and "state=Failed" in result["stdout"], result)
        require(
            "trace=ID " in result["stdout"] and "FORBIDDEN" not in result["stdout"],
            result,
        )
        observations.append(result)
    return observations


def check_stack(binary: Path) -> list[dict]:
    observations = []
    for scenario, role, code in [
        ("normal", None, None),
        ("task", "T", "C00000FD"),
        ("startup", "B", "C00000FD"),
        ("isr", "S", "C00000FD"),
        ("control", "C", "C00000FD"),
        ("backup", "D", "C00000FD"),
        ("idle", "I", "C00000FD"),
        ("boundary", "T", "C0000005"),
        ("boundary-read", "T", "C0000005"),
        ("sp-corrupt", "T", "00000000"),
    ]:
        result = execute(binary, scenario)
        output = result["stdout"]
        require(result["exit"] == (0 if role is None else 13), result)
        require("captured=1" in output if role else "FAULT" not in output, result)
        require("X" not in output.split("trace=")[1].split()[0], result)
        require("D" in output.split("trace=")[1].split()[0], result)
        if role:
            require(
                f"FAULT role={role} " in output and f"exception={code}" in output,
                result,
            )
        records = [
            dict(field.split("=", 1) for field in line.split()[1:])
            for line in output.splitlines()
            if line.startswith("STACK ")
        ]
        require({r["role"] for r in records} == {"T", "B", "I", "S", "C", "D"}, result)
        for record in records:
            low, high, sp = (int(record[key]) for key in ["low", "high", "sp"])
            require(low < sp < high and high - low == int(record["reserve"]), result)
            require(
                int(record["commit"]) > 0
                and int(record["guard"]) >= 4096
                and int(record["guarantee"]) >= 16384
                and int(record["observations"]) > 0,
                result,
            )
            if record["role"] != "S":
                require(int(record["reserve"]) == 262144, result)
            if record["role"] in {"T", "B", "I"}:
                buffer_low = int(record["buffer_low"])
                buffer_high = int(record["buffer_high"])
                require(
                    buffer_low < buffer_high and not buffer_low <= sp < buffer_high,
                    result,
                )
        observations.append(result)
    concurrent = execute(binary, "concurrent")
    require(concurrent["exit"] == 13, concurrent)
    fault_lines = [
        dict(field.split("=", 1) for field in line.split()[1:])
        for line in concurrent["stdout"].splitlines()
        if line.startswith("FAULT ")
    ]
    stack_lines = [
        dict(field.split("=", 1) for field in line.split()[1:])
        for line in concurrent["stdout"].splitlines()
        if line.startswith("STACK ")
    ]
    require(bool(fault_lines), concurrent)
    require({fault["role"] for fault in fault_lines} == {"T", "D"}, concurrent)
    for fault in fault_lines:
        require(
            fault["role"] in {"T", "D"} and fault["exception"] == "C00000FD", concurrent
        )
        require(
            any(
                row["tid"] == fault["tid"] and row["role"] == fault["role"]
                for row in stack_lines
            ),
            concurrent,
        )
    require("X" not in concurrent["stdout"].split("trace=")[1].split()[0], concurrent)
    observations.append(concurrent)
    double = execute(binary, "double-control")
    require(
        double["exit"] == 13 and "controllers_exhausted=1" in double["stdout"], double
    )
    require(
        "FAULT role=C " in double["stdout"]
        and "exception=C00000FD" in double["stdout"],
        double,
    )
    observations.append(double)
    output_failure = execute(binary, "double-control-output")
    require(
        output_failure["exit"] == 13
        and "controllers_exhausted=1" in output_failure["stderr"],
        output_failure,
    )
    require(
        "FAULT role=C " in output_failure["stderr"]
        and "exception=C00000FD" in output_failure["stderr"],
        output_failure,
    )
    observations.append(output_failure)
    unrelated = execute(binary, "unrelated")
    require(
        unrelated["exit"] & 0xFFFFFFFF == 0xC0000005
        and "FAULT" not in unrelated["stdout"],
        unrelated,
    )
    observations.append(unrelated)
    for scenario in ["api-suspend", "api-context", "api-resume"]:
        result = execute(binary, scenario)
        require(result["exit"] == 7 and "FAULT" not in result["stdout"], result)
        trace = result["stdout"].split("trace=")[1].split()[0]
        require(
            trace == "ISRAFD" and "input_closed=1 tick_closed=1" in result["stdout"],
            result,
        )
        observations.append(result)
    for role in ["S", "T", "B", "I", "C", "D"]:
        result = execute(binary, "normal", invalid_stack=role)
        require(result["exit"] == 7 and "state=Failed" in result["stdout"], result)
        require(
            "FAULT" not in result["stdout"]
            and "R" not in result["stdout"].split("trace=")[1].split()[0],
            result,
        )
        observations.append(result)
    return observations


def check_interrupt_pairing(binary: Path) -> list[dict]:
    service_ids = [128,129,130,131,132,133,134,135,136,137,138,139,
                   15,16,17,140,141,142,143,144,7,8,9,10,14,29]
    cases = {
        "disable": (1, 1, 0, "CO"),
        "all-nested": (1, 1, 0, "NCO"),
        "os-nested": (1, 1, 0, "CNO"),
        "os-all": (1, 1, 0, "CO"),
        "all-os": (1, 1, 0, "CO"),
        "unmatched": (1, 1, 0, "CO"),
        "hook-allowed": (1, 1, 0, "CO"),
        "services-disable": (1, 1, 26, "CO"),
        "services-all": (1, 1, 26, "CO"),
        "services-os": (1, 1, 26, "CO"),
        "isr-balanced": (1, 1, 0, "CO"),
        "isr-leak-disable": (1, 1, 1, "CO"),
        "isr-leak-all": (1, 1, 1, "CO"),
        "isr-leak-os": (1, 1, 1, "CO"),
        "isr-leak-mixed": (1, 1, 1, "CO"),
        "source-retain": (1, 1, 1, "OC"),
        "source-clear": (1, 1, 1, "OC"),
        "source-enable-clear": (1, 1, 1, "OC"),
        "source-global": (1, 1, 1, "OC"),
        "source-invalid": (1, 1, 7, "CO"),
        "source-hook-reject": (1, 1, 3, "CO"),
        "source-isr": (1, 1, 0, "OQC"),
        "source-outside-isr": (1, 1, 3, "ODEC"),
        "missing-disable": (0, 1, 1, "OM"),
        "missing-all": (0, 1, 1, "OM"),
        "missing-os": (0, 1, 1, "OM"),
        "missing-mixed": (0, 1, 1, "OM"),
        "disabled-first-start": (0, 0, 0, ""),
    }
    observations = []
    for scenario, (cat1, cat2, errors, trace) in cases.items():
        startup = int(scenario != "disabled-first-start")
        reason = 9 if not startup else 0
        task = 255 if scenario.startswith("missing-") or not startup else 0
        result = execute(binary, scenario)
        expected = (f"pairing scenario={scenario} startup={startup} cat1={cat1} cat2={cat2} "
                    f"isr_task={task} errors={errors} trace={trace} reason={reason}")
        error_expected = ([(9,service,"T") for service in service_ids]
                          if scenario.startswith("services-") else
                          [(3,service,"T") for service in [48,49,50,48,48,48,48]]
                          if scenario == "source-invalid" else
                          [(2,service,"B") for service in [48,49,50]]
                          if scenario == "source-hook-reject" else
                          [(2,service,"S") for service in [48,49,50]]
                          if scenario == "source-outside-isr" else
                          [(5,48,"T")] if scenario in ["source-retain", "source-clear", "source-enable-clear", "source-global"] else
                          [(9,252,"S")] if scenario.startswith("isr-leak-") else
                          [(11,253,"T")] if scenario.startswith("missing-") else [])
        actual = [(int(status),int(service),actor) for status,service,actor in re.findall(
            r"mask_error status=(\d+) service=(\d+) actor=([TBS])", result["stdout"])]
        require(result["exit"] == reason and not result["stderr"], result)
        require(expected in result["stdout"] and actual == error_expected, result)
        require(f"lifecycle=Closed state={'Ready' if startup else 'Failed'} reason={reason}"
                in result["stdout"], result)
        result["independent_expected_pairing"] = [cat1,cat2,task,errors,trace,reason]
        result["independent_expected_errors"] = error_expected
        observations.append(result)
    return observations


def check_calling_context(binary: Path) -> list[dict]:
    ids = [128,129,130,131,132,133,134,135,136,137,138,139,
           15,16,17,140,141,142,143,144,7,8,9,10,14,48,49,50,29]
    observations = []
    for scenario in ["startup", "pre", "post", "error", "alarm-callback", "shutdown",
                     "startup-shutdown", "error-shutdown", "outside-isr"]:
        queries = scenario in ["pre", "post", "error", "error-shutdown"]
        expected = [2] * 29
        if queries:
            expected[0] = 0
            for index in [1,11,15,16]:
                expected[index] = 3
        if scenario.startswith("error"):
            error_expected = [(3,130)]
        elif scenario == "shutdown":
            error_expected = []
        else:
            error_expected = [] if scenario == "outside-isr" else [(9,service) for service in [128,29]]
            error_expected += [(status, service) for status, service in zip(expected, ids) if status]
        reason = 8 if scenario.endswith("-shutdown") else 0
        result = execute(binary, scenario)
        actual = [(int(status),int(service)) for status,service in re.findall(
            r"matrix_error status=(\d+) service=(\d+)", result["stdout"])]
        summary = (f"calling scenario={scenario} errors={len(error_expected)} reason={reason} "
                   "statuses=" + ",".join(map(str, expected)))
        require(result["exit"] == reason and not result["stderr"], result)
        require(summary in result["stdout"] and actual == error_expected, result)
        require("lifecycle=Closed" in result["stdout"] and f"reason={reason}" in result["stdout"], result)
        result["independent_expected_statuses"] = expected
        result["independent_expected_errors"] = error_expected
        result["context_scope"] = ("actual configured standard Alarm callback delivered by software Counter increment"
                                   if scenario == "alarm-callback" else
                                   "no-active-ISR boundary fault injection on actual native ISR stack"
                                   if scenario == "outside-isr" else "actual standard global Hook execution")
        observations.append(result)
    return observations


def check_idle_state(binary: Path) -> list[dict]:
    observations = []
    cases = {
        "valid": [(2,29)],
        "invalid": [(2,29),(3,29)],
        "masked": [(2,29),(9,29)],
        "isr": [(2,29)],
        "unconfigured": [],
    }
    for scenario, expected in cases.items():
        result = execute(binary, scenario)
        actual = [(int(status),int(service)) for status,service in re.findall(
            r"idle_error status=(\d+) service=(\d+)", result["stdout"])]
        summary = (f"idle_state scenario={scenario} observed=1 wakes=1 errors={len(expected)} reason=0")
        require(result["exit"] == 0 and not result["stderr"], result)
        require(summary in result["stdout"] and actual == expected, result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        result["independent_expected_errors"] = expected
        result["scope"] = "single-core ignores CoreID65535; existing virtual-core no-halt idle loop runs on actual I stack then actual ISR wakes waiting automotive Task; started query remains DRAFT"
        observations.append(result)
    return observations


def check_status_modes(directory: Path) -> list[dict]:
    invalid = subprocess.run(
        [compiler(), "-std=c99", "-E", "-x", "c", "-", "-DOS_STATUS_EXTENDED=2",
         "-I", str(TARGET / "include")],
        input='#include "Os_Cfg.h"\n', capture_output=True, text=True, timeout=20,
    )
    require(invalid.returncode != 0 and "Invalid OS status configuration" in invalid.stderr,
            {"stdout": invalid.stdout, "stderr": invalid.stderr, "exit": invalid.returncode})
    observations = []
    for mode in (0, 1):
        build_directory = directory / str(mode)
        build_directory.mkdir()
        binary, _ = build(
            build_directory,
            "status_modes.c",
            (f"OS_STATUS_EXTENDED={mode}", f"EXPECTED_STATUS_EXTENDED={mode}"),
        )
        result = execute(binary, "")
        expected = f"status_modes extended={mode} errors=8 peers=1 preserved=1 reason=0"
        require(result["exit"] == 0 and not result["stderr"], result)
        require(expected in result["stdout"], result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        observations.append(result)
    capacity_directory = directory / "standard-capacity"
    capacity_directory.mkdir()
    capacity_binary, _ = build(capacity_directory, "sc1_capacity.c", ("OS_STATUS_EXTENDED=0",))
    capacity = check_capacity(capacity_binary)
    require(len(capacity) == 18, {"capacity_vectors": len(capacity)})
    return observations


def check_entry_bodies(binary: Path) -> list[dict]:
    cases = {
        "normal": (1,0,1,0,0,"OIiWo",0),
        "nested": (1,1,1,0,0,"OICciWo",0),
        "cleanup-mask": (1,0,1,0,1,"OIiEWo",0),
        "cleanup-resource": (1,0,1,0,1,"OIiEWo",0),
        "alarm": (0,0,1,1,1,"OAEaWo",0),
        "replace-owned": (0,0,0,0,0,"",1),
    }
    observations = []
    for scenario, (parents,children,workers,alarms,errors,trace,reason) in cases.items():
        result = execute(binary, scenario)
        expected = (f"entry_bodies scenario={scenario} parents={parents} children={children} "
                    f"workers={workers} alarms={alarms} errors={errors} trace={trace} reason={reason}")
        require(result["exit"] == reason and not result["stderr"], result)
        require(expected in result["stdout"], result)
        require(f"lifecycle=Closed state={'Ready' if reason == 0 else 'Failed'} reason={reason}" in result["stdout"], result)
        result["independent_expected"] = expected
        observations.append(result)
    for scenario in ["invalid-priority", "invalid-kernel", "invalid-tick", "invalid-cat1", "invalid-mailbox"]:
        result = execute(binary, scenario)
        expected = f"entry_bodies rejection={scenario} status=8 started=0"
        require(result["exit"] == 0 and result["stdout"] == expected and not result["stderr"], result)
        result["independent_expected"] = expected
        observations.append(result)
    return observations


def check_source_repetition(binary: Path) -> list[dict]:
    cases = {
        "default-enable": [(5,49,6,0,255)],
        "enable-false": [(5,49,6,0,255)],
        "enable-true": [(5,49,6,1,255)],
        "disable": [(5,48,6,0,255)],
        "disable-all": [(5,48,6,0,255)],
        "isr-enable-false": [(5,49,6,0,5)],
        "isr-enable-true": [(5,49,6,1,5)],
        "isr-disable": [(5,48,6,0,5)],
        "clear-repeat": [],
        "invalid-kernel": [(3,49,1,1,255)],
        "invalid-unconfigured": [(3,49,31,1,255)],
        "unconfigured": [],
    }
    observations = []
    for scenario, expected_errors in cases.items():
        result = execute(binary, scenario)
        parent = int(scenario.startswith("isr-"))
        trace = "PpD" if parent else "D"
        expected = (f"source_repeat scenario={scenario} deliveries=1 parents={parent} "
                    f"errors={len(expected_errors)} trace={trace} reason=0")
        actual = [tuple(map(int, row)) for row in re.findall(
            r"source_repeat_error status=(\d+) service=(\d+) id=(\d+) clear=(\d+) caller=(\d+)",
            result["stdout"])]
        require(result["exit"] == 0 and not result["stderr"], result)
        require(expected in result["stdout"] and actual == expected_errors, result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        result["independent_expected"] = expected
        result["independent_errors"] = expected_errors
        observations.append(result)
    return observations


def check_nonstatus_errors(binary: Path) -> list[dict]:
    observations = []
    for scenario in ["task", "isr", "cat1", "startup", "pre", "post", "shutdown",
                     "alarm", "error", "unconfigured"]:
        result = execute(binary, scenario)
        errors = 0 if scenario == "unconfigured" else 1
        expected = f"nonstatus scenario={scenario} probes=1 errors={errors} snapshot=pass reason=0"
        require(result["exit"] == 0 and not result["stderr"], result)
        require(expected in result["stdout"], result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        result["independent_expected"] = expected
        result["scope"] = "Real native caller/Hook; ignored void/scalar services never report; StatusType ActivateTask retains one typed error and ErrorHook snapshot"
        observations.append(result)
    return observations


def check_counter_types(binary: Path) -> list[dict]:
    observations = []
    for scenario, errors in [("configured", 9), ("unconfigured", 0)]:
        result = execute(binary, scenario)
        expected = f"counter_types scenario={scenario} checks=9 errors={errors} value=7 other=0 reason=0"
        require(result["exit"] == 0 and not result["stderr"] and expected in result["stdout"], result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        result["independent_expected"] = expected
        observations.append(result)
    for field in ("counter", "system", "alarm", "increment", "schedule"):
        for value in ("256", "65536", "max"):
            scenario = field + "-" + value
            result = execute(binary, scenario)
            expected = f"counter_rejection scenario={scenario} status={8 if field == 'counter' else 3} configured=0"
            require(result["exit"] == 0 and not result["stderr"] and result["stdout"] == expected, result)
            result["independent_expected"] = expected
            observations.append(result)
    return observations


def check_nested_interrupts(binary: Path) -> list[dict]:
    # Literal expected ordering is independent of source IDs and the native
    # dispatch implementation. All entries execute actual installed callbacks.
    cases = {
        "nested": (1, 1, 1, 3, "APCGcpHB", []),
        "native-call": (1, 1, 1, 3, "APCGcpHB", []),
        "equal": (1, 0, 1, 1, "APpCcHB", []),
        "lower": (1, 0, 1, 1, "APpCcHB", []),
        "priority-order": (1, 1, 0, 1, "ACPGB", []),
        "resources": (1, 1, 1, 3, "APCGcpHB", [(6, 5, 134)]),
        "resource-leak": (1, 1, 1, 3, "APCGcpHB", [(6, 5, 134), (6, 6, 252)]),
        "ceiling": (1, 1, 1, 3, "APCGcpHB", [(6, 5, 134)]),
        "task-ceiling": (1, 0, 1, 1, "ACcHB", [(6, 5, 134)]),
        "mask-all": (1, 1, 1, 2, "APGCcpHB", []),
        "mask-os": (1, 1, 1, 2, "APGCcpHB", []),
        "mask-cat1-leak": (1, 1, 1, 2, "APGCcpHB", []),
        "child-mask-leak": (1, 0, 1, 2, "APCcpHB", [(6, 9, 252)]),
        "hook": (1, 1, 1, 2, "APEGeCcpHB", [(20, 3, 129)]),
        "cat1-outer": (1, 1, 1, 2, "APCcpTGHB", []),
        "source-enable": (1, 0, 1, 2, "APCcpHB", []),
        "source-clear": (1, 0, 1, 2, "APDCcpHB", []),
        "source-clear-pending": (1, 0, 1, 2, "APDCcpHB", []),
        "maximum": (0, 0, 1, 30, "A" + "<" * 30 + ">" * 30 + "HB", []),
    }
    observations = []
    for scenario, (children, grandchildren, helpers, maximum, trace, errors) in cases.items():
        result = execute(binary, scenario)
        expected = (f"nested scenario={scenario} children={children} grandchildren={grandchildren} "
                    f"helpers={helpers} errors={len(errors)} depth=0 maximum={maximum} "
                    f"trace={trace} reason=0")
        actual_errors = [tuple(map(int, values)) for values in re.findall(
            r"nested_error isr=(\d+) status=(\d+) service=(\d+)", result["stdout"])]
        require(result["exit"] == 0 and not result["stderr"], result)
        require(expected in result["stdout"] and actual_errors == errors, result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        require(len(re.findall(r"^STACK role=S ", result["stdout"], re.MULTILINE)) == 1, result)
        result["independent_expected"] = expected
        result["independent_errors"] = errors
        observations.append(result)
    for rejection in ("priority", "yield", "tick", "category", "resource", "ceiling", "input"):
        scenario = "reject-" + rejection
        result = execute(binary, scenario)
        expected = f"nested_rejection scenario={scenario} status=8 configured=0"
        require(result["exit"] == 0 and not result["stderr"] and result["stdout"] == expected, result)
        result["independent_expected"] = expected
        observations.append(result)
    return observations


def check_isr_cleanup(binary: Path) -> list[dict]:
    cases = {
        "single": (1, 6, 6, "AIRPHB"),
        "nested": (1, 6, 6, "AIRPHB"),
        "maximum": (1, 6, 6, "AIRPHB"),
        "disable": (2, 9, 6, "AIDRPHB"),
        "all": (2, 9, 6, "AIDRPHB"),
        "os": (2, 9, 6, "AIDRPHB"),
        "mixed": (2, 9, 6, "AIDRPHB"),
        "unconfigured": (0, 0, 0, "AIPHB"),
        "unconfigured-mixed": (0, 0, 0, "AIPHB"),
        "balanced": (0, 0, 0, "AIPHB"),
    }
    observations = []
    for scenario, (errors, first, last, trace) in cases.items():
        result = execute(binary, scenario)
        expected = (f"isr_cleanup scenario={scenario} probes=1 helpers=1 errors={errors} "
                    f"first={first} last={last} trace={trace} reason=0")
        require(result["exit"] == 0 and not result["stderr"], result)
        require(expected in result["stdout"], result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        result["independent_expected_cleanup"] = [1, 1, errors, first, last, trace]
        observations.append(result)
    return observations


def check_returned_task(binary: Path) -> list[dict]:
    observations = []
    cases = {
        "basic": (1, 1, 0, "p0Aeq0p2M"),
        "extended": (1, 1, 0, "p0Aeq0p2M"),
        "resources": (1, 1, 1, "p0Aeq0p1Hq1p2M"),
        "internal": (1, 1, 1, "p0Aeq0p1Hq1p2M"),
        "queued": (3, 3, 0, "p0Aeq0p0Beq0p0Ceq0p2M"),
        "queued-resources": (3, 3, 1, "p0Aeq0p1Hq1p0Beq0p0Ceq0p2M"),
        "unconfigured": (1, 0, 0, "AM"),
    }
    for scenario, (entries, errors, helper, trace) in cases.items():
        result = execute(binary, scenario)
        expected = (f"returned scenario={scenario} entries={entries} errors={errors} "
                    f"helper={helper} status={11 if errors else 0} "
                    f"service={253 if errors else 0} trace={trace} reason=0")
        require(result["exit"] == 0 and not result["stderr"], result)
        require(expected in result["stdout"], result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        result["independent_expected_activation_cleanup"] = [entries, errors, helper, trace]
        observations.append(result)
    return observations


def check_task_hooks(binary: Path) -> list[dict]:
    transitions = "p0Aq0p1Hq1p0Bq0p2Lq2p0Cq0p1Jq1p2Mq2p0D"
    expected_traces = {
        "transitions": transitions,
        "internal": transitions,
        "unconfigured": "AHBLCJMD",
        "self-chain": "p0Xq0p0Yq0p2Z",
        "queued": "p0Xq0p0Yq0p2Z",
        "shutdown": "p0F",
        "noop-yield": "p0F",
        "wait-satisfied": "p0F",
    }
    observations = []
    for scenario, trace in expected_traces.items():
        result = execute(binary, scenario)
        expected = (f"task_hooks scenario={scenario} "
                    f"configured={int(scenario != 'unconfigured')} trace={trace} reason=0")
        require(result["exit"] == 0 and not result["stderr"], result)
        require(expected in result["stdout"], result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        result["independent_expected_task_transition_trace"] = trace
        observations.append(result)
    return observations


def check_error_hooks(binary: Path, service_access: int = 1, parameter_access: int = 1) -> list[dict]:
    statuses = [3,6,3,10,3,3,5,6,6,1,3,3,3,3,10,3,5,8,8,5,8,8,5,5,3,8,3,4]
    services = [130,131,132,128,129,133,134,135,136,137,138,139,15,16,17,140,141,142,143,144,7,8,9,10,14,17,15,130]
    observations = []
    for scenario in ["configured", "unconfigured"]:
        result = execute(binary, scenario)
        configured = scenario == "configured"
        expected = [(i, status, services[i] if service_access else 0,
                     1 if i == 9 else 0, parameter_access, "T")
                    for i, status in enumerate(statuses)] if configured else []
        if configured:
            expected[0:0] = [(28, 2, 130 if service_access else 0, 255,
                              parameter_access, "B")] * 2
            expected.append((0, 3, 130 if service_access else 0, 0, parameter_access, "S"))
        actual = [tuple(map(int, values[:5])) + (values[5],) for values in re.findall(
            r"error vector=(\d+) status=(\d+) service=(\d+) caller=(\d+) parameters=(\d+) actor=([TSB])",
            result["stdout"])]
        header = (f"errors calls={len(expected)} service_access={service_access} "
                  f"parameter_access={parameter_access} configured={int(configured)} reason=0")
        require(result["exit"] == 0 and not result["stderr"], result)
        require(header in result["stdout"] and actual == expected, result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        result["independent_expected_errors"] = expected
        result["configuration_switches"] = [service_access, parameter_access]
        observations.append(result)
    return observations


def check_capacity(binary: Path) -> list[dict]:
    # Figure 3-3 minima are independent of the producer's configuration arrays.
    cases = {
        "bcc1": (8, 1, False, False, False),
        "bcc2": (8, 8, False, False, False),
        "ecc1": (16, 8, True, False, False),
        "ecc2": (16, 8, True, False, False),
        "bcc1-nonpreemptive": (8, 1, False, False, True),
        "bcc2-nonpreemptive": (8, 8, False, False, True),
        "ecc1-nonpreemptive": (16, 8, True, False, True),
        "ecc2-nonpreemptive": (16, 8, True, False, True),
        "bcc2-multiplicity": (8, 8, False, True, False),
        "ecc2-multiplicity": (16, 8, True, True, False),
        "ecc1-mode2": (16, 8, True, False, False),
    }
    observations = []
    for name, (count, resources, extended, multiple, nonpreemptive) in cases.items():
        result = execute(binary, name)
        priority_count = count - int(multiple)
        header = (f"capacity case={name} tasks={count} active={count} "
                  f"priorities={priority_count} resources={resources} internal=2 "
                  f"alarms=1 modes=1 queued={3 if multiple else 0} reason=0")
        expected = []
        order = (list(range(count - 2, 1, -1)) + [1, count - 1, 0]
                 if multiple else list(range(count - 1, -1, -1)))
        order += [1]
        if multiple:
            order += [1, 1, 1, count - 1]
        entries = [0] * count
        for position, task in enumerate(order):
            entries[task] += 1
            basic = not extended or (multiple and task in [1, count - 1])
            priority = (2 if multiple and task == count - 1 else task + 1)
            if nonpreemptive and task == 2:
                priority = count
            expected.append((task, entries[task], 0, priority,
                             int(task < 2 or (nonpreemptive and task == 2)),
                             0 if basic else 255, 0, 0 if position < count else 1))
        actual = [tuple(map(int, row)) for row in re.findall(
            r"task id=(\d+) entry=(\d+) state=(\d+) priority=(\d+) internal=(\d+) events_seen=(\d+) events_final=(\d+) counter=(\d+)",
            result["stdout"])]
        require(result["exit"] == 0 and not result["stderr"], result)
        require(header in result["stdout"] and actual == expected, result)
        require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
        result["independent_expected_tasks"] = expected
        result["independent_expected_capacity"] = header
        observations.append(result)
    for name in ["bad-manual-owner", "bad-manual-hardware", "bad-task-capacity",
                 "bad-resource-capacity", "bad-internal-capacity", "bad-alarm-capacity",
                 "bad-priority"]:
        result = execute(binary, name)
        expected = f"capacity rejection={name} reason=8 ready=0"
        require(result["exit"] == 8 and result["stdout"] == expected and not result["stderr"], result)
        result["independent_expected"] = expected
        observations.append(result)
    return observations


def check_public_types(directory: Path) -> None:
    binary = directory / "public_types.exe"
    command = [
        compiler(), "-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic",
        "-I" + str(TARGET / "include"), str(TARGET / "tests/public_types.c"),
        "-I" + str(ROOT / "runtime/include"),
        "-o", str(binary),
    ]
    expected = "public_types range_and_pointer_contracts=pass access_truth_table=16\n"
    for name, extra in [
        ("header-only", []),
        ("windows-before", ["-DOS_PUBLIC_TYPES_WINDOWS_BEFORE"]),
        ("windows-after", ["-DOS_PUBLIC_TYPES_WINDOWS_AFTER"]),
        ("rte-before-os", ["-DOS_PUBLIC_RTE_BEFORE"]),
        ("rte-windows-before", ["-DOS_PUBLIC_RTE_BEFORE", "-DOS_PUBLIC_TYPES_WINDOWS_BEFORE"]),
        ("rte-windows-after", ["-DOS_PUBLIC_RTE_BEFORE", "-DOS_PUBLIC_TYPES_WINDOWS_AFTER"]),
    ]:
        actual_command = command + ["-I" + str(TARGET / "src")] + extra
        subprocess.run(actual_command, capture_output=True, text=True, check=True)
        result = subprocess.run([str(binary)], capture_output=True, text=True, timeout=5)
        require(result.returncode == 0 and result.stdout == expected and not result.stderr,
                {"exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr})


def check_public_compatibility(directory: Path) -> None:
    binary = directory / "public_compatibility.exe"
    cc = compiler()
    nm = Path(shutil.which(cc) or cc).with_name("nm.exe")
    command = [cc, "-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic",
               "-I" + str(TARGET / "include"), "-I" + str(TARGET / "src"),
               "-I" + str(ROOT / "runtime/include"),
               str(TARGET / "tests/public_compatibility.c"), "-o", str(binary)]
    expected = "public_compatibility declarations=16 evaluations=0 error_codes=23 unique=pass\n"
    for name, extra in [
        ("header-only", []),
        ("windows-before", ["-DOS_PUBLIC_TYPES_WINDOWS_BEFORE"]),
        ("windows-after", ["-DOS_PUBLIC_TYPES_WINDOWS_AFTER"]),
        ("rte-before-os", ["-DOS_PUBLIC_RTE_BEFORE"]),
        ("rte-windows-before", ["-DOS_PUBLIC_RTE_BEFORE", "-DOS_PUBLIC_TYPES_WINDOWS_BEFORE"]),
        ("rte-windows-after", ["-DOS_PUBLIC_RTE_BEFORE", "-DOS_PUBLIC_TYPES_WINDOWS_AFTER"]),
    ]:
        actual_command = command + extra
        subprocess.run(actual_command, capture_output=True, text=True, check=True, timeout=30)
        result = subprocess.run([str(binary)], capture_output=True, text=True, timeout=5)
        require(result.returncode == 0 and result.stdout == expected and not result.stderr,
                {"exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr})
        signatures = []
        for baseline in [False, True]:
            object_command = actual_command.copy()
            object_file = directory / f"{name}-{'baseline' if baseline else 'actual'}.o"
            object_command[object_command.index("-o") + 1] = str(object_file)
            object_command.append("-c")
            if baseline:
                object_command.append("-DOS_PUBLIC_COMPAT_BASELINE")
            subprocess.run(object_command, capture_output=True, text=True, check=True, timeout=30)
            symbols = subprocess.run([str(nm), "--defined-only", "--format=posix", str(object_file)],
                                     capture_output=True, text=True, check=True, timeout=5)
            signatures.append(sorted(tuple(line.split()[:2]) for line in symbols.stdout.splitlines()))
        require(signatures[0] == signatures[1], {"actual_symbols": signatures[0],
                                                "baseline_symbols": signatures[1]})


def check_vector_mutations(directory: Path) -> None:
    positive = directory / "positive"
    positive.mkdir()
    binary, _ = build(positive, "entry_bodies.c")
    check_entry_bodies(binary)
    for name in ("old-binding", "no-section"):
        target = directory / (name + "-source")
        shutil.copytree(TARGET, target)
        output = directory / (name + "-build")
        output.mkdir()
        if name == "old-binding":
            (target / "patches/0014-relocatable-interrupt-vectors.patch").unlink()
            mutant, _ = build(output, "entry_bodies.c", target=target)
            result = execute(mutant, "normal")
            require(result["exit"] == 7 and not result["stderr"], result)
        else:
            path = target / "src/Os_Vector.c"
            text = path.read_text(encoding="utf-8")
            attribute = '__attribute__((section(".os_vec"), aligned(8)))'
            require(text.count(attribute) == 1, "vector section mutation did not apply")
            path.write_text(text.replace(attribute, ""), encoding="utf-8")
            try:
                build(output, "entry_bodies.c", target=target)
            except ValueError as error:
                require("vector section is missing" in str(error), str(error))
            else:
                raise AssertionError("missing vector section passed its linked contract check")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--suite",
        choices=[
            "lifecycle",
            "stack",
            "activation",
            "finish",
            "resources",
            "events",
            "time",
            "sc1-timing",
            "public-types",
            "public-compatibility",
            "capacity",
            "error-hooks",
            "task-hooks",
            "returned-task",
            "interrupt-pairing",
            "isr-cleanup",
            "nested-interrupts",
            "counter-types",
            "nonstatus-errors",
            "source-repetition",
            "entry-bodies",
            "vector-section",
            "status-modes",
            "calling-context",
            "idle-state",
        ],
        default="lifecycle",
    )
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="autosar-epic4-os-") as temporary:
        if args.suite == "vector-section":
            check_vector_mutations(Path(temporary))
            print("epic4_interrupt_vector_section PASS: actual Win64 vector table; 2 rejected compiled mutations")
            return
        if args.suite == "status-modes":
            observations = check_status_modes(Path(temporary))
            print("epic4_sc1_status_modes PASS: 2 native status configurations; 18 Standard capacity vectors")
            return
        if args.suite == "public-types":
            check_public_types(Path(temporary))
            print("epic4_os_public_type_contracts PASS: independent C99 consumer")
            return
        if args.suite == "public-compatibility":
            check_public_compatibility(Path(temporary))
            print("epic4_os_public_compatibility PASS: 6 independent C99 consumers")
            return
        harnesses = {
            "lifecycle": "lifecycle.c",
            "stack": "native_stack.c",
            "activation": "activation.c",
            "finish": "finish_chain.c",
            "resources": "resource_preemption.c",
            "events": "event_wakeup.c",
            "time": "controlled_time.c",
            "sc1-timing": "sc1_timing.c",
            "capacity": "sc1_capacity.c",
            "error-hooks": "error_hooks.c",
            "task-hooks": "task_hooks.c",
            "returned-task": "returned_task.c",
            "interrupt-pairing": "interrupt_pairing.c",
            "isr-cleanup": "isr_cleanup.c",
            "nested-interrupts": "nested_interrupts.c",
            "counter-types": "counter_types.c",
            "nonstatus-errors": "nonstatus_errors.c",
            "source-repetition": "source_repetition.c",
            "entry-bodies": "entry_bodies.c",
            "calling-context": "calling_context.c",
            "idle-state": "idle_state.c",
        }
        checks = {
            "lifecycle": check_lifecycle,
            "stack": check_stack,
            "activation": check_activation,
            "finish": check_finish,
            "resources": check_resources,
            "events": check_events,
            "time": check_time,
            "sc1-timing": check_sc1_timing,
            "capacity": check_capacity,
            "error-hooks": check_error_hooks,
            "task-hooks": check_task_hooks,
            "returned-task": check_returned_task,
            "interrupt-pairing": check_interrupt_pairing,
            "isr-cleanup": check_isr_cleanup,
            "nested-interrupts": check_nested_interrupts,
            "counter-types": check_counter_types,
            "nonstatus-errors": check_nonstatus_errors,
            "source-repetition": check_source_repetition,
            "entry-bodies": check_entry_bodies,
            "calling-context": check_calling_context,
            "idle-state": check_idle_state,
        }
        binary, _ = build(Path(temporary), harnesses[args.suite])
        observations = checks[args.suite](binary)
        if args.suite == "counter-types":
            headers = Path(temporary) / "public-headers"
            headers.mkdir()
            check_public_types(headers)
        if args.suite == "error-hooks":
            for service_access, parameter_access in [(0, 1), (1, 0), (0, 0)]:
                variant_dir = Path(temporary) / f"macros-{service_access}{parameter_access}"
                variant_dir.mkdir()
                variant, _ = build(variant_dir, "error_hooks.c", (
                    f"OS_USE_GET_SERVICE_ID={service_access}",
                    f"OS_USE_PARAMETER_ACCESS={parameter_access}",
                ))
                observations += check_error_hooks(variant, service_access, parameter_access)
        if args.suite == "sc1-timing":
            clock_dir = Path(temporary) / "host-timer"
            clock_dir.mkdir()
            clock, clock_sources = build(clock_dir, "controlled_time.c")
            for name, epochs, kernels, values in [
                ("hardware-counter", list(range(1, 21)), list(range(1, 21)), list(range(1, 16)) + [0, 1, 2, 3, 4]),
                ("hardware-kernel-wrap", [65535, 65536, 65537], [4294967294, 4294967295, 0], [1, 2, 3]),
            ]:
                result = execute(clock, name)
                actual = [tuple(map(int, row)) for row in re.findall(r"hardware epoch=(\d+) kernel=(\d+) value=(\d+) elapsed=(\d+)", result["stdout"])]
                expected = list(zip(epochs, kernels, values, [1] * len(values)))
                require(result["exit"] == 0 and not result["stderr"], result)
                require("lifecycle=Closed state=Ready reason=0" in result["stdout"], result)
                require(actual == expected, result)
                result["independent_expected_host_timer"] = expected
                observations.append(result)
        names = {
            "lifecycle": "epic4_backend_lifecycle",
            "stack": "epic4_native_stack_fault_shutdown",
            "activation": "epic4_activation_fifo",
            "finish": "epic4_finish_chain_atomicity",
            "resources": "epic4_resource_and_preemption",
            "events": "epic4_event_wakeup_races",
            "time": "epic4_controlled_tick_and_alarm",
            "sc1-timing": "epic4_sc1_timing_capacity",
            "capacity": "epic4_sc1_class_capacity",
            "error-hooks": "epic4_standard_error_hook_parameters",
            "task-hooks": "epic4_real_task_hook_transitions",
            "returned-task": "epic4_returned_task_resource_cleanup",
            "interrupt-pairing": "epic4_standard_interrupt_pairing",
            "isr-cleanup": "epic4_category2_exit_cleanup",
            "nested-interrupts": "epic4_nested_interrupts",
            "counter-types": "epic4_public_counter_types",
            "nonstatus-errors": "epic4_nonstatus_service_errors",
            "source-repetition": "epic4_interrupt_source_repetition",
            "entry-bodies": "epic4_osek_entry_bodies",
            "calling-context": "epic4_standard_calling_context",
            "idle-state": "epic4_standard_idle_and_started_state",
        }
        name = names[args.suite]
        suffix = "; 6 header orders" if args.suite == "counter-types" else ""
        print(f"{name} PASS: {len(observations)} native vectors{suffix}")


if __name__ == "__main__":
    main()
