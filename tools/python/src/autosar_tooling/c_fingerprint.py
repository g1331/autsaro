"""Build diagnostic identities from target preprocessing and analyzer receipts."""

from __future__ import annotations

import hashlib
import json
import posixpath
import re
from bisect import bisect_left
from collections import Counter, defaultdict
from pathlib import Path

from ecu_tools.build import digest

from autosar_tooling import c_preprocessor

FORMAT = "c-diagnostic-fingerprints-v2"
SARIF_KEY = "autsaroDiagnostic/v2"
RECEIPT = "diagnostic-fingerprints.json"
SHA256 = re.compile(r"[0-9a-f]{64}")
# The pinned MISRA addon's analyse_ctu_info can report these using facts
# from units that do not contain any location in the diagnostic.
CTU = frozenset(f"misra-c2012-{rule}" for rule in (
    "2.3", "2.4", "2.5", "5.6", "5.7", "5.8", "5.9", "8.5", "8.6", "8.7",
))
TOKEN = re.compile(
    rf"(?:L|u8|u|U)?(?:{c_preprocessor.LITERAL})|"
    r"(?:\d|\.\d)(?:[\w.]|(?<=[eEpP])[+-])*|[A-Za-z_]\w*|"
    r"\.\.\.|>>=|<<=|>>|<<|->|\+\+|--|&&|\|\||[+*/%<>=!&|^\-]=|##|\S"
)
SOURCE_TOKEN = re.compile(r"/\*.*?\*/|//[^\n]*|" + TOKEN.pattern, re.DOTALL)
FUNCTION_MACRO = re.compile(r"#define [A-Za-z_]\w*\(")
LINE_SENSITIVE = re.compile(r"\b__LINE__\b|##|%:%:|(?:\\|\?\?/)[ \t]*(?:\n|$)")


def sha(value: object) -> str:
    return hashlib.sha256(json.dumps(
        value, ensure_ascii=False, separators=(",", ":"), sort_keys=True,
    ).encode("utf-8")).hexdigest()


def path_key(name: str, windows: bool = False) -> str:
    path = posixpath.normpath(name.replace("\\", "/"))
    return path.casefold() if windows else path


def source_root(lines: list[str], unit: str, windows: bool) -> str:
    """The primary GCC marker records the original root even after artifact transfer."""
    marker = next((m for line in lines if (m := c_preprocessor.LINE.fullmatch(line))), None)
    name = path_key(marker[2].replace("\\\\", "\\")) if marker else ""
    if (marker is None or marker[1] != "0"
            or not path_key(name, windows).endswith("/" + path_key(unit, windows))):
        raise ValueError(f"Primary compiler-origin marker differs: {unit}")
    return name.rsplit("/", unit.count("/") + 1)[0]


def tokens(text: str) -> list[str]:
    return TOKEN.findall(text)


def compiler_context(text: str, unit: str, sdk: dict[str, str],
                     wanted: dict[str, set[int]], *, windows: bool = False,
                     project_paths: dict[str, str] | None = None) -> tuple[str, dict]:
    """Hash the ordered expanded TU and collect expanded tokens at diagnostic anchors."""
    lines = text.splitlines()
    root = source_root(lines, unit, windows)
    root_key = path_key(root, windows)
    project_paths = project_paths or {}
    # __FILE__ can put the source checkout in a string literal as well as a line marker.
    checkout = re.compile("|".join(map(re.escape, (root + "/", root.replace("/", "\\\\") + "\\\\"))),
                          re.IGNORECASE if windows else 0)
    digest = hashlib.sha256()
    expanded = defaultdict(list)
    filename, number = unit, 0
    last_file = None
    for line in lines:
        marker = c_preprocessor.LINE.fullmatch(line)
        if marker:
            number = int(marker[1])
            path = path_key(marker[2].replace("\\\\", "\\"), windows)
            relative = path.removeprefix(root_key + "/")
            filename = sdk.get(path, project_paths.get(relative, relative))
        elif line.strip():
            line = checkout.sub("<project>/", line)
            words = tokens(line)
            if FUNCTION_MACRO.match(line):
                words.insert(3, "<function-macro>")
            if number in wanted.get(filename, ()):
                expanded[filename, number].append(words)
            if filename != last_file:
                digest.update(b"\nfile " + json.dumps(filename, ensure_ascii=False).encode("utf-8"))
                digest.update(b"\n")
                last_file = filename
            encoded = json.dumps(words, ensure_ascii=False, separators=(",", ":"))
            digest.update(encoded[1:-1].encode("utf-8"))
            digest.update(b",")
            if line.startswith("#"):
                digest.update(b"directive-end\n")
        number += not marker
    return digest.hexdigest(), expanded


