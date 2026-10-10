"""Builtin native configuration boundary, with explicit installed/development receipts."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
from pathlib import Path

DEVELOPER_TOOLS = (
    "node", "npm", "npx", "uv", "cargo", "rustc", "rustup", "python",
    "python3", "gcc", "cc", "clang", "cl", "g++", "git", "objdump", "nm",
)
EXECUTION_VARIABLES = (
    "AUTOSAR_CC", "AUTOSAR_OBJDUMP", "AUTOSAR_NM", "AUTOSAR_GIT",
    "AUTOSAR_PYTHON", "AUTOSAR_XSD_ARCHIVE", "AUTOSAR_MOD_ARCHIVE",
)
LINUX_RUNTIME_UTILITIES = (
    "bash", "readlink", "dirname", "realpath", "dbus-send", "tail", "cut", "gsettings",
)


def prepare(
    scratch: Path, platform: str, source_checkout: Path | None, *, installed: bool = True
) -> None:
    performance_tools = {}
    for field, variable in (
        ("compiler", "AUTOSAR_CC"), ("objdump", "AUTOSAR_OBJDUMP"),
        ("git", "AUTOSAR_GIT"), ("python", "AUTOSAR_PYTHON"),
    ):
        value = os.environ.get(variable)
        if value is None or not Path(value).is_absolute() or not Path(value).is_file():
            raise RuntimeError(
                f"Builtin native consumer phase requires {variable} to name an existing absolute tool file"
            )
        performance_tools[field] = value
    # Installed scenarios use only the application's original templates.
    # Development scenarios may additionally open source-owned member projects.
    for name in ("app-work", "app-path", "app-home", "app-temp", "projects", "deliveries"):
        (scratch / name).mkdir()
    if platform == "linux":
        utilities = []
        for name in LINUX_RUNTIME_UTILITIES:
            selected = shutil.which(name, path="/usr/bin:/bin")
            if selected is None:
                raise RuntimeError(f"Installed AppRun requires OS runtime utility: {name}")
            binary = Path(selected).resolve(strict=True)
            entry = scratch / "app-path" / name
            entry.symlink_to(binary)
            with binary.open("rb") as stream:
                sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
            source_stat = binary.stat()
            entry_stat = entry.lstat()
            utilities.append({
                "name": name, "binary": str(binary), "sha256": sha256,
                "sourceUid": source_stat.st_uid, "sourceGid": source_stat.st_gid,
                "privateEntry": str(entry), "entryUid": entry_stat.st_uid,
                "entryGid": entry_stat.st_gid, "entryKind": "symlink",
            })
        (scratch / "runtime-utilities.json").write_text(
            json.dumps({"platform": platform, "utilities": utilities}, indent=2),
            encoding="utf-8",
        )
    offline = os.environ.get("AUTOSAR_NATIVE_OFFLINE_RECEIPT")
    if offline is not None:
        receipt = json.loads(Path(offline).read_text(encoding="utf-8"))
        if receipt.get("platform") != platform or receipt.get("status") != "enforced":
            raise RuntimeError("Native offline receipt does not match the isolated host")
        (scratch / "network-isolation.json").write_text(
            json.dumps(receipt, indent=2), encoding="utf-8"
        )
    (scratch / "scenario.json").write_text(
        json.dumps({
            "mode": "builtin-only", "installed": installed,
            "sourceCheckout": str(source_checkout) if source_checkout else None, "platform": platform,
            "execution": "not_run", "macos": "not_run",
            "performanceTools": performance_tools,
        }, indent=2), encoding="utf-8",
    )
    (scratch / "boundary-results.json").write_text(
        json.dumps({
            "officialResources": "not_supplied",
            "sourceCheckoutAvailable": not installed,
            "networkIsolation": "not_run",
            "independentConsumer": "not_run",
            "macos": "not_run",
        }, indent=2), encoding="utf-8",
    )


def app_environment(scratch: Path, installed: bool) -> dict[str, str]:
    environment = os.environ.copy()
    for name in tuple(environment):
        if name in EXECUTION_VARIABLES or name in {
            "VIRTUAL_ENV", "PYTHONPATH", "PYTHONHOME", "NODE_OPTIONS",
            "ECU_OWNER_SOCKET", "ECU_OWNER_TOKEN", "ECU_OWNER_SCOPE", "GTK_THEME",
            "AUTOSAR_NATIVE_OFFLINE_RECEIPT",
        } or name.startswith(("UV_", "CARGO_", "RUSTUP_", "NPM_CONFIG_")):
            environment.pop(name, None)
    environment.update({
        "AUTOSAR_CONFIG_DIR": str(scratch / "app-config"),
        "APPDATA": str(scratch / "app-config"),
        "LOCALAPPDATA": str(scratch / "app-local"),
        "XDG_CONFIG_HOME": str(scratch / "config"),
        "XDG_DATA_HOME": str(scratch / "data"),
        "HOME": str(scratch / "app-home"),
        "USERPROFILE": str(scratch / "app-home"),
        "TEMP": str(scratch / "app-temp"),
        "TMP": str(scratch / "app-temp"),
        "TMPDIR": str(scratch / "app-temp"),
        "PATH": str(scratch / "app-path"),
    })
    for action, delay in (
        ("SOURCE_READ", "500"),
        ("SNAPSHOT_BUILD", "250"),
        ("SCHEMA_CHECK", "100"),
        ("SOURCE_SCAN", "250"),
        ("DEFINITION_READ", "100"),
    ):
        # Accepted only by the native-webdriver verification feature. These
        # delay real work; the uninstrumented production path ignores them.
        environment[f"AUTOSAR_VERIFY_DELAY_{action}_MS"] = delay
    exposed = {name: shutil.which(name, path=environment["PATH"]) for name in DEVELOPER_TOOLS}
    if any(exposed.values()):
        raise RuntimeError(f"Builtin-only application exposes developer tools: {exposed}")
    receipt = {
        "mode": "builtin-only", "installed": installed,
        "environment": {name: environment[name] for name in (
            "AUTOSAR_CONFIG_DIR", "HOME", "USERPROFILE", "PATH",
        )},
        "absentVariables": list(EXECUTION_VARIABLES), "tools": exposed,
        "verificationDelays": {
            name: value for name, value in environment.items()
            if name.startswith("AUTOSAR_VERIFY_DELAY_")
        },
    }
    (scratch / "application-environment.json").write_text(
        json.dumps(receipt, indent=2), encoding="utf-8",
    )
    return environment
