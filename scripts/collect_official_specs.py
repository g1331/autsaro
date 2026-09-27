"""Fetch one AUTOSAR release using its official SHA-256 manifests."""

import json
import os
import re
import subprocess
import sys
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from hashlib import sha256
from pathlib import Path

if len(sys.argv) != 2 or sys.argv[1] not in ("R24-11", "R25-11"):
    raise SystemExit("usage: python scripts/collect_official_specs.py R24-11|R25-11")
RELEASE = sys.argv[1]
ROOT = Path(__file__).resolve().parents[1] / "docs" / "official" / RELEASE
SOURCE = f"https://www.autosar.org/fileadmin/standards/{RELEASE}"


def digest(path):
    value = sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def collect():
    items = []
    for platform in ("CP", "FO"):
        hashes = ROOT / platform / f"AUTOSAR_{platform}_TR_SpecificationHashes.sha256"
        for line in hashes.read_text(encoding="utf-8").splitlines():
            expected, relative = line.split(" ", 1)
            assert re.fullmatch(r"[0-9a-f]{64}", expected)
            assert len(Path(relative).parts) == 2 and ".." not in Path(relative).parts
            items.append((platform, expected, relative))
    return items


def fetch(item):
    platform, expected, relative = item
    name = Path(relative).name
    # The R25-11 FO search indexes this file under AP; R24-11 uses FO.
    source_platform = (
        "AP"
        if RELEASE == "R25-11" and name == "AUTOSAR_FO_MMOD_MetaModel.zip"
        else platform
    )
    url = f"{SOURCE}/{source_platform}/{name}"
    target = ROOT / platform / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    result = {"path": f"{platform}/{relative}", "sha256": expected, "url": url}
    if target.exists() and digest(target) == expected:
        result["status"] = "verified"
        return result

    previous = ROOT / platform / name
    if previous != target and previous.exists() and digest(previous) == expected:
        os.replace(previous, target)
        result["status"] = "verified"
        return result

    temporary = target.with_name(name + ".part")
    if temporary.exists() and digest(temporary) == expected:
        os.replace(temporary, target)
        result["status"] = "verified"
        return result
    try:
        process = subprocess.run(
            [
                "curl.exe",
                "-fsSL",
                "-C",
                "-",
                "--retry",
                "2",
                "--connect-timeout",
                "20",
                "--speed-time",
                "120",
                "--speed-limit",
                "1024",
                "-o",
                str(temporary),
                url,
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        if process.returncode:
            result["status"] = (
                "unavailable" if "404" in process.stderr else "download-error"
            )
            result["error"] = process.stderr.strip()[-300:]
            if result["status"] == "unavailable":
                temporary.unlink(missing_ok=True)
        elif digest(temporary) != expected:
            result["status"] = "hash-mismatch"
            temporary.unlink()
        else:
            os.replace(temporary, target)
            result["status"] = "verified"
    except OSError as exc:
        result.update(status="download-error", error=str(exc))
    return result


if __name__ == "__main__":
    with ThreadPoolExecutor(max_workers=4) as pool:
        results = list(pool.map(fetch, collect()))
    (ROOT / "download-status.json").write_text(
        json.dumps(results, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    counts = Counter(row["status"] for row in results)
    print(dict(counts))
    for row in results:
        if row["status"] != "verified":
            print(row["status"], row["path"], row.get("error", ""))
    raise SystemExit(0 if counts == {"verified": len(results)} else 1)