class SourceContext:
    def __init__(self, text: str):
        # Keep offsets for the locator. Comment bodies are relevant to some raw checks.
        self.starts = [0, *(match.end() for match in re.finditer("\n", text))]
        self.tokens = list(SOURCE_TOKEN.finditer(text))

    def anchor(self, line: int, column: int) -> tuple[int, list[str], str, list[str]] | None:
        if line <= 0 or line > len(self.starts):
            return None
        start = self.starts[line - 1]
        end = self.starts[line] if line < len(self.starts) else float("inf")
        offset = start + max(0, column - 1)
        index = bisect_left(self.tokens, offset + 1, key=lambda token: token.end())
        if index >= len(self.tokens) or self.tokens[index].start() >= end:
            return None
        # Separate before/after with the anchor itself so neighboring tokens
        # cannot shift across it without changing the identity.
        return (index, [token[0] for token in self.tokens[max(0, index - 8):index]],
                self.tokens[index][0], [token[0] for token in self.tokens[index + 1:index + 9]])


def canonical_config(summary: dict, compiler_identity: dict) -> dict:
    scope = summary["scope"]
    semantic = summary["semanticIdentity"]
    tool = summary["tool"]
    return {
        "target": scope["target"],
        "options": scope["compilerFlags"],
        "includePaths": scope["includePaths"],
        "tool": {key: tool[key] for key in (
            "version", "expectedSourceRevision", "binarySha256", "addons",
        )},
        "compilerBinaries": compiler_identity,
        "compiler": {key: semantic["compiler"][key] for key in (
            "machine", "release", "predefines", "builtins",
        )},
        "models": semantic["models"],
        "libraries": semantic["libraries"],
        "addonRuntime": semantic["addonRuntime"],
        "projectChecks": {name: semantic["analysisCode"][name]
                          for name in ("c_addon.py", "c_check.py")},
        "implicitConfig": (scope.get("cppcheckConfig") or {}).get("sha256"),
    }


def inventory(summary: dict, output: Path, compiler_identity: dict) -> list[dict]:
    """Emit one identity per raw diagnostic location and program."""
    scope = summary["scope"]
    windows = scope["target"].startswith("windows-")
    sdk = {path_key(row["path"], windows): "sdk:" + row["key"] for row in scope["sdkHeaders"]}
    project_paths = {path_key(name, windows): name
                     for unit, paths in scope["projectTranslationUnits"].items()
                     for name in {*paths, *scope["cppcheckInputs"][unit]["project"]}}

    def logical_path(name):
        key = path_key(name, windows)
        return sdk.get(key, project_paths.get(key, key))

    diagnostic_paths = defaultdict(set)
    for diagnostic in summary["diagnostics"]:
        if diagnostic["id"] != "checkersReport":
            for place in diagnostic["locations"]:
                diagnostic_paths[logical_path(place["file"])].add(int(place["line"] or 0))

    project = output / "project"
    raw_context = {}
    raw_hashes = {}
    line_sensitive = set()
    consumers = defaultdict(set)
    expanded = defaultdict(dict)
    unit_hashes = {}
    unresolved = set()
    config = sha(canonical_config(summary, compiler_identity))
    compiler_settings = [*scope["compilerFlags"], *summary["semanticIdentity"]["compiler"]["builtins"]]
    line_sensitive_options = LINE_SENSITIVE.search("\n".join(compiler_settings))
    for index, unit in enumerate(scope["translationUnits"]):
        analyzer = scope["cppcheckInputs"][unit]
        if analyzer.get("unbound"):
            unresolved.add(unit)
        paths = set(scope["projectTranslationUnits"][unit]) | set(analyzer["project"])
        sdk_paths = {*scope["sdkTranslationUnits"][unit], *analyzer["sdk"]}
        for name in paths | sdk_paths:
            consumers[logical_path(name)].add(unit)
        for name in paths:
            if name not in raw_context:
                text = (project / name).read_text(encoding="utf-8")
                raw_context[name] = SourceContext(text)
                raw_hashes[name] = (sha(text), sha((
                    [line for line in text.split("\n") if line], text.endswith("\n"),
                )))
                if LINE_SENSITIVE.search(text):
                    line_sensitive.add(name)
        compiler = (output / f"compiler-origins-{index}.stdout.log").read_text(encoding="utf-8")
        expanded_digest, anchors = compiler_context(
            compiler, unit, sdk, diagnostic_paths, windows=windows, project_paths=project_paths,
        )
        defines = summary["semanticIdentity"]["defines"][unit]
        # Preserve the physical lines when a checker input may expand a line number
        # or splice a newline. Raw spelling covers branches only Cppcheck opens.
        exact_lines = (line_sensitive.intersection(analyzer["project"]) or analyzer["sdk"]
                       or line_sensitive_options or LINE_SENSITIVE.search("\n".join(defines)))
        unit_hashes[unit] = sha({
            "expanded": expanded_digest,
            "defines": defines,
            "analyzerSources": {name: raw_hashes[name][0 if exact_lines else 1]
                                for name in analyzer["project"]},
            "sdk": sorted(sdk[path_key(name, windows)] for name in sdk_paths),
        })
        for key, lines in anchors.items():
            expanded[key][unit] = lines

    programs = {row["name"]: set(row["translationUnits"]) for row in summary["programs"]}
    for header in scope["sdkHeaders"]:
        path = Path(header["path"])
        key = logical_path(header["path"])
        if key in diagnostic_paths and path.is_file() and digest(path) == header["sha256"]:
            raw_context[key] = SourceContext(path.read_text(encoding="utf-8"))
    rows = []
    for index, diagnostic in enumerate(summary["diagnostics"]):
        rule = diagnostic["id"]
        if rule == "checkersReport":
            continue
        sites = []
        for place in diagnostic["locations"]:
            name = logical_path(place["file"])
            line = int(place["line"] or 0)
            column = int(place.get("column") or 0)
            context = raw_context.get(name)
            anchor = context.anchor(line, column) if context else None
            sites.append((name, line, anchor))
        for program in diagnostic["programs"]:
            units = programs[program]
            related = units if rule in CTU else units.intersection(
                unit for name, _, _ in sites for unit in consumers[name]
            )
            complete = bool(related and sites) and not related.intersection(unresolved) and all(
                anchor is not None and units.intersection(consumers[name])
                for name, _, anchor in sites
            )
            evidence = [(name, anchor, sorted(
                (unit, tokens_at_site)
                for unit, tokens_at_site in expanded[name, line].items() if unit in units
            )) for name, line, anchor in sites]
            for position in range(len(sites) or 1):
                fingerprint = sha({
                    "version": FORMAT, "configuration": config, "rule": rule,
                    "severity": diagnostic["severity"], "program": program, "sites": evidence,
                    "position": position,
                    "translationUnits": {unit: unit_hashes[unit] for unit in related},
                }) if complete else None
                rows.append({"diagnostic": index, "locationIndex": position,
                             "program": program, "fingerprint": fingerprint})
    counts = Counter(row["fingerprint"] for row in rows)
    for row in rows:
        value = row["fingerprint"]
        if value is None:
            row["reason"] = "missing_context"
        elif counts[value] > 1:
            row.update(fingerprint=None, reason="ambiguous_context")
    return rows


