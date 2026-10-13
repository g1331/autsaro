"""Check source hygiene, changed-line formatting, host C99 syntax, and plan numbers."""

from __future__ import annotations

import argparse
import ast
import difflib
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from autosar_tooling.config import ROOT

SOURCE_SUFFIXES = {".c", ".h", ".rs", ".ts", ".tsx", ".css", ".py", ".mjs"}
FORMAT_SUFFIXES = {".c", ".h", ".rs", ".ts", ".tsx", ".css", ".mjs"}
SOURCE_ROOTS = ("core/", "src-tauri/", "runtime/", "ui/src/", "ui/tests/", "tools/", "tests/", "scripts/")
HUNK = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", re.MULTILINE)
# An epic or story word followed by its number is a development plan number, not a responsibility.
PLAN_NUMBER = re.compile(r"(?<![a-z0-9])(?:epic|story)[ _.-]?\d", re.IGNORECASE)
# Retired profile identities stay literal so their projects and deliveries are refused on open.
RETIRED_IDENTITIES = ("epic4-win64-sr-cs-v1", "epic4-single-application-v1")
# Decision records cite archived planning paths, the roadmap names planning phases like their
# Milestones, and vendored upstream content keeps its original bytes.
PLAN_NUMBER_EXEMPT = ("docs/adr/", "docs/roadmap.md", "third_party/", ".agents/skills/impeccable/")


def _capture(
    command: list[str], *, input_text: str | None = None
) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command, cwd=ROOT,
        input=input_text.encode("utf-8") if input_text is not None else None,
        capture_output=True, timeout=60, check=False,
    )


def run(
    command: list[str], *, input_text: str | None = None
) -> subprocess.CompletedProcess[str]:
    result = _capture(command, input_text=input_text)
    return subprocess.CompletedProcess(
        result.args,
        result.returncode,
        result.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n"),
        result.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n"),
    )


def repository_paths(*, untracked_only: bool = False) -> list[Path]:
    command = ["git", "ls-files", "-z"]
    if untracked_only:
        command.extend(["--others", "--exclude-standard"])
    else:
        command.extend(["--cached", "--others", "--exclude-standard"])
    result = _capture(command)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace"))
    return [Path(value.decode("utf-8")) for value in result.stdout.split(b"\0") if value]


def source_paths(*, untracked_only: bool = False) -> list[Path]:
    paths = repository_paths(untracked_only=untracked_only)
    return sorted(
        (
            path
            for path in paths
            if path.suffix in SOURCE_SUFFIXES
            and (
                path.as_posix().startswith(SOURCE_ROOTS)
                or path.as_posix() in ("ui/vite.config.ts", "ui/vitest.config.ts")
            )
            and (ROOT / path).is_file()
        ),
        key=lambda path: path.as_posix(),
    )


def hygiene_errors(path: Path) -> list[str]:
    data = (ROOT / path).read_bytes()
    name = path.as_posix()
    errors = []
    if data.startswith(b"\xef\xbb\xbf"):
        errors.append(f"{name}: UTF-8 BOM is not allowed")
    try:
        data.decode("utf-8")
    except UnicodeDecodeError as error:
        errors.append(f"{name}: invalid UTF-8 at byte {error.start}")
        return errors
    if data and not data.endswith(b"\n"):
        errors.append(f"{name}: missing final newline")
    for number, line in enumerate(data.splitlines(), 1):
        if line.endswith((b" ", b"\t")):
            errors.append(f"{name}:{number}: trailing whitespace")
        if b"\t" in line:
            errors.append(f"{name}:{number}: tab character")
    if path.suffix == ".py":
        try:
            ast.parse(data, filename=name)
        except SyntaxError as error:
            errors.append(f"{name}:{error.lineno}: Python syntax error: {error.msg}")
    return errors


def plan_number_errors(paths: list[Path] | None = None) -> list[str]:
    errors = []
    for path in repository_paths() if paths is None else paths:
        name = path.as_posix()
        if name.startswith(PLAN_NUMBER_EXEMPT) or not (ROOT / path).is_file():
            continue
        if match := PLAN_NUMBER.search(name):
            errors.append(f"{name}: development plan number {match.group(0)!r} in path")
        data = (ROOT / path).read_bytes()
        if b"\0" in data:
            continue
        try:
            text = data.decode("utf-8")
        except UnicodeDecodeError:
            continue
        for number, line in enumerate(text.splitlines(), 1):
            for identity in RETIRED_IDENTITIES:
                line = line.replace(identity, "")
            if match := PLAN_NUMBER.search(line):
                errors.append(f"{name}:{number}: development plan number {match.group(0)!r}")
    return errors


