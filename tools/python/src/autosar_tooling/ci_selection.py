"""Shared changed-input classification for required CI and local verification."""

DOCUMENTATION = (
    "docs/", ".agents/", ".claude/", ".codex/", ".impeccable/", ".github/ISSUE_TEMPLATE/",
)
DRIVERS = {
    "tools/python/src/autosar_tooling/" + name + ".py"
    for name in ("c_check", "c_acceptance", "c_sdk", "c_comparison", "c_addon")
}
GUIDELINE_INDEX = ".agents/skills/misra-c2012/references/baseline-index.json"
DESKTOP_PATHS = (
    "ui/", "src-tauri/", "core/", "runtime/", "tests/desktop/", "tools/python/",
    "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".node-version", ".python-version",
    "pyproject.toml", "uv.lock", "LICENSE", "NOTICE", ".github/actions/",
    ".github/workflows/checks.yml", ".github/workflows/desktop.yml",
    "tests/python/test_ci_selection.py", "tests/python/test_desktop_workflow.py",
)
NON_DESKTOP = ("docs/", "tests/python/", "scripts/", "fuzz/", ".github/", ".agents/")


def is_analysis(path: str) -> bool:
    return (
        path in DRIVERS or path == GUIDELINE_INDEX
        or path.startswith("docs/standards/") and path.endswith(".md")
        and not path.startswith("docs/standards/rte/generated-retention/")
    )


def is_documentation(path: str) -> bool:
    if path.startswith(("docs/standards/", "docs/official/")) or path in DRIVERS:
        return False
    if path == GUIDELINE_INDEX:
        return False
    if path.startswith(DOCUMENTATION) or path == ".github/PULL_REQUEST_TEMPLATE.md":
        return True
    # Nested runtime Markdown, LICENSE and NOTICE are delivered assets.
    return "/" not in path and path.endswith(".md")


def select_changes(rows: list[dict] | None, *, pull_request: bool = True) -> dict[str, bool]:
    """Apply the same conservative policy to PR records and local Git records.

    Pushes retain the full non-GUI matrix; their paths only select desktop scenarios.
    Missing/truncated lists and renames select every area.
    """
    listing = None if rows is None else [
        path for row in rows
        for path in (row["filename"], row.get("previous_filename")) if path
    ]
    reliable = bool(rows) and len(rows) < 3000 and all(
        row["status"] in {"modified", "added", "removed"} and not row.get("previous_filename")
        for row in rows
    )
    code = native = analysis = application = True
    if pull_request and reliable:
        code = not all(map(is_documentation, listing))
        analysis_only = all(
            row["status"] == "modified" and is_analysis(row["filename"]) for row in rows
        )
        native = code and not analysis_only and not all(
            is_documentation(path) or path.startswith("ui/") for path in listing
        )
        analysis = code and (native or analysis_only)
        application = code and not analysis_only
    unknown = listing is not None and any(
        not (is_documentation(path) or is_analysis(path)
             or path.startswith(DESKTOP_PATHS + NON_DESKTOP)) for path in listing
    )
    desktop = application and (
        unknown or listing is None or not listing or len(listing) >= 3000
        or any(path.startswith(DESKTOP_PATHS) for path in listing)
    )
    if pull_request and not reliable:
        desktop = True
    return {"code": code, "native": native, "desktop": desktop,
            "analysis": analysis, "application": application}
