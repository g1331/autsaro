"""Per-image process timing from the ETW lifecycle trace, read against a same-job probe.

A slow hosted runner makes every process slow, while a heavier product or fixture makes only
its own processes slow. The `python -I -S -c pass` probe that `windows_etw` times before the
tests start (and again at stop) does not depend on the product, so it separates the two.
"""

from __future__ import annotations

import datetime
import json
import math
import os
import statistics
import sys
from collections import defaultdict
from pathlib import Path

BASELINE = Path(__file__).with_name("windows_timing_baseline.json")
KERNEL_PROCESS = "{22fb2cd6-0e7b-422b-a0c7-2fad1fd0e716}"
# Healthy runs of one shard differ by up to 1.6x in a per-image median and by 1.4x in the
# probe (runs 44539209173 to 44558822646); the slow run of #591 was 3.8x on the probe.
SLOW_RATIO = 2.0
MIN_SAMPLES = 20
# Baselines cover short-lived tool processes; roots such as python.exe live for whole tests.
SHORT_LIVED_P95_S = 5.0
SHOWN_IMAGES = 12


def seconds_between(created: str, exited: str) -> float:
    """Kernel FILETIME payload of one process; both ends share the same clock."""

    def parse(value: str) -> datetime.datetime:
        return datetime.datetime.fromisoformat(value.strip())

    return (parse(exited) - parse(created)).total_seconds()


def process_records(path: Path) -> tuple[list[tuple[str, float, int]], float | None]:
    """(image, lifetime in s, CPU cycles) of every process that exited, and the CPU MHz."""
    images: dict[tuple[str, str], str] = {}
    exits: dict[tuple[str, str], tuple[float, int]] = {}
    mhz = None
    with path.open(encoding="utf-8") as source:
        for line in source:
            event = json.loads(line)
            fields = event["fields"]
            if mhz is None and "CPUSpeed" in fields:
                mhz = float(fields["CPUSpeed"])
            if event["provider"].get("Guid") != KERNEL_PROCESS:
                continue
            if event["event_id"] not in {"1", "2"}:
                continue
            key = (fields["ProcessID"].strip(), fields["CreateTime"].strip())
            if event["event_id"] == "1":
                images[key] = fields["ImageName"].rsplit("\\", 1)[-1].lower()
            else:
                exits[key] = (
                    seconds_between(key[1], fields["ExitTime"]),
                    int(fields["CPUCycleCount"]),
                )
    return [
        (images[key], lifetime, cycles)
        for key, (lifetime, cycles) in exits.items()
        if key in images
    ], mhz


def percentile(ordered: list[float], fraction: float) -> float:
    return ordered[max(0, math.ceil(fraction * len(ordered)) - 1)]


def image_statistics(
    records: list[tuple[str, float, int]], mhz: float | None
) -> dict[str, dict]:
    lifetimes: dict[str, list[float]] = defaultdict(list)
    cpu: dict[str, list[float]] = defaultdict(list)
    for image, lifetime, cycles in records:
        lifetimes[image].append(lifetime)
        if mhz:
            cpu[image].append(cycles / (mhz * 1_000_000))
    result = {}
    for image, values in lifetimes.items():
        ordered = sorted(values)
        result[image] = {
            "count": len(values),
            "total_s": sum(values),
            "wall_median_ms": statistics.median(values) * 1000,
            "wall_p95_ms": percentile(ordered, 0.95) * 1000,
            "wall_max_s": ordered[-1],
            "cpu_median_ms": statistics.median(cpu[image]) * 1000 if mhz else None,
        }
    return result


def read_probe(folder: Path) -> dict[str, float | None]:
    """Median of the 12 quiet-time probe launches before the tests and after them, in ms."""

    def median_ms(path: Path, key: str) -> float | None:
        if not path.exists():
            return None
        return (
            statistics.median(json.loads(path.read_text(encoding="utf-8"))[key]) * 1000
        )

    return {
        "start_ms": median_ms(folder / "overhead.json", "without_etw_s"),
        "end_ms": median_ms(folder / "reference-end.json", "samples_s"),
    }


def baseline_entry(label: str | None, baseline: dict) -> dict | None:
    return baseline.get("labels", {}).get(label) if label else None


def reference_probe_ms(label: str | None, baseline: dict) -> float | None:
    """The shard's own baseline, else the median over every recorded shard."""
    entry = baseline_entry(label, baseline)
    if entry:
        return entry["probe_ms"]
    known = [value["probe_ms"] for value in baseline.get("labels", {}).values()]
    return statistics.median(known) if known else None


def compare(
    stats: dict[str, dict], probe: dict, label: str | None, baseline: dict
) -> dict:
    expected = reference_probe_ms(label, baseline)
    observed = [value for value in probe.values() if value is not None]
    probe_ratio = max(observed) / expected if observed and expected else None
    entry = baseline_entry(label, baseline)
    ratios = {}
    for image, reference in (entry["images"] if entry else {}).items():
        current = stats.get(image)
        if current and current["count"] >= MIN_SAMPLES:
            ratios[image] = current["wall_median_ms"] / reference["wall_ms"]
    process_ratio = statistics.median(ratios.values()) if ratios else None
    if probe_ratio is not None and probe_ratio >= SLOW_RATIO:
        verdict = "runner-slow"
    elif process_ratio is not None and process_ratio >= SLOW_RATIO:
        verdict = "processes-slow-probe-normal"
    elif probe_ratio is None and process_ratio is None:
        verdict = "no-reference"
    else:
        verdict = "normal"
    return {
        "label": label,
        "verdict": verdict,
        "probe_ratio": probe_ratio,
        "probe_baseline_ms": expected,
        "process_ratio": process_ratio,
        "image_ratios": ratios,
    }


