"""Fixed dual-host reference vectors, rebuilt and observed outside the package."""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

from ecu_tools.build import build, refuse_links, sealed_sources
from ecu_tools.process import OwnedProcess, ProcessSpec

CASES = (
    ("Alpha CAN transmit and receive", "Alpha"),
    ("Beta CAN receive and transmit", "Beta"),
    ("Alpha diagnostic rejection then recovery", "Alpha"),
)


def verify(project: Path, output: Path, report_path: Path | None) -> None:
    refuse_links(project)
    project = project.resolve(strict=True)
    refuse_links(output)
    output = output.absolute().resolve()
    if output == project or output.is_relative_to(project) or project.is_relative_to(output):
        raise ValueError("Reference build directory must be outside the source package")
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise ValueError("Reference build directory must be new or empty")
    if report_path is not None:
        refuse_links(report_path)
        report_path = report_path.absolute().resolve()
        if report_path == project or report_path.is_relative_to(project) or report_path.exists():
            raise ValueError("Report path must be new and outside the reference package")
        if report_path == output or report_path.is_relative_to(output):
            raise ValueError("Report path must be outside the private build directory")
    report = {
        "format": "autosar-host-reference-report-v1",
        "status": "failed", "python": sys.version.split()[0], "checks": [],
        "buildDirectory": str(output), "error": None,
    }

    def checked(name: str, detail: str) -> None:
        report["checks"].append({"name": name, "status": "passed", "detail": detail})

    try:
        _, target = sealed_sources(project)
        if target.get("profile") != "host-reference" or target.get("members") != ["Alpha", "Beta"]:
            raise ValueError("Unsupported fixed dual-host reference package")
        report["target"] = target["target"]
        for ecu in ("Alpha", "Beta"):
            _, member = sealed_sources(project / ecu)
            if member.get("profile") != "host" or member["target"] != target["target"]:
                raise ValueError(f"Reference member target/profile differs: {ecu}")
        checked("package integrity", "Root and both ECU SHA-256 closures match")
        vectors = json.loads((project / "vectors.json").read_text(encoding="utf-8"))
        cases = vectors.get("cases")
        timeout = vectors.get("timeoutMs")
        if (vectors.get("format") != "autosar-host-reference-v1"
                or not isinstance(cases, list) or len(cases) != len(CASES)
                or type(timeout) is not int or not 1 <= timeout <= 60000):
            raise ValueError("Unsupported reference vectors or execution deadline")
        for case, (name, ecu) in zip(cases, CASES, strict=True):
            if (case.get("name") != name or case.get("ecu") != ecu
                    or not isinstance(case.get("input"), str) or not case["input"].strip()
                    or not isinstance(case.get("expected"), list) or not case["expected"]
                    or any(not isinstance(line, str) or not line for line in case["expected"])):
                raise ValueError(f"Invalid fixed reference case: {name}")
            case["input"].encode("ascii")
        output.mkdir(parents=True, mode=0o700, exist_ok=True)
        logs = output / "logs"
        logs.mkdir(mode=0o700)
        binaries = {}
        for ecu in ("Alpha", "Beta"):
            binaries[ecu] = build(project / ecu, output / ecu, "host")
            checked(f"{ecu} build", "Pinned native C99 build, separate source and output")
        for index, case in enumerate(cases):
            stdin = output / f"case-{index}.stdin"
            stdin.write_bytes(case["input"].encode("ascii"))
            spec = ProcessSpec(
                (str(binaries[case["ecu"]]),), logs, None,
                time.monotonic_ns() + timeout * 1_000_000,
                logs, f"reference-case-{index}", stdin,
            )
            result = OwnedProcess(spec).wait()
            if not result.success:
                report["processStatus"] = result.status
                report["exitCode"] = result.exit_code
                raise RuntimeError(
                    f"{case['name']}: status={result.status} exit={result.exit_code}; "
                    f"stdout={result.stdout} stderr={result.stderr}"
                )
            actual = result.stdout.read_text(encoding="ascii").splitlines()
            if actual != case["expected"]:
                raise ValueError(f"{case['name']}: expected {case['expected']!r}, actual {actual!r}")
            checked(case["name"], "; ".join(actual))
        sealed_sources(project)
        report["status"] = "passed"
        print(f"REFERENCE_VERIFY PASS target={target['target']} cases={len(CASES)}")
    except (OSError, ValueError, RuntimeError, AssertionError) as error:
        report["error"] = str(error)
        raise
    finally:
        if report_path is not None:
            report_path.parent.mkdir(parents=True, exist_ok=True)
            refuse_links(report_path)
            with report_path.open("x", encoding="utf-8") as stream:
                json.dump(report, stream, indent=2)
                stream.write("\n")
            print(f"Reference report: {report_path}")
