"""Source-level C coding contracts, including inactive preprocessor branches.

This lexer preserves offsets; it does not preprocess or infer comment semantics.
Compiler/MISRA checks remain responsible for C validity and normative semantics.
"""

from __future__ import annotations

import collections
import hashlib
import itertools
import json
import re
from dataclasses import dataclass
from pathlib import Path

from autosar_tooling.config import ROOT

BASELINE = Path(__file__).with_name("c_standard_baseline.txt")
MANUAL_ASSESSMENT_SOURCES = (
    "docs/standards/os/native-facilities/storage-interrupts.md",
    "docs/standards/bsw-general/language-tokens.md",
    "docs/standards/types/public-types.md",
    "docs/standards/types/storage.md",
    "docs/standards/types/declarations.md",
)
Key = tuple[str, str, str]
TOKEN = re.compile(
    r'/\*.*?\*/|//[^\n]*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|'
    r'^[ \t]*\#(?:\\\n|[^\n])*|[A-Za-z_]\w*|\d[\w.]*|[^\s]',
    re.DOTALL | re.MULTILINE,
)
IDENTIFIER = re.compile(r"[A-Za-z_]\w*\Z")
NATIVE = {"char", "short", "int", "long", "signed", "unsigned", "float", "double", "_Bool"}
FIELDS = ("Preconditions", "Context", "Reentrancy", "Ownership")
PARAM = re.compile(r"@param\[(in|out|in,out)\]\s+(\w+)\s+([^@\n]+)")
DEVIATION = re.compile(r"MR12\s+(RULE|DIR)\s+(\d+\.\d+)\s+VIOLATION:")


@dataclass(frozen=True)
class Token:
    text: str
    start: int
    end: int


def lex(source: str) -> list[Token]:
    return [Token(m.group(), m.start(), m.end()) for m in TOKEN.finditer(source)]


def is_comment(token: Token) -> bool:
    return token.text.startswith(("/*", "//"))


def _manual_records(root: Path) -> tuple[dict, dict]:
    records = {}
    identities = {}
    for name in MANUAL_ASSESSMENT_SOURCES:
        data = (root / name).read_bytes()
        identities[name] = hashlib.sha256(data).hexdigest()
        active = False
        for line, text in enumerate(data.decode("utf-8").splitlines(), 1):
            if text == "<!-- misra-manual-begin -->":
                if active:
                    raise ValueError(f"Nested manual assessment block: {name}:{line}")
                active = True
            elif text == "<!-- misra-manual-end -->":
                if not active:
                    raise ValueError(f"Unopened manual assessment block: {name}:{line}")
                active = False
            elif active and re.match(r"\| [DR]\d+\.", text):
                cells = [cell.strip() for cell in text.split("|")[1:-1]]
                if len(cells) != 5 or not re.fullmatch(r"[DR]\d+\.\d+", cells[0]):
                    raise ValueError(f"Malformed manual assessment: {name}:{line}")
                guideline, category, scope, status, conclusion = cells
                if (guideline in records or not scope or not conclusion or (category == "Mandatory" and status == "retained")
                        or status not in {"satisfied", "not_applicable", "retained", "not_assessed"}):
                    raise ValueError(f"Invalid or duplicate manual assessment: {name}:{line}")
                records[guideline] = {"id": guideline, "category": category, "analysisScope": scope,
                                      "status": status, "conclusion": conclusion,
                                      "source": {"file": name, "line": line}}
        if active:
            raise ValueError(f"Unclosed manual assessment block: {name}")
    return records, identities


def manual_assessment(rules: list[dict], root: Path = ROOT) -> dict:
    """Summarize documented frozen evidence; never assess the current source seal."""
    records, identities = _manual_records(root)
    uncovered = {row["id"]: row for row in rules if not row["automaticChecker"]}
    if len({row["id"] for row in rules}) != len(rules):
        raise ValueError("Duplicate checker catalog guideline")
    if set(records) - set(uncovered):
        raise ValueError("Manual assessment contains checker-covered or unknown guidelines")
    if any(row["category"] != uncovered[key]["category"] for key, row in records.items()):
        raise ValueError("Manual assessment category differs from the fixed catalog")
    rows = [records.get(key, {"id": key, "category": row["category"], "status": "not_assessed",
                              "conclusion": "Manual record missing"}) for key, row in uncovered.items()]
    counts = collections.Counter(row["status"] for row in rows)
    unclassified = counts["not_assessed"]
    return {
        "format": "misra-manual-assessment-v1",
        "scope": "Frozen configurations and sources cited by the records; not the current analyzed source seal",
        "inputIdentity": {
            "parserSha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "selectionSha256": hashlib.sha256("\n".join(MANUAL_ASSESSMENT_SOURCES).encode()).hexdigest(),
            "documents": identities,
        },
        "checkerUncovered": {"total": len(uncovered), "classified": len(rows) - unclassified,
                             "unclassified": unclassified, "classificationComplete": unclassified == 0,
                             "statusCounts": dict(sorted(counts.items())), "records": rows},
        "checkerCovered": {"total": len(rules) - len(uncovered),
                           "ids": [row["id"] for row in rules if row["automaticChecker"]],
                           "assessment": "not_assessed",
                           "limitations": "coverage.knownCheckerLimitations remain applicable"},
        "conformanceAssessment": "not_established",
    }


