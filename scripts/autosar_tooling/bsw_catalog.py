"""Materialize the reviewed current host BSW declaration/source inventory.

Only the selected plain C declarations below are accepted. This is an explicit
contract manifest, not a general C parser or a standard-conformity result.
Default operation verifies the checked-in inventory; --write updates it after
reviewed runtime changes. Product parsing never reads fixture expectations.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "runtime/contracts/bsw-v1.json"
SELECTED = {
    "Can_Init": "Can",
    "Can_MainFunction_Wakeup": "Can",
    "Can_MainFunction_Read": "Can",
    "Can_MainFunction_Write": "Can",
    "CanIf_Init": "CanIf",
    "CanIf_Transmit": "CanIf",
    "PduR_Init": "PduR",
    "PduR_Transmit": "PduR",
    "CanTp_Init": "CanTp",
    "CanTp_AdvanceTime": "CanTp",
    "Com_Init": "Com",
    "Com_SetSignal": "Com",
    "Com_GetSignal": "Com",
    "Com_AdvanceTime": "Com",
    "Com_TriggerTransmit": "Com",
    "Dcm_Init": "Dcm",
    "Dcm_AdvanceTime": "Dcm",
}
OUTPUTS = {("Com_GetSignal", "value"), ("Com_GetSignal", "valid")}


def materialize(root: Path = ROOT) -> dict:
    entries = {}
    sources = {}

    def record(path: Path) -> str:
        relative = path.relative_to(root).as_posix()
        sources[relative] = hashlib.sha256(path.read_bytes()).hexdigest()
        return relative

    for symbol, module in sorted(SELECTED.items()):
        headers = sorted((root / "runtime/include").glob(f"{module}*.h"))
        scheduled = root / f"runtime/include/SchM_{module}.h"
        if scheduled.is_file():
            headers.append(scheduled)
        matches = []
        for header in headers:
            declaration = re.search(
                r"^([A-Za-z_]\w*)\s+" + re.escape(symbol) + r"\s*\(([^;{}]*)\)\s*;",
                header.read_text(encoding="utf-8"),
                re.MULTILINE,
            )
            if declaration:
                matches.append((header, declaration))
        if len(matches) != 1:
            raise ValueError(f"selected declaration is missing/duplicated: {symbol}")
        header, declaration = matches[0]
        source = root / f"runtime/src/{module}.c"
        definition = re.findall(
            r"^[A-Za-z_]\w*\s+" + re.escape(symbol) + r"\s*\([^;{}]*\)\s*\{",
            source.read_text(encoding="utf-8"),
            re.MULTILINE,
        )
        if len(definition) != 1:
            raise ValueError(
                f"selected source producer is missing/duplicated: {symbol}"
            )
        arguments = []
        if declaration[2].strip() != "void":
            for argument in declaration[2].split(","):
                parsed = re.fullmatch(
                    r"(.*?)\b([A-Za-z_]\w*)\s*(\[\d+\])?", argument.strip()
                )
                if parsed is None:
                    raise ValueError(
                        f"declaration exceeds the selected plain C grammar: {symbol}"
                    )
                native_type = parsed[1].strip() + (" *" if parsed[3] else "")
                arguments.append(
                    {
                        "name": parsed[2],
                        "nativeType": " ".join(native_type.replace("*", " * ").split()),
                        "direction": "OUT" if (symbol, parsed[2]) in OUTPUTS else "IN",
                    }
                )
        entries[symbol] = {
            "module": module,
            "returnType": declaration[1],
            "arguments": arguments,
            "header": record(header),
            "source": record(source),
            "stage": "current_host_bsw",
        }
    # Record the local type/header closure used by these public declarations.
    pending = list(sources)
    while pending:
        relative = pending.pop()
        text = (root / relative).read_text(encoding="utf-8")
        for name in re.findall(r'^\s*#include\s+"([^"/\\]+)"', text, re.MULTILINE):
            dependency = root / "runtime/include" / name
            if dependency.is_file():
                identity = dependency.relative_to(root).as_posix()
                if identity not in sources:
                    record(dependency)
                    pending.append(identity)
    return {
        "formatVersion": 1,
        "profile": "epic4-win64-sr-cs-v1",
        "scope": "Current selected host BSW declarations and actual source producers; not standard signature certification or new target link evidence",
        "entries": entries,
        "sources": dict(sorted(sources.items())),
    }


def probe(current: dict) -> None:
    from autosar_tooling.os_suites import compiler, native_session, run_native

    with tempfile.TemporaryDirectory(prefix="autosar-epic4-bsw-contract-") as temporary:
        directory = Path(temporary)
        source = directory / "current_bsw_contract.c"
        headers = sorted({entry["header"] for entry in current["entries"].values()})
        lines = ["#include <stddef.h>"]
        lines.extend('#include "' + Path(header).name + '"' for header in headers)
        # This address/type consumer initializes no ECU and has no routes.
        # Behavioral profile consumers compile the real generated producers.
        lines.extend(
            [
                "const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};",
                "const EcuReceiveRoute *const Ecu_ReceiveRoutes = NULL;",
                "const size_t Ecu_ReceiveRouteCount = 0u;",
                "const EcuTransmitRoute *const Ecu_TransmitRoutes = NULL;",
                "const size_t Ecu_TransmitRouteCount = 0u;",
            ]
        )
        for symbol, entry in current["entries"].items():
            arguments = (
                ", ".join(argument["nativeType"] for argument in entry["arguments"])
                or "void"
            )
            lines.append(
                f"static {entry['returnType']} (*const bind_{symbol})({arguments}) = {symbol};"
            )
        predicates = " || ".join(
            f"(bind_{symbol} == NULL)" for symbol in current["entries"]
        )
        lines.append("int main(void) { return " + predicates + "; }")
        source.write_text("\n".join(lines) + "\n", encoding="utf-8")
        binary = directory / (
            "current_bsw_contract.exe" if os.name == "nt" else "current_bsw_contract"
        )
        inputs = sorted(
            path
            for path in (ROOT / "runtime/src").glob("*.c")
            if path.name != "ecu_host_main.c"
        )
        inputs.extend(sorted((ROOT / "runtime/host/src").glob("*.c")))
        target = (
            "windows-x64-controlled-v1"
            if os.name == "nt"
            else "linux-x64-controlled-v1"
        )
        with native_session(directory, "bsw-contract"):
            cc = compiler(target)
            command = [
                cc,
                "-std=c99",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-I" + str(ROOT / "runtime/include"),
                "-I" + str(ROOT / "runtime/src"),
                "-I" + str(ROOT / "runtime/host/include"),
                str(source),
                *map(str, inputs),
                *(["-lbcrypt"] if os.name == "nt" else []),
                "-o",
                str(binary),
            ]
            built = run_native(command, timeout=60)
            if built.returncode != 0 or built.stderr:
                raise ValueError(
                    f"current BSW contract link failed: {built.returncode}: {built.stderr}"
                )
            ran = run_native([str(binary)], timeout=5)
        if ran.returncode != 0 or ran.stdout or ran.stderr:
            raise ValueError("current BSW address/type probe failed")


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--probe", action="store_true")
    args = parser.parse_args(argv)
    current = materialize()
    if args.write:
        MANIFEST.parent.mkdir(parents=True, exist_ok=True)
        MANIFEST.write_text(json.dumps(current, indent=2) + "\n", encoding="utf-8")
    else:
        recorded = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if recorded != current:
            raise ValueError(
                "BSW inventory differs; review runtime changes before explicitly regenerating"
            )
    if args.probe:
        probe(current)
    print(f"bsw_catalog PASS: {len(current['entries'])} selected current producers")


if __name__ == "__main__":
    main()
