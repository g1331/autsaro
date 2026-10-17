"""Check source hygiene, changed-line formatting, C headers and clang-tidy, plan numbers, document sizes and paired tool versions."""

from __future__ import annotations

import argparse
import ast
import collections
import difflib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from ecu_tools.process import ProcessSpec, run_bounded

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
PLAN_NUMBER_EXEMPT = ("docs/adr/", "docs/roadmap/", "third_party/", ".agents/skills/impeccable/")
# CONTRIBUTING "Documentation layout": a document beyond about 20 KB is split into a directory
# with a README.md index. Changelogs, single-decision ADRs and traceability archives are
# exempt, and vendored content keeps its bytes. AGENTS.md is held to the same limit: some agents
# load only a bounded amount of project instructions and silently drop the rest.
DOCUMENT_LIMIT = 20 * 1024
DOCUMENT_SIZE_EXEMPT = (
    "CHANGELOG.md", "docs/adr/", "docs/project/archive/",
    "third_party/", ".agents/skills/impeccable/",
)
# One LLVM release provides both; PyPI publishes them as separate projects, sometimes days apart.
PAIRED_TOOLS = ("clang-format", "clang-tidy")


def _capture(
    command: list[str], *, input_text: str | None = None, timeout: int = 60
) -> subprocess.CompletedProcess[bytes]:
    try:
        result = subprocess.run(
            command, cwd=ROOT,
            input=input_text.encode("utf-8") if input_text is not None else None,
            capture_output=True, timeout=timeout, check=False,
        )
    except subprocess.TimeoutExpired as error:
        raise RuntimeError(
            f"quality: argv={command!r} cwd={ROOT} status=timeout exit_code=None\n"
            f"stdout:\n{(error.stdout or b'')[-8192:].decode('utf-8', errors='replace')}\n"
            f"stderr:\n{(error.stderr or b'')[-8192:].decode('utf-8', errors='replace')}"
        ) from error
    if result.returncode:
        result.stderr = (
            f"quality: argv={command!r} cwd={ROOT} status=exited exit_code={result.returncode}\n"
            f"stdout:\n{result.stdout[-8192:].decode('utf-8', errors='replace')}\n"
            f"stderr:\n{result.stderr[-8192:].decode('utf-8', errors='replace')}"
        ).encode()
    return result


def run(
    command: list[str], *, input_text: str | None = None, timeout: int = 60
) -> subprocess.CompletedProcess[str]:
    result = _capture(command, input_text=input_text, timeout=timeout)
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


def document_size_errors(paths: list[Path] | None = None) -> list[str]:
    errors = []
    for path in repository_paths() if paths is None else paths:
        name = path.as_posix()
        if (
            path.suffix != ".md"
            or name.startswith(DOCUMENT_SIZE_EXEMPT)
            or not (ROOT / path).is_file()
        ):
            continue
        # A Windows checkout with CRLF line ends measures the same as the repository.
        size = len((ROOT / path).read_bytes().replace(b"\r\n", b"\n"))
        if size > DOCUMENT_LIMIT:
            errors.append(
                f"{name}: {size} bytes exceeds the {DOCUMENT_LIMIT}-byte document limit; "
                "split it into a same-name directory with a README.md index"
            )
    return errors


def paired_tool_errors(lock: Path | None = None) -> list[str]:
    packages = tomllib.loads((lock or ROOT / "uv.lock").read_text(encoding="utf-8"))["package"]
    versions = {package["name"]: package["version"] for package in packages if package["name"] in PAIRED_TOOLS}
    if len(versions) == len(PAIRED_TOOLS) and len(set(versions.values())) == 1:
        return []
    found = ", ".join(f"{name} {versions.get(name, 'missing')}" for name in PAIRED_TOOLS)
    return [f"uv.lock: {found}; upgrade clang-format and clang-tidy together to the same LLVM release"]


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


