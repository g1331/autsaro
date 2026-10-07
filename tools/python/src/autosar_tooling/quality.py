"""Check source hygiene, changed-line formatting, and host C99 syntax."""

from __future__ import annotations

import argparse
import ast
import difflib
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


def source_paths(*, untracked_only: bool = False) -> list[Path]:
    command = ["git", "ls-files", "-z"]
    if untracked_only:
        command.extend(["--others", "--exclude-standard"])
    else:
        command.extend(["--cached", "--others", "--exclude-standard"])
    result = _capture(command)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace"))
    paths = (
        Path(value.decode("utf-8")) for value in result.stdout.split(b"\0") if value
    )
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


def changed_lines(path: Path, base: str, untracked: set[Path]) -> set[int]:
    if path in untracked:
        return set(
            range(1, len((ROOT / path).read_text(encoding="utf-8").splitlines()) + 1)
        )
    result = run(
        [
            "git",
            "diff",
            "--no-ext-diff",
            "--no-color",
            "--unified=0",
            base,
            "--",
            path.as_posix(),
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


def c_syntax_errors() -> list[str]:
    sources = sorted(
        [
            *(ROOT / "runtime" / "src").glob("*.c"),
            *(ROOT / "runtime" / "host" / "src").glob("*.c"),
        ]
    )
    command = [
        "gcc",
        "-std=c99",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        "-fsyntax-only",
        "-I",
        str(ROOT / "runtime" / "include"),
        "-I",
        str(ROOT / "runtime" / "host" / "include"),
        "-I",
        str(ROOT / "runtime" / "src"),
        *(str(path) for path in sources),
    ]
    result = run(command)
    if result.returncode:
        return ["Host C99 syntax check failed:\n" + result.stderr.strip()]
    for header in sorted((ROOT / "runtime" / "include").glob("*.h")):
        result = run(
            [
                "gcc",
                "-std=c99",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-fsyntax-only",
                "-I",
                str(ROOT / "runtime" / "include"),
                "-I",
                str(ROOT / "runtime" / "host" / "include"),
                "-include",
                str(header),
                "-x",
                "c",
                "-",
            ],
            input_text="",
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
            else changed_lines(path, base, untracked)
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
    print(
        f"Quality scope={scope} baseline={base}; formatted files checked={checked}; C syntax={'checked' if changed_c or scope == 'runtime' or all_format else 'not requested'}"
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