def adjacent(source: str, tokens: list[Token], start: int) -> str:
    """Only the comment immediately before the declaration can describe it."""
    comments = [t for t in tokens if t.end <= start and is_comment(t)]
    if comments and not source[comments[-1].end:start].strip():
        return comments[-1].text
    return ""


def trailing(source: str, end: int) -> bool:
    line = source[end:].split("\n", 1)[0]
    return bool(re.match(r"\s*/\*\*<\s*[^\s*/]", line))


def comment_body(comment: str) -> str:
    """Discard C/Doxygen delimiters before deciding whether prose exists."""
    return re.sub(r"(?m)^\s*\* ?", "", re.sub(r"/\*+[<!]?|\*/|//[/!]?", "", comment)).strip()


def include_scope(text: str) -> str | None:
    """Normalize directive spacing, retaining the literal header and delimiter."""
    space = r"(?:[ \t\f\v]|/\*.*?\*/)*"
    match = re.fullmatch(
        rf'{space}#{space}include{space}(<[^>\r\n]+>|"[^"\r\n]+")'
        rf"{space}(?://[^\r\n]*)?",
        re.sub(r"\\\r?\n", "", text).strip(),
        re.DOTALL,
    )
    return "#include " + match[1] if match else None


def directive_scope(text: str) -> str | None:
    """Identify literal includes and undef targets without accepting record locators as C."""
    included = include_scope(text)
    if included is not None:
        return included
    text = re.sub(r"\\\r?\n", "", text).strip()
    text = re.sub(r"/\*.*?\*/|//[^\r\n]*", " ", text, flags=re.DOTALL)
    match = re.fullmatch(r"\s*#\s*undef\s+([A-Za-z_]\w*)\s*", text)
    return "#undef " + match[1] if match else None


def record_scope(text: str) -> str | None:
    """Identify a file anchor, symbol, or concrete repeated directive."""
    if text == "@file" or IDENTIFIER.fullmatch(text):
        return text
    occurrence = re.fullmatch(r"(.+?)\s+\[([1-9][0-9]*)\]", text, re.DOTALL)
    normalized = directive_scope(occurrence[1] if occurrence else text)
    if normalized is not None:
        return normalized + (" [" + occurrence[2] + "]" if occurrence else "")
    # An approved malformed directive must remain unmatched, rather than disappear.
    return text if text.lstrip().startswith("#") else None


def function(tokens: list[Token]) -> tuple[str, list[str]] | None:
    """Recognize direct function declarators, not function pointer objects/types."""
    words = [t.text for t in tokens]
    if "typedef" in words or "=" in words:
        return None
    attribute_depth = 0
    for index, word in enumerate(words):
        if index and words[index - 1] in {"__attribute__", "__declspec"} and word == "(":
            attribute_depth = 1
            continue
        if attribute_depth:
            attribute_depth += (word == "(") - (word == ")")
            continue
        if word == "(" and index and IDENTIFIER.fullmatch(words[index - 1]) and words[index + 1:index + 2] != ["*"]:
            name = words[index - 1]
            if name in NATIVE | {"void", "__attribute__", "__declspec", "sizeof"}:
                continue
            parameters = []
            segment = []
            nested = 1
            for value in words[index + 1:]:
                if value == ")" and nested == 1:
                    if segment and segment != ["void"]:
                        parameters.append(parameter_name(segment))
                    return name, parameters
                if value == "," and nested == 1:
                    parameters.append(parameter_name(segment))
                    segment = []
                else:
                    segment.append(value)
                nested += (value == "(") - (value == ")")
    return None


def parameter_name(words: list[str]) -> str:
    # Callback parameters have their name inside (*name), arrays before [extent].
    for index in range(len(words) - 2):
        if words[index:index + 2] == ["(", "*"]:
            return words[index + 2] if IDENTIFIER.fullmatch(words[index + 2]) else "<unnamed>"
    words = words[:words.index("[")] if "[" in words else words
    qualifiers = {"const", "volatile", "restrict", "struct", "union", "enum"}
    names = [word for word in words if IDENTIFIER.fullmatch(word) and word not in qualifiers]
    if any(word in NATIVE for word in words):
        names = [word for word in names if word not in NATIVE]
        return names[-1] if names else "<unnamed>"
    return names[-1] if len(names) > 1 else "<unnamed>"


