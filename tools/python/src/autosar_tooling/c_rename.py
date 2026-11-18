"""Prove private C object renames using sealed sources and both analysis front ends."""

from __future__ import annotations

import gzip
import json
import re
import zlib
from collections import Counter, defaultdict
from pathlib import Path

from ecu_tools.build import FILE_LIST, digest, safe_relative, tool_argument

from autosar_tooling import c_comparison, c_standard

EVIDENCE = "identifier-evidence"
# These rules compare essential types. A spelling-dependent rule needs its own proof.
TYPE_RULES = {"misra-c2012-10.3", "misra-c2012-10.4"}
MARKER = re.compile(r'# (\d+) "((?:[^"\\]|\\.)*)"(?: .*)?')
IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z_0-9]*\Z")
HORIZONTAL_SPACE = re.compile(r"[ \t]+")


class EvidenceError(ValueError):
    """The optional rename proof cannot be used."""


def _normalize_expansion(expansion: str, output: Path) -> str:
    # -g adds a working-directory marker after GCC's first source marker.
    # Only the known cwd at that position is irrelevant to source provenance.
    lines = expansion.split("\n")
    paths = {tool_argument(output), str(output).replace("\\", "\\\\")}
    if len(lines) > 1 and lines[0].startswith('# 0 "') and MARKER.fullmatch(lines[0]) and lines[1] in {
            f'# 1 "{path}//"' for path in paths}:
        lines[1] = '# 1 "@cwd@//"'
    expansion = "\n".join(lines)
    root = output / "project"
    # Preserve relative source locations and the suffix of __FILE__ strings.
    for prefix in {tool_argument(root), str(root).replace("\\", "\\\\")}:
        expansion = expansion.replace(prefix + "/", "@project@/")
        expansion = expansion.replace(prefix + "\\\\", "@project@/")
    return expansion


def _units(summary: dict) -> set[str]:
    scope = summary["scope"]
    if not scope.get("cppcheckInputs"):
        return set()
    return {name for name in scope["translationUnits"]
            if name.endswith(".c")
            and c_comparison.location_scope(scope, {"file": name, "line": 1}) == "in-scope"}


def _proof_path(output: Path, name: str) -> Path:
    return output / EVIDENCE / (name + ".json.gz")


def record(output: Path, summary: dict, candidates: set[str]) -> dict[str, str]:
    """Pack existing GCC expansions and Cppcheck bindings for the transferred gate."""
    if not candidates:
        return {}
    units = _units(summary) & candidates
    root = output / "project"
    bindings = defaultdict(dict)
    for program in summary["programs"]:
        name = program["name"]
        directory = output / name if len(summary["programs"]) > 1 else output
        for path in (directory / "addon-receipts").glob("*.json"):
            receipt = json.loads(path.read_text(encoding="utf-8"))
            source = Path(receipt["source"])
            if ("privateBindings" in receipt and source.is_relative_to(root)
                    and source.relative_to(root).as_posix() in units):
                bindings[source.relative_to(root).as_posix()][name] = receipt["privateBindings"]
    hashes = {}
    for index, unit in enumerate(summary["scope"]["translationUnits"]):
        if unit not in units:
            continue
        path = _proof_path(output, unit)
        if not path.is_file():
            trace = output / f"compiler-origins-{index}.stdout.log"
            if unit not in bindings or not trace.is_file():
                continue
            expansion = trace.read_text(encoding="utf-8")
            path.parent.mkdir(parents=True, exist_ok=True)
            with gzip.open(path, "wt", compresslevel=4, encoding="utf-8", newline="\n") as stream:
                json.dump({"gcc": _normalize_expansion(expansion, output), "bindings": bindings[unit]}, stream)
        hashes[unit] = digest(path)
    return hashes