# Public headers include ComStack_Cfg.h and SchM_<Mip>.h, which a generated project provides;
# the independent bsw-contracts fixture stands in. Os_Host.h selects the private host header of
# the checking platform. The sources also need generated configuration headers, so they are
# compiled with generated projects (c-check, native, clang-tidy below).
CONFIGURATION = ROOT / "core" / "tests" / "fixtures" / "bsw-contracts"
PUBLIC_INCLUDES = [ROOT / "runtime" / name / "include" for name in ("bsw", "host", "ecu", "os")]
OS_PRIVATE = [ROOT / "runtime" / "os" / "src", ROOT / "runtime" / "os" / "src" / "host" / "linux"]


def public_headers() -> list[Path]:
    return sorted(header for directory in PUBLIC_INCLUDES for header in directory.rglob("*.h"))


def header_unit(header: Path) -> tuple[str, list[str]]:
    """Return a translation unit that uses the header the way its users do."""
    if header.name.endswith("_MemMap.h"):
        # A section protocol is a repeated-include TU, not a standalone empty header.
        prefix = header.name.removesuffix("_MemMap.h").upper()
        source = (
            f'#define {prefix}_START_SEC_CODE\n#include "{header.name}"\n'
            f"{prefix}_CODE void quality_section(void) {{}}\n"
            f'#define {prefix}_STOP_SEC_CODE\n#include "{header.name}"\n'
        )
        return source, []
    # A header of macros only would otherwise leave an empty, non-ISO translation unit.
    return "typedef int quality_unit;\n", ["-include", str(header)]


def c_syntax_errors() -> list[str]:
    compiler = os.environ.get("AUTOSAR_CC", "gcc")
    errors = []
    for header in public_headers():
        source, forced_header = header_unit(header)
        result = run(
            [
                compiler,
                "-std=c99",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-fsyntax-only",
                *(f"-I{directory}" for directory in (*PUBLIC_INCLUDES, *OS_PRIVATE, CONFIGURATION)),
                "-I",
                str(header.parent),
                *forced_header,
                "-x",
                "c",
                "-",
            ],
            input_text=source,
        )
        if result.returncode:
            name = header.relative_to(ROOT).as_posix()
            errors.append(f"{name}: public header is not self-contained:\n{result.stderr.strip()}")
    return errors


def file_header_errors(path: Path) -> list[str]:
    """Runtime C files open with the file description of the C coding standard."""
    with (ROOT / path).open("rb") as source:
        first = source.readline()
    # Either the one-line form or a block whose next line is the @brief.
    if first.startswith(b"/** @file ") or first.rstrip(b"\r\n") == b"/** @file":
        return []
    return [f"{path.as_posix()}:1: runtime C file must start with a /** @file <responsibility> comment"]


# clang-tidy analyzes generated samples: runtime sources need the generated configuration
# headers. Findings are keyed by check, file and source line text, so moving code does not
# churn the baseline; the FreeRTOS kernel and the user application keep their own style.
CLANG_TIDY_CONFIG = ROOT / ".clang-tidy"
CLANG_TIDY_BASELINE = ROOT / "tools" / "python" / "src" / "autosar_tooling" / "clang_tidy_baseline.txt"
CLANG_TIDY_FOREIGN = ("kernel/", "app/")
# Generated names come from the RTE/BSW patterns and configured short names, not from authors.
CLANG_TIDY_RUNTIME_ONLY = {"readability-identifier-naming"}
CLANG_TIDY_LINE = re.compile(r"^(.+?):(\d+):(\d+): (warning|error): (.*?) \[([\w.,-]+)\]$")

ClangTidyKey = tuple[str, str, str]