def contract_rules(comment: str, parameters: list[str], *, returns_void: bool = False) -> list[str]:
    rules = []
    if not comment.startswith("/**") or comment.startswith("/**<"):
        return ["api-contract"]
    brief = re.split(r"@param|@return|@par|Preconditions:|Context:|Reentrancy:|Ownership:", comment)[0]
    if not re.sub(r"[/\s*]|@brief", "", brief):
        rules.append("api-purpose")
    body = comment_body(comment)
    boundary = r"(?=\b(?:" + "|".join(FIELDS) + r"):|@|\Z)"
    for field in FIELDS:
        value = re.search(rf"\b{field}:\s*(.*?)" + boundary, body, re.DOTALL)
        if not value or not value[1].strip():
            rules.append("api-" + field.lower())
    void_return = returns_void and re.search(r"@par\s+Return\s+None\.", body)
    returned = re.search(r"@return\s+(.*?)" + boundary, body, re.DOTALL)
    if not void_return and (not returned or not returned[1].strip()):
        rules.append("api-return")
    documented = [match[1] for match in PARAM.findall(body) if match[2].strip(" */")]
    positional = []
    section = re.search(r"@par\s+Parameters\s+(.*?)(?=@par|@return|Preconditions:|Context:|Reentrancy:|Ownership:|\Z)", body, re.DOTALL)
    if section:
        positional = [int(m[0]) if m[1] in {"in", "out", "in,out"} and m[2].strip(" */") else -1 for m in re.findall(r"(?m)^\s*\*?\s*(\d+)\.\s*\[([^]]*)\]\s*([^@\n]*)", section[1])]
    expected_positions = [index for index, name in enumerate(parameters, 1) if name == "<unnamed>"]
    if (collections.Counter(documented) != collections.Counter(name for name in parameters if name != "<unnamed>")
        or positional != expected_positions):
        rules.append("api-parameters")
    return rules


def void_function(tokens: list[Token], name: str) -> bool:
    prefix = [token.text for token in tokens[:next(i for i, token in enumerate(tokens) if token.text == name)]]
    return "void" in prefix and "*" not in prefix


def verify_memory_mapping_pattern(source: str) -> int:
    """Check the shared START/include/body/STOP/include protocol before class checks."""
    active = None
    expecting = False
    header = None
    count = 0
    # C removes splices and replaces comments before recognizing preprocessing lines.
    source = re.sub(r"\\\r?\n", "", source)
    source = re.sub(
        r'("(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\')|/\*.*?\*/|//[^\n]*',
        lambda match: match[1] if match[1] is not None else " ",
        source, flags=re.DOTALL,
    )
    for token in lex(source):
        text = token.text.strip()
        included = include_scope(text)
        if included and re.search(r'_MemMap\.h[">]$', included):
            if not expecting:
                raise AssertionError("unexpected MemMap include")
            if header is not None and header != included:
                raise AssertionError("mismatched MemMap header")
            header = included if active is not None else None
            expecting = False
            count += 1
            continue
        if expecting:
            raise AssertionError("mapping marker must immediately include its header")
        marker = re.fullmatch(r"#\s*define\s+(\w+)_(START|STOP)_SEC_(\w+)", text)
        if marker:
            prefix, direction, section = marker.groups()
            if direction == "START":
                if active is not None:
                    raise AssertionError("nested mapping")
                active = (prefix, section)
            else:
                if active != (prefix, section):
                    raise AssertionError("mismatched mapping")
                active = None
            expecting = True
        elif text.startswith("#"):
            if active is not None:
                raise AssertionError("preprocessing inside mapped section")
    if active is not None or expecting:
        raise AssertionError("unclosed memory mapping")
    return count


def memory_mapping_findings(name: str, source: str) -> collections.Counter[Key]:
    try:
        verify_memory_mapping_pattern(source)
    except AssertionError as error:
        return collections.Counter({("memory-mapping", name, str(error)): 1})
    return collections.Counter()


