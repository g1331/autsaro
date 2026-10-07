"""One explicitly authorized UAC prompt for a bounded, isolated native controller."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import secrets
import subprocess
import sys
import tempfile
from ctypes import wintypes as w
from pathlib import Path

from autosar_tooling.acceptance.native_token import (
    process_identity,
    secure_private_tree,
    token_identity,
)
from autosar_tooling.acceptance.native_windows_wfp import hash_root, prepare


class ShellExecuteInfo(ctypes.Structure):
    _fields_ = [
        ("size", w.DWORD), ("mask", w.ULONG), ("window", w.HWND),
        ("verb", w.LPCWSTR), ("file", w.LPCWSTR), ("parameters", w.LPCWSTR),
        ("directory", w.LPCWSTR), ("show", ctypes.c_int), ("instance", w.HINSTANCE),
        ("idList", w.LPVOID), ("className", w.LPCWSTR), ("classKey", w.HKEY),
        ("hotKey", w.DWORD), ("icon", w.HANDLE), ("process", w.HANDLE),
    ]


def prepare_envelope(binary: Path, runtime: Path, checkout: Path, *, activation_authorized: bool = False):
    """Hash source trees and a high-private/medium-ack activation contract."""
    binary = binary.resolve(strict=True)
    runtime = runtime.resolve(strict=True)
    checkout = checkout.resolve()
    if checkout.exists():
        raise RuntimeError("Frozen build checkout must already be unavailable")
    if not (runtime / "msedgewebview2.exe").is_file():
        raise RuntimeError("Provide an existing licensed fixed WebView2 runtime folder")
    caller = token_identity()
    if caller["elevated"] or not 0x2000 <= caller["integrityRid"] < 0x3000:
        raise RuntimeError("UAC envelope must originate from the actual medium caller")
    evidence = Path(tempfile.mkdtemp(prefix="autosar-native-elevated-"))
    private_root = evidence.parent / (evidence.name + "-runtime")
    receipts = {}
    for name in ("controller-status.json", "controller.log", "wfp-prepared.json", "wfp-activation-ack.json"):
        path = evidence / name
        with path.open("x", encoding="utf-8") as stream:
            stream.write("{}" if name.endswith(".json") else "")
            stream.flush()
            os.fsync(stream.fileno())
        identity = path.stat()
        receipts[name] = {"device": identity.st_dev, "inode": identity.st_ino}
    plan = {
        "binary": str(binary), "webviewRuntime": str(runtime),
        "sourceCheckout": str(checkout), "evidence": str(evidence),
        "callerToken": caller, "receipts": receipts,
        "callerProcess": process_identity(), "activationNonce": secrets.token_hex(32),
        "dynamicWfpActivation": {
            "authorized": activation_authorized, "method": "dynamic-wfp-exact-app-non-loopback",
            "callerSid": caller["sid"], "handshake": "high-private-prepare-medium-hash-ack",
            "privateRoot": str(private_root),
            "targetPathStatus": "INTENDED-not-yet-created",
            "hashRoots": [
                {"target": str(private_root / "private-app"),
                 "source": hash_root(binary.parent, binary.parent.parent)},
                {"target": str(private_root / "private-webview-runtime"),
                 "source": hash_root(runtime, runtime.parent)},
            ],
        },
        "tools": {name: os.environ.get(name) for name in (
            "AUTOSAR_CC", "AUTOSAR_OBJDUMP", "AUTOSAR_GIT", "AUTOSAR_PYTHON",
        )},
    }
    secure_private_tree(evidence, caller["sid"])
    content = json.dumps(plan, sort_keys=True).encode()
    plan_path = evidence / "controller-plan.json"
    plan_path.write_bytes(content)
    return evidence, plan_path, hashlib.sha256(content).hexdigest()


def acknowledge_prepared(evidence: Path, plan: dict, digest: str):
    """Authorize only actual high-created paths, AppIDs and the hashed input trees."""
    if plan["dynamicWfpActivation"]["authorized"] is not True:
        raise PermissionError("A declarative envelope cannot authorize WFP activation")
    from autosar_tooling.acceptance.native_windows_offline import (
        open_parent_receipt,
        write_terminal_status,
    )

    path = evidence / "wfp-prepared.json"
    try:
        contents = path.read_text(encoding="utf-8")
        receipt = json.loads(contents)
    except json.JSONDecodeError:
        return False  # In-place writer may not yet have flushed/truncated.
    if receipt.get("status") != "prepared-awaiting-medium-ack":
        return False
    actual = receipt["plan"]
    if receipt["controllerPlanSha256"] != digest or actual["callerSid"] != plan["callerToken"]["sid"]:
        raise RuntimeError("High-private preparation differs from the hashed caller/envelope")
    if process_identity() != plan["callerProcess"]:
        raise RuntimeError("Activation acknowledgement must originate from the exact live medium caller")
    controller = receipt["controllerProcess"]
    if (process_identity(controller["pid"]) != controller
            or controller["token"]["sid"] != plan["callerToken"]["sid"]
            or not controller["token"]["elevated"]
            or receipt["activationNonce"] != plan["activationNonce"]):
        raise RuntimeError("Prepared receipt differs from the actual live high controller/session")
    if actual["privateRoot"] != plan["dynamicWfpActivation"]["privateRoot"]:
        raise RuntimeError("Actual high-private root differs from the intended hashed root")
    context = {"callerProcess": plan["callerProcess"], "controllerProcess": controller,
               "activationNonce": plan["activationNonce"], "controllerPlanSha256": digest}
    if actual["activationContext"] != context:
        raise RuntimeError("Actual final WFP plan differs from its original caller/controller/nonce context")
    roots = plan["dynamicWfpActivation"]["hashRoots"]
    expected_paths = []
    for expected, observed in zip(roots, actual["hashRoots"], strict=True):
        if (observed["path"] != expected["target"]
                or observed["sha256"] != expected["source"]["sha256"]
                or observed["files"] != expected["source"]["files"]):
            raise RuntimeError("High-private immutable tree differs from the hashed source")
        expected_paths.extend(str(Path(expected["target"]) / entry["path"])
                              for entry in observed["files"] if entry["path"].lower().endswith(".exe"))
    if {entry["path"] for entry in actual["programs"]} != set(expected_paths):
        raise RuntimeError("High-private AppID inventory differs from the exact declared executables")
    current = prepare([Path(entry["path"]) for entry in actual["programs"]], Path(actual["privateRoot"]),
                      hash_roots=[Path(entry["path"]) for entry in actual["hashRoots"]],
                      activation_context=context)
    if current != actual:
        raise RuntimeError("Actual high-private paths, AppIDs or hash roots changed before acknowledgement")
    wfp_digest = hashlib.sha256(json.dumps(actual, sort_keys=True).encode()).hexdigest()
    if receipt["wfpPlanSha256"] != wfp_digest:
        raise RuntimeError("High-private actual WFP plan hash differs")
    acknowledgement = {
        "controllerPlanSha256": digest, "wfpPlanSha256": wfp_digest,
        "callerToken": token_identity(), "callerProcess": plan["callerProcess"],
        "controllerProcess": controller, "activationNonce": plan["activationNonce"],
        "activationAuthorized": True,
    }
    acknowledgement["hashAck"] = hashlib.sha256(
        json.dumps(acknowledgement, sort_keys=True).encode()
    ).hexdigest()
    with open_parent_receipt(evidence, "wfp-activation-ack.json",
                             plan["receipts"]["wfp-activation-ack.json"]) as stream:
        write_terminal_status(stream, acknowledgement)
    return True


def launch(binary: Path, runtime: Path, checkout: Path, *, activation_authorized: bool = False) -> int:
    if os.name != "nt":
        raise RuntimeError("UAC controller launcher requires Windows")
    if not activation_authorized:
        raise PermissionError("A reviewed dynamic-WFP activation grant is required")
    binary = binary.resolve(strict=True)
    runtime = runtime.resolve(strict=True)
    checkout = checkout.resolve()
    if checkout.exists():
        raise RuntimeError("Frozen build checkout must already be unavailable")
    evidence, plan_path, digest = prepare_envelope(binary, runtime, checkout,
                                                 activation_authorized=activation_authorized)
    plan = json.loads(plan_path.read_text(encoding="utf-8"))
    arguments = ["-m", "autosar_tooling.acceptance.native_windows_offline", "--control-plan", str(plan_path), "--control-plan-sha256", digest]
    shell = ctypes.WinDLL("shell32", use_last_error=True)
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    shell.ShellExecuteExW.argtypes = [ctypes.POINTER(ShellExecuteInfo)]
    kernel.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
    kernel.GetExitCodeProcess.argtypes = [w.HANDLE, ctypes.POINTER(w.DWORD)]
    kernel.CloseHandle.argtypes = [w.HANDLE]
    request = ShellExecuteInfo()
    request.size = ctypes.sizeof(request)
    request.mask = 0x40 | 0x100 | 0x400  # NOCLOSEPROCESS, NOASYNC, no unrelated error UI.
    request.verb = "runas"
    request.file = sys.executable
    request.parameters = subprocess.list2cmdline(arguments)
    request.directory = str(Path.cwd())
    request.show = 0
    print(f"User-approved UAC controller plan SHA-256={digest}; evidence={evidence}", flush=True)
    if not shell.ShellExecuteExW(ctypes.byref(request)):
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        status = 0x102
        acknowledged = False
        for _ in range(1800):
            status = kernel.WaitForSingleObject(request.process, 1000)
            if status != 0x102:
                break
            if not acknowledged:
                acknowledged = acknowledge_prepared(evidence, plan, digest)
        if status == 0x102:
            (evidence / "cancel-controller.request").write_text("bounded parent deadline", encoding="utf-8")
            status = kernel.WaitForSingleObject(request.process, 30_000)
            if status == 0x102:
                raise RuntimeError(f"Elevated controller cleanup unconfirmed after bounded cancellation; evidence={evidence}")
        if status != 0:
            raise ctypes.WinError(ctypes.get_last_error())
        code = w.DWORD()
        if not kernel.GetExitCodeProcess(request.process, ctypes.byref(code)):
            raise ctypes.WinError(ctypes.get_last_error())
        receipt = json.loads((evidence / "controller-status.json").read_text(encoding="utf-8"))
        if receipt["exitCode"] != code.value:
            raise RuntimeError("Elevated controller process exit differs from its actual receipt")
        if receipt["planSha256"] != digest:
            raise RuntimeError("Elevated controller receipt differs from the authorized plan hash")
        print(json.dumps(receipt, indent=2))
        return code.value
    except BaseException as error:
        (evidence / "cancel-controller.request").write_text(
            "bounded medium controller refusal/cancellation", encoding="utf-8"
        )
        if kernel.WaitForSingleObject(request.process, 30_000) == 0x102:
            error.add_note(f"Elevated controller cleanup unconfirmed after cancellation; evidence={evidence}")
        raise
    finally:
        kernel.CloseHandle(request.process)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--webview-runtime", type=Path, required=True)
    parser.add_argument("--source-checkout", type=Path, required=True)
    parser.add_argument("--activate-dynamic-wfp", action="store_true",
                        help="Use only with a separately reviewed activation and product-launch grant")
    args = parser.parse_args()
    return launch(args.binary, args.webview_runtime, args.source_checkout,
                  activation_authorized=args.activate_dynamic_wfp)


if __name__ == "__main__":
    raise SystemExit(main())