def evidence(output: Path, summary_digest: str) -> dict[str, str]:
    manifest = output / "analysis-identity.json"
    try:
        record = json.loads(manifest.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {}
    except (OSError, ValueError) as error:
        raise EvidenceError("Identifier evidence manifest unreadable") from error
    if not isinstance(record, dict) or record.get("summarySha256") != summary_digest:
        raise EvidenceError("Identifier evidence belongs to a different analysis summary")
    hashes = record.get("identifierEvidence", {})
    if not isinstance(hashes, dict):
        raise EvidenceError("Identifier evidence manifest has an invalid inventory")
    return hashes


def _verified_path(output: Path, name: str, identity: str) -> Path:
    if not safe_relative(name):
        raise EvidenceError("Invalid identifier evidence source")
    path = _proof_path(output, name)
    try:
        actual = digest(path)
    except OSError as error:
        raise EvidenceError(f"Identifier evidence file missing or unreadable: {name}") from error
    if actual != identity:
        raise EvidenceError(f"Identifier evidence bytes differ: {name}")
    return path


def verify(output: Path, hashes: dict) -> None:
    for name, identity in hashes.items():
        _verified_path(output, name, identity)


def _proof(output: Path, name: str, hashes: dict) -> dict:
    if name not in hashes:
        raise EvidenceError(f"Identifier evidence entry missing: {name}")
    path = _verified_path(output, name, hashes[name])
    try:
        with gzip.open(path, "rt", encoding="utf-8") as stream:
            proof = json.load(stream)
    except (OSError, EOFError, ValueError, zlib.error) as error:
        raise EvidenceError(f"Identifier evidence payload unreadable: {name}") from error
    if not isinstance(proof, dict) or not isinstance(proof.get("gcc"), str) or not proof["gcc"] or not isinstance(
            proof.get("bindings"), dict) or any(
                not isinstance(configs, list) or not all(_valid_bindings(config) for config in configs)
                for configs in proof["bindings"].values()):
        raise EvidenceError(f"Identifier evidence payload has an invalid structure: {name}")
    return proof


def _valid_bindings(config: dict) -> bool:
    return (isinstance(config, dict) and isinstance(config.get("externs"), list)
            and all(isinstance(name, str) for name in config["externs"])
            and isinstance(config.get("uses"), list)
            and all(isinstance(row, list) and len(row) == 5 and isinstance(row[2], str)
                    and all(type(row[index]) is int for index in (0, 1, 3, 4)) for row in config["uses"]))


def _identifier_changes(old: str, new: str) -> list[tuple[c_standard.Token, c_standard.Token]] | None:
    """Keep non-identifier spelling, line breaks and token adjacency unchanged."""
    before, after = c_standard.lex(old), c_standard.lex(new)
    if len(before) != len(after):
        return None
    old_end = new_end = 0
    changes = []
    for left, right in zip(before, after):
        old_gap, new_gap = old[old_end:left.start], new[new_end:right.start]
        if old_gap != new_gap and HORIZONTAL_SPACE.sub(" ", old_gap) != HORIZONTAL_SPACE.sub(" ", new_gap):
            return None
        old_end, new_end = left.end, right.end
        if left.text == right.text:
            continue
        if not IDENTIFIER.fullmatch(left.text) or not IDENTIFIER.fullmatch(right.text):
            return None
        changes.append((left, right))
    if HORIZONTAL_SPACE.sub(" ", old[old_end:]) != HORIZONTAL_SPACE.sub(" ", new[new_end:]):
        return None
    return changes


def _physical_changes(old: str, new: str) -> dict | None:
    substitutions = _identifier_changes(old, new)
    if not substitutions:
        return None
    # C translation phases can join tokens before the offset lexer sees them.
    unsupported = r"\?\?[=/'()!<>-]|%:|\\[uU]"
    outside_directives = r"(?m)^[ \t]*\#(?:\\\n|[^\n])*"
    if any(re.search(unsupported, source) or "\\\n" in re.sub(outside_directives, "", source)
           for source in (old, new)):
        return None
    changes = defaultdict(list)
    for left, right in substitutions:
        line = old.count("\n", 0, left.start) + 1
        old_column = left.start - old.rfind("\n", 0, left.start)
        new_column = right.start - new.rfind("\n", 0, right.start)
        changes[line].append(((line, old_column, left.text), (line, new_column, right.text)))
    # Horizontal padding can shift columns on a rename line. Other edited
    # lines have no identifier substitution to verify against GCC.
    lines = {line for line, (left, right) in enumerate(zip(old.split("\n"), new.split("\n")), 1)
             if left != right}
    return changes if lines == changes.keys() else None


def _name_positions(source: str, names: set[str]) -> dict:
    positions = defaultdict(set)
    for token in c_standard.lex(source):
        if token.text in names:
            line = source.count("\n", 0, token.start) + 1
            column = token.start - source.rfind("\n", 0, token.start)
            positions[line, token.text].add((line, column, token.text))
    return positions


def _binding_pairs(before: dict, after: dict, changes: dict,
                   old_positions: dict, new_positions: dict) -> bool:
    if (any(name in before["externs"] for _, name in old_positions)
            or any(name in after["externs"] for _, name in new_positions)):
        return False
    old_to_new = {left: right for rows in changes.values() for left, right in rows}
    new_to_old = {right: left for left, right in old_to_new.items()}
    def bindings(rows, substitutions, physical):
        # Cppcheck may locate a macro argument at the invocation column. GCC
        # independently checks the exact renamed token multiplicity on that line.
        declarations = defaultdict(set)
        for line, column, name, decl_line, decl_column in rows:
            declaration = decl_line, decl_column, name
            if declaration not in substitutions:
                continue
            position = line, column, name
            candidates = physical[line, name]
            if position not in candidates and len(candidates) == 1:
                position = next(iter(candidates))
            if position not in substitutions:
                return None
            declarations[position].add(declaration)
        return declarations

    old_bindings = bindings(before["uses"], old_to_new, old_positions)
    new_bindings = bindings(after["uses"], new_to_old, new_positions)
    if old_bindings is None or new_bindings is None:
        return False
    for left, right in old_to_new.items():
        old_decl = old_bindings[left]
        new_decl = new_bindings[right]
        if not old_decl:
            return False
        if len(old_decl) > 1 or {old_to_new[token] for token in old_decl} != new_decl:
            return False
    return True


def _expansions(before: str, after: str, unit: str, changes: dict) -> set[int] | None:
    old_lines, new_lines = before.splitlines(), after.splitlines()
    if not old_lines or len(old_lines) != len(new_lines):
        return None
    forward, reverse = {}, {}
    for rows in changes.values():
        for left, right in rows:
            old_name, new_name = left[2], right[2]
            if (forward.setdefault(old_name, new_name) != new_name
                    or reverse.setdefault(new_name, old_name) != old_name):
                return None
    destinations = re.compile(r"\b(?:" + "|".join(reverse) + r")\b")
    emitted = defaultdict(Counter)
    name, line = "", 0
    for old, new in zip(old_lines, new_lines):
        marker = MARKER.fullmatch(old)
        if marker:
            if old != new:
                return None
            line, name = int(marker[1]), marker[2].replace("\\\\", "/")
            continue
        if destinations.search(old):
            return None
        if name == "@project@/" + unit and not old.startswith("#") and re.search(
                r"\b(?:asm|__asm|__asm__|alias|__alias__|weakref|__weakref__)\b", old):
            return None
        # GCC pads expanded macro tokens to the invocation column, including
        # lines attributed to a macro's header rather than the invocation.
        old_code, new_code = old.strip(" \t"), new.strip(" \t")
        if old_code != new_code:
            if name != "@project@/" + unit or line not in changes:
                return None
            actual = _identifier_changes(old_code, new_code)
            if not actual:
                return None
            emitted[line].update((a.text, b.text) for a, b in actual)
        line += 1
    if emitted.keys() != changes.keys() or any(pairs != Counter((a[2], b[2]) for a, b in changes[line])
           for line, pairs in emitted.items()):
        return None
    return set(emitted)


def _source_changes(previous: dict, current: dict, old_root: Path, new_root: Path) -> dict:
    before, after = previous["scope"], current["scope"]
    dependencies = before["projectTranslationUnits"]
    analyzer_inputs = before.get("cppcheckInputs")
    if (dependencies != after["projectTranslationUnits"] or analyzer_inputs is None
            or analyzer_inputs != after.get("cppcheckInputs")
            or before.get("sdkLineDirectives") or after.get("sdkLineDirectives")):
        return {}
    files = set((old_root / FILE_LIST).read_text(encoding="utf-8").splitlines())
    if files != set((new_root / FILE_LIST).read_text(encoding="utf-8").splitlines()):
        return {}
    # These verified producer receipts contain hashes of the very sources
    # being compared. The selected configurations are checked by compare().
    sources = files - {"meta/assets.json", "meta/integration.json", "target.json"}
    changed = {name for name in sources if digest(old_root / name) != digest(new_root / name)}
    if not changed <= _units(previous) & _units(current):
        return {}
    result = {}
    for name in changed:
        if any(unit != name and name in paths for unit, paths in dependencies.items()):
            return {}
        if any(unit != name and name in paths["project"] for unit, paths in analyzer_inputs.items()):
            return {}
        old = (old_root / name).read_text(encoding="utf-8")
        new = (new_root / name).read_text(encoding="utf-8")
        positions = _physical_changes(old, new)
        if positions is None:
            return {}
        result[name] = old, new, positions
    return result


def equivalent(previous: dict, current: dict, baseline: Path, output: Path, comparison: dict,
               hashes: dict, previous_headers: set | None, headers: set | None) -> Counter:
    """Count only complete, same-position, same-program replacements in the strict delta."""
    result = Counter()
    if not hashes or not any(hashes.values()) or not any(row["rule"] in TYPE_RULES for row in comparison["new"]):
        return result
    sources = _source_changes(previous, current, baseline / "project", output / "project")
    for name, (old, new, changes) in sources.items():
        proofs = []
        for side, directory in (("baseline", baseline), ("current", output)):
            try:
                proofs.append(_proof(directory, name, hashes[side]))
            except EvidenceError as error:
                raise EvidenceError(f"{side}: {error}") from error
        before, after = proofs
        active = _expansions(before["gcc"], after["gcc"], name, changes)
        programs = {row["name"] for row in current["programs"] if name in row["translationUnits"]}
        if not active:
            return Counter()
        if not programs or before["bindings"].keys() != programs or after["bindings"].keys() != programs:
            raise EvidenceError(f"Identifier evidence program inventory differs: {name}")
        old_positions = _name_positions(old, {left[2] for rows in changes.values() for left, _ in rows})
        new_positions = _name_positions(new, {right[2] for rows in changes.values() for _, right in rows})
        for program in programs:
            left, right = before["bindings"][program], after["bindings"][program]
            if not left or not right:
                raise EvidenceError(f"Identifier evidence has no configurations: {name} ({program})")
            if len(left) != len(right) or not all(
                    _binding_pairs(a, b, changes, old_positions, new_positions)
                    for a, b in zip(left, right)):
                return Counter()
        result.update(_pairs(previous, current, name, old, new, active, comparison, previous_headers, headers))
    return result


def _pairs(previous, current, name, old, new, active, comparison, previous_headers, headers):
    old_lines, new_lines = old.split("\n"), new.split("\n")
    # Match the same raw token after rename and horizontal padding shift, not
    # another operand/operator on the same line. Unlocated reports get no credit.
    anchors = {
        line: [{token.start + 1: index for index, token in enumerate(c_standard.lex(source[line - 1]))}
               for source in (old_lines, new_lines)]
        for line in active
    }
    def positions(summary, scope_headers, side):
        found = defaultdict(Counter)
        for diagnostic in summary["diagnostics"]:
            if diagnostic["id"] not in TYPE_RULES:
                continue
            for location in diagnostic["locations"]:
                line = int(location["line"])
                if (location["file"] == name and line in active
                        and c_comparison.location_scope(summary["scope"], location,
                                                        headers=scope_headers) == "in-scope"):
                    column = location.get("column")
                    if not str(column).isdecimal() or (anchor := anchors[line][side].get(int(column))) is None:
                        return None
                    found[diagnostic["id"], line, anchor].update(diagnostic["programs"])
        return found

    before, after = positions(previous, previous_headers, 0), positions(current, headers, 1)
    if before is None or before != after:
        return Counter()
    removed, added = Counter(), Counter()
    for (rule, line, _anchor), programs in before.items():
        for program, count in programs.items():
            removed[rule, name, old_lines[line - 1], program] += count
            added[rule, name, new_lines[line - 1], program] += count

    def strict(side):
        return Counter({(row["rule"], row["file"], row["source"], row["program"]): row["count"]
                        for row in comparison[side] if row["file"] == name and row["rule"] in TYPE_RULES})

    return added if removed == strict("removed") and added == strict("new") else Counter()
