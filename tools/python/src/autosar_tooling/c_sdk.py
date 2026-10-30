"""SDK source evidence from actual preprocessing and installed package manifests."""

from __future__ import annotations

import gzip
import hashlib
import json
import re
import shlex
from pathlib import Path

from ecu_tools.build import digest


def provider_supported(name: str, windows: bool) -> bool:
    if windows:
        return name in {
            "mingw-w64-x86_64-headers",
            "mingw-w64-x86_64-crt",
            "mingw-w64-x86_64-gcc",
            "mingw-w64-x86_64-gcc-libs",
        }
    return name in {"libc6-dev", "linux-libc-dev"} or bool(
        re.fullmatch(r"(?:libgcc-\d+-dev|gcc-\d+(?:-x86-64-linux-gnu)?)", name)
    )


def debian_headers(
    status: Path = Path("/var/lib/dpkg/status"), info: Path = Path("/var/lib/dpkg/info")
) -> dict:
    """Installed-package evidence, not a supplier conformance or archive signature claim."""
    records = {}
    if not status.is_file():
        return records
    for paragraph in status.read_text(encoding="utf-8").split("\n\n"):
        fields = dict(
            line.split(": ", 1)
            for line in paragraph.splitlines()
            if ": " in line and not line.startswith(" ")
        )
        name = fields.get("Package", "")
        if not provider_supported(name, False):
            continue
        if (
            fields.get("Status") != "install ok installed"
            or fields.get("Architecture") != "amd64"
        ):
            continue
        manifests = [info / (name + ":amd64.md5sums"), info / (name + ".md5sums")]
        for manifest in manifests:
            if not manifest.is_file():
                continue
            for line in manifest.read_text(encoding="utf-8").splitlines():
                checksum, _, filename = line.partition("  ")
                if re.fullmatch("[0-9a-f]{32}", checksum):
                    path = Path("/") / filename
                    records[str(path)] = {
                        "provider": name,
                        "version": fields["Version"],
                        "header": filename,
                        "algorithm": "md5",
                        "expected": checksum,
                    }
    return records


def msys_headers(compiler: Path) -> dict:
    """Bind to the selected compiler's installation, never another pacman on PATH."""
    records = {}
    prefix = compiler.parent.parent
    if prefix.name.lower() != "mingw64":
        return records
    database = prefix.parent / "var/lib/pacman/local"
    if not database.is_dir():
        return records
    for package in sorted(database.iterdir()):
        desc, mtree = package / "desc", package / "mtree"
        if not desc.is_file() or not mtree.is_file():
            continue
        fields = {}
        for part in desc.read_text(encoding="utf-8").split("\n\n"):
            lines = part.splitlines()
            if len(lines) >= 2:
                fields[lines[0]] = lines[1]
        name = fields.get("%NAME%", "")
        if not provider_supported(name, True):
            continue
        with gzip.open(mtree, "rt", encoding="utf-8") as stream:
            for line in stream:
                values = shlex.split(line)
                if not values or not values[0].startswith("./"):
                    continue
                attributes = dict(
                    value.split("=", 1) for value in values[1:] if "=" in value
                )
                checksum = attributes.get("sha256digest", "")
                filename = values[0][2:]
                if re.fullmatch("[0-9a-f]{64}", checksum):
                    path = (prefix.parent / filename).resolve()
                    records[str(path)] = {
                        "provider": name,
                        "version": fields["%VERSION%"],
                        "header": filename,
                        "algorithm": "sha256",
                        "expected": checksum,
                    }
    return records


def header_identity(
    path: Path, root: Path, target: str, machine: str, packages: dict
) -> dict:
    """A same-named project file or a locally patched SDK file is never adopted SDK."""
    path = path.resolve()
    record = {
        "path": path.as_posix(),
        "header": path.as_posix(),
        "target": target,
        "sha256": digest(path),
        "category": "unclassified",
        "reason": "unknown provider or missing package manifest",
    }
    package = packages.get(str(path))
    if path.is_relative_to(root):
        record["reason"] = "project source"
    elif package:
        record.update(package)
        wanted = (
            "x86_64-w64-mingw32" if target.startswith("windows") else "x86_64-linux-gnu"
        )
        if machine != wanted or not provider_supported(
            package["provider"], target.startswith("windows")
        ):
            record["reason"] = "target/provider mismatch"
        elif (
            hashlib.new(package["algorithm"], path.read_bytes()).hexdigest()
            != package["expected"]
        ):
            record["reason"] = "installed header differs from provider original"
        else:
            record.update(
                category="sdk",
                reason="installed provider original matches package manifest",
            )
    record["key"] = hashlib.sha256(
        json.dumps(
            {k: v for k, v in record.items() if k not in {"path", "reason"}},
            sort_keys=True,
        ).encode()
    ).hexdigest()
    return record


