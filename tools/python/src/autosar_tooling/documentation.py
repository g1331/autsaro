"""Offline repository documentation contracts, used by quality."""

from __future__ import annotations

import html
import json
import re
from pathlib import Path
from urllib.parse import unquote, urlsplit

EXCLUDED = ("docs/project/archive/", "third_party/")


def prose(source: str) -> str:
    """Mask code and comments while preserving line numbers."""
    source = re.sub(
        r"<!--.*?-->", lambda m: "\n" * m[0].count("\n"), source, flags=re.DOTALL
    )
    lines = source.splitlines(keepends=True)
    fence = None
    for index, line in enumerate(lines):
        marker = re.match(r" {0,3}(`{3,}|~{3,})", line)
        if marker and fence is None:
            fence = marker[1]
        elif fence and re.match(
            r" {0,3}" + re.escape(fence[0]) + "{" + str(len(fence)) + r",}\s*$", line
        ):
            fence = None
            lines[index] = "\n"
            continue
        if fence or line.startswith(("    ", "\t")):
            lines[index] = "\n"
    return re.sub(
        r"(`+).*?\1",
        lambda m: "".join("\n" if c == "\n" else " " for c in m[0]),
        "".join(lines),
    )


def anchors(source: str) -> set[str]:
    # Inline code contributes its text to a heading, even though links inside it do not.
    source = re.sub(r"(`+)(.*?)\1", r"\2", source)
    source = prose(source)
    result = set(re.findall(r"""\b(?:id|name)=["']([^"']+)["']""", source))
    counts: dict[str, int] = {}
    lines = source.splitlines()
    for index, line in enumerate(lines):
        heading = re.match(r" {0,3}#{1,6}\s+(.+?)(?:\s+#+)?\s*$", line)
        title = heading[1] if heading else None
        if (
            index + 1 < len(lines)
            and re.fullmatch(r" {0,3}(?:=+|-+)\s*", lines[index + 1])
            and line.strip()
        ):
            title = line.strip()
        if title is None:
            continue
        title = re.sub(r"!?\[([^\]]+)\]\([^)]*\)", r"\1", title)
        title = html.unescape(re.sub(r"<[^>]*>", "", title)).lower()
        slug = "".join(c for c in title if c.isalnum() or c in "_ -").replace(" ", "-")
        candidate = slug
        while candidate in counts:
            counts[slug] = counts.get(slug, 0) + 1
            candidate = f"{slug}-{counts[slug]}"
        counts[candidate] = 0
        result.add(candidate)
    return result


def links(source: str):
    source = prose(source)
    definitions = {}
    definition = re.compile(r"(?m)^ {0,3}\[([^\]]+)\]:\s*(<[^>]+>|\S+)")
    for match in definition.finditer(source):
        definitions[" ".join(match[1].lower().split())] = match[2].strip("<>")
    pattern = re.compile(
        r"""!?\[([^\]\n]*(?:\[[^\]\n]*\][^\]\n]*)*)\]\(\s*(<[^>]*>|(?:[^\s()\\]|\\.|\([^()]*\))*)(?:\s+["'][^\n]*?["'])?\s*\)"""
        r"""|!?\[([^\]\n]+)\](?:\[([^\]\n]*)\])?"""
        r"""|<(?:a|img)\b[^>]*?\b(?:href|src)=["']([^"']+)["']""",
        re.IGNORECASE,
    )
    for match in pattern.finditer(source):
        target = match[2] if match[2] is not None else match[5]
        if target is None:
            key = " ".join((match[4] or match[3]).lower().split())
            target = definitions.get(key)
            if target is None:
                # A shortcut label can be ordinary prose; explicit references cannot.
                if match[4] is not None:
                    yield source.count("\n", 0, match.start()) + 1, None
                continue
        yield (
            source.count("\n", 0, match.start()) + 1,
            html.unescape(target.strip("<>")),
        )