def clang_tidy_project(project: Path, tidy: str) -> collections.Counter[ClangTidyKey]:
    from ecu_tools.build import kernel_patch_environment

    # git apply and clang-tidy run in other directories than the caller's.
    project = project.resolve()
    target = json.loads((project / "target.json").read_text(encoding="utf-8"))
    if not str(target.get("target", "")).startswith("linux-"):
        raise RuntimeError(f"{project.name}: the clang-tidy baseline covers Linux samples only")
    integration = json.loads((project / "meta" / "integration.json").read_text(encoding="utf-8"))
    origins = {copy: source for source, copy in integration["originalBswSources"].items()}
    findings: collections.Counter[ClangTidyKey] = collections.Counter()
    seen = set()
    with tempfile.TemporaryDirectory() as temporary:
        scratch = Path(temporary)
        # The build applies the pinned kernel patches first; its headers declare port hooks.
        kernel = scratch / "kernel"
        shutil.copytree(project / "kernel", kernel)
        git = shutil.which("git")
        if git is None:
            raise RuntimeError(f"{project.name}: kernel patch requires Git on PATH")
        for patch in target["kernelPatches"]:
            run_bounded(ProcessSpec.seconds(
                [
                    git, "-c", "core.autocrlf=false", "apply", "--ignore-space-change",
                    *("--include=" + name for name in patch["includes"]),
                    str(project / patch["path"].removeprefix("runtime/")),
                ],
                kernel, 30, scratch, f"{project.name}: kernel-patch",
                env=kernel_patch_environment(kernel),
            ))
        includes = []
        for name in target["includePaths"]:
            root = kernel / name.removeprefix("kernel/") if name.startswith("kernel/") else project / name
            includes.append(f"-I{root}")
        units = [
            name for name in (*target["sources"], *target["entries"].values())
            if not name.startswith(CLANG_TIDY_FOREIGN)
        ]
        database = [
            {
                "directory": str(project),
                "file": str(project / name),
                "arguments": ["clang", *target["compilerFlags"], *includes, "-c", str(project / name)],
            }
            for name in units
        ]
        (scratch / "compile_commands.json").write_text(json.dumps(database), encoding="utf-8")

        def analyze(name: str) -> subprocess.CompletedProcess[str]:
            return run(
                [tidy, "--quiet", f"--config-file={CLANG_TIDY_CONFIG}", "-p", str(scratch), str(project / name)],
                timeout=600,
            )

        with ThreadPoolExecutor(max_workers=os.cpu_count() or 2) as pool:
            results = list(pool.map(analyze, units))
        for name, result in zip(units, results):
            matches = [CLANG_TIDY_LINE.match(line) for line in result.stdout.splitlines()]
            if result.returncode or any(match and match[4] == "error" for match in matches):
                raise RuntimeError(f"{project.name}: clang-tidy could not analyze {name}:\n{result.stdout}{result.stderr}")
            for match in filter(None, matches):
                try:
                    relative = Path(match[1]).resolve().relative_to(project).as_posix()
                except ValueError:
                    continue
                if relative.startswith(CLANG_TIDY_FOREIGN):
                    continue
                origin = origins.get(relative)
                for check in match[6].split(","):
                    if check in CLANG_TIDY_RUNTIME_ONLY and origin is None:
                        continue
                    # A header diagnostic repeats for every translation unit including it.
                    if (relative, match[2], match[3], check) in seen:
                        continue
                    seen.add((relative, match[2], match[3], check))
                    line = (project / relative).read_text(encoding="utf-8").splitlines()[int(match[2]) - 1]
                    findings[(check, origin or relative, line.strip())] += 1
    return findings


def read_clang_tidy_baseline(path: Path = CLANG_TIDY_BASELINE) -> collections.Counter[ClangTidyKey]:
    entries: collections.Counter[ClangTidyKey] = collections.Counter()
    for line in path.read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            check, name, text = line.split(" ", 2)
            entries[(check, name, text)] += 1
    return entries


