"""Compare complete C analyses by diagnostic source text, retaining occurrences."""

from __future__ import annotations

import hashlib
import json
import re
from collections import Counter
from pathlib import Path

from ecu_tools.build import (
    FILE_HASHES,
    digest,
    refuse_links,
    require_kernel_identity,
    safe_relative,
)

from autosar_tooling import c_sdk
from autosar_tooling.c_standard import MANUAL_ASSESSMENT_SOURCES


def _read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


MATRIX_TARGETS = ("linux-x64-controlled-v1", "windows-x64-controlled-v1")
MATRIX_SAMPLES = ("standard-ecu", "user-application", "component-pipeline", "no-diagnostic")
SEMANTIC_VERSION = 4
BSWM_MODES = ("COMM_NO_COMMUNICATION", "COMM_SILENT_COMMUNICATION", "COMM_FULL_COMMUNICATION")
BSWM_CALLOUT_PARAMETER = (
    "/AUTOSAR/EcucDefs/BswM/BswMConfig/BswMModeControl/BswMAction/"
    "BswMAvailableActions/BswMUserCallout/BswMUserCalloutFunction"
)


def upgraded_bswm_callouts(source: bytes, handle: int) -> bytes | None:
    """Recognize only one complete set of the three legacy callout values."""
    for mode in BSWM_MODES:
        old = f"<VALUE>Ecu_HostBusSM_ApplyMode({handle}u, {mode})</VALUE>".encode()
        new = f"<VALUE>Ecu_HostBusSM_ConfirmMode({handle}u, {mode})</VALUE>".encode()
        if source.count(old) != 1 or new in source:
            return None
        source = source.replace(old, new)
    return source