def renamed_sources(base: str) -> dict[Path, Path]:
    """Map each path renamed since base to its old path."""
    result = run(["git", "diff", "--no-ext-diff", "-M", "--name-status", "-z", base])
    if result.returncode:
        raise RuntimeError(result.stderr.strip())
    fields = result.stdout.split("\0")
    renamed = {}
    index = 0
    while index < len(fields) - 1:
        status = fields[index]
        if status.startswith(("R", "C")):
            renamed[Path(fields[index + 2])] = Path(fields[index + 1])
            index += 3
        else:
            index += 2
    return renamed


def changed_lines(
    path: Path, base: str, untracked: set[Path], renamed: dict[Path, Path] | None = None
) -> set[int]:
    if path in untracked:
        return set(
            range(1, len((ROOT / path).read_text(encoding="utf-8").splitlines()) + 1)
        )
    # Naming the old path lets git pair a renamed file, so only its edited lines count.
    old = [renamed[path].as_posix()] if renamed and path in renamed else []
    result = run(
        [
            "git",
            "diff",
            "--no-ext-diff",
            "--no-color",
            "-M",
            "--unified=0",
            base,
            "--",
            path.as_posix(),
            *old,
        ]
    )
    if result.returncode:
        raise RuntimeError(result.stderr.strip())
    lines = set()
    for match in HUNK.finditer(result.stdout):
        start = int(match.group(1))
        count = int(match.group(2) or "1")
        lines.update(range(start, start + count))
    return lines


def clang_format() -> str | None:
    return shutil.which("clang-format")


def formatted_source(path: Path, source: str) -> str:
    if path.suffix == ".rs":
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / path.name
            target.write_text(source, encoding="utf-8", newline="\n")
            result = run(
                [
                    "rustfmt",
                    "--edition",
                    "2024",
                    "--config",
                    "skip_children=true",
                    str(target),
                ]
            )
            if result.returncode:
                raise RuntimeError(result.stderr.strip())
            return target.read_text(encoding="utf-8")
    if path.suffix in {".c", ".h"}:
        formatter = clang_format()
        if formatter is None:
            raise RuntimeError("clang-format is missing; run uv sync --locked")
        command = [formatter, "--style=file", f"--assume-filename={path.as_posix()}"]
    else:
        prettier = ROOT / "ui" / "node_modules" / "prettier" / "bin" / "prettier.cjs"
        if not prettier.is_file():
            raise RuntimeError("Prettier is missing; run npm ci --prefix ui")
        command = [
            "node",
            str(prettier),
            "--stdin-filepath",
            path.as_posix(),
            "--config",
            "ui/.prettierrc.json",
        ]
    result = run(command, input_text=source)
    if result.returncode:
        raise RuntimeError(result.stderr.strip())
    return result.stdout


def changed_format_errors(
    path: Path, source: str, formatted: str, changed: set[int]
) -> list[str]:
    before = source.replace("\r\n", "\n").splitlines()
    after = formatted.replace("\r\n", "\n").splitlines()
    errors = []
    for tag, first, end, _, _ in difflib.SequenceMatcher(
        None, before, after, autojunk=False
    ).get_opcodes():
        if tag == "equal":
            continue
        affected = (
            set(range(first + 1, end + 1)) if first != end else {first, first + 1}
        )
        if affected & changed:
            errors.append(
                f"{path.as_posix()}:{first + 1}: format differs from configured formatter"
            )
    return errors


# The CAN headers include ComStack_Cfg.h, which a generated project provides; the
# independent bsw-contracts fixture stands in. The CAN sources also need the generated
# Ecu_Config.h/Ecu_Execution.h, so they are compiled with generated projects (c-check, native).
CONFIGURATION = ROOT / "core" / "tests" / "fixtures" / "bsw-contracts"
MAINLINE_CAN = ROOT / "runtime" / "bsw" / "include"
HOST_ADAPTERS = ROOT / "runtime" / "host" / "include"
MAINLINE_CAN_HEADERS = [MAINLINE_CAN / name for name in ("Can_CfgTypes.h", "Can_GeneralTypes.h",
                                                         "Can_MemMap.h", "SchM_Can.h", "standard/Can.h")]
MAINLINE_CAN_HEADERS += [HOST_ADAPTERS / name for name in ("Can_Host.h", "Ecu_Status.h")]


