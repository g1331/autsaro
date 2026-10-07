"""Elevated queued Windows acceptance; deny networking only for private copied binaries."""

from __future__ import annotations

import argparse
import base64
import ctypes
import hashlib
import json
import logging
import os
import shutil
import stat
import sys
import time
from pathlib import Path

from ecu_tools.process import ProcessSpec, run_bounded

from autosar_tooling.acceptance.native_token import (
    MediumToken,
    process_identity,
    secure_private_tree,
)
from autosar_tooling.acceptance.native_windows_wfp import PrivateWfp, hash_root, prepare

LOGGER = logging.getLogger(__name__)


def powershell(script: str, evidence: Path, stage: str):
    program = Path(os.environ["SystemRoot"]) / "System32/WindowsPowerShell/v1.0/powershell.exe"
    encoded = base64.b64encode(script.encode("utf-16-le")).decode("ascii")
    return run_bounded(ProcessSpec.seconds(
        [str(program), "-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", encoded],
        evidence, 60, evidence, stage,
    ))


def prepare_private_inputs(plan: dict, plan_sha256: str) -> dict:
    """Create high-owned immutable copies; obtain actual AppIDs before medium ack."""
    activation = plan["dynamicWfpActivation"]
    caller_sid = plan["callerToken"]["sid"]
    if (activation["authorized"] is not True
            or activation["method"] != "dynamic-wfp-exact-app-non-loopback"
            or activation["callerSid"] != caller_sid
            or activation["handshake"] != "high-private-prepare-medium-hash-ack"):
        raise PermissionError("Hashed envelope does not authorize dynamic WFP preparation")
    token = MediumToken(caller_sid, caller_process=plan["callerProcess"])
    token.close()
    if process_identity(plan["callerProcess"]["pid"]) != plan["callerProcess"]:
        raise RuntimeError("The hashed original medium caller is no longer the same live process")
    controller = process_identity()
    evidence = Path(plan["evidence"]).resolve(strict=True)
    private_root = Path(activation["privateRoot"])
    expected_root = evidence.parent / (evidence.name + "-runtime")
    if private_root != expected_root:
        raise RuntimeError("High-private runtime must be the exact new sibling of its receipt envelope")
    for ancestor in private_root.absolute().parents:
        if ancestor.lstat().st_file_attributes & stat.FILE_ATTRIBUTE_REPARSE_POINT:
            raise RuntimeError(f"High-private runtime ancestor contains a reparse point: {ancestor}")
    private_root.mkdir()
    secure_private_tree(private_root, caller_sid)
    programs = []
    roots = []
    for entry in activation["hashRoots"]:
        source = Path(entry["source"]["path"])
        target = Path(entry["target"])
        if target.parent != private_root:
            raise RuntimeError("High-private tree target is outside the hashed root")
        if hash_root(source, source.parent) != entry["source"]:
            raise RuntimeError("Hashed source tree changed before high-private copy")
        shutil.copytree(source, target)
        secure_private_tree(target, caller_sid)
        copied = hash_root(target, private_root)
        if (copied["sha256"] != entry["source"]["sha256"]
                or copied["files"] != entry["source"]["files"]
                or hash_root(source, source.parent) != entry["source"]):
            raise RuntimeError("High-private source/copy/after hash closure differs")
        roots.append(target)
        programs.extend(sorted(target.rglob("*.exe")))
    context = {"callerProcess": plan["callerProcess"], "controllerProcess": controller,
               "activationNonce": plan["activationNonce"], "controllerPlanSha256": plan_sha256}
    actual = prepare(programs, private_root, hash_roots=roots, activation_context=context)
    digest = hashlib.sha256(json.dumps(actual, sort_keys=True).encode()).hexdigest()
    with open_parent_receipt(evidence, "wfp-prepared.json",
                             plan["receipts"]["wfp-prepared.json"]) as stream:
        write_terminal_status(stream, {"status": "prepared-awaiting-medium-ack", "plan": actual,
                                       "wfpPlanSha256": digest, "controllerPlanSha256": plan_sha256,
                                       "controllerProcess": controller,
                                       "activationNonce": plan["activationNonce"]})
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        if process_identity(plan["callerProcess"]["pid"]) != plan["callerProcess"]:
            raise RuntimeError("Original medium caller died/changed before dynamic WFP activation")
        if (evidence / "cancel-controller.request").exists():
            raise RuntimeError("Controller canceled before dynamic WFP activation")
        try:
            acknowledgement = json.loads((evidence / "wfp-activation-ack.json").read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            acknowledgement = {}
        if acknowledgement.get("activationAuthorized") is True:
            hash_ack = acknowledgement.pop("hashAck")
            if (hashlib.sha256(json.dumps(acknowledgement, sort_keys=True).encode()).hexdigest() != hash_ack
                    or acknowledgement["controllerPlanSha256"] != plan_sha256
                    or acknowledgement["wfpPlanSha256"] != digest
                    or acknowledgement["callerToken"] != plan["callerToken"]
                    or acknowledgement["callerProcess"] != plan["callerProcess"]
                    or acknowledgement["controllerProcess"] != controller
                    or acknowledgement["activationNonce"] != plan["activationNonce"]):
                raise RuntimeError("Medium activation acknowledgement differs from the hashed exact session")
            current = prepare(programs, private_root, hash_roots=roots, activation_context=context)
            if current != actual:
                raise RuntimeError("Actual final AppIDs/hash roots changed after medium acknowledgement")
            return actual
        time.sleep(0.05)
    raise TimeoutError("Actual high-private WFP plan was not acknowledged by its medium caller")


def profile_diagnostic(evidence: Path) -> dict:
    """Read ActiveStore profiles without requiring or changing their enabled values."""
    result = powershell(
        "$ErrorActionPreference='Stop'; ConvertTo-Json -InputObject "
        "@(Get-NetFirewallProfile -PolicyStore ActiveStore | "
        "Select-Object Name,Enabled,AllowLocalFirewallRules)",
        evidence, "readonly-firewall-profile-diagnostic",
    )
    return {"profiles": json.loads(result.stdout.read_text(encoding="utf-8-sig")),
            "stdout": str(result.stdout), "stderr": str(result.stderr),
            "exitCode": result.exit_code, "readOnly": True}


def run(plan: dict, plan_sha256: str) -> int:
    if os.name != "nt":
        raise RuntimeError("Windows dynamic WFP isolation requires its native host")
    if not ctypes.windll.shell32.IsUserAnAdmin():
        raise PermissionError("Only the explicitly authorized high controller may activate dynamic WFP")
    source_checkout = Path(plan["sourceCheckout"]).resolve()
    if source_checkout.exists():
        raise RuntimeError("Original frozen build checkout must be unavailable")
    evidence = Path(plan["evidence"]).resolve(strict=True)
    actual = prepare_private_inputs(plan, plan_sha256)
    root = Path(actual["privateRoot"])
    native = root / "private-app" / Path(plan["binary"]).name
    runtime = root / "private-webview-runtime"
    before = profile_diagnostic(evidence)
    isolation = PrivateWfp(actual)
    names = ("AUTOSAR_NATIVE_OFFLINE_RECEIPT", "AUTOSAR_NATIVE_CANCEL_PATH",
             "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER")
    previous = {name: os.environ.get(name) for name in names}
    try:
        observed = isolation.start(activation_authorized=True)
        if observed["arbitration"]["equalHigherPermitOrCalloutCandidates"]:
            raise RuntimeError("Current equal/higher WFP permit/callout candidates require separate live policy review")
        receipt = evidence / "network-isolation.json"
        receipt.write_text(json.dumps({
            "platform": "windows", "method": actual["method"], "status": "enforced",
            "binary": str(native), "filters": observed["filters"],
            "kernelReadback": observed, "wfpPlan": actual,
            "activeFirewallProfiles": before["profiles"], "profileDiagnostic": before,
            "webviewRuntime": str(runtime), "userNetworkChanged": False,
        }, indent=2), encoding="utf-8")
        os.environ.update({
            "AUTOSAR_NATIVE_OFFLINE_RECEIPT": str(receipt),
            "AUTOSAR_NATIVE_CANCEL_PATH": str(evidence / "cancel-controller.request"),
            "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER": str(runtime),
        })
        from autosar_tooling.acceptance.desktop import run as native_run

        # native_run strictly terminates/waits its attached Job before returning.
        return native_run("windows", native, True, source_checkout, True,
                          caller_process=plan["callerProcess"],
                          output_directory=evidence / "desktop")
    finally:
        primary_error = sys.exception()
        for name, value in previous.items():
            if value is None:
                os.environ.pop(name, None)
            else:
                os.environ[name] = value
        try:
            cleanup = isolation.close()
            after = profile_diagnostic(evidence)
            if after["profiles"] != before["profiles"]:
                raise RuntimeError("Read-only before/after firewall profile diagnostic changed")
            cleanup["profileDiagnosticAfter"] = after
            (evidence / "wfp-cleanup.json").write_text(json.dumps(cleanup, indent=2), encoding="utf-8")
        except BaseException as cleanup_error:
            (evidence / "wfp-cleanup.json").write_text(json.dumps({
                "status": "unconfirmed", "error": str(cleanup_error),
            }, indent=2), encoding="utf-8")
            if primary_error is None:
                raise
            primary_error.add_note(f"Owned dynamic WFP cleanup also failed: {cleanup_error}")
            LOGGER.exception("Owned dynamic WFP cleanup failed; preserving the original failure")
        print(f"Private offline artifacts/logs retained: {evidence}")


def open_parent_receipt(evidence: Path, name: str, identity: dict):
    """Open the medium-precreated object without create/replace/truncate flags."""
    stream = (evidence / name).open("r+", encoding="utf-8")
    observed = os.fstat(stream.fileno())
    if observed.st_dev != identity["device"] or observed.st_ino != identity["inode"]:
        stream.close()
        raise RuntimeError(f"Parent receipt object changed after authorization: {name}")
    return stream


def write_terminal_status(stream, status: dict):
    stream.seek(0)
    stream.write(json.dumps(status, indent=2))
    stream.flush()
    stream.truncate()
    stream.flush()
    os.fsync(stream.fileno())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--control-plan", type=Path, required=True)
    parser.add_argument("--control-plan-sha256", required=True)
    args = parser.parse_args()
    contents = args.control_plan.read_bytes()
    if hashlib.sha256(contents).hexdigest() != args.control_plan_sha256:
        raise RuntimeError("UAC controller plan changed after authorization")
    plan = json.loads(contents)
    evidence = Path(plan["evidence"]).resolve(strict=True)
    status = {"status": "failed", "exitCode": 1, "controllerPid": os.getpid(), "planSha256": args.control_plan_sha256}
    log = open_parent_receipt(evidence, "controller.log", plan["receipts"]["controller.log"])
    try:
        terminal = open_parent_receipt(evidence, "controller-status.json", plan["receipts"]["controller-status.json"])
    except BaseException:
        log.close()
        raise
    original_stdout, original_stderr = sys.stdout, sys.stderr
    sys.stdout = sys.stderr = log
    try:
        parent = plan["callerToken"]
        if parent["elevated"] or not 0x2000 <= parent["integrityRid"] < 0x3000:
            raise RuntimeError("Hashed parent token is not medium/non-elevated")
        token = MediumToken(parent["sid"], caller_process=plan["callerProcess"])
        try:
            status["linkedMediumToken"] = token.linked_identity
            status["callerMediumPrimaryToken"] = token.assert_ordinary(token.handle)
        finally:
            token.close()
        for name, value in plan["tools"].items():
            if value:
                os.environ[name] = value
        code = run(plan, args.control_plan_sha256)
        status.update(status="succeeded" if code == 0 else "failed", exitCode=code)
    except BaseException as error:
        status["error"] = str(error)
        status["errorNotes"] = list(getattr(error, "__notes__", ()))
        LOGGER.exception("Windows native acceptance controller failed")
    finally:
        sys.stdout, sys.stderr = original_stdout, original_stderr
        try:
            log.flush()
            log.truncate()
            os.fsync(log.fileno())
        finally:
            log.close()
        try:
            write_terminal_status(terminal, status)
        finally:
            terminal.close()
    return status["exitCode"]


if __name__ == "__main__":
    raise SystemExit(main())
