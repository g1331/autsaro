"""Collect conditional evaluation evidence with the target GCC preprocessor."""

from __future__ import annotations

import bisect
import gzip
import json
import re
import shutil
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from autosar_tooling import c_acceptance, c_comparison
from ecu_tools.build import digest, tool_argument

WARNINGS = (
    "-Wundef", "-Wexpansion-to-defined", "-Wno-error=undef",
    "-Wno-error=expansion-to-defined", "-fdiagnostics-format=json",
)
MARK = "AUTSARO_CONDITIONAL_TRACE_"
LITERAL = r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\''
COMMENT = re.compile(rf"({LITERAL})|/\*.*?\*/|//[^\n]*", re.DOTALL)
DEFINED = re.compile(r"\bdefined\s*(?:\(\s*([A-Za-z_]\w*)\s*\)|([A-Za-z_]\w*))")
DIRECTIVE = re.compile(r"(?m)^[ \t]*#\s*(if|elif|ifdef|ifndef|else|endif)\b([^\n]*)")
LINE = re.compile(r'# (\d+) "((?:[^"\\]|\\.)*)"((?: \d+)*)')
MACRO = re.compile(r"#(define|undef) ([A-Za-z_]\w*)(.*)")
WORD = re.compile(rf"{LITERAL}|(?:\d|\.\d)(?:[\w.]|(?<=[eEpP])[+-])*|([A-Za-z_]\w*)")


def directives(source: str):
    """Apply line splicing and comment removal while retaining physical positions."""
    splices = list(re.finditer(r"\\\n", source))
    removed = [match.start() - index * 2 for index, match in enumerate(splices)]

    def physical(position):
        return position + 2 * bisect.bisect_right(removed, position)

    logical = re.sub(r"\\\n", "", source)
    masked = COMMENT.sub(lambda match: match[1] or re.sub(r"[^\n]", " ", match[0]), logical)
    for match in DIRECTIVE.finditer(masked):
        start = physical(match.start())
        end = physical(match.end())
        yield match[1], match[2].strip(), start, end, source.count("\n", 0, start) + 1


def instrument(source: str, name: str, scope: dict) -> tuple[str, dict]:
    """Probe in the same branch and macro state as the original expression."""
    if MARK in source:
        raise ValueError(f"Reserved preprocessing evidence marker in {name}")
    stack = []
    points = {}
    chunks = []
    last = 0
    for kind, expression, start, end, line in directives(source):
        if kind in {"if", "ifdef", "ifndef"}:
            stack.append(0)
        category = c_comparison.location_scope(scope, {"file": name, "line": str(line)})
        successor = source.count("\n", 0, end) + 2
        if kind in {"if", "elif"} and category == "in-scope":
            if kind == "elif":
                stack[-1] += 1
            operands = tuple(dict.fromkeys(match[1] or match[2] for match in DEFINED.finditer(expression)))
            expanded = DEFINED.sub(lambda match: MARK + "DEFINED_" + (match[1] or match[2]), expression)
            rhs = f"{MARK}EXPRESSION {expanded}"
            if re.search(r"\b(?:__has_include(?:_next)?|__has_embed)\b", expression):
                raise ValueError(f"Context-specific preprocessor operator at {name}:{line}")
            probes = (
                f"#line {line}\n{rhs}\n"
                f"#line {line}\n#if (({expression}) == 0)\n{MARK}VALUE 0\n"
                f"#else\n#line {line}\n#if (({expression}) == 1)\n{MARK}VALUE 1\n"
                f"#else\n{MARK}VALUE other\n#endif\n#endif\n"
                f"#line {line}\n#if {expression}"
            )
            chunks.append(source[last:start])
            if kind == "elif":
                chunks.append("#else\n")
            chunks.append(probes)
            # Restore the physical line of the original directive's successor.
            chunks.append(f"\n#line {successor}")
            last = end
            points[line] = (kind, expression, operands)
        elif points and kind in {"elif", "else", "endif"}:
            nested = stack.pop() if kind == "endif" else 0
            chunks.append(source[last:start])
            chunks.append("#endif\n" * nested)
            chunks.append(source[start:end])
            if source[end:].strip():
                chunks.append(f"\n#line {successor}")
            last = end
        elif kind == "endif":
            stack.pop()
    chunks.append(source[last:])
    return "".join(chunks), points


def identifiers(text: str) -> set[str]:
    return {match[1] for match in WORD.finditer(text) if match[1]}