def c_syntax_errors() -> list[str]:
    compiler = os.environ.get("AUTOSAR_CC", "gcc")
    for header in sorted(MAINLINE_CAN_HEADERS):
        # A section protocol is a repeated-include TU, not a standalone empty header.
        if header.name == "Can_MemMap.h":
            source = (
                '#define CAN_START_SEC_CODE\n#include "Can_MemMap.h"\n'
                'CAN_CODE void quality_can_section(void) {}\n'
                '#define CAN_STOP_SEC_CODE\n#include "Can_MemMap.h"\n'
                '#define CAN_START_SEC_VAR_CLEARED_UNSPECIFIED\n#include "Can_MemMap.h"\n'
                'int quality_can_zero CAN_VAR_CLEARED;\n'
                '#define CAN_STOP_SEC_VAR_CLEARED_UNSPECIFIED\n#include "Can_MemMap.h"\n'
            )
            forced_header = []
        else:
            source = ""
            forced_header = ["-include", str(header)]
        result = run(
            [
                compiler,
                "-std=c99",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-fsyntax-only",
                "-I",
                str(MAINLINE_CAN),
                "-I",
                str(HOST_ADAPTERS),
                "-I",
                str(CONFIGURATION),
                *forced_header,
                "-x",
                "c",
                "-",
            ],
            input_text=source,
        )
        if result.returncode:
            return [
                f"Host C99 header {header.name} is not self-contained:\n{result.stderr.strip()}"
            ]
    return []


def scoped_paths(scope: str) -> list[Path]:
    prefixes = {
        "ui": ("ui/",),
        "tooling": ("tools/", "tests/python/", "tests/desktop/", "scripts/"),
        "core": ("core/",),
        "desktop": ("src-tauri/", "core/"),
        "runtime": ("runtime/", "tools/python/src/ecu_tools/"),
        "all": (*SOURCE_ROOTS, "ui/vite.config.ts", "ui/vitest.config.ts"),
    }[scope]
    return [path for path in source_paths() if path.as_posix().startswith(prefixes)]


def check(base: str, *, all_format: bool = False, scope: str = "all") -> list[str]:
    errors = []
    untracked = set(source_paths(untracked_only=True))
    renamed = {} if all_format else renamed_sources(base)
    changed_c = False
    checked = 0
    for path in scoped_paths(scope):
        path_errors = hygiene_errors(path)
        errors.extend(path_errors)
        if any("invalid UTF-8" in error for error in path_errors):
            continue
        changed = (
            set(
                range(
                    1, len((ROOT / path).read_text(encoding="utf-8").splitlines()) + 1
                )
            )
            if all_format
            else changed_lines(path, base, untracked, renamed)
        )
        if path.suffix in {".c", ".h"} and changed:
            changed_c = True
        if path.suffix not in FORMAT_SUFFIXES or not changed:
            continue
        checked += 1
        source = (ROOT / path).read_text(encoding="utf-8")
        try:
            formatted = formatted_source(path, source)
        except (OSError, RuntimeError) as error:
            errors.append(f"{path.as_posix()}: formatter failed: {error}")
            continue
        errors.extend(changed_format_errors(path, source, formatted, changed))
    if changed_c or scope == "runtime" or all_format:
        errors.extend(c_syntax_errors())
    numbers = plan_number_errors()
    errors.extend(numbers)
    print(
        f"Quality scope={scope} baseline={base}; formatted files checked={checked}; C syntax={'checked' if changed_c or scope == 'runtime' or all_format else 'not requested'}; plan numbers={len(numbers)}"
    )
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--base",
        help="Compare source lines against this commit (default: HEAD for pending edits)",
    )
    parser.add_argument(
        "--all-format",
        action="store_true",
        help="Audit formatting of all source files, including legacy code",
    )
    parser.add_argument(
        "--scope",
        choices=("ui", "tooling", "core", "desktop", "runtime", "all"),
        default="all",
    )
    arguments = parser.parse_args(argv)
    base = arguments.base or "HEAD"
    revision = run(["git", "rev-parse", "--verify", f"{base}^{{tree}}"])
    if revision.returncode:
        print(
            f"Invalid format baseline {base}: {revision.stderr.strip()}",
            file=sys.stderr,
        )
        return 2
    try:
        errors = check(base, all_format=arguments.all_format, scope=arguments.scope)
    except (OSError, RuntimeError) as error:
        print(f"Quality check could not run: {error}", file=sys.stderr)
        return 2
    for error in errors[:30]:
        print(error, file=sys.stderr)
    if len(errors) > 30:
        print(f"... {len(errors) - 30} more issues", file=sys.stderr)
    if errors:
        return 1
    print("Requested source checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