def findings(name: str, source: str, *, bsw: bool | None = None, known_switches: frozenset[str] = frozenset()) -> collections.Counter[Key]:
    result: collections.Counter[Key] = collections.Counter()
    lines = source.splitlines()
    tokens = lex(source)
    bsw = name.startswith("runtime/bsw/") if bsw is None else bsw
    public = name.endswith(".h") and "include" in Path(name).parts

    def add(rule: str, token: Token) -> None:
        line = source.count("\n", 0, token.start)
        result[(rule, name, lines[line].strip())] += 1

    directives = [t for t in tokens if t.text.lstrip().startswith("#")]
    guard = None
    if name.endswith(".h") and directives:
        match = re.fullmatch(r"\s*#\s*ifndef\s+(\w+)\s*", directives[0].text)
        if match and any(re.fullmatch(rf"\s*#\s*define\s+{match[1]}\s*", t.text) for t in directives[1:]):
            guard = match[1]
    definition_guards = set()
    validation_guards = set()
    for first, second, third in zip(directives, directives[1:], directives[2:]):
        default = re.fullmatch(r"\s*#\s*ifndef\s+(\w+)\s*", first.text)
        if (default and re.match(rf"\s*#\s*define\s+{default[1]}\b", second.text)
            and re.fullmatch(r"\s*#\s*endif\s*", third.text)
            and not re.sub(r"/\*.*?\*/|//[^\n]*", "", source[first.end:second.start] + source[second.end:third.start], flags=re.DOTALL).strip()):
            definition_guards.add(first.start)
        # A value-range or explicit-override diagnostic validates a definition;
        # it does not enable a feature through the symbol's presence.
        if (re.match(r"\s*#\s*(?:if|ifndef)\b", first.text) and re.match(r"\s*#\s*error\b", second.text)
            and re.fullmatch(r"\s*#\s*endif\s*", third.text)
            and not re.sub(r"/\*.*?\*/|//[^\n]*", "", source[first.end:second.start] + source[second.end:third.start], flags=re.DOTALL).strip()):
            identifiers = set(re.findall(r"\b[A-Z][A-Z0-9_]+\b", first.text)) - {"STD_ON", "STD_OFF"}
            required = re.fullmatch(r"\s*#\s*ifndef\s+\w+\s*", first.text)
            consistency = re.fullmatch(r"\s*#\s*if\s+\(?\s*[A-Z][A-Z0-9_]+\s*!=\s*[A-Z][A-Z0-9_]+\s*\)?\s*", first.text)
            if required or consistency or (identifiers and all(
                (re.search(rf"{symbol}\s*!=\s*(?:0|STD_OFF)\b", first.text)
                 and re.search(rf"{symbol}\s*!=\s*(?:1|STD_ON)\b", first.text))
                or (re.search(rf"defined\s*\(\s*{symbol}\s*\)", first.text)
                    and re.search(rf"{symbol}\s*!=\s*(?:[01]|STD_ON|STD_OFF)\b", first.text))
                for symbol in identifiers
            )):
                validation_guards.add(first.start)
    # Version presence/value assertions reject an incompatible import; they do
    # not select functionality by a configuration switch's presence.
    for index, first in enumerate(directives):
        if not re.match(r"\s*#\s*if\b", first.text):
            continue
        chain = []
        for current in directives[index:]:
            chain.append(current)
            if re.match(r"\s*#\s*endif\b", current.text):
                break
        commands = [re.match(r"\s*#\s*(\w+)", item.text)[1] for item in chain]
        if not re.fullmatch(r"if error(?: elif error)* endif", " ".join(commands)):
            continue
        if any(re.sub(r"/\*.*?\*/|//[^\n]*", "", source[a.end:b.start], flags=re.DOTALL).strip()
               for a, b in itertools.pairwise(chain)):
            continue
        conditions = chain[:-1:2]
        identifiers = set(re.findall(r"\b[A-Z][A-Z0-9_]+\b", " ".join(item.text for item in conditions)))
        if identifiers and all(re.fullmatch(r"[A-Z][A-Z0-9_]*_(?:AR_RELEASE_(?:MAJOR|MINOR|REVISION)|SW_(?:MAJOR|MINOR|PATCH))_VERSION", name)
                               for name in identifiers):
            validation_guards.update(item.start for item in conditions)
    switches = set(known_switches)
    for token in directives:
        match = re.match(r"\s*#\s*define\s+(\w+)\s+\(?(?:STD_(?:ON|OFF)|[01](?:[uUlL]*))\)?\s*$", re.sub(r"/\*.*?\*/|//[^\n]*", "", token.text, flags=re.DOTALL))
        if match:
            switches.add(match[1])
    group = ""
    for token in tokens:
        if is_comment(token):
            group = token.text
            continue
        directive = re.match(r"\s*#\s*(\w+)\s*(.*)", token.text, re.DOTALL)
        if directive:
            command, body = directive.groups()
            if command in {"ifdef", "ifndef", "if", "elif"} and token.start not in definition_guards | validation_guards:
                identifiers = set(re.findall(r"\b[A-Z][A-Z0-9_]+\b", body)) - {"STD_ON", "STD_OFF", guard}
                for switch in sorted(identifiers):
                    if Path(name).name.startswith("Rte") and (
                        switch in {"RTE_CORE", "RTE_APPLICATION_HEADER_FILE"}
                        or switch.startswith("RTE_RUNNABLEAPI_")
                    ):
                        continue
                    if (re.search(r"_(?:START_SEC_|STOP_SEC_|MEMMAP(?:_|$))", switch)
                        or (name.endswith("_MemMap.h") and switch.endswith(("_CODE", "_THREAD_LOCAL")))):
                        continue
                    if (
                        command in {"ifdef", "ifndef"}
                        or switch in switches
                        or "defined" in body
                        or body.strip() == switch
                        or re.search(r"(?:_API|_SUPPORT|_DETECT|_ENABLED)$", switch)
                    ) and not re.search(rf"\(\s*{switch}\s*==\s*STD_ON\s*\)", body):
                        add("config-switch", token)
            if public and command == "define":
                macro = re.match(r"(\w+)(.*)", body, re.DOTALL)
                if macro and macro[1] != guard and not name.endswith("_MemMap.h") and not re.search(r"_(?:START|STOP)_SEC_", macro[1]):
                    tail = source[token.end:].split("\n", 1)[0]
                    inline = any(comment_body(c) for c in re.findall(r"/\*.*?\*/|//[^\n]*", token.text + tail, re.DOTALL))
                    comment = adjacent(source, tokens, token.start)
                    if macro[2].startswith("("):
                        # Whitespace separates object-like replacements from function-like macros.
                        params = macro[2].split(")", 1)[0][1:].split(",")
                        for rule in contract_rules(comment, [p.strip() for p in params if p.strip()]):
                            add("macro-" + rule, token)
                    else:
                        documentation = comment + " " + group + " " + token.text
                        if re.search(r"\bSTD_(?:ON|OFF)\b", macro[2]):
                            if not re.search(r"\bECUC_?", documentation):
                                add("macro-configuration", token)
                        elif not inline and not re.search(
                            r"SWS_|Published|version", comment + " " + group, re.IGNORECASE
                        ):
                            add("macro-comment", token)
            continue
        # Native types in code, not strings, comments or macro bodies (checked below).
        if bsw and token.text in NATIVE and Path(name).name != "Platform_Types.h":
            add("bsw-native-type", token)
        if token.text not in {",", ";"}:
            group = ""
    for token in directives:
        if bsw and Path(name).name != "Platform_Types.h":
            match = re.match(r"\s*#\s*define\s+\w+\s*(.*)", token.text, re.DOTALL)
            if match:
                for value in lex(match[1]):
                    if value.text in NATIVE:
                        offset = token.start + match.start(1)
                        add("bsw-native-type", Token(value.text, offset + value.start, offset + value.end))

    code = [t for t in tokens if not is_comment(t) and not t.text.lstrip().startswith("#")]
    if bsw and name.endswith(".c"):
        for index, token in enumerate(code[:-1]):
            if token.text not in {"Det_ReportError", "Det_ReportRuntimeError", "Det_ReportTransientFault"} or code[index + 1].text != "(":
                continue
            depth, argument = 0, 0
            for value in code[index + 2:]:
                if value.text == ")" and depth == 0:
                    break
                if value.text == "," and depth == 0:
                    argument += 1
                elif argument in {0, 2} and value.text[0].isdigit():
                    add("det-numeric-id", value)
                depth += (value.text == "(") - (value.text == ")")
    stack: list[str] = []
    statement: list[Token] = []
    parens = 0
    initializer_declaration: list[Token] = []
    aggregate_declaration: list[Token] = []
    for token in code:
        word = token.text
        if word == "{" and parens == 0:
            words = [t.text for t in statement]
            aggregate = ("body" if function(statement) else "linkage" if words == ["extern", '"C"']
                         else "initializer" if "=" in words else "enum" if "enum" in words
                         else "record" if set(words) & {"struct", "union"} else "body")
            if (not stack or stack == ["linkage"]) and aggregate == "initializer":
                initializer_declaration = statement
            if (not stack or stack == ["linkage"]) and aggregate in {"record", "enum"}:
                aggregate_declaration = statement
            if ((not stack or stack == ["linkage"]) and aggregate != "body" and public and "typedef" in words
                and not comment_body(adjacent(source, tokens, statement[0].start))):
                add("typedef-comment", statement[0])
            if (not stack or stack == ["linkage"]) and aggregate == "body" and public:
                declaration = function(statement)
                if declaration:
                    for rule in contract_rules(adjacent(source, tokens, statement[0].start), declaration[1], returns_void=void_function(statement, declaration[0])):
                        add(rule, statement[0])
            stack.append(aggregate)
            statement = []
            continue
        if word == "}" and parens == 0:
            if (stack and stack[-1] == "enum" and statement and public
                and not trailing(source, statement[-1].end)):
                add("member-comment", statement[0])
            stack_kind = stack.pop() if stack else "body"
            statement = (initializer_declaration if (not stack or stack == ["linkage"]) and stack_kind == "initializer"
                         else [*aggregate_declaration, token] if (not stack or stack == ["linkage"]) and stack_kind in {"record", "enum"} else [])
            continue
        if stack and "body" in stack:
            continue
        statement.append(token)
        parens += (word == "(") - (word == ")")
        if word == "," and parens == 0 and stack and stack[-1] == "enum":
            if public and not trailing(source, token.end):
                add("member-comment", statement[0])
            statement = []
        if word != ";" or parens:
            continue
        words = [t.text for t in statement]
        if stack and stack[-1] in {"record", "enum"}:
            if public and not trailing(source, token.end):
                add("member-comment", statement[0])
        else:
            comment = adjacent(source, tokens, statement[0].start)
            if "typedef" in words:
                if public and "}" not in words and not comment_body(comment):
                    add("typedef-comment", statement[0])
            elif declaration := function(statement):
                if public:
                    for rule in contract_rules(comment, declaration[1], returns_void=void_function(statement, declaration[0])):
                        add(rule, statement[0])
            elif ("extern" not in words
                  and not (words[0] in {"struct", "union", "enum"} and len(words) == 3)
                  and not ("}" in words and words[-2] == "}")
                  and not trailing(source, token.end)):
                add("state-comment", statement[0])
        statement = []
    return result