def trace_output(text: str, root: Path, points: dict) -> tuple[list[dict], list[str]]:
    """Decode ordered GCC line/definition evidence and recover uninstrumented output."""
    paths = {}
    events = []
    plain = []
    definitions = {}
    current = None
    filename = ""
    line = 0
    for text_line in text.splitlines():
        marker = LINE.fullmatch(text_line)
        if marker:
            line = int(marker[1])
            path = marker[2].replace("\\\\", "\\").replace('\\"', '"')
            if path not in paths:
                absolute = Path(path)
                paths[path] = (absolute.relative_to(root).as_posix()
                               if absolute.is_relative_to(root) else path)
            filename = paths[path]
            flags = marker[3].split()
            if "1" in flags or "2" in flags:
                events.append({"kind": "enter" if "1" in flags else "return",
                               "file": filename, "line": line})
                plain.append(f"#{line} {filename} {marker[3]}")
            continue
        if text_line.startswith(MARK + "EXPRESSION "):
            point = points.get(filename, {}).get(line)
            if point is None or current is not None:
                raise ValueError(f"Unexpected conditional marker at {filename}:{line}")
            kind, source, operands = point
            expanded = text_line.removeprefix(MARK + "EXPRESSION ")
            tested = {name: definitions.get(name) for name in operands}
            expanded = re.sub(rf"\b{MARK}DEFINED_(\w+)", lambda m: str(int(m[1] in definitions)), expanded)
            names = identifiers(source) | identifiers(expanded)
            active = {name: definitions[name] for name in sorted(names & definitions.keys())}
            pending = list(active)
            while pending:
                definition = active[pending.pop()]
                for name in sorted(identifiers(definition["body"]) & definitions.keys() - active.keys()):
                    active[name] = definitions[name]
                    pending.append(name)
            if "__COUNTER__" in names or any(
                "__COUNTER__" in identifiers(definition["body"]) for definition in active.values()
            ):
                raise ValueError(f"Stateful conditional at {filename}:{line}")
            current = {"kind": kind, "file": filename, "line": line, "source": source,
                       "expanded": expanded, "definedTests": tested, "definitions": active,
                       "unexpandedIdentifiers": sorted(identifiers(expanded))}
        elif text_line.startswith(MARK + "VALUE "):
            value = text_line.removeprefix(MARK + "VALUE ")
            if current is None or value not in {"0", "1", "other"}:
                raise ValueError(f"Unexpected conditional value at {filename}:{line}")
            events.append({**current, "value": int(value) if value in {"0", "1"} else value})
            current = None
        elif text_line.startswith(MARK):
            raise ValueError(f"Malformed conditional marker at {filename}:{line}")
        elif text_line.strip():
            plain.append(text_line.rstrip())
            macro = MACRO.fullmatch(text_line)
            if macro:
                definition = {"file": filename, "line": line, "body": macro[3].strip(),
                              "function": macro[3].startswith("(")}
                previous = definitions.get(macro[2])
                if macro[1] == "define":
                    definitions[macro[2]] = definition
                else:
                    definitions.pop(macro[2], None)
                if filename in points:
                    events.append({"kind": macro[1], "name": macro[2], **definition,
                                   "previousDefinition": previous})
        line += 1
    if current is not None:
        raise ValueError("Missing conditional value")
    return events, plain


def collect(scope: dict, root: Path, output: Path, run) -> dict:
    """Reuse the actual c-check preprocessing outputs as the reference."""
    started = time.monotonic()
    observed = scope["projectTranslationUnits"]
    compiler = scope["diagnosticCompiler"]["path"]
    with tempfile.TemporaryDirectory(prefix="preprocess-", dir=output) as directory:
        scratch = Path(directory).resolve()
        shadow = scratch / "project"
        shutil.copytree(root, shadow)
        points = {}
        for name in sorted({name for paths in observed.values() for name in paths}):
            path = shadow / name
            rewritten, points[name] = instrument(path.read_text(encoding="utf-8"), name, scope)
            if points[name]:
                path.write_text(rewritten, encoding="utf-8")
        args = [compiler, *scope["compilerFlags"]]
        args.extend("-I" + tool_argument(shadow / name) for name in scope["includePaths"])
        args.extend(("-E", "-dD", f"-fmacro-prefix-map={tool_argument(shadow)}={tool_argument(root)}"))

        def replay(item):
            index, unit = item
            stage = f"compiler-origins-{index}"
            reference = (output / f"{stage}.stdout.log").read_text(encoding="utf-8")
            diagnostics = (output / f"{stage}.stderr.log").read_text(encoding="utf-8")
            _, baseline = trace_output(reference, root, {})
            _, text, _ = run([*args, tool_argument(shadow / unit)], scratch, f"trace-{index}", 60,
                             context=f"Preprocessing evidence failed for {unit}", cwd=output)
            events, plain = trace_output(text, shadow, points)
            if plain != baseline:
                raise ValueError(f"Preprocessing probes changed emitted tokens or include order: {unit}")
            return unit, {"expandedSha256": c_acceptance._hash(baseline), "events": events,
                          "compilerDiagnostics": json.loads(diagnostics or "[]")}

        with ThreadPoolExecutor(max_workers=scope["compilerWorkers"]) as executor:
            report = output / "preprocessing.json.gz"
            # Stream one TU at a time into the existing full analysis artifact.
            with gzip.open(report, "wt", compresslevel=3, encoding="utf-8", newline="\n") as stream:
                stream.write('{"format":"conditional-evaluations-v1","translationUnits":{')
                separator = ""
                for name, unit in executor.map(replay, enumerate(observed)):
                    stream.write(separator + json.dumps(name) + ":")
                    json.dump(unit, stream, ensure_ascii=False, separators=(",", ":"))
                    separator = ","
                stream.write("}}\n")
    elapsed = round(time.monotonic() - started, 2)
    print(f"Conditional evaluation evidence: {len(observed)} units in {elapsed:.2f}s")
    return {"file": report.name, "sha256": digest(report), "elapsedSeconds": elapsed}