def clang_tidy_errors(samples: Path) -> list[str]:
    tidy = shutil.which("clang-tidy")
    if tidy is None:
        raise RuntimeError("clang-tidy is missing; run uv sync --locked")
    projects = sorted(path for path in samples.iterdir() if (path / "target.json").is_file())
    if not projects:
        raise RuntimeError(f"No generated samples in {samples}")
    current: collections.Counter[ClangTidyKey] = collections.Counter()
    for project in projects:
        # Samples share the runtime sources; a finding counts once however many include it.
        current |= clang_tidy_project(project, tidy)
    return clang_tidy_comparison(current, read_clang_tidy_baseline())


def clang_tidy_comparison(
    current: collections.Counter[ClangTidyKey], baseline: collections.Counter[ClangTidyKey]
) -> list[str]:
    errors = [
        f"{name}: new clang-tidy finding [{check}]: {text}"
        for (check, name, text), count in sorted((current - baseline).items())
        for _ in range(count)
    ]
    errors += [
        f"{CLANG_TIDY_BASELINE.relative_to(ROOT).as_posix()}: fixed finding, remove: {check} {name} {text}"
        for (check, name, text), count in sorted((baseline - current).items())
        for _ in range(count)
    ]
    return errors


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


def check(
    base: str, *, all_format: bool = False, scope: str = "all", c_samples: Path | None = None
) -> list[str]:
    errors = []
    untracked = set(source_paths(untracked_only=True))
    renamed = {} if all_format else renamed_sources(base)
    changed_c = False
    checked = 0
    for path in scoped_paths(scope):
        path_errors = hygiene_errors(path)
        errors.extend(path_errors)
        if path.suffix in {".c", ".h"} and path.as_posix().startswith("runtime/"):
            errors.extend(file_header_errors(path))
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
    from autosar_tooling.c_standard import (
        baseline_errors,
        deviation_findings,
        generated_findings,
        runtime_findings,
    )

    coding = runtime_findings()
    coding.update(deviation_findings())
    if c_samples is not None:
        coding |= generated_findings(c_samples)
    errors.extend(baseline_errors(coding, generated=c_samples is not None))
    tidy = clang_tidy_errors(c_samples) if c_samples is not None else []
    errors.extend(tidy)
    numbers = plan_number_errors()
    errors.extend(numbers)
    oversized = document_size_errors()
    errors.extend(oversized)
    paired = paired_tool_errors()
    errors.extend(paired)
    print(
        f"Quality scope={scope} baseline={base}; formatted files checked={checked}; C syntax={'checked' if changed_c or scope == 'runtime' or all_format else 'not requested'}; clang-tidy={f'{len(tidy)} new or fixed' if c_samples is not None else 'not requested'}; plan numbers={len(numbers)}; oversized documents={len(oversized)}; paired tool versions={'mismatched' if paired else 'equal'}"
    )
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    baseline = parser.add_mutually_exclusive_group()
    baseline.add_argument(
        "--base",
        help="Compare source lines against this commit (default: HEAD for pending edits)",
    )
    baseline.add_argument(
        "--merge-base",
        metavar="BRANCH",
        help="Compare against the commit where HEAD forked from BRANCH, as a pull request into it does",
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
    parser.add_argument(
        "--c-samples",
        type=Path,
        help="Also run clang-tidy over these generated Linux samples against the committed baseline",
    )
    arguments = parser.parse_args(argv)
    base = arguments.base or "HEAD"
    if arguments.merge_base:
        fork = run(["git", "merge-base", "HEAD", arguments.merge_base])
        if fork.returncode:
            print(
                f"No merge base with {arguments.merge_base}: {fork.stderr.strip()}",
                file=sys.stderr,
            )
            return 2
        base = fork.stdout.strip()
    revision =run(["git", "rev-parse", "--verify", f"{base}^{{tree}}"])
    if revision.returncode:
        print(
            f"Invalid format baseline {base}: {revision.stderr.strip()}",
            file=sys.stderr,
        )
        return 2
    try:
        errors = check(
            base,
            all_format=arguments.all_format,
            scope=arguments.scope,
            c_samples=arguments.c_samples,
        )
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