def switch_definitions(sources: list[str]) -> frozenset[str]:
    return frozenset(match[1] for source in sources for token in lex(source)
                     if (match := re.fullmatch(r"\s*#\s*define\s+(\w+)\s+\(?STD_(?:ON|OFF)\)?\s*", re.sub(r"/\*.*?\*/|//[^\n]*", "", token.text, flags=re.DOTALL))))


def runtime_findings(root: Path = ROOT) -> collections.Counter[Key]:
    result: collections.Counter[Key] = collections.Counter()
    sources = {path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
               for path in sorted((root / "runtime").rglob("*")) if path.suffix in {".c", ".h"}}
    switches = switch_definitions(list(sources.values()))
    for name, source in sources.items():
        result.update(findings(name, source, known_switches=switches))
        result.update(memory_mapping_findings(name, source))
    return result


def baseline_errors(current: collections.Counter[Key], baseline: Path = BASELINE, *, generated: bool = False, documentation: bool = False) -> list[str]:
    expected: collections.Counter[Key] = collections.Counter()
    for line in baseline.read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            rule, name, text = line.split(" ", 2)
            if ((documentation and rule.startswith("clang-documentation"))
                or (not documentation and not rule.startswith("clang-documentation")
                    and (generated or not name.startswith("emitted/")))):
                expected[(rule, name, text)] += 1
    return [
        f"{name}: new C standard finding [{rule}]: {text}"
        for (rule, name, text), count in sorted((current - expected).items())
        for _ in range(count)
    ] + [
        f"{baseline.name}: fixed finding, remove: {rule} {name} {text}"
        for (rule, name, text), count in sorted((expected - current).items())
        for _ in range(count)
    ]


