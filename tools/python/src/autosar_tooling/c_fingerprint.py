"""Build diagnostic identities from target preprocessing and analyzer receipts."""

from __future__ import annotations

import hashlib
import json
import posixpath
import re
from bisect import bisect_left, bisect_right
from collections import Counter, defaultdict
from pathlib import Path

from ecu_tools.build import digest

from autosar_tooling import c_preprocessor

FORMAT = "c-diagnostic-fingerprints-v4"
SARIF_KEY = "autsaroDiagnostic/v4"
RECEIPT = "diagnostic-fingerprints.json"
SHA256 = re.compile(r"[0-9a-f]{64}")
# #568 adds rejection checks and reports the actual TU search path via cc1.
# Successful scans retain their diagnostic inputs. Full analysis receipts still
# bind the actual driver hash; only this exact revision reuses the disposition identity.
REJECTION_ONLY_DRIVER_REVISIONS = {
    # R11 admits sealed V3 packages only for source-tree baseline analysis.
    "0c512478462d5f65d405502f61f96135b223c591baabd2d15e02317d560e7418":
        "0f933b3b794d240d3a97c674bec20c50dbe0634f9753b831fdbae942d437ebba",
    "a13646572c73a07345e658afb22a52c490894c97029a42e71aafdaef84ecb993":
        "0f933b3b794d240d3a97c674bec20c50dbe0634f9753b831fdbae942d437ebba",
}
# The pinned MISRA addon's analyse_ctu_info can report these using facts
# from units that do not contain any location in the diagnostic.
CTU_KINDS = {
    "misra-c2012-2.3": ("MisraTypedefInfo",),
    "misra-c2012-2.4": ("MisraTagName",),
    "misra-c2012-2.5": ("MisraMacro",),
    "misra-c2012-5.6": ("MisraTypedefInfo",),
    "misra-c2012-5.7": ("MisraTagName",),
    "misra-c2012-5.8": ("MisraExternalIdentifiers", "MisraInternalIdentifiers", "MisraLocalIdentifiers"),
    "misra-c2012-5.9": ("MisraInternalIdentifiers",),
    "misra-c2012-8.5": ("MisraExternalIdentifiers",),
    "misra-c2012-8.6": ("MisraExternalIdentifiers",),
    "misra-c2012-8.7": ("MisraExternalIdentifiers", "MisraUsage"),
}
CTU = frozenset(CTU_KINDS)
TOKEN = re.compile(
    rf"(?:L|u8|u|U)?(?:{c_preprocessor.LITERAL})|"
    r"(?:\d|\.\d)(?:[\w.]|(?<=[eEpP])[+-])*|[A-Za-z_]\w*|"
    r"\.\.\.|>>=|<<=|>>|<<|->|\+\+|--|&&|\|\||[+*/%<>=!&|^\-]=|##|\S"
)
SOURCE_TOKEN = re.compile(r"/\*.*?\*/|//[^\n]*|" + TOKEN.pattern, re.DOTALL)
FUNCTION_MACRO = re.compile(r"#define [A-Za-z_]\w*\(")
LINE_SENSITIVE = re.compile(r"\b__LINE__\b|##|%:%:|(?:\\|\?\?/)[ \t]*(?:\n|$)")
LINE_SPLICE = re.compile(r"(?:\\|\?\?/)\n")
PREPROCESSING_LINE = re.compile(r"(?m)^[ \t\f\v]*(?:#|%:|\?\?=)[ \t\f\v]*(\w*)[^\n]*")
ASCII_LOWER = str.maketrans("ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz")


def sha(value: object) -> str:
    return hashlib.sha256(json.dumps(
        value, ensure_ascii=False, separators=(",", ":"), sort_keys=True,
    ).encode("utf-8")).hexdigest()


def path_key(name: str, windows: bool = False) -> str:
    path = posixpath.normpath(name.replace("\\", "/"))
    # Only identify ASCII case aliases across hosts. Non-ASCII spelling stays exact.
    return path.translate(ASCII_LOWER) if windows else path


