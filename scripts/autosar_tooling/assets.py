"""Explicit maintenance of project-owned digests; upstream identities stay fixed."""

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
    from autosar_tooling import bsw_catalog

    # Validate every immutable upstream source before calculating any writes.
    for name in ("source-manifest.json", "posix-source-manifest.json"):
        upstream = json.loads((root / "third_party/freertos" / name).read_text())
        for relative, expected in upstream["files"].items():
            if digest(root, f"third_party/freertos/{relative}") != expected:
                raise ValueError(
                    f"Modified upstream source: {relative}; restore it or use the reviewed upstream upgrade procedure"
                )
    documents = {}
    changes = []
    bsw_path = root / "runtime/contracts/bsw-v1.json"
    current = bsw_catalog.materialize(root)
    if json.loads(bsw_path.read_text()) != current:
        documents[bsw_path] = current
        changes.append("runtime/contracts/bsw-v1.json")
    inventory_path = root / "runtime/contracts/assets-v1.json"
    inventory = json.loads(inventory_path.read_text())
    updated = copy.deepcopy(inventory)
    seen = set()
    for item in updated["assets"]:
        if item["path"] in seen:
            raise ValueError(f"Duplicate asset: {item['path']}")
        seen.add(item["path"])
        actual = digest(root, item["path"])
        if actual != item["sha256"]:
            if item["license"] != "Apache-2.0" or item["path"].startswith(
                "third_party/"
            ):
                raise ValueError(
                    f"Refusing to update third-party asset identity: {item['path']}"
                )
            changes.append(item["path"])
            item["sha256"] = actual
    if updated != inventory:
        documents[inventory_path] = updated
    tool_path = root / "scripts/ecu_tools/workbench-v2-assets.json"
    tools = json.loads(tool_path.read_text())
    updated_tools = copy.deepcopy(tools)
    for item in updated_tools["files"]:
        actual = digest(root, "scripts/ecu_tools/" + item["path"])
        if item["sha256"] != actual:
            changes.append("scripts/ecu_tools/" + item["path"])
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
            "Review source changes, then run uv run dev assets update; builds never update trusted identities."
        )
        return 1
    print(
        f"Asset inventories {'updated' if documents and update else 'consistent'}; {len(changes)} changes"
    )
    return 0
