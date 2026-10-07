"""Repository-local development settings; never write to the user's environment."""

from __future__ import annotations

import json
import os
import sys
import tomllib
from pathlib import Path


def repository_root() -> Path:
    source = Path(__file__).resolve().parents[4]
    if (source / "pyproject.toml").is_file() and (source / "core/resources/official.json").is_file():
        return source
    # A wheel has no source checkout beside site-packages. Its documented entry
    # point is the repository root; do not search other working directories.
    return Path.cwd().resolve()


ROOT = repository_root()
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
    """Official development inputs, independent of test fixtures."""
    definition = root / "core/resources/official.json"
    if not definition.is_file():
        raise FileNotFoundError(f"Repository resource definition is missing: {definition}; run the command from the repository root")
    records = json.loads(definition.read_text(encoding="utf-8"))
    return {item["name"]: (item["environment"], item["path"], item["sha256"]) for item in records}


def archive_path(variable: str, relative: str, root: Path = ROOT) -> Path:
    selected = Path(os.environ.get(variable) or relative).expanduser()
    return selected if selected.is_absolute() else root / selected
