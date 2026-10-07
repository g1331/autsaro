"""Strict ownership guard for native v2 packages, preceding unchanged v1 tools."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath

FORMAT = "autosar-workbench-handoff-v2"
SLOT = "epic4-single-application-v1"
OWNERS = {"configuration", "user-application", "product", "generated", "target", "build"}


def _keys(value: object, required: set[str], optional: set[str] | None = None) -> dict:
    if not isinstance(value, dict) or set(value) - required - (optional or set()) or required - set(value):
        raise ValueError("Invalid or unknown v2 object fields")
    return value


def _relative(value: object) -> str:
    if not isinstance(value, str) or not value or "\\" in value or ":" in value:
        raise ValueError("Invalid v2 portable path")
    if PurePosixPath(value).is_absolute() or any(
        part in ("", ".", "..") or part.endswith((".", " "))
        or any(ord(char) < 32 or char in '<>"|?*' for char in part)
        for part in value.split("/")
    ):
        raise ValueError("Unsafe v2 portable path")
    return value


def _digest(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def validate(
    project: Path,
    expected_metadata: dict,
    expected_policy: dict,
    expected_payload: dict,
) -> None:
    # The wrapper checks every Python tool's compiled hash before importing us.
    # Only after that check is the original immutable seal implementation imported.
    from ecu_tools.build import sealed_sources

    names, target = sealed_sources(project)
    project = project.resolve(strict=True)
    metadata = _keys(target.get("nativeDelivery"), {
        "format", "producerVersion", "profileId", "targetId", "projectPath",
        "ownershipPath", "resourceIdentities", "inputSnapshots",
    })
    if metadata != expected_metadata or metadata["format"] != FORMAT:
        raise ValueError("Native input/resource identity differs from this producer's snapshot")
    if metadata["targetId"] != target["target"] or metadata["ownershipPath"] != "workbench-ownership.json":
        raise ValueError("Native target/ownership boundary differs")
    resources = _keys(metadata["resourceIdentities"], {"ruleSetIdentity", "requiredExtensionDefinitions"})
    rule = _keys(resources["ruleSetIdentity"], {"release", "rulesVersion", "sha256"})
    if rule["release"] != "R24-11" or not all(isinstance(rule[key], str) for key in rule):
        raise ValueError("Native rule identity is invalid")
    if set(names) != set(expected_policy) | {metadata["ownershipPath"]}:
        raise ValueError("Native source producer closure differs")
    for path, identity in expected_payload.items():
        _relative(path)
        if path not in names or _digest(project / path) != identity:
            raise ValueError(f"Immutable native product/generated/input bytes changed: {path}")
    ledger = _keys(json.loads((project / metadata["ownershipPath"]).read_text(encoding="utf-8")), {
        "formatVersion", "producerVersion", "profileId", "files",
    })
    if type(ledger["formatVersion"]) is not int or ledger["formatVersion"] != 1:
        raise ValueError("Unknown ownership format version")
    if ledger["producerVersion"] != metadata["producerVersion"] or ledger["profileId"] != metadata["profileId"]:
        raise ValueError("Unknown ownership producer/profile")
    if not isinstance(ledger["files"], list):
        raise TypeError("Invalid ownership payload list")
    paths: list[str] = []
    for row in ledger["files"]:
        row = _keys(row, {"path", "owner", "producerId", "sha256"}, {"snapshotOf"})
        path = _relative(row["path"])
        paths.append(path)
        if row["owner"] not in OWNERS or path not in expected_policy:
            raise ValueError("Unknown native owner or payload path")
        policy = {key: row[key] for key in ("owner", "producerId", "snapshotOf") if key in row}
        if policy != expected_policy[path]:
            raise ValueError(f"Ownership cannot grant this path write authority: {path}")
        if _digest(project / path) != row["sha256"]:
            raise ValueError(f"Ownership payload identity differs: {path}")
        if row["owner"] == "user-application":
            if row["producerId"] != SLOT or "snapshotOf" not in row:
                raise ValueError("User application is not a declared immutable producer snapshot")
            _relative(row["snapshotOf"])
        elif "snapshotOf" in row:
            raise ValueError("Only a user application snapshot may declare snapshotOf")
    if paths != sorted(set(paths)) or set(paths) != set(expected_policy):
        raise ValueError("Ownership payload closure is not canonical and complete")
    manifest = _keys(json.loads((project / metadata["projectPath"]).read_text(encoding="utf-8")), {
        "formatVersion", "declaredRelease", "profileHint", "inputs",
        "applicationInputs", "acceptedExtensionDefinitions",
    })
    if type(manifest["formatVersion"]) is not int or manifest["formatVersion"] != 1:
        raise ValueError("Unknown native member manifest version")
    if manifest["declaredRelease"] != "R24-11" or manifest["acceptedExtensionDefinitions"] != resources["requiredExtensionDefinitions"]:
        raise ValueError("Native member manifest is bound to different release/required definitions")
    mappings: dict[str, dict] = {}
    for source in manifest["inputs"]:
        source = _keys(source, {"path", "roleHint"})
        logical = _relative(source["path"])
        if logical.lower() in mappings:
            raise ValueError("Duplicate native member identity")
        mappings[logical.lower()] = {"path": logical, "kind": "arxml", "role": source["roleHint"]}
    for source in manifest["applicationInputs"]:
        source = _keys(source, {"path", "producerSlot"})
        logical = _relative(source["path"])
        if logical.lower() in mappings or source["producerSlot"] != SLOT:
            raise ValueError("Unknown or conflicting live application member")
        mappings[logical.lower()] = {"path": logical, "kind": "application", "role": "user-application", "producerSlot": SLOT}
    seen: set[str] = set()
    for source in metadata["inputSnapshots"]:
        source = _keys(source, {"logicalPath", "packagePath", "kind", "sha256", "role"}, {"producerSlot"})
        logical = _relative(source["logicalPath"])
        package = _relative(source["packagePath"])
        member = mappings.get(logical.lower())
        if logical.lower() in seen or member is None:
            raise ValueError("Missing or repeated native input mapping")
        seen.add(logical.lower())
        if source["kind"] != member["kind"] or source["role"] != member["role"] or source.get("producerSlot") != member.get("producerSlot"):
            raise ValueError("Native input kind/role/producer membership differs")
        if source["kind"] == "arxml" and package != "inputs/" + logical:
            raise ValueError("Configuration snapshot escaped its owned package boundary")
        if source["kind"] == "application" and package != "src/Application.c":
            raise ValueError("Application snapshot escaped its actual compiled producer path")
        if package not in names or _digest(project / package) != source["sha256"]:
            raise ValueError("Actual source/application snapshot identity differs")
    if seen != set(mappings):
        raise ValueError("Native member and snapshot closures differ")
    if bool(target.get("handoff")):
        declared = json.loads((project / "handoff.json").read_text(encoding="utf-8"))
        if declared != metadata:
            raise ValueError("Handoff source/resource identity differs from native build metadata")
    elif "handoff.json" in names:
        raise ValueError("Source-only delivery cannot silently become a handoff")
