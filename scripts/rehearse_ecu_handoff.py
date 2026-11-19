"""Build and run a received ECU package with only Python and its pinned native tools."""

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

# Match the independent component-pipeline receiver vector in test_offline_delivery.py.
PIPELINE_TRACE = (
    b"BEGIN 0\nRX 800 4 78563412\nRX 1792 8 0322123400000000\nCOMMIT\n"
    b"BEGIN 1\nCOMMIT\nBEGIN 10\nRX 1792 8 0322123400000000\nCOMMIT\n"
    b"BEGIN 10\nCOMMIT\nBEGIN 20\nRX 1792 8 0322123400000000\nCOMMIT\n"
)
PIPELINE_BATCHES = [
    (0, 2, []),
    (1, 0, [("1", "1800", "8", "0762123400000000")]),
    (10, 1, [("10", "801", "4", "78563412"), ("10", "1800", "8", "0762123412345678")]),
    (10, 0, []),
    (20, 1, [("20", "801", "4", "78563412"), ("20", "1800", "8", "0762123412345678")]),
]


SAMPLES = ("standard-ecu", "user-application", "component-pipeline", "no-diagnostic")
SEAL = Path("meta/files.sha256")
COMMIT = re.compile(
    r"COMMIT_OK batch=(\d+) epoch=(\d+) sequence=(\d+) input_first=(\d+) inputs=(\d+)"
    r" status=0 input_status=0 transport_status=0 transport_epoch=0 transport_count=0"
)
FRAME = re.compile(
    r"OUT epoch=(\d+) sequence=\d+ ticket=\d+ pdu=\d+ id=(\d+) dlc=(\d+) data=([0-9a-f]+)"
)


