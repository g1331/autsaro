"""Run the Linux native C tests and render GCC line and branch coverage."""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import defaultdict
from functools import cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
NATIVE_TESTS = (
    "runtime_contracts::",
    "host::",
    "integration::generated_c::generated_rte_com_header_links_independent_handle_callbacks",
    # On Linux these protocol consumers run the shared, sealed HostBatch image.
    "integration::protocol::single_component_host_batch_commits_refuses_and_closes_on_faults",
    "integration::protocol::single_component_receive_timers_use_complete_periods",
)


def run_tests(scratch: Path, profiles: Path, errors: Path) -> int:
    native = scratch / "native"
    native.mkdir()
    env = {
        **os.environ,
        "AUTOSAR_CC": os.environ.get("AUTOSAR_CC", "/usr/bin/gcc-13"),
        "AUTOSAR_OBJDUMP": os.environ.get("AUTOSAR_OBJDUMP", "/usr/bin/objdump"),
        "AUTOSAR_NM": os.environ.get("AUTOSAR_NM", "/usr/bin/nm"),
        "AUTOSAR_GIT": os.environ.get("AUTOSAR_GIT", "/usr/bin/git"),
        "AUTOSAR_PYTHON": sys.executable,
        "AUTOSAR_C_COVERAGE": "1",
        "AUTOSAR_OS_OUTPUT_DIRECTORY": str(scratch / "os"),
        "AUTOSAR_OS_SHARD": "0/1",
        "TMPDIR": str(native),
        "GCOV_PREFIX": str(profiles),
        "GCOV_PREFIX_STRIP": str(len(scratch.parts) - 1),
        "GCOV_ERROR_FILE": str(errors),
    }
    env.pop("AUTOSAR_SANITIZE", None)
    cargo = ["cargo", "test", "--locked", "-p", "autosar-config-core",
             "--features", "native-tests", "--test", "native", "--no-fail-fast"]
    native_status = subprocess.run([*cargo, "--", *NATIVE_TESTS], cwd=ROOT, env=env, check=False)
    os_status = subprocess.run(
        [sys.executable, "-B", "-m", "unittest", "discover", "-s", "tests/python/native",
         "-p", "test_os_suites.py"],
        cwd=ROOT, env=env, check=False,
    )
    return native_status.returncode or os_status.returncode


def restore_profiles(scratch: Path, profiles: Path) -> None:
    for data in profiles.rglob("*.gcda"):
        destination = scratch / data.relative_to(profiles)
        # A test can discard an intermediate build. Its counters have no usable notes.
        if destination.with_suffix(".gcno").is_file():
            data.replace(destination)


def gcovr(*args: str) -> str:
    result = subprocess.run(
        [sys.executable, "-m", "gcovr", *args], cwd=ROOT, text=True, stderr=subprocess.PIPE,
        check=False,
    )
    print(result.stderr, end="", file=sys.stderr)
    result.check_returncode()
    return result.stderr


def runtime_path(relative: Path) -> Path:
    _, family, *rest = relative.parts
    if family == "bsw":
        return Path("runtime", "bsw", relative.stem.split("_", 1)[0], *rest)
    if family == "os" and rest[:2] == ["src", "host"]:
        return Path("runtime", "host", "os", *rest[2:])
    return relative


@cache
def original_sources(project: Path) -> dict[str, str]:
    metadata = json.loads((project / "meta/integration.json").read_text(encoding="utf-8"))
    return {delivered: original for original, delivered in metadata["originalBswSources"].items()}


def report_source(source: Path, scratch: Path) -> Path | None:
    if source.is_relative_to(ROOT / "runtime"):
        return runtime_path(source.relative_to(ROOT))
    if not source.is_relative_to(scratch / "native"):
        return None
    project = next((parent for parent in source.parents
                    if (parent / "meta/integration.json").is_file()), None)
    if project is None:
        return None
    relative = source.relative_to(project)
    if original := original_sources(project).get(relative.as_posix()):
        original = Path(original)
        # Only exact delivered copies contribute to the repository source totals.
        if original.parts[0] == "runtime" and source.read_bytes() == (ROOT / original).read_bytes():
            return runtime_path(original)
        return None
    if relative.parts[0] != "generated":
        return None
    digest = hashlib.sha256(source.read_bytes()).hexdigest()
    return Path("generated", *relative.parts[1:-1], relative.stem, digest, relative.name)


def coverage_builds(scratch: Path) -> list[list[str]]:
    """Keep GCC's per-executable note sets separate during collection."""
    builds = defaultdict(list)
    for note in sorted(scratch.rglob("*.gcno")):
        # GCC prefixes a linked executable's notes with its output name.
        # Prefer the longest prefix when one test produces multiple executables.
        names = note.stem.split("-")
        candidates = (note.with_name("-".join(names[:end])) for end in range(len(names), 0, -1))
        output = next(
            (candidate for candidate in candidates if candidate.is_file() and os.access(candidate, os.X_OK)),
            note,
        )
        data = note.with_suffix(".gcda")
        builds[output].append(str(data if data.is_file() else note))
    return list(builds.values())