def link_errors(root: Path, paths: list[Path]) -> list[str]:
    errors = []
    available = {path.as_posix() for path in paths if (root / path).exists()}
    directories = {parent.as_posix() for path in paths for parent in path.parents}
    cache = {}
    for path in paths:
        if (
            path.suffix.lower() != ".md"
            or path.as_posix().startswith(EXCLUDED)
            or not (root / path).is_file()
        ):
            continue
        for line, target in links((root / path).read_text(encoding="utf-8")):
            prefix = f"{path.as_posix()}:{line}: "
            if target is None:
                errors.append(prefix + "undefined Markdown reference")
                continue
            url = urlsplit(target)
            if url.scheme or url.netloc or not target:
                continue
            name = unquote(url.path)
            destination = (
                (root / name.lstrip("/"))
                if name.startswith("/")
                else (root / path.parent / name if name else root / path)
            )
            try:
                relative = destination.resolve().relative_to(root.resolve()).as_posix()
            except ValueError:
                errors.append(prefix + f"link leaves repository: {target}")
                continue
            if relative not in available and relative not in directories:
                errors.append(prefix + f"missing link target: {target}")
                continue
            if url.fragment and destination.suffix.lower() in {".md", ".html", ".htm"}:
                if relative not in cache:
                    cache[relative] = anchors(destination.read_text(encoding="utf-8"))
                if unquote(url.fragment) not in cache[relative]:
                    errors.append(prefix + f"missing link anchor: {target}")
    return errors


def bilingual_errors(changed: set[Path], known: set[Path]) -> list[str]:
    errors = []
    for path in sorted(changed):
        if path.parent != Path(".") or path.suffix != ".md":
            continue
        partner = (
            Path(path.name.removesuffix(".zh-CN.md") + ".md")
            if path.name.endswith(".zh-CN.md")
            else Path(path.stem + ".zh-CN.md")
        )
        if (
            partner in known or path.name.endswith(".zh-CN.md")
        ) and partner not in changed:
            errors.append(
                f"{path}: bilingual document must change together with {partner}"
            )
    return errors


def changelog_errors(source: str, added: set[int]) -> list[str]:
    errors = []
    group = None
    for number, line in enumerate(source.splitlines(), 1):
        if line.startswith("## "):
            group = None
        if line.startswith("### "):
            group = line[4:].strip()
        if number not in added or not line.strip() or line.startswith("#"):
            continue
        if group not in {"Breaking", "Added", "Changed", "Fixed"} or not re.fullmatch(
            r"- \S.* \(#\d+\)", line
        ):
            errors.append(
                f"CHANGELOG.md:{number}: use a single-line entry ending in (#N) under Breaking/Added/Changed/Fixed"
            )
    return errors


def official_errors(root: Path, paths: list[Path]) -> list[str]:
    manifest = json.loads(
        (root / "core/resources/official.json").read_text(encoding="utf-8")
    )
    names = {Path(item["path"]).name.casefold() for item in manifest}
    errors = []
    for path in paths:
        name = path.as_posix()
        if not (root / path).is_file():
            continue
        if (
            path.name.casefold() in names
            or re.fullmatch(
                r"AUTOSAR_(?:CP|FO|AP)_.*\.(?:pdf|zip)", path.name, re.IGNORECASE
            )
            or (name.startswith("docs/official/") and path.suffix.lower() != ".md")
        ):
            errors.append(
                f"{name}: official AUTOSAR files must remain outside version control"
            )
    return errors


def check(base: str) -> list[str]:
    from autosar_tooling import quality

    root = quality.ROOT
    paths = quality.repository_paths()
    result = quality.run(
        ["git", "diff", "--no-ext-diff", "--name-only", "--no-renames", "-z", base]
    )
    original = quality.run(["git", "ls-tree", "-r", "--name-only", "-z", base])
    if result.returncode or original.returncode:
        raise RuntimeError(result.stderr + original.stderr)
    changed = {Path(name) for name in result.stdout.split("\0") if name}
    untracked = set(quality.repository_paths(untracked_only=True))
    changed |= untracked
    known = set(paths) | {Path(name) for name in original.stdout.split("\0") if name}
    errors = link_errors(root, paths)
    errors.extend(bilingual_errors(changed, known))
    changelog = root / "CHANGELOG.md"
    if changelog.is_file():
        errors.extend(
            changelog_errors(
                changelog.read_text(encoding="utf-8"),
                quality.changed_lines(Path("CHANGELOG.md"), base, untracked),
            )
        )
    errors.extend(official_errors(root, paths))
    return errors