def path_index(entries, windows: bool) -> dict[str, str]:
    index = {}
    for name, identity in entries:
        key = path_key(name, windows)
        if key in index:
            raise ValueError(f"Ambiguous registered fingerprint path: {name} ({key})")
        index[key] = identity
    return index


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
                     project_paths: dict[str, str] | None = None,
                     file_tokens: dict | None = None, primary_prefix: list | None = None) -> tuple[str, dict]:
    """Hash the ordered expanded TU and collect expanded tokens at diagnostic anchors."""
    lines = text.splitlines()
    root = source_root(lines, unit, windows)
    root_key = path_key(root, windows)
    project_paths = project_paths or {}
    # __FILE__ can put the source checkout in a string literal as well as a line marker.
    checkout = re.compile("|".join(map(re.escape, (root + "/", root.replace("/", "\\\\") + "\\\\"))),
                          re.IGNORECASE | re.ASCII if windows else 0)
    digest = hashlib.sha256()
    expanded = defaultdict(list)
    filename, number = unit, 0
    last_file = None
    prefix_size = 0
    for line in lines:
        marker = c_preprocessor.LINE.fullmatch(line)
        if marker:
            number = int(marker[1])
            path = path_key(marker[2].replace("\\\\", "\\"), windows)
            relative = path.removeprefix(root_key + "/")
            filename = sdk.get(path, project_paths.get(relative, relative))
            if primary_prefix is not None and filename != unit and not filename.startswith("<"):
                prefix_size = len(file_tokens.get(unit, ()))
        elif line.strip():
            line = checkout.sub("<project>/", line)
            words = tokens(line)
            if FUNCTION_MACRO.match(line):
                words.insert(3, "<function-macro>")
            if number in wanted.get(filename, ()):
                expanded[filename, number].append(words)
            if file_tokens is not None:
                file_tokens[filename].append(words)
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
    if primary_prefix is not None:
        primary_prefix.extend(file_tokens.get(unit, ())[:prefix_size])
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


def analyzer_header_inputs(text: str, exact_lines: bool) -> dict:
    """Retain raw input Cppcheck can use before or while opening a header."""
    removed, offsets = [], [0]
    for splice in LINE_SPLICE.finditer(text):
        removed.append(splice.start() - offsets[-1])
        offsets.append(offsets[-1] + len(splice[0]))
    logical = LINE_SPLICE.sub("", text)
    # A block comment is whitespace even when it spans physical lines.
    masked = c_preprocessor.COMMENT.sub(lambda match: match[1] or " " * len(match[0]), logical)
    directives, include_end = [], 0
    for directive in PREPROCESSING_LINE.finditer(masked):
        start = directive.start() + offsets[bisect_left(removed, directive.start())]
        end = directive.end() + offsets[bisect_right(removed, directive.end())]
        directives.append(text[start:end])
        if directive[1] == "include":
            include_end = end
    prefix = text[:include_end]
    if not exact_lines:
        prefix = "\n".join(line for line in prefix.split("\n") if line)
    return {"prefix": prefix, "directives": directives}


def ctu_facts(output: Path, program: dict, compiler_roots: dict[str, str],
              sdk: dict[str, str], project_paths: dict[str, str], windows: bool) -> tuple[dict, dict]:
    """Index the actual pinned addon's per-program facts by location and by name."""
    cache = output / program["name"] / "cache"
    if not cache.is_dir() and program["name"] == "target":
        cache = output / "cache"
    by_location, by_name = defaultdict(set), defaultdict(list)
    units = program["translationUnits"]
    roots = {path_key(compiler_roots[unit], windows) for unit in units}
    root, = roots
    seen = set()
    for item in (cache / "files.txt").read_text(encoding="utf-8").splitlines():
        carrier, source = item.split(":::", 1)
        unit = project_paths.get(path_key(source, windows).removeprefix(root + "/"))
        if unit not in units or unit in seen or Path(carrier).name != carrier:
            raise ValueError("Invalid CTU translation-unit inventory")
        seen.add(unit)
        for text in (cache / (carrier + ".ctu-info")).read_text(encoding="utf-8").splitlines():
            record = json.loads(text)
            kind = record["summary"]
            for fact in record["data"]:
                key = path_key(fact["file"], windows)
                file = sdk.get(key, project_paths.get(key.removeprefix(root + "/"), key))
                position = (file, int(fact["line"]), int(fact["column"])) if "line" in fact else (file,)
                attributes = {key: value for key, value in fact.items()
                              if key not in {"name", "file", "line", "column"}}
                name = fact["name"]
                by_name[kind, name].append((unit, position, attributes))
                if len(position) == 3:
                    by_location[kind, position].add(name)
    if seen != set(units):
        raise ValueError("Incomplete CTU translation-unit inventory")
    return by_location, by_name


def ctu_dependencies(rule: str, locations: list[tuple], facts: tuple[dict, dict],
                     sources: dict, expanded: dict, unit_hashes: dict, headers: dict) -> list | None:
    by_location, by_name = facts
    kinds = CTU_KINDS[rule]
    names = {name for position in locations for kind in kinds
             for name in by_location.get((kind, position), ())}
    if not names:
        return None
    result = []
    for kind in kinds:
        for name in sorted(names):
            for unit, position, attributes in by_name.get((kind, name), ()):
                file = position[0]
                source = unit_hashes[unit] if unit == file else expanded[unit].get(file)
                anchor = None
                if len(position) == 3 and file in sources:
                    anchor = sources[file].anchor(*position[1:])
                if source is None and anchor is None:
                    return None
                result.append((kind, name, unit, file, anchor, source, headers[unit], attributes))
    return sorted(result, key=sha)