def collect(scratch: Path, source_root: Path, source_filter: str,
            inputs: list[str]) -> tuple[dict, list[str]]:
    trace_path = scratch / "raw.json"
    diagnostics = gcovr(
        "--root", str(source_root), "--gcov-executable", "/usr/bin/gcov-13",
        "--gcov-object-directory", str(scratch), "--filter", source_filter,
        "--gcov-ignore-parse-errors", "negative_hits.warn",
        "--merge-mode-functions", "separate", "--json", str(trace_path), *inputs,
    )
    negative_counts = [line for line in diagnostics.splitlines() if "Ignoring negative hits in:" in line]
    return json.loads(trace_path.read_text(encoding="utf-8")), negative_counts


def branch_layout(entry: dict) -> str:
    # A branch number is local to one GCC build. Even unchanged source can have
    # a different branch layout when a fixture selects other defines or flags.
    layout = [
        (line["line_number"], line.get("function_name"),
         [{key: value for key, value in branch.items() if key != "count"}
          for branch in line["branches"]])
        for line in entry["lines"]
    ]
    return hashlib.sha256(json.dumps(layout, sort_keys=True).encode()).hexdigest()


def render(scratch: Path, output: Path) -> None:
    runtime = re.escape(str(ROOT / "runtime"))
    native = re.escape(str(scratch / "native"))
    records = defaultdict(lambda: defaultdict(list))
    executed = set()
    negative_counts = set()
    sealed_host_batch_ran = False
    format_version = None
    # gcovr needs source paths under its report root. The checkout and the
    # temporary ECU projects have different roots until the paths are mapped.
    for inputs in coverage_builds(scratch):
        roots = [(ROOT, rf"{runtime}/(?:bsw|ecu|host|os)/(?:src|include)/")]
        if Path(inputs[0]).is_relative_to(scratch / "native"):
            roots.append((scratch, rf"{native}/.*/(?:bsw|generated|integration|os)/(?:src|include)/"))
        for root, source_filter in roots:
            trace, warnings = collect(scratch, root, source_filter, inputs)
            negative_counts.update(warnings)
            format_version = trace["gcovr/format_version"]
            for entry in trace["files"]:
                source = (root / entry["file"]).resolve()
                if relative := report_source(source, scratch):
                    records[relative][branch_layout(entry)].append((source, entry))
                    if any(line["count"] for line in entry["lines"]):
                        executed.add(relative.as_posix())
                        if relative.name == "Ecu_HostBatch.c" and source.is_relative_to(scratch):
                            sealed_host_batch_ran = True
    sources = scratch / "sources"
    files = []
    for relative, layouts in records.items():
        for digest, entries in layouts.items():
            # Preserve separate branch denominators for incompatible layouts.
            report_path = relative if len(layouts) == 1 else (
                relative.parent / relative.stem / digest[:16] / relative.name
            )
            destination = sources / report_path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(entries[0][0], destination)
            for _, entry in entries:
                entry["file"] = report_path.as_posix()
                files.append(entry)
    if format_version is None:
        raise ValueError("No GCC coverage notes were produced")
    merged = scratch / "merged.json"
    merged.write_text(
        json.dumps({"gcovr/format_version": format_version, "files": files}), encoding="utf-8",
    )
    gcovr(
        "--root", str(sources), "--merge-mode-functions", "separate",
        "--json-add-tracefile", str(merged),
        "--html-nested", str(output / "index.html"), "--json", str(output / "coverage.json"),
    )
    if negative_counts:
        (output / "gcov-warnings.txt").write_text(
            "\n".join(sorted(negative_counts)) + "\n", encoding="utf-8",
        )
    for prefix in ("runtime/bsw/", "runtime/os/", "runtime/host/", "generated/",
                   "runtime/ecu/src/"):
        if not any(path.startswith(prefix) for path in executed):
            raise ValueError(f"No executed C coverage recorded for {prefix}")
    if not sealed_host_batch_ran:
        raise ValueError("No executed C coverage recorded for the sealed HostBatch")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "build/coverage/c")
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.error("C coverage requires the Linux native GCC toolchain")
    output = args.output.resolve()
    output.mkdir(parents=True)
    with tempfile.TemporaryDirectory(prefix="autsaro-cov-") as directory:
        scratch = Path(directory).resolve() / "build"
        scratch.mkdir()
        profiles = scratch.parent / "profiles"
        errors = scratch.parent / "gcov-errors.log"
        status = run_tests(scratch, profiles, errors)
        if errors.exists():
            print(errors.read_text(encoding="utf-8"), file=sys.stderr)
            status = status or 1
        restore_profiles(scratch, profiles)
        render(scratch, output)
        return status


if __name__ == "__main__":
    raise SystemExit(main())