def host_address_instances(root: Path) -> tuple[dict, collections.Counter[Key]]:
    """Identify reviewed native address expressions without accepting other conversions."""
    document = "docs/standards/os/native-facilities/addresses.md"
    title = "## 主机地址表示类别处置"
    patterns = (
        ("runtime/os/src/host/linux/Os_HostLinux.c", "11.5", "Os_HostCreateActor",
         "thread->mapping = mmap(NULL, reserve, (int)protection, (int)mapping_flags, -1, 0);"),
        ("runtime/os/src/host/linux/Os_HostLinux.c", "11.4", "Os_HostStackRange",
         "*low = (uintptr_t)thread->mapping;"),
        ("runtime/os/src/host/linux/Os_StackLinux.c", "11.4", "Os_StackRecordBuffer",
         "current_stack->kernel_buffer_high = (uintptr_t)&top[1];"),
        ("runtime/os/src/Os_Stack.c", "11.4", "Os_StackRecordBuffer",
         "current_stack->kernel_buffer_high = (uintptr_t)&top[1];"),
        ("runtime/os/src/Os_Stack.c", "11.6", "Os_StackRegister",
         "record->reserve_low = (uintptr_t)region.AllocationBase;"),
        ("runtime/os/src/Os_Stack.c", "11.6", "Os_StackRegister",
         "if ((VirtualQuery((const void *)cursor, &region, sizeof(region)) != sizeof(region)) ||"),
        ("runtime/os/src/Os_Stack.c", "11.6", "Os_StackCheck",
         "invalid = VirtualQuery((const void *)sp, &region, sizeof(region)) != sizeof(region);"),
        ("runtime/os/src/Os_Stack.c", "11.6", "Os_StackObserve",
         "invalid = VirtualQuery((const void *)sp, &region, sizeof(region)) != sizeof(region);"),
    )
    instances = {}
    errors: collections.Counter[Key] = collections.Counter()
    for name, rule, owner, expression in patterns:
        path = root / name
        if not path.is_file():
            continue
        source = path.read_text(encoding="utf-8")
        tokens = [token for token in lex(source) if not is_comment(token)]
        expected = [token.text for token in lex(expression)]
        bodies = []
        for index, token in enumerate(tokens[:-1]):
            if token.text != owner or tokens[index + 1].text != "(":
                continue
            depth = 1
            cursor = index + 2
            while cursor < len(tokens) and depth:
                depth += (tokens[cursor].text == "(") - (tokens[cursor].text == ")")
                cursor += 1
            if cursor == len(tokens) or tokens[cursor].text != "{":
                continue
            start = cursor + 1
            depth = 1
            cursor += 1
            while cursor < len(tokens) and depth:
                depth += (tokens[cursor].text == "{") - (tokens[cursor].text == "}")
                cursor += 1
            if depth == 0:
                bodies.append(tokens[start:cursor - 1])
        matches = []
        if len(bodies) == 1:
            body = bodies[0]
            for index, token in enumerate(body):
                if [item.text for item in body[index:index + len(expected)]] == expected:
                    matches.append(token.start)
        if len(matches) != 1:
            errors[("deviation-category-pattern", name, owner)] += 1
        elif not (root / document).is_file() or title not in (root / document).read_text(encoding="utf-8"):
            errors[("deviation-category-record", name, owner)] += 1
        else:
            instances[(name, matches[0])] = (rule, "host-address:" + owner + ":" + " ".join(expected), document, title)
    return instances, errors


def code_words(source: str) -> list[str]:
    """Normalize comments and continued directives, retaining replacement tokens."""
    words = []
    for token in lex(re.sub(r"\\\r?\n", "", source)):
        if is_comment(token):
            continue
        if token.text.lstrip().startswith("#"):
            words.append("#")
            inner = [item for item in lex(token.text.lstrip()[1:]) if not is_comment(item)]
            if len(inner) > 1 and inner[0].text == "define":
                function_macro = len(inner) > 2 and inner[2].text == "(" and inner[2].start == inner[1].end
                words.append("<function-macro>" if function_macro else "<object-macro>")
            index = 0
            while index < len(inner):
                if (index + 1 < len(inner) and inner[index].text == inner[index + 1].text == "#"
                        and inner[index].end == inner[index + 1].start):
                    words.append("##")
                    index += 2
                else:
                    words.append(inner[index].text)
                    index += 1
        else:
            words.append(token.text)
    return words


