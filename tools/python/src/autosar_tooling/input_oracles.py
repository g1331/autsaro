"""Check the original Epic 4 fixture against independent literal expectations.

This fixture auditor is not the product integration-plan parser. XSD validation
is performed separately by the Rust epic4_reference_input_baseline entry.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from zipfile import ZipFile

from autosar_tooling.config import ROOT

FIXTURE = ROOT / "core/tests/fixtures/epic4"
NS = {"a": "http://autosar.org/schema/r4.0"}
Q = "{" + NS["a"] + "}"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_xml(data: bytes) -> ET.Element:
    if b"<!DOCTYPE" in data or b"<!ENTITY" in data:
        raise ValueError("fixture contains a DTD or entity")
    return ET.fromstring(data)


def objects(tree: ET.Element, file: str) -> dict:
    found = {}

    def visit(node: ET.Element, parent: str) -> None:
        name = node.findtext("a:SHORT-NAME", namespaces=NS)
        path = parent + "/" + name if name else parent
        if name:
            if path in found:
                raise ValueError(f"duplicate fixture object {file}:{path}")
            found[path] = (node, file)
        for child in node:
            visit(child, path)

    visit(tree, "")
    return found


def owned_children(node: ET.Element):
    for child in node:
        if child.find("a:SHORT-NAME", NS) is None:
            yield child
            yield from owned_children(child)


def audit(inputs: dict, external: dict, expectations: dict) -> list[dict]:
    trees = {file: load_xml(data) for file, data in inputs.items()}
    local = {}
    for file, tree in trees.items():
        for path, item in objects(tree, file).items():
            if path in local:
                raise ValueError(f"duplicate fixture object {file}:{path}")
            local[path] = item
    shadows = sorted(local.keys() & external.keys())
    if shadows:
        raise ValueError(
            f"local AUTOSAR object shadows pinned MOD definition: {shadows[0]}"
        )
    issues = []

    def issue(category: str, file: str, path: str) -> None:
        issues.append({"category": category, "file": file, "object": path})

    ecus = [p for p, (v, _) in local.items() if v.tag == Q + "ECU-INSTANCE"]
    if len(ecus) != 1:
        issue("TARGET_NOT_UNIQUE", "extract.arxml", "/Extract/ReferenceEcu")
    apps = [
        p
        for p, (v, _) in local.items()
        if v.tag == Q + "SW-COMPONENT-PROTOTYPE"
        and v.findtext("a:TYPE-TREF", namespaces=NS) == "/Application/EchoApplication"
    ]
    if len(apps) != 1:
        issue(
            "MULTIPLE_INSTANCES",
            "application.arxml",
            "/Application/ReferenceComposition",
        )
    unsupported = {
        "VARIATION-POINT": "VARIANT_UNSELECTED",
        "QUEUED-RECEIVER-COM-SPEC": "QUEUED_UNSUPPORTED",
        "QUEUED-SENDER-COM-SPEC": "QUEUED_UNSUPPORTED",
        "DATA-READ-ACCESSS": "IMPLICIT_UNSUPPORTED",
        "DATA-WRITE-ACCESSS": "IMPLICIT_UNSUPPORTED",
        "MODE-ACCESS-POINTS": "MODE_UNSUPPORTED",
        "DATA-TRANSFORMATIONS": "TRANSFORMER_UNSUPPORTED",
    }
    for path, (node, file) in local.items():
        if file == "unrelated.arxml":
            continue
        for child in owned_children(node):
            if child.tag.removeprefix(Q) in unsupported:
                issue(unsupported[child.tag.removeprefix(Q)], file, path)
        if (
            node.tag == Q + "RUNNABLE-ENTITY"
            and node.findtext("a:CAN-BE-INVOKED-CONCURRENTLY", namespaces=NS) == "true"
        ):
            issue("REENTRANCY_UNSUPPORTED", file, path)
    for symbol in expectations["required_current_entries"]:
        if "/Bsw/" + symbol not in local:
            issue("BSW_ENTRY_MISSING", "bsw.arxml", "/Bsw/" + symbol)
    for symbol, signature in expectations["bsw_signatures"].items():
        item = local.get("/Bsw/" + symbol)
        if item is None:
            issue("BSW_ENTRY_MISSING", "bsw.arxml", "/Bsw/" + symbol)
            continue

        def native_type(argument: ET.Element | None) -> str:
            if argument is None:
                return "void"
            reference = argument.findtext(
                "a:SW-DATA-DEF-PROPS/a:SW-DATA-DEF-PROPS-VARIANTS/"
                "a:SW-DATA-DEF-PROPS-CONDITIONAL/a:BASE-TYPE-REF",
                namespaces=NS,
            )
            target = local.get(reference)
            return (
                "missing"
                if target is None
                else target[0].findtext("a:NATIVE-DECLARATION", namespaces=NS)
            )

        entry = item[0]
        arguments = [
            {
                "name": a.findtext("a:SHORT-NAME", namespaces=NS),
                "native_type": native_type(a),
                "direction": a.findtext("a:DIRECTION", namespaces=NS),
            }
            for a in entry.findall("a:ARGUMENTS/a:SW-SERVICE-ARG", NS)
        ]
        if (
            native_type(entry.find("a:RETURN-TYPE", NS)) != signature["return"]
            or arguments != signature["arguments"]
        ):
            issue("BSW_SIGNATURE_CONFLICT", "bsw.arxml", "/Bsw/" + symbol)
    for check in expectations["checks"]:
        item = local.get(check["object"])
        actual = (
            []
            if item is None
            else [node.text for node in item[0].findall(check["xpath"], NS)]
        )
        if actual != check["expected"]:
            issue(check["category"], check["file"], check["object"])
    all_objects = external | local
    for path, (node, file) in local.items():
        for ref in owned_children(node):
            dest = ref.get("DEST")
            if not dest:
                continue
            target = all_objects.get(ref.text)
            if target is None:
                issue("REFERENCE_UNRESOLVED", file, path)
            elif target[0].tag != Q + dest:
                issue("REFERENCE_DEST", file, path)
    return issues


def verify_fixture_manifest() -> dict:
    """Validate owned fixture bytes without requiring the external oracle."""
    manifest = json.loads((FIXTURE / "manifest.json").read_text(encoding="utf-8"))
    declared = {entry["path"] for entry in manifest["files"]}
    discovered = {
        path.relative_to(FIXTURE).as_posix()
        for path in FIXTURE.rglob("*")
        if path.is_file()
        and path.relative_to(FIXTURE).as_posix() not in {"README.md", "manifest.json"}
    }
    if len(declared) != len(manifest["files"]) or declared != discovered:
        raise ValueError("fixture file set differs from raw-byte manifest")
    for entry in manifest["files"]:
        path = FIXTURE / entry["path"]
        if digest(path) != entry["sha256"]:
            raise ValueError(f"fixture hash mismatch: {entry['path']}")
    return manifest


def main() -> None:
    from autosar_tooling.config import archive_path

    manifest = verify_fixture_manifest()
    mod = archive_path("AUTOSAR_MOD_ARCHIVE", manifest["external_mod"]["path"])
    if not mod.is_file():
        raise ValueError("not_run: required local R24-11 MOD archive is missing")
    if digest(mod) != manifest["external_mod"]["sha256"]:
        raise ValueError("R24-11 MOD archive missing or hash mismatch")
    with ZipFile(mod) as archive:
        tree = load_xml(archive.read("AUTOSAR_CP_MOD_ECUConfigurationParameters.arxml"))
    external = objects(tree, "external R24-11 MOD")
    expected = json.loads((FIXTURE / "expectations.json").read_text(encoding="utf-8"))
    positive = {p.name: p.read_bytes() for p in (FIXTURE / "positive").glob("*.arxml")}
    issues = audit(positive, external, expected)
    if issues:
        raise ValueError(f"positive semantic expectations failed: {issues}")
    for symbol, signature in expected["bsw_signatures"].items():
        module = symbol.split("_", 1)[0]
        header_paths = [ROOT / f"runtime/include/{module}.h"] + sorted(
            (ROOT / "runtime/include").glob(f"{module}_*.h")
        )
        scheduled_header = ROOT / f"runtime/include/SchM_{module}.h"
        if scheduled_header.is_file():
            header_paths.append(scheduled_header)
        header = "\n".join(path.read_text(encoding="utf-8") for path in header_paths)
        source = (ROOT / f"runtime/src/{module}.c").read_text(encoding="utf-8")
        declaration = re.search(
            r"^([A-Za-z_]\w*)\s+" + re.escape(symbol) + r"\s*\(([^;{}]*)\)\s*;",
            header,
            re.MULTILINE,
        )
        if declaration is None or not re.search(
            r"^[A-Za-z_]\w*\s+" + re.escape(symbol) + r"\s*\([^;{}]*\)\s*\{",
            source,
            re.MULTILINE,
        ):
            raise ValueError(f"current BSW producer missing: {symbol}")
        native_arguments = []
        if declaration[2].strip() != "void":
            for argument in declaration[2].split(","):
                parameter = re.fullmatch(
                    r"(.*?)\b([A-Za-z_]\w*)\s*(\[\d+\])?", argument.strip()
                )
                if parameter is None:
                    raise ValueError(
                        f"unrecognized current BSW parameter: {symbol}: {argument}"
                    )
                spelling = parameter[1].strip() + (" *" if parameter[3] else "")
                native_arguments.append(" ".join(spelling.replace("*", " * ").split()))
        declared_arguments = [a["native_type"] for a in signature["arguments"]]
        if (
            declaration[1] != signature["return"]
            or native_arguments != declared_arguments
        ):
            raise ValueError(f"current BSW signature changed: {symbol}")
    cases = json.loads((FIXTURE / "negative/cases.json").read_text(encoding="utf-8"))
    results = []
    for case in cases["cases"]:
        inputs = positive.copy()
        for override in case["overrides"]:
            path = FIXTURE / override
            inputs[path.name] = path.read_bytes()
        if case["expected_xsd"] == "fail":
            results.append({"id": case["id"], "semantic": "not_run"})
            continue
        issues = audit(inputs, external, expected)
        matches = [
            i
            for i in issues
            if i["category"] == case["category"]
            and i["file"] == case["file"]
            and (
                i["object"].startswith(case["object"])
                or case["object"].startswith(i["object"])
            )
        ]
        if not matches:
            raise ValueError(f"{case['id']}: expected {case['category']}, got {issues}")
        results.append({"id": case["id"], "semantic": "reject", "diagnostics": matches})
    print(
        json.dumps(
            {"positive": "pass", "negative": results, "xsd": "Rust entry required"}
        )
    )


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, ET.ParseError) as error:
        print(f"epic4 fixture audit failed: {error}", file=sys.stderr)
        sys.exit(1)