def source_lines(path: Path) -> list[str]:
    # Keep physical line numbers while excluding comments and string/character data.
    text = re.sub(
        r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|/\*.*?\*/|//[^\n]*',
        lambda match: re.sub(r"[^\n]", " ", match[0]),
        path.read_text(encoding="utf-8"),
        flags=re.DOTALL,
    )
    return text.splitlines()


def dependency_paths(text: str) -> list[str]:
    """Decode Make escapes, retaining Windows separators that are not escapes."""
    target, colon, names = text.replace("\\\n", " ").partition(":")
    if target != "autsaro-tu" or not colon:
        raise ValueError("Compiler header dependency inventory missing")
    return [
        re.sub(r"\\([ \t#:\\])", r"\1", name).replace("$$", "$")
        for name in re.findall(r"(?:\\[ \t#:\\]|[^\s])+", names)
    ]


def dependency_headers(text: str, root: Path) -> set[Path]:
    """The compiler's dependency output proves which files were actually opened."""
    paths = {Path(name).resolve() for name in dependency_paths(text)}
    if not paths:
        raise ValueError("Compiler header dependency inventory empty")
    return {path for path in paths if not path.is_relative_to(root)}


def macro_closure(names: set[str], definitions: dict, original: str) -> dict:
    """Snapshot transitive active replacements; cycles and unresolved aliases are unsafe."""
    selected = {}
    visiting = set()

    def visit(name):
        if name in visiting:
            return True
        if name in selected:
            return selected[name]["unresolved"]
        definition = definitions[name]
        body = definition["value"]
        parameters = set()
        if definition["function"]:
            signature = re.match(r"\(([^)]*)\)(.*)", body)
            if signature is None:
                selected[name] = {**definition, "use": original, "unresolved": True}
                return True
            parameters = set(re.findall(r"\b[A-Za-z_]\w*\b", signature[1]))
            body = signature[2]
        identifiers = set(re.findall(r"\b[A-Za-z_]\w*\b", body)) - parameters
        alias = body.strip()
        while alias.startswith("(") and alias.endswith(")"):
            alias = alias[1:-1].strip()
        unresolved = "#" in body or (
            not definition["function"]
            and bool(re.fullmatch(r"[A-Za-z_]\w*", alias))
            and alias not in definitions
        )
        selected[name] = {**definition, "use": original, "unresolved": unresolved}
        visiting.add(name)
        for dependency in sorted(identifiers & definitions.keys()):
            unresolved = visit(dependency) or unresolved
        visiting.remove(name)
        selected[name]["unresolved"] = unresolved
        return unresolved

    for name in sorted(names):
        visit(name)
    return selected


def preprocessing(text: str, root: Path, candidates: set[str]) -> dict:
    """Track active definitions at emitted project lines; unknown expansions stay unknown."""
    definitions = {}
    inputs = {}
    contents = {}
    filename, number = "", 0
    owned = None
    for line in text.splitlines():
        marker = re.fullmatch(r'# (\d+) "((?:[^"\\]|\\.)*)"(?: .*)?', line)
        if marker:
            number = int(marker[1])
            filename = marker[2].replace("\\\\", "\\").replace('\\"', '"')
            owned = None
            if not filename.startswith("<"):
                path = Path(filename).resolve()
                if path.is_relative_to(root) and path.is_file():
                    owned = path
                    if path not in contents:
                        contents[path] = source_lines(path)
            continue
        definition = re.match(r"#define (\w+)(.*)", line)
        if definition:
            source = (
                Path(filename).resolve()
                if filename and not filename.startswith("<")
                else None
            )
            origin = (
                source.relative_to(root).as_posix()
                if source and source.is_relative_to(root)
                else filename
            )
            definitions[definition[1]] = {
                "file": origin,
                "line": number,
                "value": definition[2].strip(),
                "function": definition[2].startswith("("),
            }
        elif line.startswith("#undef "):
            definitions.pop(line[7:].strip(), None)
        elif owned and 1 <= number <= len(contents[owned]):
            original = contents[owned][number - 1]
            names = (
                set(re.findall(r"\b[A-Za-z_]\w*\b", original))
                & candidates
                & definitions.keys()
            )
            if names:
                key = owned.relative_to(root).as_posix() + ":" + str(number)
                inputs[key] = macro_closure(names, definitions, original)
        number += 1
    return inputs


