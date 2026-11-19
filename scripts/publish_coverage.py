"""Assemble GitHub Pages coverage reports from one Coverage reports run."""

import argparse
import json
import shutil
from pathlib import Path


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_badge(output: Path, name: str, label: str, percent: str) -> None:
    # Use red below 60%, amber below 80%, and green from 80%.
    value = float(percent)
    color = "#b42318" if value < 60 else "#8a5300" if value < 80 else "#216e39"
    label_width = len(label) * 7 + 12
    value_width = (len(percent) + 1) * 7 + 12
    width = label_width + value_width
    (output / f"{name}.svg").write_text(
        f"""<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="20"
     viewBox="0 0 {width} 20" role="img" aria-label="{label}: {percent}%">
  <title>{label}: {percent}%</title>
  <rect width="{width}" height="20" rx="3" fill="{color}"/>
  <rect width="{label_width}" height="20" rx="3" fill="#444"/>
  <path d="M{label_width - 3} 0h3v20h-3z" fill="#444"/>
  <g fill="#fff" text-anchor="middle" font-family="Verdana, sans-serif" font-size="11">
    <text x="{label_width / 2:g}" y="14">{label}</text>
    <text x="{label_width + value_width / 2:g}" y="14">{percent}%</text>
  </g>
</svg>
""",
        encoding="utf-8",
    )


def assemble(reports: Path, output: Path) -> None:
    c = reports / "coverage-c-linux"
    rust = reports / "coverage-rust-linux/core"
    ui = reports / "coverage-ui-linux"
    python = reports / "coverage-python-linux"

    c_totals = read_json(c / "coverage-summary.json")
    rust_totals = read_json(rust / "coverage.json")["data"][0]["totals"]
    ui_totals = read_json(ui / "coverage-summary.json")["total"]
    python_totals = read_json(python / "coverage.json")["totals"]

    # Publish only the HTML reports and their assets. Keep diagnostic and machine-readable
    # outputs in the run artifacts, including the workspace-wide Rust report.
    ignore = shutil.ignore_patterns("*.json", "lcov*", "gcov-warnings.txt")
    for name, source in (("c", c), ("rust", rust / "html"), ("ui", ui), ("python", python / "html")):
        if not (source / "index.html").is_file():
            raise FileNotFoundError(source / "index.html")
        shutil.copytree(source, output / name, ignore=ignore)

    write_badge(output, "c-lines", "C lines", f"{c_totals['line_percent']:.1f}")
    write_badge(output, "c-branches", "C branches", f"{c_totals['branch_percent']:.1f}")
    write_badge(output, "rust-core", "Rust core", f"{rust_totals['lines']['percent']:.2f}")
    write_badge(output, "ui-lines", "UI lines", f"{ui_totals['lines']['pct']:g}")
    write_badge(output, "python", "Python", python_totals["percent_covered_display"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reports", type=Path, help="Directory containing downloaded run artifacts")
    parser.add_argument("output", type=Path, help="Coverage directory in the Pages site")
    args = parser.parse_args()
    assemble(args.reports, args.output)