RETAINED_APPLICATION = "core/tests/fixtures/component-pipeline/applications/Ingress.c"


def deviation_findings(root: Path = ROOT) -> collections.Counter[Key]:
    """Match approved table scopes to adjacent source locations, without approving them."""
    records: collections.Counter[tuple[str, str, str, str]] = collections.Counter()
    locations: dict[tuple[str, str, str, str], tuple[str, str]] = {}
    shapes = {}
    record_errors: collections.Counter[Key] = collections.Counter()
    for path in sorted((root / "docs/standards").rglob("*.md")):
        for line in path.read_text(encoding="utf-8").splitlines():
            cells = line.split("|")
            if len(cells) < 6:
                continue
            rule = re.search(r"MISRA C:2012 (Rule|Directive) (\d+\.\d+)", cells[1])
            if not rule or not re.match(r"(?:维护者\s+\S|Approved\s+by\s+\S)", cells[4].strip(), re.IGNORECASE):
                continue
            if "精确124键决定" in cells[4] and not re.search(r"shape=`([^`]+)`", cells[3]):
                record_errors[("deviation-shape-record", path.relative_to(root).as_posix(), line.strip())] += 1
            kind = "RULE" if rule[1] == "Rule" else "DIR"
            scope = cells[2]
            pieces = re.split(r"(`(?:runtime/[^`]+|" + re.escape(RETAINED_APPLICATION) + r")`)", scope)
            for index in range(1, len(pieces), 2):
                name = pieces[index].strip("`")
                for item in re.findall(r"`([^`]+)`", pieces[index + 1]):
                    symbol = record_scope(item)
                    if symbol is None:
                        continue
                    key = (kind, rule[2], name, symbol)
                    records[key] += 1
                    locations[key] = (path.relative_to(root).as_posix(), line.strip())
                    shape = re.search(r"shape=`([^`]+)`", cells[3])
                    if shape:
                        shapes[key] = code_words(shape[1].replace("&#124;", "|"))
    instances, result = host_address_instances(root)
    result.update(record_errors)
    for (name, _), (rule, symbol, document, title) in instances.items():
        key = ("RULE", rule, name, symbol)
        records[key] += 1
        locations[key] = (document, title)
    comments: collections.Counter[tuple[str, str, str, str]] = collections.Counter()
    source_locations = {}
    paths = set((root / "runtime").rglob("*")) | {root / RETAINED_APPLICATION}
    for path in sorted(path for path in paths if path.is_file()):
        if path.suffix not in {".c", ".h"}:
            continue
        name = path.relative_to(root).as_posix()
        source = path.read_text(encoding="utf-8")
        tokens = lex(source)
        directive_counts: collections.Counter[str] = collections.Counter()
        directive_numbers = {}
        for token in tokens:
            scope = directive_scope(token.text) if token.text.lstrip().startswith("#") else None
            if scope is not None:
                directive_counts[scope] += 1
                directive_numbers[token.start] = directive_counts[scope]
        for index, token in enumerate(tokens):
            if not is_comment(token) and not token.text.lstrip().startswith("#"):
                continue
            marker_text = token.text if is_comment(token) else " ".join(
                item.text for item in lex(token.text.lstrip()[1:]) if is_comment(item)
            )
            candidates = list(re.finditer(r"\bMR12\b", marker_text))
            matches = list(DEVIATION.finditer(marker_text))
            for candidate in candidates:
                match = next((m for m in matches if m.start() == candidate.start()), None)
                reason = comment_body(marker_text[match.end():]) if match else ""
                if not match or not reason:
                    line = source.splitlines()[source.count("\n", 0, token.start)].strip()
                    result[("deviation-format", name, line)] += 1
            for match in matches:
                if not comment_body(marker_text[match.end():]):
                    continue
                line_start = source.rfind("\n", 0, token.start) + 1
                preceding = [t for t in tokens[:index] if t.start >= line_start and not is_comment(t)]
                following = ([token] if token.text.lstrip().startswith("#") else preceding
                             or [t for t in tokens[index + 1:] if not is_comment(t)])
                declaration = []
                depth = 0
                for item in following:
                    declaration.append(item)
                    if item.text == "{":
                        if depth == 0 and function(declaration):
                            break
                        depth += 1
                    elif item.text == "}":
                        depth -= 1
                    if (item.text == ";" and depth == 0) or item.text.lstrip().startswith("#"):
                        break
                declarator = declaration
                words = [t.text for t in declarator]
                if "{" in words and not function(declarator[:words.index("{")]):
                    # Aggregate members and initializer values do not name the owning object.
                    opening = words.index("{")
                    depth = 1
                    closing = opening + 1
                    while closing < len(words) and depth:
                        depth += (words[closing] == "{") - (words[closing] == "}")
                        closing += 1
                    declarator = declarator[:opening] + declarator[closing:]
                    words = [t.text for t in declarator]
                direct = function(declarator)
                macro = re.match(r"\s*#\s*define\s+(\w+)", words[0]) if words else None
                included = directive_scope(words[0]) if words else None
                file_scoped = (match[1] == "DIR" and match[2] == "4.10" and path.suffix == ".h"
                               and index == 0 and is_comment(token)
                               and re.match(r"/\*\*\s*@file(?:\s|$)", token.text))
                symbol = "@file" if file_scoped else included or (
                    direct[0]
                    if direct
                    else macro[1]
                    if macro
                    else parameter_name([
                        word for word in (words[:words.index("=")] if "=" in words else words[:-1])
                        if not re.fullmatch(r"[A-Z][A-Z0-9_]*(?:_MEMMAP|_VAR_[A-Z0-9_]+)", word)
                    ])
                )
                instance = instances.get((name, declaration[0].start)) if declaration else None
                if instance and match[1] == "RULE" and match[2] == instance[0]:
                    symbol = instance[1]
                if included is not None and not file_scoped:
                    numbered = included + " [" + str(directive_numbers[declaration[0].start]) + "]"
                    if (match[1], match[2], name, numbered) in records:
                        symbol = numbered
                    elif directive_counts[included] > 1:
                        line = source.splitlines()[source.count("\n", 0, token.start)].strip()
                        result[("deviation-ambiguous", name, line)] += 1
                key = (match[1], match[2], name, symbol)
                if key in shapes and declaration:
                    actual = code_words(" ".join(item.text for item in declaration))
                    if actual != shapes[key]:
                        line = source.splitlines()[source.count("\n", 0, token.start)].strip()
                        result[("deviation-shape", name, line)] += 1
                comments[key] += 1
                source_locations[key] = (name, source.splitlines()[source.count("\n", 0, token.start)].strip())
    for entries, positions in ((records, locations), (comments, source_locations)):
        for key, count in entries.items():
            if count > 1:
                name, line = positions[key]
                result[("deviation-duplicate", name, line)] += count - 1
    for key, count in (comments - records).items():
        name, line = source_locations[key]
        result[("deviation-unrecorded", name, line)] += count
    for key, count in (records - comments).items():
        name, line = locations[key]
        result[("deviation-uncommented", name, line)] += count
    return result