def integer_value(literal: str) -> int:
    value = literal.rstrip("uUlL")
    return int(
        value,
        16
        if value.lower().startswith("0x")
        else 8
        if value.startswith("0") and not re.search("[89]", value)
        else 10,
    )


def classify(
    diagnostics: list[dict], scope: dict, unsigned: list[dict], root: Path | None = None
) -> dict:
    """Attach evidence without altering raw diagnostics or project/patch responsibility."""
    headers = {row["path"]: row for row in scope.get("sdkHeaders", [])}
    per_unit = scope.get("sdkMacroInputs", {})
    discovered = {
        value[2:].split("=", 1)[0].split("(", 1)[0]
        for values in scope.get("compilerDiscoveredDefines", {}).values()
        for value in values
    }
    contents = {}
    for diagnostic in diagnostics:
        origins = []
        responsibility = (
            "project-patch"
            if diagnostic["patched"]
            else "adopted-freertos"
            if diagnostic["adopted"]
            else "project"
        )
        category = responsibility
        for location in diagnostic["locations"]:
            key = location["file"] + ":" + str(location["line"])
            header = headers.get(location["file"])
            if header:
                origins.append(
                    {
                        "kind": "header",
                        "category": header["category"],
                        "headerKey": header["key"],
                    }
                )
                category = header["category"]
            elif Path(location["file"]).is_absolute():
                origins.append(
                    {
                        "kind": "header",
                        "category": "unclassified",
                        "reason": "diagnostic file is outside actual compiler header closure",
                        "headerKey": hashlib.sha256(
                            location["file"].encode()
                        ).hexdigest(),
                    }
                )
                category = "unclassified"
            inputs = [values[key] for values in per_unit.values() if key in values]
            # A shared header may have different active definitions in different TUs.
            for values in inputs:
                for name, definition in values.items():
                    source = headers.get(Path(definition["file"]).as_posix())
                    if source:
                        origin = {
                            "kind": "macro-input",
                            "macro": name,
                            "definitionLine": definition["line"],
                            "value": definition["value"],
                            "category": source["category"],
                            "headerKey": source["key"],
                        }
                        if origin not in origins:
                            origins.append(origin)
            if root is not None:
                path = (root / location["file"]).resolve()
                if path.is_relative_to(root) and path.is_file() and location["line"]:
                    if path not in contents:
                        contents[path] = source_lines(path)
                    number = int(location["line"])
                    if 1 <= number <= len(contents[path]):
                        used = (
                            set(
                                re.findall(
                                    r"\b[A-Za-z_]\w*\b", contents[path][number - 1]
                                )
                            )
                            & discovered
                        )
                        known = {origin.get("macro") for origin in origins}
                        for name in sorted(used - known):
                            origins.append(
                                {
                                    "kind": "unresolved-macro",
                                    "macro": name,
                                    "category": "unclassified",
                                    "reason": "compiler macro input has no active provider-header evidence",
                                    "headerKey": hashlib.sha256(
                                        (scope["target"] + ":" + name).encode()
                                    ).hexdigest(),
                                }
                            )
                            category = "unclassified"
            if diagnostic["id"] == "misra-c2012-7.2":
                tokens = [
                    dict(values)
                    for values in {
                        tuple(sorted(row.items()))
                        for row in unsigned
                        if all(
                            str(row[k]) == str(location[k])
                            for k in ("file", "line", "column")
                        )
                    }
                ]
                # Attribute only actual checker operands whose literal value is an
                # unambiguous active object macro. Other rules retain input evidence.
                for token in tokens:
                    matches = [
                        o
                        for o in origins
                        if o["kind"] == "macro-input"
                        and re.fullmatch(r"0[xX][0-9a-fA-F]+|[0-9]+", o["value"])
                        and integer_value(o["value"]) == integer_value(token["literal"])
                    ]
                    if (
                        len(tokens) == 1
                        and len(matches) == 1
                        and all(
                            not any(
                                definition.get("unresolved", False)
                                for definition in values.values()
                            )
                            and any(
                                name == matches[0]["macro"]
                                and "use" in definition
                                and definition["value"] == matches[0]["value"]
                                and headers.get(
                                    Path(definition["file"]).as_posix(), {}
                                ).get("key")
                                == matches[0]["headerKey"]
                                for name, definition in values.items()
                            )
                            and not any(
                                integer_value(literal)
                                == integer_value(token["literal"])
                                for definition in values.values()
                                for literal in re.findall(
                                    r"\b(?:0[xX][0-9a-fA-F]+|[0-9]+)[uUlL]*\b",
                                    definition.get("use", ""),
                                )
                            )
                            and not any(
                                name != matches[0]["macro"]
                                and any(
                                    integer_value(literal)
                                    == integer_value(token["literal"])
                                    for literal in re.findall(
                                        r"\b(?:0[xX][0-9a-fA-F]+|[0-9]+)[uUlL]*\b",
                                        definition["value"],
                                    )
                                )
                                for name, definition in values.items()
                            )
                            for values in inputs
                        )
                    ):
                        matches[0]["kind"] = "unsigned-literal"
                if origins:
                    category = (
                        "sdk"
                        if len(tokens) == 1
                        and not any(o["category"] == "unclassified" for o in origins)
                        and any(
                            o["kind"] == "unsigned-literal" and o["category"] == "sdk"
                            for o in origins
                        )
                        else "unclassified"
                    )
        diagnostic["sourceOrigins"] = origins
        if scope.get("sdkLineDirectives") and origins:
            for origin in origins:
                origin.update(
                    category="unclassified",
                    reason="project line directives prevent physical source attribution",
                )
            category = "unclassified"
        if len(diagnostic["locations"]) > 1 and category == "sdk":
            category = "mixed"
        diagnostic["sourceCategory"] = category
    return summary(
        diagnostics, list(headers.values()), not scope.get("sdkLineDirectives", False)
    )