def canonical_config(summary: dict, compiler_identity: dict) -> dict:
    scope = summary["scope"]
    semantic = summary["semanticIdentity"]
    tool = summary["tool"]
    driver = semantic["analysisCode"]["c_check.py"]
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
        "projectChecks": {"c_addon.py": semantic["analysisCode"]["c_addon.py"],
                          "c_check.py": REJECTION_ONLY_DRIVER_REVISIONS.get(driver, driver)},
        "implicitConfig": (scope.get("cppcheckConfig") or {}).get("sha256"),
    }


def inventory(summary: dict, output: Path, compiler_identity: dict) -> list[dict]:
    """Emit one identity per raw diagnostic location and program."""
    scope = summary["scope"]
    windows = scope["target"].startswith("windows-")
    sdk = path_index(((row["path"], "sdk:" + row["key"]) for row in scope["sdkHeaders"]), windows)
    project_names = {name for unit, paths in scope["projectTranslationUnits"].items()
                     for name in (*paths, *scope["cppcheckInputs"][unit]["project"])}
    project_paths = path_index(((name, name) for name in sorted(project_names)), windows)

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
    compiler_roots, ctu_expanded, ctu_headers = {}, {}, {}
    unresolved = set()
    config = sha(canonical_config(summary, compiler_identity))
    compiler_settings = [*scope["compilerFlags"], *summary["semanticIdentity"]["compiler"]["builtins"]]
    line_sensitive_options = LINE_SENSITIVE.search("\n".join(compiler_settings))
    has_ctu = any(row["id"] in CTU for row in summary["diagnostics"])
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
        emissions = defaultdict(list) if has_ctu else None
        prefix = [] if has_ctu else None
        expanded_digest, anchors = compiler_context(
            compiler, unit, sdk, diagnostic_paths, windows=windows, project_paths=project_paths,
            file_tokens=emissions, primary_prefix=prefix,
        )
        if has_ctu:
            compiler_roots[unit] = source_root(compiler.splitlines(), unit, windows)
            ctu_expanded[unit] = {name: sha(words) for name, words in emissions.items()}
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
        if has_ctu:
            # A fact in a header can depend on other declarations in its
            # consumer. Keep header spelling and expansions, excluding the
            # primary source body when it contributes no same-name fact.
            ctu_headers[unit] = sha({
                "expanded": {name: value for name, value in ctu_expanded[unit].items() if name != unit},
                "defines": defines,
                "primaryBeforeHeaders": sha(prefix),
                "analyzerHeaderInputs": analyzer_header_inputs(
                    (project / unit).read_text(encoding="utf-8"), bool(exact_lines),
                ),
                "analyzerHeaders": {name: raw_hashes[name][0 if exact_lines else 1]
                                    for name in analyzer["project"] if name != unit},
                "sdk": sorted(sdk[path_key(name, windows)] for name in sdk_paths),
            })
        for key, lines in anchors.items():
            expanded[key][unit] = lines

    programs = {row["name"]: set(row["translationUnits"]) for row in summary["programs"]}
    facts = {row["name"]: ctu_facts(output, row, compiler_roots, sdk, project_paths, windows)
             for row in summary["programs"]} if has_ctu else {}
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
        locations = []
        for place in diagnostic["locations"]:
            name = logical_path(place["file"])
            line = int(place["line"] or 0)
            column = int(place.get("column") or 0)
            context = raw_context.get(name)
            anchor = context.anchor(line, column) if context else None
            sites.append((name, line, anchor))
            locations.append((name, line, column))
        for program in diagnostic["programs"]:
            units = programs[program]
            related = units.intersection(
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
            dependencies = None
            if complete and rule in CTU:
                dependencies = ctu_dependencies(
                    rule, locations, facts[program], raw_context, ctu_expanded, unit_hashes, ctu_headers,
                )
                complete = dependencies is not None and not units.intersection(unresolved)
            for position in range(len(sites) or 1):
                fingerprint = sha({
                    "version": FORMAT, "configuration": config, "rule": rule,
                    "severity": diagnostic["severity"], "program": program, "sites": evidence,
                    "position": position,
                    **({"ctu": dependencies} if rule in CTU else {
                        "translationUnits": {unit: unit_hashes[unit] for unit in related},
                    }),
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