def generated_findings(samples: Path) -> collections.Counter[Key]:
    """Use the same public-header rules for actual emitted headers, not Rust strings."""
    result: collections.Counter[Key] = collections.Counter()
    projects = sorted(p for p in samples.iterdir() if (p / "target.json").is_file())
    if not projects:
        raise RuntimeError(f"No generated samples in {samples}")
    bsw_modules = {p.stem.split("_", 1)[0] for p in (ROOT / "runtime/bsw/include").glob("*.h")}
    for project in projects:
        integration = json.loads((project / "meta/integration.json").read_text(encoding="utf-8"))
        copies = set(integration["originalBswSources"].values())
        current: collections.Counter[Key] = collections.Counter()
        headers = {path: path.read_text(encoding="utf-8") for path in sorted(project.rglob("*.h")) if not path.relative_to(project).as_posix().startswith(("kernel/", "app/"))}
        switches = switch_definitions(list(headers.values()))
        for path, source in headers.items():
            relative = path.relative_to(project).as_posix()
            if relative.startswith(("kernel/", "app/")) or relative in copies:
                continue
            if "include" in path.relative_to(project).parts:
                current.update(findings("emitted/" + relative, source, bsw=path.stem.split("_", 1)[0] in bsw_modules, known_switches=switches))
        for path in sorted(project.rglob("*")):
            if path.suffix in {".c", ".h"} and not path.relative_to(project).as_posix().startswith("kernel/"):
                relative = path.relative_to(project).as_posix()
                current.update(memory_mapping_findings("emitted/" + relative, path.read_text(encoding="utf-8")))
        result |= current
    return result


def documentation_locations(output: str, project: Path, *, foreign_roots: tuple[Path, ...] = ()) -> dict[tuple, Key]:
    integration = json.loads((project / "meta/integration.json").read_text(encoding="utf-8"))
    origins = {copy: source for source, copy in integration["originalBswSources"].items()}
    result = {}
    for match in re.finditer(r"^(.+?):(\d+):(\d+): warning: (.*?)\[(-Wdocumentation[^]]*)\]$", output, re.MULTILINE):
        path = Path(match[1]).resolve()
        if any(path.is_relative_to(root.resolve()) for root in foreign_roots):
            continue
        if not path.is_relative_to(project.resolve()):
            raise RuntimeError(f"Documentation diagnostic outside generated project: {match[0]}")
        relative = path.relative_to(project.resolve()).as_posix()
        if relative.startswith(("kernel/", "app/")):
            continue
        line = path.read_text(encoding="utf-8").splitlines()[int(match[2]) - 1].strip()
        name = origins.get(relative, "emitted/" + relative)
        position = (name, int(match[2]), int(match[3]), match[4], match[5], line)
        result[position] = ("clang-documentation" + match[5].removeprefix("-Wdocumentation"), name, line)
    return result


def documentation_findings(output: str, project: Path, *, foreign_roots: tuple[Path, ...] = ()) -> collections.Counter[Key]:
    return collections.Counter(documentation_locations(output, project, foreign_roots=foreign_roots).values())
