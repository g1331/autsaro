"""Consume builtin-only native deliveries in a separate, explicitly tooled phase."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path

from ecu_tools.process import ProcessSpec, run_bounded

# Independent expected automotive values; never calculated by the generator or
# read back from its profile.txt, packing implementation, or exported test data.
HOST_VECTORS = {
    "NorthSensor-source": {
        "input": "G 0\nT 10\nM 2\nT 20\nM 1\nT 30\nS 0 16909060\nT 40\nG 0\n",
        "expected": ["V 0 305419896 1", "X 865 4 78563412", "X 865 4 78563412", "X 865 4 04030201", "V 0 16909060 1"],
    },
    "SouthActuator-source": {
        "input": "G 0\nT 40\nM 2\nT 80\nM 1\nT 120\nS 0 305419896\nT 160\nG 0\n",
        "expected": ["V 0 16909060 1", "X 913 4 04030201", "X 913 4 04030201", "X 913 4 78563412", "V 0 305419896 1"],
    },
}


def hashes(directory: Path) -> dict[str, str]:
    result = {}
    for member in directory.rglob("*"):
        if member.is_symlink():
            raise RuntimeError(f"Consumer refuses linked source member: {member}")
        if member.is_file():
            result[member.relative_to(directory).as_posix()] = hashlib.sha256(member.read_bytes()).hexdigest()
    return result


def consume(results_path: Path, output: Path, legacy_reference: Path | None) -> int:
    results = json.loads(results_path.read_text(encoding="utf-8"))
    if results.get("format") != "autosar-builtin-native-acceptance-v1":
        raise RuntimeError("Consumer requires actual native builtin acceptance results")
    required = {"NorthSensor-source", "SouthActuator-source", "GatewayReference-source"}
    names = [item["name"] for item in results.get("packages", [])]
    if not required <= set(names) or len(names) != len(set(names)):
        raise RuntimeError("Consumer requires both distinct CAN deliveries and the standard delivery, without duplicate names")
    if output.exists():
        raise RuntimeError("Consumer scratch must be a new directory")
    output.mkdir(mode=0o700, parents=True)
    python = Path(os.environ["AUTOSAR_PYTHON"]).resolve(strict=True)
    report = {
        "format": "autosar-builtin-independent-consumer-v1", "status": "failed",
        "configurationResults": str(results_path), "configurationStatus": results.get("status"),
        "checks": [], "commands": [],
        "legacyV1": "not_run", "error": None,
    }
    originals = {}
    def execute(argv: list[str], seconds: int, scope: str, stdin_file: Path | None = None):
        result = run_bounded(ProcessSpec.seconds(
            argv, output, seconds, output, scope, stdin_file=stdin_file,
        ))
        report["commands"].append({
            "argv": argv, "scope": scope, "processStatus": result.status,
            "exitCode": result.exit_code, "stdout": str(result.stdout),
            "stderr": str(result.stderr), "stdin": str(stdin_file) if stdin_file else None,
        })
        if not result.success:
            raise RuntimeError(f"Independent {scope} failed: status={result.status}, exit={result.exit_code}, stdout={result.stdout}, stderr={result.stderr}")
        return result
    try:
        for item in results["packages"]:
            source = Path(item["directory"]).resolve(strict=True)
            if item.get("source") != "passed":
                raise RuntimeError(f"Package source was not accepted: {source}")
            originals[str(source)] = hashes(source)
            copied = output / "sources" / item["name"]
            shutil.copytree(source, copied)
            before = hashes(copied)
            tool = copied / "tools/ecu-tool.py"
            if item["profile"] == "ecu":
                expected = {"periodMs": 20, "receiveCanId": 1104, "transmitCanId": 1105}
                actual = json.loads((copied / "verification/inputs.json").read_text(encoding="utf-8"))
                for name, value in expected.items():
                    if actual.get(name) != value:
                        raise AssertionError(f"Actual exported input differs from independent expectation: {name}={actual.get(name)}")
                command = [str(python), "-I", "-S", str(tool), "verify", "--project", str(copied), "--build-directory", str(output / "builds" / item["name"])]
                result = execute(command, 900, f"consume-{item['name']}")
                expected_protocols = ("protocol=echo PASS", "protocol=ncr-recovery PASS", "protocol=malformed PASS", "ECU_HANDOFF_VERIFY PASS")
                observed = result.stdout.read_text(encoding="utf-8")
                if not all(marker in observed for marker in expected_protocols):
                    raise AssertionError(f"Independent production protocol results are incomplete: {observed}")
                if item["name"] == "GatewayUserApplication-source":
                    suffix = ".exe" if sys.platform == "win32" else ""
                    binary = output / "builds" / item["name"] / f"ecu_host_batch{suffix}"
                    stdin = output / "user-application-high-bit.stdin"
                    stdin.write_text(
                        "BEGIN 0\nCOMMIT\nBEGIN 20\nRX 1104 4 275941b1\nCOMMIT\nBEGIN 40\nCOMMIT\n",
                        encoding="ascii", newline="\n",
                    )
                    result = execute([str(binary)], 60, "user-application-low31-behavior", stdin)
                    observed = result.stdout.read_text(encoding="utf-8").splitlines()
                    if any(line.startswith(("REJECT", "COMMIT_ERROR")) for line in observed):
                        raise AssertionError(f"Actual user application batch was refused: {observed}")
                    frames = [line for line in observed if " id=1105 dlc=4 data=" in line]
                    if not any(line.endswith("data=27594131") for line in frames) or any(line.endswith("data=275941b1") for line in frames):
                        raise AssertionError(f"Compiled live user logic did not mask the high bit: {frames}")
                    report["checks"].append({
                        "name": "live-user-application-low31", "status": "passed",
                        "inputValue": "0xb1415927", "expectedValue": "0x31415927",
                        "stdout": str(result.stdout), "stderr": str(result.stderr),
                        "exitCode": result.exit_code,
                    })
                    command = [str(binary)]
            elif item["profile"] == "host":
                vector = HOST_VECTORS[item["name"]]
                build_directory = output / "builds" / item["name"]
                command = [str(python), "-I", "-S", str(tool), "build", "--project", str(copied), "--output", str(build_directory), "--mode", "host"]
                execute(command, 900, f"build-{item['name']}")
                suffix = ".exe" if sys.platform == "win32" else ""
                binary = build_directory / f"ecu_host{suffix}"
                stdin = output / f"{item['name']}.stdin"
                stdin.write_text(vector["input"], encoding="ascii", newline="\n")
                command = [str(binary)]
                result = execute(command, 60, f"behavior-{item['name']}", stdin)
                observed = result.stdout.read_text(encoding="ascii").splitlines()
                if observed != vector["expected"]:
                    raise AssertionError(f"Independent legacy CAN/BUS_OFF/recovery vector differs: expected={vector['expected']!r}; actual={observed!r}")
            else:
                raise RuntimeError(f"Unsupported actual package profile: {item['profile']}")
            if hashes(copied) != before:
                raise AssertionError("Independent build/behavior changed immutable source package")
            report["checks"].append({
                "name": item["name"], "status": "passed", "profile": item["profile"],
                "source": "unchanged", "build": "passed", "behavior": "passed",
                "argv": command, "processStatus": result.status, "exitCode": result.exit_code,
                "stdout": str(result.stdout), "stderr": str(result.stderr),
                "movedSource": str(copied), "sourceUnchanged": True,
            })
        if legacy_reference is not None:
            source = legacy_reference.resolve(strict=True)
            originals[str(source)] = hashes(source)
            copied = output / "sources" / "legacy-v1-reference"
            shutil.copytree(source, copied)
            command = [str(python), "-I", "-S", str(copied / "tools/ecu-tool.py"), "verify", "--project", str(copied), "--build-directory", str(output / "builds/legacy-v1"), "--report-path", str(output / "legacy-v1-result.json")]
            result = execute(command, 900, "legacy-v1-consumer")
            report["legacyV1"] = "passed"
            report["checks"].append({"name": "legacy-v1", "status": "passed", "exitCode": result.exit_code, "stdout": str(result.stdout)})
        for source, before in originals.items():
            if hashes(Path(source)) != before:
                raise AssertionError(f"Original delivery changed during independent consumption: {source}")
        report["status"] = "passed" if legacy_reference is not None else "new-packages-passed"
        return 0
    except (OSError, ValueError, RuntimeError, AssertionError, KeyError) as error:
        report["error"] = str(error)
        raise
    finally:
        (output / "independent-consumer.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--legacy-reference", type=Path)
    args = parser.parse_args()
    return consume(args.results, args.output, args.legacy_reference)


if __name__ == "__main__":
    raise SystemExit(main())