def summary(
    diagnostics: list[dict], headers: list[dict], macro_trace_complete: bool = True
) -> dict:
    sdk_keys, unclassified = set(), set()
    for row in diagnostics:
        for origin in row.get("sourceOrigins", []):
            key = (
                row["id"],
                tuple((p["file"], p["line"], p["column"]) for p in row["locations"]),
                origin["headerKey"],
            )
            (sdk_keys if origin["category"] == "sdk" else unclassified).add(key)
    return {
        "sdkDiagnosticKeys": len(sdk_keys),
        "unclassifiedDiagnosticKeys": len(unclassified),
        "unresolvedMacroDiagnostics": sum(
            row.get("sourceCategory") == "unclassified" for row in diagnostics
        ),
        "unclassifiedHeaders": sum(row["category"] != "sdk" for row in headers),
        "macroTraceComplete": macro_trace_complete,
        "complete": bool(headers)
        and macro_trace_complete
        and not unclassified
        and not any(row.get("sourceCategory") == "unclassified" for row in diagnostics)
        and all(row["category"] == "sdk" for row in headers),
    }


def semantic(scope: dict) -> str:
    """Paths are provenance; provider/header bytes and active definitions are semantic."""
    headers = scope.get("sdkHeaders", [])
    packages = (
        msys_headers(Path(scope["diagnosticCompiler"]["path"]))
        if scope["target"].startswith("windows")
        else debian_headers()
    )
    for header in headers:
        path = Path(header["path"])
        expected = {
            key: header[key]
            for key in ("provider", "version", "header", "algorithm", "expected")
            if key in header
        }
        if digest(path) != header["sha256"] or packages.get(str(path)) != (
            expected if "provider" in header else None
        ):
            raise ValueError("SDK source/provider identity changed during analysis")
    keys = {row["path"]: row["key"] for row in headers}
    identity = {
        "lineDirectives": scope.get("sdkLineDirectives", False),
        "headers": sorted(
            (
                {k: v for k, v in row.items() if k not in {"path", "reason"}}
                for row in headers
            ),
            key=lambda row: row["key"],
        ),
        "translationUnits": {
            unit: sorted(keys[path] for path in paths)
            for unit, paths in scope.get("sdkTranslationUnits", {}).items()
        },
        "macroInputs": {
            unit: sorted(
                {
                    json.dumps(
                        {
                            "macro": name,
                            "headerKey": keys[Path(definition["file"]).as_posix()],
                            **{
                                key: definition[key]
                                for key in ("line", "value", "function")
                            },
                        },
                        sort_keys=True,
                    )
                    for values in locations.values()
                    for name, definition in values.items()
                    if Path(definition["file"]).as_posix() in keys
                }
            )
            for unit, locations in scope.get("sdkMacroInputs", {}).items()
        },
    }

    return hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()
