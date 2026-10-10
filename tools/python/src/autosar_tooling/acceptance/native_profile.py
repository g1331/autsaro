"""Private real-application inputs; developer tools remain outside the app boundary."""

from __future__ import annotations

import json
import os
import shutil
import sys
from pathlib import Path

from ecu_tools.process import ProcessSpec, run_bounded

from autosar_tooling.config import ROOT


def prepare(
    scratch: Path,
    platform: str,
    installed: bool,
    source_checkout: Path | None,
    builtin_only: bool = False,
) -> None:
    if installed:
        if source_checkout is None or not source_checkout.is_absolute():
            raise RuntimeError(
                "Installed verification requires an absolute unavailable build checkout"
            )
        availability_reason = "not_found"
        stat_errno = None
        try:
            source_available = source_checkout.exists()
        except PermissionError as error:
            source_available = False
            availability_reason = "caller_inaccessible"
            stat_errno = error.errno
        if source_available:
            raise RuntimeError(f"Build checkout must be unavailable: {source_checkout}")
        (scratch / "source-checkout-boundary.json").write_text(
            json.dumps(
                {
                    "sourceCheckout": str(source_checkout),
                    "sourceCheckoutAvailable": False,
                    "availabilityReason": availability_reason,
                    "statErrno": stat_errno,
                    "callerUid": os.getuid() if hasattr(os, "getuid") else None,
                    "callerPid": os.getpid(),
                    "platform": platform,
                },
                indent=2,
            ),
            encoding="utf-8",
        )
    if builtin_only:
        from autosar_tooling.acceptance.native_builtin import prepare as prepare_builtin

        prepare_builtin(scratch, platform, source_checkout, installed=installed)
        return
    fixture = ROOT / "core/tests/fixtures/epic4/positive"
    shutil.copytree(fixture, scratch / "inputs")
    shutil.copytree(fixture, scratch / "expected")
    (scratch / "inputs.json").write_text(
        json.dumps(
            [str(path) for path in sorted((scratch / "inputs").glob("*.arxml"))]
        ),
        encoding="utf-8",
    )
    resources = {}
    for key, name in (
        ("xsdArchive", "AUTOSAR_XSD_ARCHIVE"),
        ("modArchive", "AUTOSAR_MOD_ARCHIVE"),
    ):
        source = Path(os.environ[name]).resolve(strict=True)
        if installed:
            dependency = scratch / "dependencies" / source.name
            dependency.parent.mkdir(exist_ok=True)
            shutil.copyfile(source, dependency)
            source = dependency
        resources[key] = str(source)
    (scratch / "resources.json").write_text(json.dumps(resources), encoding="utf-8")
    (scratch / "scenario.json").write_text(
        json.dumps(
            {
                "installed": installed,
                "sourceCheckout": str(source_checkout) if source_checkout else None,
            }
        ),
        encoding="utf-8",
    )
    (scratch / "app-work").mkdir()
    if platform != "macos":
        if installed:
            from autosar_tooling.acceptance.native_consumer import hashes

            reference = Path(os.environ["AUTOSAR_NATIVE_LEGACY_REFERENCE"]).resolve(
                strict=True,
            )
            before = hashes(reference)
            destination = scratch / "legacy"
            shutil.copytree(reference, destination)
            copied = hashes(destination)
            after = hashes(reference)
            if before != copied or before != after:
                raise RuntimeError("Declared genuine v1 reference changed during private copy")
            (scratch / "legacy-reference-copy.json").write_text(
                json.dumps({
                    "source": str(reference),
                    "destination": str(destination),
                    "before": before,
                    "copied": copied,
                    "after": after,
                }, indent=2),
                encoding="utf-8",
            )
        suffix = ".exe" if platform == "windows" else ""
        target = (
            "windows-x64-controlled-v1"
            if platform == "windows"
            else "linux-x64-controlled-v1"
        )
        if not installed:
            reference_result = run_bounded(
                ProcessSpec.seconds(
                    [
                        str(ROOT / f"core/target/debug/package_host_reference{suffix}"),
                        str(scratch / "legacy"),
                        "--target",
                        target,
                    ],
                    ROOT,
                    180,
                    scratch,
                    "native-reference",
                )
            )
            if not reference_result.success:
                raise RuntimeError(reference_result.stderr.read_text(encoding="utf-8"))
        # Current ARXML import uses native definitions. Exercise legacy editing
        # through the supported sealed v1 handoff entry, with the same inputs.
        command = [
            str(ROOT / f"core/target/debug/generate_epic4_ecu{suffix}"),
            "--handoff", "--target", target,
            "--output", str(scratch / "legacy-ecu"),
        ]
        for source in sorted((scratch / "inputs").glob("*.arxml")):
            command += ["--input", str(source)]
        preview = run_bounded(ProcessSpec.seconds(
            command, ROOT, 180, scratch, "native-legacy-ecu-preview",
        ))
        if not preview.success:
            raise RuntimeError(preview.stderr.read_text(encoding="utf-8"))
        revision = json.loads(preview.stdout.read_text(encoding="utf-8"))["revision"]
        generated = run_bounded(ProcessSpec.seconds(
            [*command, "--write", "--revision", revision],
            ROOT, 180, scratch, "native-legacy-ecu-generate",
        ))
        if not generated.success:
            raise RuntimeError(generated.stderr.read_text(encoding="utf-8"))


