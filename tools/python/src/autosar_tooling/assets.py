"""Validate runtime asset metadata and maintain the independent seal-guard identity."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path, PurePosixPath

from autosar_tooling.config import ROOT


def digest(root: Path, relative: str) -> str:
    path = PurePosixPath(relative)
    if (
        not path.parts
        or path.is_absolute()
        or any(part in ("..", ".") for part in path.parts)
        or "\\" in relative
        or ":" in relative
    ):
        raise ValueError(f"Unsafe asset path: {relative}")
    selected = root
    for part in path.parts:
        selected = selected / part
        if selected.is_symlink() or selected.is_junction():
            raise ValueError(f"Linked asset path: {relative}")
    if not selected.is_file():
        raise ValueError(f"Missing asset: {relative}")
    return hashlib.sha256(selected.read_bytes()).hexdigest()


def maintain(*, update: bool = False, root: Path = ROOT) -> int:
    # Validate every immutable upstream source before calculating any writes.
    for name in ("source-manifest.json", "posix-source-manifest.json"):
        upstream = json.loads((root / "third_party/freertos" / name).read_text())
        for relative, expected in upstream["files"].items():
            if digest(root, f"third_party/freertos/{relative}") != expected:
                raise ValueError(
                    f"Modified upstream source: {relative}; restore it or use the reviewed upstream upgrade procedure"
                )
        for relative, expected in upstream.get("patches", {}).items():
            if digest(root, relative) != expected:
                raise ValueError(f"Modified controlled patch: {relative}; use the reviewed upstream upgrade procedure")
    documents = {}
    changes = []
    inventory_path = root / "runtime/contracts/assets.json"
    inventory = json.loads(inventory_path.read_text())
    if inventory.get("format") != "autosar-asset-inventory-v2":
        raise ValueError("Invalid asset inventory format")
    seen = set()
    for item in inventory["assets"]:
        if set(item) != {"path", "license", "targets", "profiles", "role", "responsibility"}:
            raise ValueError(f"Invalid asset fields: {item.get('path')}")
        if item["path"] in seen:
            raise ValueError(f"Duplicate asset: {item['path']}")
        seen.add(item["path"])
        digest(root, item["path"])
        if (item["license"] != "Apache-2.0" or not item["responsibility"]
                or item["role"] not in {"bsw", "host", "os", "patch", "target", "delivery"}
                or not item["targets"] or not set(item["targets"]) <= {
                    "windows-x64-controlled-v1", "linux-x64-controlled-v1"}
                or item["profiles"] != ["ecu"]):
            raise ValueError(f"Invalid asset metadata: {item['path']}")
    tool_path = root / "tools/python/src/ecu_tools/build-tool-inventory.json"
    tools = json.loads(tool_path.read_text())
    updated_tools = copy.deepcopy(tools)
    for item in updated_tools["files"]:
        actual = digest(root, "tools/python/src/ecu_tools/" + item["path"])
        if item["sha256"] != actual:
            changes.append("tools/python/src/ecu_tools/" + item["path"])
            item["sha256"] = actual
    if updated_tools != tools:
        documents[tool_path] = updated_tools
    for changed in changes:
        print(f"{'Update' if update else 'Mismatch'}: {changed}")
    if update:
        # Only an explicit update writes tracked manifests. Review all resulting diffs.
        for path, value in documents.items():
            temporary = path.with_name(path.name + ".tmp")
            temporary.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
            temporary.replace(path)
    if changes and not update:
        print(
            "Review source changes, then run uv run --locked python -m autosar_tooling assets update; builds never update trusted identities."
        )
        return 1
    print(
        f"Asset inventories {'updated' if documents and update else 'consistent'}; {len(changes)} changes"
    )
    return 0


def check_line_endings(root: Path = ROOT) -> None:
    """Check fixed checkout bytes, including CRLF exceptions, through Git attributes."""
    import subprocess

    paths = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True,
    ).stdout
    attributes = subprocess.run(
        ["git", "check-attr", "-z", "--stdin", "text", "eol"], input=paths,
        cwd=root, check=True, capture_output=True,
    ).stdout.split(b"\0")
    for index in range(0, len(attributes) - 1, 6):
        name, _, text, _, _, value = attributes[index:index + 6]
        if text == b"unset" or value not in (b"lf", b"crlf"):
            continue
        relative = name.decode("utf-8")
        contents = (root / relative).read_bytes()
        invalid = b"\r" in contents if value == b"lf" else (
            b"\n" in contents.replace(b"\r\n", b"")
            or b"\r" in contents.replace(b"\r\n", b"")
        )
        if invalid:
            raise ValueError(f"Line endings violate eol={value.decode()}: {relative}")