def write_receipt(output: Path, rows: list[dict]) -> str:
    receipt = {
        "format": FORMAT, "summarySha256": digest(output / "summary.json"),
        "occurrences": rows,
    }
    path = output / RECEIPT
    path.write_text(json.dumps(receipt, separators=(",", ":")) + "\n", encoding="utf-8")
    return digest(path)


def read_receipt(output: Path, summary: dict, expected_sha: str) -> list[dict]:
    evidence = (output / RECEIPT).read_bytes()
    if hashlib.sha256(evidence).hexdigest() != expected_sha:
        raise ValueError("Fingerprint receipt differs from the raw analysis manifest")
    receipt = json.loads(evidence)
    if receipt["format"] != FORMAT or receipt["summarySha256"] != digest(output / "summary.json"):
        raise ValueError("Fingerprint receipt does not bind the transferred raw summary")
    rows = receipt["occurrences"]
    validate_occurrences(summary, rows)
    return rows


def validate_occurrences(summary: dict, rows: list[dict]) -> None:
    expected = {(index, position, program)
                for index, diagnostic in enumerate(summary["diagnostics"])
                if diagnostic["id"] != "checkersReport"
                for position in range(len(diagnostic["locations"]) or 1)
                for program in diagnostic["programs"]}
    found, fingerprints = set(), set()
    for row in rows:
        key = row["diagnostic"], row["locationIndex"], row["program"]
        value = row["fingerprint"]
        reason = row.get("reason")
        if (key not in expected or key in found
                or (value is not None and (not SHA256.fullmatch(value) or value in fingerprints))
                or (value is None and reason not in {"missing_context", "ambiguous_context"})):
            raise ValueError("Fingerprint receipt has invalid or duplicate occurrences")
        found.add(key)
        if value is not None:
            fingerprints.add(value)
    if found != expected:
        raise ValueError("Fingerprint receipt occurrence coverage differs")


def verdict(current: list[dict], previous: list[dict], approved: set[str]) -> dict:
    """Compare in-scope inventories. The caller supplies validated approvals."""
    before = {row["fingerprint"] for row in previous if row["fingerprint"] is not None}
    after = {row["fingerprint"] for row in current if row["fingerprint"] is not None}
    unavailable = sum(row["fingerprint"] is None for row in [*current, *previous])
    additions = after - before
    unaccepted, stale = additions - approved, approved - after
    return {
        "status": "incomplete" if unavailable else "blocked" if unaccepted or stale else "passed",
        "unavailable": unavailable,
        "new": sorted(additions), "removed": sorted(before - after),
        "unacceptedNew": sorted(unaccepted), "staleDispositions": sorted(stale),
    }