def app_environment(
    scratch: Path, installed: bool, builtin_only: bool = False
) -> dict[str, str]:
    if builtin_only:
        from autosar_tooling.acceptance.native_builtin import (
            app_environment as builtin_environment,
        )

        return builtin_environment(scratch, installed)
    environment = os.environ.copy()
    for name in (
        "AUTOSAR_XSD_ARCHIVE",
        "AUTOSAR_MOD_ARCHIVE",
        "AUTOSAR_NATIVE_LEGACY_REFERENCE",
    ):
        environment.pop(name, None)
    environment.update(
        {
            "AUTOSAR_CONFIG_DIR": str(scratch / "app-config"),
            "APPDATA": str(scratch / "app-config"),
            "LOCALAPPDATA": str(scratch / "app-local"),
            "XDG_CONFIG_HOME": str(scratch / "config"),
            "XDG_DATA_HOME": str(scratch / "data"),
        }
    )
    if installed:
        for name in tuple(environment):
            if name in {
                "VIRTUAL_ENV",
                "PYTHONPATH",
                "PYTHONHOME",
                "NODE_OPTIONS",
            } or name.startswith(("UV_", "CARGO_", "RUSTUP_", "NPM_CONFIG_")):
                environment.pop(name, None)
        python = Path(sys._base_executable).resolve(strict=True)
        if python.is_relative_to(ROOT):
            raise RuntimeError(
                "Installed app requires external declared CPython, not the checkout venv"
            )
        environment["AUTOSAR_PYTHON"] = str(python)
        directories = [
            Path(environment[name]).parent for name in ("AUTOSAR_CC", "AUTOSAR_GIT")
        ]
        if os.name == "nt":
            system = Path(os.environ["SystemRoot"])
            directories.extend((system / "System32", system))
        else:
            directories.extend((Path("/usr/bin"), Path("/bin")))
        environment["PATH"] = os.pathsep.join(
            dict.fromkeys(str(path) for path in directories)
        )
        for tool in ("node", "npm", "uv", "cargo", "rustc"):
            if shutil.which(tool, path=environment["PATH"]):
                raise RuntimeError(
                    f"Installed application PATH still exposes developer tool: {tool}"
                )
    receipt = {
        name: environment[name]
        for name in (
            "AUTOSAR_CONFIG_DIR",
            "AUTOSAR_CC",
            "AUTOSAR_OBJDUMP",
            "AUTOSAR_GIT",
            "AUTOSAR_PYTHON",
            "PATH",
        )
        if name in environment
    }
    (scratch / "application-environment.json").write_text(
        json.dumps({"installed": installed, "environment": receipt}, indent=2),
        encoding="utf-8",
    )
    return environment