def raw_identity(summary: dict) -> str:
    """Bind raw analysis independently of the later matrix acceptance annotations."""
    raw = {key: value for key, value in summary.items()
           if key not in {"projectAcceptance", "complianceScope"}}
    raw["diagnostics"] = [{key: value for key, value in row.items() if key != "complianceScope"}
                          for row in summary["diagnostics"]]
    return hashlib.sha256(json.dumps(raw, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def scope_receipt(summary: dict, sample: str) -> dict:
    return {"format": "c-misra-scope-input-v1", "sample": sample,
            "analysisIdentity": raw_identity(summary),
            **{key: summary["scope"][key] for key in
               ("target", "translationUnits", "projectTranslationUnits")}}


def scope_matrix(receipts: list[dict]) -> dict:
    """Require exactly eight raw dependency receipts before applying rule 2."""
    expected = {(target, sample) for target in MATRIX_TARGETS for sample in MATRIX_SAMPLES}
    found = set()
    for row in receipts:
        if (set(row) != {"format", "target", "sample", "analysisIdentity",
                         "translationUnits", "projectTranslationUnits"}
                or row["format"] != "c-misra-scope-input-v1"
                or not re.fullmatch(r"[0-9a-f]{64}", row["analysisIdentity"])):
            raise ValueError("Invalid matrix scope receipt")
        pair = row["target"], row["sample"]
        if pair not in expected or pair in found:
            raise ValueError("Unexpected or duplicated matrix scope receipt")
        found.add(pair)
        units, dependencies = row["translationUnits"], row["projectTranslationUnits"]
        if (not isinstance(units, list) or not units or len(units) != len(set(units))
                or any(not safe_relative(unit) for unit in units)
                or not isinstance(dependencies, dict) or set(dependencies) != set(units)
                or any(not isinstance(paths, list) or paths != sorted(set(paths))
                       or unit not in paths or any(not safe_relative(name) for name in paths)
                       for unit, paths in dependencies.items())):
            raise ValueError("Matrix GCC dependency receipt incomplete")
    if found != expected:
        raise ValueError(f"Matrix scope incomplete: {sorted(expected - found)}")
    members = sorted(receipts, key=lambda row: (row["target"], row["sample"]))
    return {"format": "c-misra-scope-matrix-v1", "members": members,
            "targetHeaders": sorted(target_headers(members))}


def matrix_headers(summary: dict, matrix: dict) -> set[str]:
    if matrix != scope_matrix(matrix["members"]):
        raise ValueError("Matrix scope union differs from dependency receipts")
    if not any(row == scope_receipt(summary, row["sample"])
               for row in matrix["members"] if row["target"] == summary["scope"]["target"]):
        raise ValueError("Matrix scope does not bind this raw analysis")
    return set(matrix["targetHeaders"])


def annotate_scope(summary: dict, headers: set[str] | None = None) -> None:
    for diagnostic in summary["diagnostics"]:
        diagnostic["complianceScope"] = [location_scope(summary["scope"], location, headers=headers)
                                         for location in diagnostic["locations"]]
    summary["complianceScope"] = scope_statistics(summary, headers=headers)


def platform_location(name: str) -> bool:
    """ADR 0030 rule 1: platform selection owns the complete directory."""
    return name.startswith(("os/src/host/", "kernel/portable/"))


def target_unit(name: str) -> bool:
    return not platform_location(name) and not name.startswith("integration/")


def target_headers(scopes: list[dict]) -> set[str]:
    """Union GCC -MD receipts from target TUs, across samples and programs."""
    headers = set()
    for scope in scopes:
        dependencies = scope.get("projectTranslationUnits", {})
        for unit in scope["translationUnits"]:
            if not target_unit(unit):
                continue
            headers.update(dependencies.get(unit, []))
    return headers


def location_scope(scope: dict, location: dict, *, headers: set[str] | None = None) -> str:
    """Classify a physical position, including mixed adopted/project diagnostics."""
    name = location["file"]
    if platform_location(name):
        return "host-platform"
    if name.startswith("integration/"):
        if name.endswith(".c"):
            return "host-integration"
        dependencies = scope.get("projectTranslationUnits")
        units = [unit for unit in scope["translationUnits"] if target_unit(unit)]
        if not isinstance(dependencies, dict) or any(unit not in dependencies for unit in units):
            raise ValueError("MISRA scope requires complete GCC project dependency receipts")
        if name not in (target_headers([scope]) if headers is None else headers):
            return "host-integration"
    if (name.startswith("kernel/") and "patchedLines" in scope
            and int(location["line"]) not in scope["patchedLines"].get(name, [])):
        return "adopted-kernel"
    return "in-scope"


def scope_statistics(summary: dict, *, headers: set[str] | None = None) -> dict:
    """Count physical locations once; program multiplicity is kept by comparison."""
    categories = Counter(location_scope(summary["scope"], location, headers=headers)
                         for diagnostic in summary["diagnostics"]
                         if diagnostic["id"] != "checkersReport"
                         for location in diagnostic["locations"])
    inside = categories["in-scope"]
    total = sum(categories.values())
    raw = [diagnostic for diagnostic in summary["diagnostics"]
           if diagnostic["id"] != "checkersReport"]
    raw_outside = sum(bool(row["locations"]) and all(
        location_scope(summary["scope"], location, headers=headers) != "in-scope"
        for location in row["locations"]) for row in raw)
    return {"total": total, "inScope": inside, "outOfScope": total - inside,
            "categories": dict(sorted(categories.items())),
            "diagnostics": {"total": len(raw), "inScope": len(raw) - raw_outside,
                            "outOfScope": raw_outside,
                            "informational": len(summary["diagnostics"]) - len(raw)}}


def diagnostic_counts(summary: dict, root: Path, *, in_scope_only: bool = False,
                      headers: set[str] | None = None) -> Counter:
    counts = Counter()
    for diagnostic in summary["diagnostics"]:
        if diagnostic["id"] == "checkersReport":
            continue
        locations = diagnostic["locations"]
        if not locations:
            raise ValueError(f"Diagnostic has no source location: {diagnostic['id']}")
        for location in locations:
            if in_scope_only and location_scope(summary["scope"], location, headers=headers) != "in-scope":
                continue
            name = location["file"]
            if not safe_relative(name):
                raise ValueError(f"Diagnostic outside sample: {name}")
            lines = (root / name).read_text(encoding="utf-8").split("\n")
            line = int(location["line"])
            if not 1 <= line <= len(lines):
                raise ValueError(f"Diagnostic source line unavailable: {name}:{line}")
            for program in diagnostic["programs"]:
                counts[diagnostic["id"], name, lines[line - 1], program] += 1
    return counts


def _complete(summary: dict) -> None:
    if (summary["error"] is not None or not summary["programs"]
            or any(program["exitCode"] not in (0, 1) for program in summary["programs"])):
        raise ValueError("Analysis is incomplete")
    incomplete = {"misra-config", "syntaxError", "preprocessorErrorDirective", "internalError",
                  "cppcheckError", "addonError", "missingInclude", "missingIncludeSystem", "toomanyconfigs"}
    if any(row["id"] in incomplete for row in summary["diagnostics"]):
        raise ValueError("Analysis contains configuration/parsing failures")


def _semantic(summary: dict) -> dict:
    identity = summary.get("semanticIdentity")
    def require(condition):
        if not condition:
            raise ValueError("Semantic identity missing, unsupported or incomplete")
    def hashes(value, keys, nullable=False):
        require(isinstance(value, dict) and set(value) == set(keys))
        require(all((nullable and item is None) or
                    (isinstance(item, str) and re.fullmatch(r"[0-9a-f]{64}", item))
                    for item in value.values()))
    def strings(value, nonempty=False):
        return isinstance(value, list) and (not nonempty or bool(value)) and all(isinstance(x, str) and x for x in value)
    if isinstance(identity, dict) and identity.get("version") in (2, 3):
        raise ValueError(f"Semantic identity version {identity['version']} is superseded by "
                         f"version {SEMANTIC_VERSION}; regenerate the analysis with the current tool")
    require(isinstance(identity, dict) and set(identity) ==
            {"version", "compiler", "defines", "models", "libraries", "analysisCode", "addonRuntime", "sdk"})
    require(type(identity["version"]) is int and identity["version"] == SEMANTIC_VERSION)
    compiler = identity["compiler"]
    require(isinstance(compiler, dict) and set(compiler) ==
            {"machine", "release", "predefines", "builtins", "includeSearch"})
    require(isinstance(compiler["machine"], str) and bool(compiler["machine"]))
    require(isinstance(compiler["release"], str) and bool(re.fullmatch(r"[0-9]+(?:\.[0-9]+)*", compiler["release"])))
    require(strings(compiler["predefines"], True) and strings(compiler["builtins"], True))
    search = compiler["includeSearch"]
    require(isinstance(search, dict) and set(search) == {"quote", "system", "environment"}
            and strings(search["quote"]) and strings(search["system"], True)
            and isinstance(search["environment"], dict)
            and set(search["environment"]) == set(c_sdk.SEARCH_ENVIRONMENT)
            and all(value is None or isinstance(value, str)
                    for value in search["environment"].values()))
    defines = identity["defines"]
    require(isinstance(defines, dict) and set(defines) == set(summary["scope"]["translationUnits"]))
    require(all(strings(value) for value in defines.values()))
    hashes(identity["models"], ("platformModel", "platformLibrary"), True)
    hashes(identity["libraries"], ("std", "windows" if summary["scope"]["target"].startswith("windows") else "posix", "implicitStd"))
    hashes(identity["analysisCode"], ("c_check.py", "c_addon.py", "c_comparison.py", "c_sdk.py", "c_standard.py", "c_retention.py", "c_acceptance.py", "c_preprocessor.py", "build.py", "process.py", "owner.py", "launch.py", "windows_job.py"))
    hashes(identity["addonRuntime"], ("runaddon.py", "cppcheck.py", "cppcheckdata.py"))
    assessment = summary["coverage"]["manualAssessment"]
    require(assessment["format"] == "misra-manual-assessment-v1")
    manual = assessment["inputIdentity"]
    require(set(manual) == {"parserSha256", "selectionSha256", "documents"})
    hashes(manual["documents"], MANUAL_ASSESSMENT_SOURCES)
    require(isinstance(manual["parserSha256"], str) and bool(re.fullmatch("[0-9a-f]{64}", manual["parserSha256"])))
    require(manual["selectionSha256"] == hashlib.sha256("\n".join(MANUAL_ASSESSMENT_SOURCES).encode()).hexdigest())
    require(isinstance(identity["sdk"], str) and bool(re.fullmatch("[0-9a-f]{64}", identity["sdk"])))
    # Both comparison modes rest on the recorded inventory and opened headers.
    c_sdk.validate_inventory(summary["scope"])
    sdk = c_sdk.semantic_inputs(summary["scope"])
    if hashlib.sha256(json.dumps(sdk, sort_keys=True).encode()).hexdigest() != identity["sdk"]:
        raise ValueError("Semantic identity differs: sdk receipt")
    return identity


def _source_sdk(summary: dict, identity: dict) -> dict:
    """Separate verified SDK header evidence from source-dependent usage inventories."""
    sdk = c_sdk.semantic_inputs(summary["scope"])
    headers = {row["header"]: row for row in sdk["headers"]}
    header_names = {row["key"]: row["header"] for row in sdk["headers"]}
    macros = {}
    for uses in sdk["macroInputs"].values():
        for use in uses:
            definition = json.loads(use)
            key = header_names[definition["headerKey"]], definition["macro"]
            macros.setdefault(key, set()).add((definition["value"], definition["function"]))
    defines = {}
    for unit, values in identity["defines"].items():
        for value in values:
            match = re.fullmatch(r"-D([A-Za-z_]\w*)(.*)", value)
            if match is None:
                raise ValueError("Semantic identity differs: defines format")
            defines[unit, match[1]] = match[2]
    return {"headers": headers, "macros": macros, "defines": defines,
            "inventory": sdk["inventory"], "links": sdk["links"],
            "lineDirectives": sdk["lineDirectives"]}


def _compare_source_sdk(current: dict, previous: dict, renames: dict) -> None:
    now = _source_sdk(current, current["semanticIdentity"])
    before = _source_sdk(previous, previous["semanticIdentity"])
    before["defines"] = {(renames.get(unit, unit), name): value
                         for (unit, name), value in before["defines"].items()}
    if now["lineDirectives"] != before["lineDirectives"]:
        raise ValueError("Semantic identity differs: sdk line directives")
    # Every provider file the compiler can find must match, opened or not: a header that
    # only one tree opens, or a different file behind the same include, changes the result.
    files = now["inventory"].keys() | before["inventory"].keys()
    changed = sorted(name for name in files
                     if now["inventory"].get(name) != before["inventory"].get(name))
    if changed:
        raise ValueError(f"Semantic identity differs: sdk inventory: {changed[0]}")
    names = now["links"].keys() | before["links"].keys()
    changed = sorted(name for name in names if now["links"].get(name) != before["links"].get(name))
    if changed:
        raise ValueError(f"Semantic identity differs: sdk inventory: link {changed[0]}")
    for field in ("headers", "macros", "defines"):
        # Source changes may open more or fewer headers and use other macros. What both
        # trees consumed must agree byte for byte, as a second check on the inventory.
        for key in now[field].keys() & before[field].keys():
            if now[field][key] != before[field][key]:
                label = "defines" if field == "defines" else "sdk " + field
                raise ValueError(f"Semantic identity differs: {label}: {key}")


def _configuration(root: Path, *, source_tree: bool = False) -> dict:
    refuse_links(root / "meta/integration.json", root)
    metadata = _read(root / "meta/integration.json")
    plan = dict(metadata["plan"])
    if "runtimeSources" not in plan:
        if metadata.get("format") in {"autosar-ecu-integration-v2", "autosar-ecu-integration-v3"}:
            raise ValueError("Runtime source provenance missing")
        return plan
    sources, origins = plan["runtimeSources"], metadata.get("originalBswSources")
    if not isinstance(sources, dict) or not isinstance(origins, dict):
        raise TypeError("Runtime source provenance invalid")
    records = (root / FILE_HASHES).read_text(encoding="utf-8").splitlines()

    def require_source(name, identity):
        if not isinstance(name, str) or not safe_relative(name):
            raise ValueError("Runtime source path invalid")
        refuse_links(root / name, root)
        if records.count(f"{identity}  {name}") != 1 or digest(root / name) != identity:
            raise ValueError(f"Runtime source identity differs: {name}")

    require_source("meta/integration.json", digest(root / "meta/integration.json"))
    selected = dict(sources)
    for original, identity in sources.items():
        if not isinstance(original, str) or not safe_relative(original):
            raise ValueError("Runtime source origin invalid")
        if not isinstance(identity, str) or not re.fullmatch(r"[0-9a-f]{64}", identity):
            raise ValueError("Runtime source identity invalid")
        if original.startswith("runtime/") and Path(original).suffix in {".c", ".h"} and original in origins:
            copied = origins[original]
            require_source(copied, identity)
            # Delivered bytes may change; their selected source/path binding may not.
            selected[original] = copied
    if source_tree:
        # Producer bytes may differ in a freshly scanned source tree.
        # Keep selected origins and delivered paths as configuration identity.
        selected = {name: value for name, value in selected.items() if name in origins}
        # The native rule inventory includes the Rust generator itself, and
        # the definition fingerprint includes that inventory identity.
        if "ruleSetIdentity" in plan:
            plan["ruleSetIdentity"] = {key: value for key, value in
                                       plan["ruleSetIdentity"].items() if key != "sha256"}
        if "validationDependencies" in plan:
            plan["validationDependencies"] = {key: value for key, value in
                                              plan["validationDependencies"].items()
                                              if key not in {"rules.sha256", "definitions.sha256"}}
    plan["runtimeSources"] = selected
    return plan


def _r11_baseline_plan(plan: dict) -> dict:
    """Project V2's single-channel standard Classical selection into V3."""
    if plan["formatVersion"] != 2:
        raise ValueError("R11 baseline plan version differs")
    signals = plan["signals"]
    channels = {signal["trigger"].rsplit("/", 1)[0] for signal in signals}
    if len(channels) != 1:
        raise ValueError("R11 baseline physical channel is ambiguous")
    runtime = plan["communicationRuntime"]
    can = {**runtime["can"], "physicalChannel": channels.pop(),
           "receiveIdType": "STANDARD", "transmitIdType": "STANDARD"}
    result = {**plan, "formatVersion": 3, "communicationRuntime": {**runtime, "can": can},
              "signals": [{**signal, "extended": False, "acceptsFd": False} for signal in signals]}
    composition = plan["composition"]
    result["composition"] = {**composition, "networkEndpoints": [
        {**endpoint, "transport": {**endpoint["transport"], "extended": False, "acceptsFd": False}}
        for endpoint in composition["networkEndpoints"]
    ]}
    for name in ("diagnostic", "diagnosticTransport"):
        if plan.get(name) is not None:
            result[name] = {**plan[name], "requestExtended": False,
                            "responseExtended": False, "requestAcceptsFd": False}
    return result


def _bswm_baseline_plan(previous: dict, current: dict) -> dict:
    """Compare only the exact three-callout transition in the source gate."""
    old = previous.get("modeRuntime")
    new = current.get("modeRuntime")
    if not isinstance(old, dict) or not isinstance(new, dict):
        return previous
    handle = old.get("channelHandle")
    rules = old.get("rules")
    if (type(handle) is not int or not 0 <= handle <= 255 or not isinstance(rules, list)
            or len(rules) != len(BSWM_MODES)):
        return previous
    updated = []
    replacements = {}
    for index, (rule, mode) in enumerate(zip(rules, BSWM_MODES, strict=True)):
        args = f"({handle}u, {mode})"
        if (not isinstance(rule, dict) or type(rule.get("mode")) is not int
                or rule["mode"] != index or rule.get("callout") != "Ecu_HostBusSM_ApplyMode" + args):
            return previous
        replacement = "Ecu_HostBusSM_ConfirmMode" + args
        replacements[rule["callout"]] = replacement
        updated.append({**rule, "callout": replacement})
    upgraded = {**old, "rules": updated}
    if upgraded != new:
        return previous
    configuration = []
    for entry in previous["configuration"]:
        parameters = entry.get("parameters", {})
        values = parameters.get(BSWM_CALLOUT_PARAMETER)
        if values is not None:
            if len(values) != 1 or values[0] not in replacements:
                return previous
            parameters = {**parameters, BSWM_CALLOUT_PARAMETER: [replacements.pop(values[0])]}
            entry = {**entry, "parameters": parameters}
        configuration.append(entry)
    if replacements or configuration != current["configuration"]:
        return previous
    return {**previous, "modeRuntime": upgraded, "configuration": configuration}


def _bswm_input_digests(old_root: Path, project: Path, handle: int) -> tuple[str, str]:
    relative = "inputs/ecuc.arxml"
    contents = []
    hashes = []
    for root in (old_root, project):
        path = root / relative
        refuse_links(path, root)
        source = path.read_bytes()
        fingerprint = hashlib.sha256(source).hexdigest()
        records = (root / FILE_HASHES).read_text(encoding="utf-8").splitlines()
        if records.count(f"{fingerprint}  {relative}") != 1:
            raise ValueError("BswM callout transition input is not sealed")
        contents.append(source)
        hashes.append(fingerprint)
    if upgraded_bswm_callouts(contents[0], handle) != contents[1]:
        raise ValueError("Sample generation configuration differs outside the BswM callouts")
    return hashes[0], hashes[1]


def _translated_ecuc_digest(rows: list[dict], name: str, digests: tuple[str, str]) -> list[dict]:
    matches = [row for row in rows if row.get("logicalPath") == "ecuc.arxml"]
    if (len(matches) != 1 or matches[0].get(name) != digests[0]
            or (name == "sha256" and matches[0].get("packagePath") != "inputs/ecuc.arxml")):
        raise ValueError("BswM callout transition input identity differs")
    return [{**row, name: digests[1]} if row is matches[0] else row for row in rows]


def _can_fd_baseline_plan(plan: dict) -> dict:
    """Project the previous Classical-only configuration into the FD-capable plan."""
    runtime = plan["communicationRuntime"]
    can = runtime["can"]
    records = {record["path"]: record for record in plan["configuration"]}
    controller = records[can["controller"]]
    default_rate, = controller["references"][controller["definition"] + "/CanControllerDefaultBaudrate"]
    baud = records[default_rate]
    nominal, = baud["parameters"][baud["definition"] + "/CanControllerBaudRate"]
    bitrate = int(float(nominal)) * 1000
    composition = plan["composition"]
    return {
        **plan,
        "communicationRuntime": {**runtime, "can": {
            **can, "nominalBitrate": bitrate, "dataBitrate": None, "brs": False, "fdPadding": 0,
        }},
        "signals": [{**signal, "fd": False} for signal in plan["signals"]],
        "composition": {**composition, "networkEndpoints": [
            {**endpoint, "transport": {**endpoint["transport"], "fd": False}}
            for endpoint in composition["networkEndpoints"]
        ]},
    }


def _com_layout_predecessor(plan: dict, previous: dict) -> dict:
    """Project the explicit layout onto COM's original four-byte UINT32 profile."""
    layout = {
        "bigEndian": False, "nativeType": "uint32", "initialValue": 0,
        "frameStartBit": 0, "frameBigEndian": False, "frameOffset": 0,
        "pduLength": 4, "unusedByte": 0,
    }
    added = layout.keys() | {"frameMapping"}

    def legacy(signal):
        if not added <= signal.keys():
            return signal
        mapping = signal["frameMapping"]
        # The sealed input snapshots must still match. The old validator already
        # required a unique PDU mapping under the selected frame.
        owner, _, name = mapping.rpartition("/") if isinstance(mapping, str) else ("", "", "")
        if (signal["startBit"] != 0 or signal["bitLength"] != 32 or signal["dlc"] != 4
                or any(type(signal.get(key)) is not type(value) or signal[key] != value
                       for key, value in layout.items())
                or owner != signal["frame"] or not name):
            return signal
        return {key: value for key, value in signal.items() if key not in added}

    signals = previous.get("signals", [])
    if not signals or any(added & signal.keys() for signal in signals):
        return plan
    composition = plan["composition"]
    return {
        **plan,
        "signals": [legacy(signal) for signal in plan["signals"]],
        "composition": {**composition, "networkEndpoints": [
            {**endpoint, "transport": legacy(endpoint["transport"])}
            for endpoint in composition["networkEndpoints"]
        ]},
    }


def compare(
    current: dict, project: Path, output: Path, baseline: Path, *,
    source_tree: bool = False, renames: dict[str, str] | None = None,
) -> dict:
    """A baseline is evidence for comparison, never permission to skip analysis."""
    try:
        renames = renames or {}

        def mapped(value):
            if isinstance(value, str):
                return renames.get(value, value)
            if isinstance(value, list):
                return [mapped(item) for item in value]
            if isinstance(value, dict):
                return {renames.get(key, key): mapped(item) for key, item in value.items()}
            return value

        def normalized(value, field=""):
            if isinstance(value, dict):
                return {key: normalized(item, key) for key, item in value.items()
                        if not (source_tree and field == "ruleSetIdentity" and key == "sha256")}
            if isinstance(value, list):
                items = [normalized(item) for item in value]
                if source_tree and field == "includePaths":
                    # Selected host provider directories may move with source layout.
                    # Their actual dependencies remain bound by each complete scan.
                    items = [item for item in items if not item.startswith("os/src/host/")]
                # These are source sets, emitted in filename order. Renaming a
                # source can reorder them without changing the analysis closure.
                if (source_tree and field in {"sources", "kernelSources", "translationUnits",
                                              "supplemental"}
                        and all(isinstance(item, str) for item in items)):
                    return sorted(items)
                return items
            return value

        previous = _read(baseline / "summary.json")
        old_root = baseline / "project"
        _complete(current)
        _complete(previous)
        for field in ("version", "expectedSourceRevision", "addons"):
            if current["tool"][field] != previous["tool"][field]:
                raise ValueError(f"Tool identity differs: {field}")
        if current["coverage"] != previous["coverage"]:
            raise ValueError("Checker coverage differs")
        if digest(old_root / FILE_HASHES) != previous["sourceSealSha256"]:
            raise ValueError("Baseline source seal identity differs")
        if any(digest(root / FILE_HASHES) != current["sourceSealSha256"]
               for root in (project, output / "project")):
            raise ValueError("Current source seal identity differs")
        # Source bytes may change; the selected target/configuration/link closure may not.
        def target_configuration(path, scope):
            value = _read(path / "target.json")
            if source_tree:
                # These digests include the producer's source bytes. The complete
                # selected plan and all remaining target configuration are compared below.
                value = {key: item for key, item in value.items()
                         if key not in {"definitionFingerprint", "inputSha256"}}
                if scope["patchedKernel"] != value.pop("kernelPatches", []):
                    raise ValueError("Kernel patch receipt differs from the analyzed target")
                if "kernelSha256" in value:
                    # Compare the selected files, after checking each tree's own bytes.
                    require_kernel_identity(path / "kernel", value["kernelSha256"])
                    value["kernelSha256"] = sorted(value["kernelSha256"])
            return value
        current_target = target_configuration(project, current["scope"])
        previous_target = mapped(target_configuration(old_root, previous["scope"]))
        plan = _configuration(project, source_tree=source_tree)
        previous_plan = mapped(_configuration(old_root, source_tree=source_tree))
        if source_tree and plan.get("modeRuntime") != previous_plan.get("modeRuntime"):
            upgraded = _bswm_baseline_plan(previous_plan, plan)
            if upgraded is not previous_plan:
                digests = _bswm_input_digests(old_root, project, upgraded["modeRuntime"]["channelHandle"])
                previous_plan = {**upgraded, "sources":
                                 _translated_ecuc_digest(upgraded["sources"], "rawSha256", digests)}
                delivery = previous_target.get("nativeDelivery")
                if delivery is not None:
                    previous_target["nativeDelivery"] = {**delivery, "inputSnapshots":
                        _translated_ecuc_digest(delivery["inputSnapshots"], "sha256", digests)}
        r11_transition = source_tree and (previous_target.get("format"), current_target.get("format")) == (
            "autosar-build-target-v3", "autosar-build-target-v4",
        )
        if r11_transition:
            # Match the target across the protocol migration after checking each source seal.
            previous_target.update(format="autosar-build-target-v4", hostBatchProtocol="HostBatchV2")
        can_fd_transition = (source_tree and current_target.get("canFrameProfile") == "classical-and-fd"
                             and "canFrameProfile" not in previous_target)
        if can_fd_transition:
            previous_target["canFrameProfile"] = "classical-and-fd"
        if source_tree:
            match current_target.get("nativeDelivery"), previous_target.get("nativeDelivery"):
                case {"format": "autosar-workbench-handoff-v5" as version}, (
                    {"format": "autosar-workbench-handoff-v4"} as previous_delivery
                ):
                    # v5 can carry inactive applications. The rest of the handoff
                    # still has to match, including its input snapshots and slots.
                    previous_delivery["format"] = version
        if normalized(current_target) != normalized(previous_target):
            raise ValueError("Target/sample configuration differs")
        scope_fields = ("target", "profile", "entries", "compilerFlags", "includePaths",
                        "translationUnits", "supplemental")
        if not source_tree:
            scope_fields += ("patchedKernel",)
        for field in scope_fields:
            if normalized(current["scope"][field], field) != normalized(
                    mapped(previous["scope"][field]), field):
                raise ValueError(f"Target/sample scope differs: {field}")
        for root in (project, old_root):
            if not (root / "meta/integration.json").is_file():
                raise ValueError("Sample generation identity unavailable")
        if r11_transition:
            previous_plan = _r11_baseline_plan(previous_plan)
        if can_fd_transition:
            previous_plan = _can_fd_baseline_plan(previous_plan)
        output_plan = _configuration(output / "project", source_tree=source_tree)
        comparable_plan = normalized(plan)
        normalized_previous = normalized(previous_plan)
        if source_tree and comparable_plan != normalized_previous:
            comparable_plan = normalized(_com_layout_predecessor(plan, previous_plan))
        if comparable_plan != normalized_previous or normalized(plan) != normalized(output_plan):
            raise ValueError("Sample generation configuration differs")
        def programs(summary, rename=False):
            rows = [(row["name"], renames.get(row["entry"], row["entry"])
                     if rename else row["entry"],
                     normalized(mapped(row["translationUnits"]) if rename
                                else row["translationUnits"], "translationUnits"))
                    for row in summary["programs"]]
            return sorted(rows) if source_tree else rows
        if programs(current) != programs(previous, rename=True):
            raise ValueError("Program identity differs")
        current_identity, previous_identity = _semantic(current), _semantic(previous)
        if source_tree:
            _compare_source_sdk(current, previous, renames)
        for field in ("compiler", "defines", "models", "libraries", "analysisCode", "addonRuntime", "sdk"):
            if source_tree and field in {"defines", "sdk"}:
                continue
            previous_value = previous_identity[field]
            if field == "defines":
                previous_value = mapped(previous_value)
            if current_identity[field] != previous_value:
                raise ValueError(f"Semantic identity differs: {field}")
        now = diagnostic_counts(current, output / "project")
        before = Counter()
        for (rule, name, source, program), count in diagnostic_counts(previous, old_root).items():
            before[rule, renames.get(name, name), source, program] += count
        def rows(counts):
            if source_tree:
                return [{"rule": rule, "file": name, "source": source,
                         "program": program, "count": count}
                        for (rule, name, source, program), count in sorted(counts.items())]
            locations = Counter()
            for (rule, name, source, _program), count in counts.items():
                locations[rule, name, source] += count
            return [{"rule": rule, "file": name, "source": source, "count": count}
                    for (rule, name, source), count in sorted(locations.items())]
        return {"new": rows(now - before), "removed": rows(before - now)}
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise ValueError(
            f"C baseline refused: {error}. Download the full c-analysis artifact with its project sources, "
            "or compute the matching baseline locally with c-check; local analysis is still required."
        ) from error