VERDICTS = {
    "runner-slow": "runner slow: the reference probe is {probe} the baseline",
    "processes-slow-probe-normal": (
        "reference probe normal but baselined processes are {process} slower: "
        "the product or fixture does more or heavier work"
    ),
    "normal": "within the healthy range of this shard's baseline",
    "no-reference": "no baseline for this job; read the absolute figures",
}


def number(value: float | None, digits: int = 0) -> str:
    return "-" if value is None else f"{value:.{digits}f}"


def ratio_text(value: float | None) -> str:
    return "-" if value is None else f"{value:.1f}x"


def render(
    stats: dict[str, dict], probe: dict, comparison: dict, baseline: dict
) -> str:
    entry = baseline_entry(comparison["label"], baseline)
    reference = entry["images"] if entry else {}
    verdict = VERDICTS[comparison["verdict"]].format(
        probe=ratio_text(comparison["probe_ratio"]),
        process=ratio_text(comparison["process_ratio"]),
    )
    lines = [
        f"### Windows process timing ({comparison['label'] or 'no label'})",
        "",
        f"Verdict: {verdict}.",
        "",
        (
            "Reference probe `python -I -S -c pass`, median of 12 launches: "
            f"start {number(probe['start_ms'], 1)} ms, end {number(probe['end_ms'], 1)} ms, "
            f"baseline {number(comparison['probe_baseline_ms'], 1)} ms "
            f"(slow at {SLOW_RATIO:.0f}x)."
        ),
        "",
        (
            "| image | count | wall median ms | wall p95 ms | wall max s | cpu median ms "
            "| baseline wall ms | ratio |"
        ),
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    # One-off system processes (svchost, installers) say nothing about the job's load.
    common = [image for image in stats if stats[image]["count"] >= MIN_SAMPLES]
    ranked = sorted(common, key=lambda image: -stats[image]["total_s"])
    for image in ranked[:SHOWN_IMAGES]:
        value = stats[image]
        base = reference.get(image, {}).get("wall_ms")
        lines.append(
            f"| {image} | {value['count']} | {number(value['wall_median_ms'])} "
            f"| {number(value['wall_p95_ms'])} | {number(value['wall_max_s'], 1)} "
            f"| {number(value['cpu_median_ms'])} | {number(base)} "
            f"| {ratio_text(comparison['image_ratios'].get(image))} |"
        )
    return "\n".join(lines) + "\n"


def read_baseline(path: Path = BASELINE) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_summary(folder: Path, label: str | None, baseline: dict | None = None) -> str:
    """Write process-timing.json and .md next to the trace; return the Markdown."""
    records, mhz = process_records(folder / "lifecycle.jsonl")
    stats = image_statistics(records, mhz)
    probe = read_probe(folder)
    if baseline is None:
        baseline = read_baseline()
    comparison = compare(stats, probe, label, baseline)
    text = render(stats, probe, comparison, baseline)
    (folder / "process-timing.json").write_text(
        json.dumps(
            {
                "probe": probe,
                "comparison": comparison,
                "cpu_mhz": mhz,
                "images": stats,
            },
            indent=1,
        ),
        encoding="utf-8",
    )
    (folder / "process-timing.md").write_text(text, encoding="utf-8")
    return text


def publish(text: str) -> None:
    print(text, flush=True)
    destination = os.environ.get("GITHUB_STEP_SUMMARY")
    if destination:
        with Path(destination).open("a", encoding="utf-8") as output:
            output.write(text + "\n")


def baseline_images(per_run: list[dict[str, dict]]) -> dict[str, dict]:
    """Short-lived tool images present in every run, medians across runs."""
    common = set.intersection(*(set(stats) for stats in per_run))
    images = {}
    for image in sorted(common):
        runs = [stats[image] for stats in per_run]
        if all(
            run["count"] >= MIN_SAMPLES
            and run["wall_p95_ms"] < SHORT_LIVED_P95_S * 1000
            for run in runs
        ):
            images[image] = {
                "wall_ms": round(
                    statistics.median(r["wall_median_ms"] for r in runs), 1
                ),
                "cpu_ms": round(statistics.median(r["cpu_median_ms"] for r in runs), 1),
            }
    return images


def record_baseline(
    label: str, source: str, folders: list[Path], path: Path = BASELINE
) -> None:
    """Replace one label's baseline with the medians of healthy runs' ETW folders."""
    per_run = []
    probes = []
    for folder in folders:
        records, mhz = process_records(folder / "lifecycle.jsonl")
        per_run.append(image_statistics(records, mhz))
        probes.append(read_probe(folder)["start_ms"])
    baseline = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}
    baseline.setdefault("labels", {})[label] = {
        "source": source,
        "probe_ms": round(statistics.median(probes), 1),
        "images": baseline_images(per_run),
    }
    path.write_text(json.dumps(baseline, indent=1) + "\n", encoding="utf-8")


def main() -> None:
    """`summarize ETW_FOLDER [LABEL]` or `baseline LABEL SOURCE ETW_FOLDER...`."""
    action, *arguments = sys.argv[1:]
    if action == "summarize":
        folder, *label = arguments
        publish(write_summary(Path(folder), label[0] if label else None))
    elif action == "baseline":
        label, source, *folders = arguments
        record_baseline(label, source, [Path(folder) for folder in folders])
    else:
        raise SystemExit(main.__doc__)


if __name__ == "__main__":
    main()
