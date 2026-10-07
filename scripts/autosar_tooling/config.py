"""Repository-local development settings; never write to the user's environment."""

from __future__ import annotations

import json
import os
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ALLOWED_VARIABLES = {
    "AUTOSAR_XSD_ARCHIVE",
    "AUTOSAR_MOD_ARCHIVE",
    "AUTOSAR_SAMPLE_ARCHIVE",
    "AUTOSAR_CC",
    "AUTOSAR_OBJDUMP",
    "AUTOSAR_NM",
    "AUTOSAR_GIT",
    "AUTOSAR_PYTHON",
    "VCPKG_ROOT",
    "VCPKGRS_TRIPLET",
    "LIBCLANG_PATH",
    "RUST_TEST_THREADS",
}
PATH_VARIABLES = ALLOWED_VARIABLES - {"VCPKGRS_TRIPLET", "RUST_TEST_THREADS"}


def environment(root: Path = ROOT) -> dict[str, str]:
    env = dict(os.environ)
    path = root / "dev.local.toml"
    if path.exists():
        settings = tomllib.loads(path.read_text(encoding="utf-8"))
        if set(settings) - {"environment"}:
            raise ValueError("dev.local.toml only accepts an [environment] table")
        values = settings.get("environment", {})
        if not isinstance(values, dict):
            raise ValueError("dev.local.toml: environment must be a table")
        for name, value in values.items():
            if name not in ALLOWED_VARIABLES or not isinstance(value, str) or not value:
                raise ValueError(f"Invalid development setting: {name}")
            if name in PATH_VARIABLES:
                selected = Path(value).expanduser()
                value = str(selected if selected.is_absolute() else root / selected)
            env.setdefault(name, value)
    env.setdefault("AUTOSAR_PYTHON", str(Path(sys.executable).absolute()))
    env.setdefault("RUST_TEST_THREADS", "2")
    env.setdefault("PYTHONUTF8", "1")
    return env


def archives(root: Path = ROOT) -> dict[str, tuple[str, str, str | None]]:
    """Read XSD identity from the same constants used by the Rust validator."""
    schema = (root / "core/src/schema.rs").read_text(encoding="utf-8")
    relative = re.search(r'pub const SCHEMA_ZIP: &str\s*=\s*"([^"]+)"', schema)
    digest = re.search(r'pub const XSD_SHA256: &str\s*=\s*"([^"]+)"', schema)
    if not relative or not digest:
        raise ValueError("Cannot read the core XSD identity")

    def constant(name: str) -> str:
        match = re.search(r"pub const " + name + r': &str\s*=\s*"([^"]+)"', schema)
        if not match:
            raise ValueError(f"Cannot read core resource path: {name}")
        return match[1]

    fixture = json.loads(
        (root / "core/tests/fixtures/epic4/manifest.json").read_text(encoding="utf-8")
    )["external_mod"]
    return {
        "R24-11 XSD": ("AUTOSAR_XSD_ARCHIVE", relative[1], digest[1]),
        "R24-11 MOD": (
            "AUTOSAR_MOD_ARCHIVE",
            constant("MOD_ZIP"),
            fixture["sha256"],
        ),
        "R24-11 samples": (
            "AUTOSAR_SAMPLE_ARCHIVE",
            constant("SAMPLE_ZIP"),
            None,
        ),
    }


def archive_path(variable: str, relative: str, root: Path = ROOT) -> Path:
    selected = Path(os.environ.get(variable) or relative).expanduser()
    return selected if selected.is_absolute() else root / selected