def digest(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def regression_seals(root: Path) -> dict[str, str | dict[str, str]]:
    actual = {}
    for sample in SAMPLES:
        name = (Path(sample) / SEAL).as_posix()
        try:
            actual[name] = digest(root / name)
        except OSError as error:
            actual[name] = {"error": str(error)}
    return actual


def check_seals(actual: dict[str, str | dict[str, str]], expected: object) -> None:
    if not isinstance(expected, dict) or actual.keys() != expected.keys():
        raise ValueError("trusted seal inventory must contain exactly the four fixed paths")
    mismatches = []
    for name, observed in actual.items():
        if isinstance(observed, dict):
            mismatches.append(f"{name}: could not read seal: {observed['error']}")
        elif observed != expected[name]:
            mismatches.append(f"{name}: expected {expected[name]}, found {observed}")
    if mismatches:
        raise ValueError("\n".join(mismatches))


def receiver_environment(receiver: Path) -> dict[str, str]:
    tools = {}
    for name in ("AUTOSAR_CC", "AUTOSAR_OBJDUMP", "AUTOSAR_GIT"):
        tool = Path(os.environ[name])
        if not tool.is_absolute():
            raise ValueError(f"{name} must be an absolute tool path")
        tools[name] = str(tool.resolve(strict=True))
    if os.name == "nt":
        system = Path(os.environ["SYSTEMROOT"])
        directories = [system / "System32", system]
        kept = ("SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT", "TEMP", "TMP", "PROCESSOR_ARCHITECTURE")
    else:
        directories = [Path(path) for path in os.defpath.split(os.pathsep) if path]
        kept = ("TMPDIR", "LANG")
    directories += [Path(tool).parent for tool in tools.values()]
    directories.append(receiver.parent)
    env = {name: os.environ[name] for name in kept if name in os.environ}
    env["PATH"] = os.pathsep.join(dict.fromkeys(map(str, directories)))
    return {**env, **tools}


def check_batch(output: str, *, pipeline: bool) -> None:
    lines = output.splitlines()
    if not lines or lines[0] != "READY HostBatchV1":
        raise ValueError("HostBatch did not become ready")
    expected = PIPELINE_BATCHES if pipeline else [(1, 0, [])]
    frames = []
    index = 0
    previous = 0
    closed = False
    for line in lines[1:]:
        kind = line.split(" ", 1)[0]
        if kind in {"REJECT", "COMMIT_ERROR"}:
            raise ValueError("HostBatch rejected a legal batch")
        if kind == "lifecycle=Closed":
            if index != len(expected) or closed:
                raise ValueError("HostBatch closed outside the end of the final batch")
            if not line.startswith("lifecycle=Closed state=Ready reason=0 "):
                raise ValueError("HostBatch did not close in the ready state")
            closed = True
        if kind == "OUT":
            if index == len(expected):
                raise ValueError(f"HostBatch frame after the final receipt: {line}")
            frame = FRAME.fullmatch(line)
            if not frame:
                raise ValueError(f"malformed HostBatch frame: {line}")
            frames.append(frame.groups())
        if kind != "COMMIT_OK":
            continue
        if index == len(expected):
            raise ValueError(f"unexpected HostBatch receipt: {line}")
        epoch, inputs, wanted_frames = expected[index]
        match = COMMIT.fullmatch(line)
        if not match:
            raise ValueError(f"malformed or unsuccessful HostBatch receipt: {line}")
        batch, received_epoch, sequence, first_input, received_inputs = map(int, match.groups())
        input_order = previous < first_input <= sequence - inputs if inputs else first_input == 0
        if ((batch, received_epoch, received_inputs) != (index + 1, epoch, inputs)
                or sequence <= previous or not input_order):
            raise ValueError(f"unexpected HostBatch receipt: {line}")
        if pipeline and frames != wanted_frames:
            raise ValueError(f"unexpected component-pipeline frames for batch {batch}: {frames}")
        frames.clear()
        index += 1
        previous = sequence
    if index != len(expected):
        raise ValueError(f"unexpected HostBatch receipt count: {index}")
    if not closed:
        raise ValueError("HostBatch did not close in the ready state")


def run(command: list[str], log: Path, env: dict[str, str], *, stdin: bytes = b"") -> str:
    print(f"{log.name}: {subprocess.list2cmdline(command)}", flush=True)
    with log.open("wb") as output:
        try:
            subprocess.run(
                command, cwd=log.parent, env=env, input=stdin, stdout=output, stderr=subprocess.STDOUT,
                check=True, timeout=300,
            )
        except subprocess.SubprocessError as error:
            raise ValueError(f"{error}; see {log}") from error
    return log.read_text(encoding="utf-8", errors="replace")


def rehearse(args: argparse.Namespace) -> None:
    package = args.package.resolve()
    results = args.results.resolve()
    entry = package / "tools" / "ecu-tool.py"
    if not entry.is_file():
        raise ValueError(f"sealed package entry is missing: {entry}")
    if results.is_relative_to(package) or results.exists():
        raise ValueError("results must be an unused directory outside the package")

    before = digest(package / SEAL)
    edit = None
    if args.edit_receipt:
        edit = json.loads(args.edit_receipt.read_text(encoding="utf-8"))
        if Path(edit["package"]).resolve() != package or edit["sealSha256"] != before:
            raise ValueError("received package differs from the recorded desktop export")
    receiver = Path(sys.executable).absolute()
    env = receiver_environment(receiver)
    results.mkdir()
    (results / "identity.json").write_text(json.dumps({
        "package": str(package),
        "sealSha256": before,
        "edit": edit,
        "python": str(receiver),
        "resolvedPython": str(receiver.resolve(strict=True)),
        "pythonVersion": sys.version,
        "path": env["PATH"],
    }, indent=2) + "\n", encoding="utf-8")
    tool = [str(receiver), "-I", "-S", str(entry)]
    for mode in ("host-batch", "probe"):
        run(
            [*tool, "build", "--project", str(package), "--output", str(results / mode),
             "--mode", mode],
            results / f"build-{mode}.log", env,
        )

    suffix = ".exe" if os.name == "nt" else ""
    probe = run([str(results / "probe" / f"ecu_probe{suffix}")], results / "probe.log", env)
    if "ecu_probe completed=20" not in probe.splitlines():
        raise ValueError(f"probe did not complete 20 ticks; see {results / 'probe.log'}")
    batch_log = results / "host-batch.log"
    batch = run(
        [str(results / "host-batch" / f"ecu_host_batch{suffix}")],
        batch_log, env, stdin=PIPELINE_TRACE if args.pipeline else b"BEGIN 1\nCOMMIT\n",
    )
    try:
        check_batch(batch, pipeline=args.pipeline)
    except ValueError as error:
        raise ValueError(f"{error}; see {batch_log}") from error

    if args.verify:
        run(
            [*tool, "verify", "--project", str(package), "--build-directory", str(results / "verify")],
            results / "verify.log", env,
        )
    if digest(package / SEAL) != before:
        raise ValueError("package seal changed during the rehearsal")
    print(f"PASS {package.name}: independent HostBatch and probe")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    native = commands.add_parser("run", help="build and run one sealed project")
    native.add_argument("package", type=Path, help="received sealed project directory")
    native.add_argument("results", type=Path, help="unused directory outside the package")
    native.add_argument("--edit-receipt", type=Path, help="bind to an actual desktop export")
    behavior = native.add_mutually_exclusive_group()
    behavior.add_argument("--verify", action="store_true", help="run the standard ECU behavior oracle")
    behavior.add_argument("--pipeline", action="store_true", help="check the component-pipeline frames")
    seals = commands.add_parser("seals", help="record or compare the four fixed regression seals")
    seals.add_argument("root", type=Path, help="directory containing the four sample directories")
    seals.add_argument("--expected", type=Path, help="trusted sender inventory or prior receipt")
    args = parser.parse_args()
    try:
        if args.command == "run":
            rehearse(args)
        else:
            actual = regression_seals(args.root)
            print(json.dumps(actual, indent=2))
            expected = actual
            if args.expected:
                expected = json.loads(args.expected.read_text(encoding="utf-8"))
            check_seals(actual, expected)
    except (ValueError, OSError, KeyError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
