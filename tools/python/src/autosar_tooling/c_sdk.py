"""SDK source evidence from actual preprocessing and installed package manifests."""

from __future__ import annotations

import gzip
import hashlib
import json
import os
import re
import shlex
from pathlib import Path, PurePath, PurePosixPath, PureWindowsPath

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
        for listing in (info / (name + ":amd64.list"), info / (name + ".list")):
            if listing.is_file():
                for filename in listing.read_text(encoding="utf-8").splitlines():
                    path = Path(filename)
                    if path.is_symlink():
                        records[str(path)] = {
                            "provider": name, "version": fields["Version"],
                            "header": path.as_posix().lstrip("/"), "algorithm": "symlink",
                            "expected": path.readlink().as_posix(),
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
                # ALPM archive metadata is stored in pacman's database, not at /.
                if filename in {".BUILDINFO", ".PKGINFO", ".MTREE", ".INSTALL", ".CHANGELOG"}:
                    continue
                if attributes.get("type") == "link":
                    path = prefix.parent / filename
                    records[str(path)] = {
                        "provider": name, "version": fields["%VERSION%"],
                        "header": filename, "algorithm": "symlink",
                        "expected": attributes["link"],
                    }
                    continue
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


def header_key(record: dict) -> str:
    """Identify a consumed header by relative path, provider and bytes, not package revision."""
    return hashlib.sha256(
        json.dumps(
            {k: v for k, v in record.items() if k not in {"path", "reason", "key", "version"}},
            sort_keys=True,
        ).encode()
    ).hexdigest()


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
    if package and package["algorithm"] == "symlink":
        package = None  # A materialized alias has no independent original-file digest.
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
    record["key"] = header_key(record)
    return record


def source_code(text: str) -> str:
    # Keep physical line numbers while excluding comments and string/character data.
    return re.sub(
        r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|/\*.*?\*/|//[^\n]*',
        lambda match: re.sub(r"[^\n]", " ", match[0]),
        text,
        flags=re.DOTALL,
    )


def source_lines(path: Path) -> list[str]:
    return source_code(path.read_text(encoding="utf-8")).splitlines()


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
    paths = {}
    filename, number = "", 0
    owned = None
    origin = ""
    for line in text.splitlines():
        marker = re.fullmatch(r'# (\d+) "((?:[^"\\]|\\.)*)"(?: .*)?', line)
        if marker:
            number = int(marker[1])
            filename = marker[2].replace("\\\\", "\\").replace('\\"', '"')
            owned = None
            origin = filename
            if filename and not filename.startswith("<"):
                # GCC changes provenance at line markers, not at each macro definition.
                # This snapshot belongs only to this preprocessor output and source tree.
                if filename not in paths:
                    paths[filename] = Path(filename).resolve()
                path = paths[filename]
                if path.is_relative_to(root):
                    origin = path.relative_to(root).as_posix()
                if path.is_relative_to(root) and path.is_file():
                    owned = path
                    if path not in contents:
                        contents[path] = source_lines(path)
            continue
        definition = re.match(r"#define (\w+)(.*)", line)
        if definition:
            definitions[definition[1]] = {
                "file": origin,
                "line": number,
                "value": definition[2].strip(),
                "function": definition[2].startswith("("),
            }
        elif line.startswith("#undef "):
            definitions.pop(line[7:].strip(), None)
        if (
            owned
            and not line.startswith("#undef ")
            and 1 <= number <= len(contents[owned])
        ):
            original = contents[owned][number - 1]
            names = (
                set(
                    re.findall(
                        r"\b[A-Za-z_]\w*\b", definition[2] if definition else original
                    )
                )
                & candidates
                & definitions.keys()
            )
            if definition:
                names.discard(definition[1])
                if definitions[definition[1]]["function"]:
                    signature = re.match(r"\(([^)]*)\)", definition[2])
                    if signature:
                        names -= set(re.findall(r"\b[A-Za-z_]\w*\b", signature[1]))
            if names:
                key = owned.relative_to(root).as_posix() + ":" + str(number)
                inputs[key] = macro_closure(names, definitions, original)
                if definition:
                    for value in inputs[key].values():
                        value["reference"] = True
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


def integer_literals(text: str) -> set[int]:
    numbers = re.findall(
        r"(?<![\w.])(?:\.[0-9]|[0-9])(?:[0-9A-Za-z_.]|(?<=[eEpP])[+-])*",
        source_code(text),
    )
    return {
        integer_value(literal)
        for literal in numbers
        if re.fullmatch(r"(?:0[xX][0-9a-fA-F]+|[0-9]+)[uUlL]*", literal)
    }


def classify(
    diagnostics: list[dict], scope: dict, unsigned: list[dict], root: Path | None = None
) -> dict:
    """Attach evidence without altering raw diagnostics or project/patch responsibility."""
    headers = {row["path"]: row for row in scope.get("sdkHeaders", [])}
    per_unit = scope.get("sdkMacroInputs", {})
    compiler = scope.get("compilerSemanticInputs", {})
    predefines = {}
    for line in compiler.get("predefines", []):
        if match := re.fullmatch(r"#define (\w+)(.*)", line):
            predefines[match[1]] = (match[2].strip(), match[2].startswith("("))
    compiler_key = hashlib.sha256(
        json.dumps(compiler, sort_keys=True).encode()
    ).hexdigest()
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
                            "kind": "macro-reference"
                            if definition.get("reference")
                            else "macro-input",
                            "macro": name,
                            "definitionLine": definition["line"],
                            "value": definition["value"],
                            "category": source["category"],
                            "headerKey": source["key"],
                        }
                        if origin not in origins:
                            origins.append(origin)
                    elif definition["file"] == "<built-in>" and predefines.get(
                        name
                    ) == (definition["value"], definition["function"]):
                        origin = {
                            "kind": "compiler-predefine",
                            "macro": name,
                            "value": definition["value"],
                            "category": "compiler-predefine",
                            "compilerKey": compiler_key,
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
                        and integer_value(token["literal"])
                        in integer_literals(o["value"])
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
                                integer_value(token["literal"])
                                in integer_literals(definition.get("use", ""))
                                for definition in values.values()
                            )
                            and not any(
                                name != matches[0]["macro"]
                                and integer_value(token["literal"])
                                in integer_literals(definition["value"])
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
                elif (
                    tokens
                    and inputs
                    and not all(
                        not any(
                            definition.get("unresolved", False)
                            for definition in values.values()
                        )
                        and all(
                            any(
                                integer_value(token["literal"])
                                in integer_literals(
                                    definition.get("use", "")
                                    + " "
                                    + definition["value"]
                                )
                                for definition in values.values()
                            )
                            for token in tokens
                        )
                        for values in inputs
                    )
                ):
                    # A wrapper's later-line arguments are not proved by its first line.
                    category = "unclassified"
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
    sdk_keys, compiler_keys, unclassified = set(), set(), set()
    for row in diagnostics:
        for origin in row.get("sourceOrigins", []):
            key = (
                row["id"],
                tuple((p["file"], p["line"], p["column"]) for p in row["locations"]),
                origin.get("headerKey", origin.get("compilerKey")),
            )
            if origin["category"] == "sdk":
                sdk_keys.add(key)
            elif origin["category"] == "compiler-predefine":
                compiler_keys.add(key)
            else:
                unclassified.add(key)
    return {
        "sdkDiagnosticKeys": len(sdk_keys),
        "compilerPredefineDiagnosticKeys": len(compiler_keys),
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


def bind_wait_definitions(scope: dict) -> None:
    """Bind captured wait definitions to actual provider bytes before publishing receipts."""
    headers = {row["path"]: row for row in scope["sdkHeaders"]}
    for locations in scope["sdkMacroInputs"].values():
        for values in locations.values():
            if "INFINITE" not in values:
                continue
            for definition in values.values():
                if definition["file"] not in headers:
                    continue
                path = Path(definition["file"])
                lines = path.read_text(encoding="utf-8").splitlines()
                index = definition["line"] - 1
                original = lines[index] if 0 <= index < len(lines) else ""
                while original.endswith("\\") and index + 1 < len(lines):
                    index += 1
                    original = original[:-1] + lines[index]
                definition["original"] = source_code(original).strip()
                definition["headerKey"] = headers[definition["file"]]["key"]


def validate_sources(data: dict) -> None:
    """Validate normal SDK receipts, independently of diagnostic presence or spelling."""
    scope = data["scope"]
    classification = data["sourceClassification"]
    if not classification["complete"] or classification["unclassifiedDiagnosticKeys"]:
        raise ValueError("Incomplete SDK source provenance")
    target = scope["target"]
    windows = target.startswith("windows")
    headers = {}
    for row in scope["sdkHeaders"]:
        label = "SDK header " + row["path"]
        if row["path"] in headers:
            raise ValueError(label + ": duplicate identity")
        if row["target"] != target or row["category"] != "sdk":
            raise ValueError(label + ": target/category mismatch")
        if (
            not provider_supported(row.get("provider", ""), windows)
            or not row.get("version")
            or not row.get("header")
        ):
            raise ValueError(label + ": missing or unsupported package identity")
        if not re.fullmatch(r"[0-9a-f]{64}", row.get("sha256", "")):
            raise ValueError(label + ": missing original digest")
        algorithm = "sha256" if windows else "md5"
        if (
            row.get("algorithm") != algorithm
            or not re.fullmatch(
                r"[0-9a-f]{64}" if windows else r"[0-9a-f]{32}", row.get("expected", "")
            )
            or windows
            and row["sha256"] != row["expected"]
        ):
            raise ValueError(label + ": original/package digest mismatch")
        if row.get("key") != header_key(row):
            raise ValueError(label + ": identity key mismatch")
        headers[row["path"]] = row
    units = scope["translationUnits"]
    dependencies = scope["sdkTranslationUnits"]
    inputs = scope["sdkMacroInputs"]
    if not headers or set(dependencies) != set(units) or set(inputs) != set(units):
        raise ValueError("SDK translation units: incomplete dependency/macro inventory")
    if set(headers) != {path for paths in dependencies.values() for path in paths}:
        raise ValueError("SDK headers: dependency inventory mismatch")
    if not windows:
        return
    if scope["compilerSemanticInputs"]["machine"] != "x86_64-w64-mingw32" or scope.get(
        "sdkLineDirectives", False
    ):
        raise ValueError("SDK wait: target or source tracing mismatch")
    port = "kernel/portable/MSVC-MingW/port.c"
    port_wait = False
    for unit, locations in inputs.items():
        for location, values in locations.items():
            if "INFINITE" not in values:
                continue
            label = "SDK wait " + unit + ":" + location
            definition = values["INFINITE"]
            use = definition["use"]
            filename, _, line = location.rpartition(":")
            if (
                not line.isdecimal()
                or int(line) < 1
                or not re.search(r"\bINFINITE\b", use)
            ):
                raise ValueError(label + ": missing call source")
            for name, item in values.items():
                if item["unresolved"] or item["use"] != use:
                    raise ValueError(
                        label + ": unresolved or inconsistent active closure"
                    )
                header = headers.get(item["file"])
                if header is None:
                    continue  # Project wrappers remain project-owned.
                if (
                    item["file"] not in dependencies[unit]
                    or item.get("headerKey") != header["key"]
                ):
                    raise ValueError(label + ": definition/header dependency mismatch")
                match = re.fullmatch(
                    r"#\s*define\s+(\w+)(.*)", item.get("original", "")
                )
                if (
                    not match
                    or match[1] != name
                    or item["line"] < 1
                    or match[2].startswith("(") != item["function"]
                    or re.sub(r"\s+", "", match[2]) != re.sub(r"\s+", "", item["value"])
                ):
                    raise ValueError(
                        label + ": active definition differs from original"
                    )
            if (
                definition["file"] not in headers
                or definition["function"]
                or definition.get("reference", False)
            ):
                raise ValueError(
                    label + ": INFINITE is not an active provider object macro"
                )
            closure = macro_closure({"INFINITE"}, values, use)
            if any(item["unresolved"] for item in closure.values()):
                raise ValueError(label + ": unresolved INFINITE definition")
            if (
                unit == port
                and filename == port
                and re.search(r"\bWaitForSingleObject\s*\([^;]*\bINFINITE\b", use)
            ):
                port_wait = True
    if not port_wait:
        raise ValueError("SDK wait " + port + ": missing active wait call source")


SEARCH_ENVIRONMENT = ("CPATH", "C_INCLUDE_PATH")


def provider_root(compiler: str, windows: bool) -> PurePath:
    """The directory that provider-relative names hang from, in the target's path syntax."""
    return PureWindowsPath(compiler).parent.parent.parent if windows else PurePosixPath("/")


def relative_name(path: str | Path, root: PurePath) -> str:
    """Name a path relative to the provider root, or keep it absolute outside the root.

    The target's path syntax decides containment, not the syntax of the running host.
    """
    pure = type(root)(plain_path(path))
    return pure.relative_to(root).as_posix() if pure.is_relative_to(root) else pure.as_posix()


def plain_path(path: str | Path) -> str:
    """A Windows extended-length prefix names the same file; names do not carry it."""
    text = os.fspath(path)
    if text.startswith("\\\\?\\UNC\\"):
        return "\\\\" + text[8:]
    return text.removeprefix("\\\\?\\")


def include_search(header_log: str, directory: Path, root: PurePath, environ: dict) -> dict:
    """The compiler's effective include search order, with the variables that feed it.

    Compilers print only directories that exist; a missing one is reported on an
    "ignoring nonexistent directory" line and so is absent here, which makes a
    directory that appears on one side only change the sequence. A rooted entry
    without a drive, such as MinGW's "/mingw64/include", resolves against the drive
    of the compiler's working directory.
    """
    sections = {'#include "..." search starts here:': [], "#include <...> search starts here:": []}
    collecting = None
    for line in header_log.splitlines():
        text = line.strip()
        if text in sections:
            collecting = sections[text]
        elif text == "End of search list.":
            collecting = None
        elif collecting is not None:
            collecting.append(relative_name((directory / text).resolve(strict=True), root))
    quote, system = sections.values()
    if not system:
        raise ValueError("Compiler reported no system include paths")
    return {
        "quote": quote,
        "system": system,
        "environment": {name: environ.get(name) for name in SEARCH_ENVIRONMENT},
    }


def search_directories(search: dict, root: PurePath) -> list[Path]:
    """The recorded search directories, rebuilt in the target's path syntax."""
    return [Path((root / name).as_posix()) for name in (*search["quote"], *search["system"])]


def installed_records(packages: dict) -> list[tuple[str, dict]]:
    """Check every supported provider file against its package manifest, with actual digests."""
    if not packages:
        raise ValueError("SDK environment provider inventory missing")
    records = []
    for name, package in packages.items():
        if package["algorithm"] == "symlink":
            path = Path(name)
            target = path.parent / package["expected"]
            if path.is_symlink():
                if path.resolve(strict=True) != target.resolve(strict=True):
                    raise ValueError(f"SDK environment link changed: {name}")
            elif not path.is_file() or digest(path) != digest(target):
                # MSYS2 can materialize a file link as a copy on Windows.
                raise ValueError(f"SDK environment link content differs: {name}")
            records.append((name, {**package, "sha256": digest(path) if path.is_file() else None}))
            continue
        data = Path(name).read_bytes()
        if hashlib.new(package["algorithm"], data).hexdigest() != package["expected"]:
            raise ValueError(f"SDK environment differs from provider original: {name}")
        records.append((name, {**package, "sha256": hashlib.sha256(data).hexdigest()}))
    return records


def environment_digest(records: list[tuple[str, dict]]) -> dict:
    """Record the entire supported provider inventory for traceability.

    The record does not decide whether two analyses are comparable: package revisions
    and files outside the include search space (libraries, documentation) cannot
    reach the preprocessor, and image rollouts make them differ between jobs.
    """
    rows = sorted((row for _, row in records), key=lambda row: (row["provider"], row["header"]))
    return {"format": "sdk-provider-environment-v1", "files": len(rows),
            "sha256": hashlib.sha256(json.dumps(rows, sort_keys=True).encode()).hexdigest()}


def environment_identity(packages: dict) -> dict:
    return environment_digest(installed_records(packages))


def link_target(path: str, root: PurePath) -> str:
    """A link's resolved target, relative to the provider root or explicitly outside it."""
    pure = type(root)(plain_path(path))
    if pure.is_relative_to(root):
        return pure.relative_to(root).as_posix()
    return "outside:" + pure.as_posix()


def entry_kind(entry: os.DirEntry, link: bool) -> str | None:
    """"dir" or "file" for what an entry leads to, None when a link leads nowhere usable.

    Windows reports a symlink whose target syntax is invalid (a relative target written
    with forward slashes, say) as an error instead of a missing file; such a link is as
    dangling as one whose target is absent.
    """
    try:
        return "dir" if entry.is_dir() else "file" if entry.is_file() else None
    except OSError:
        if not link:
            raise
        return None


def walk_search_directory(directory: Path, visited: set):
    """What the compiler meets below a search directory, as events.

    ("file", path) is a regular file, also one reached through a link. ("link", path, real,
    kind) is every symlink or junction met on the way, whether or not it is followed;
    kind is "dir", "file" or "dangling". ("alias", path, real) is a directory that was
    entered before and is reached again under another path, through a link higher up.
    Each real directory is entered once, under the first path that reaches it, which also
    ends cycles; every other path that reaches it is reported as a link or alias, so no
    arrival path goes unrecorded.
    """
    visited.add(os.path.normcase(os.path.realpath(directory)))

    def walk(path):
        with os.scandir(path) as entries:
            children = sorted(entries, key=lambda entry: entry.name)
        for entry in children:
            real = os.path.realpath(entry.path)
            link = entry.is_symlink() or entry.is_junction()
            kind = entry_kind(entry, link)
            if link and kind is None:
                # Nothing to resolve to: the link's own target, as written, names it.
                real = os.path.normpath(
                    os.path.join(os.path.dirname(entry.path), os.readlink(entry.path)))
            if kind == "dir":
                seen = os.path.normcase(real) in visited
                if link:
                    yield "link", entry.path, real, "dir"
                elif seen and os.path.normcase(os.path.abspath(entry.path)) != (
                        os.path.normcase(real)):
                    yield "alias", entry.path, real
                if not seen:
                    visited.add(os.path.normcase(real))
                    yield from walk(entry.path)
            else:
                if link:
                    yield "link", entry.path, real, kind or "dangling"
                if kind == "file":
                    yield "file", entry.path

    yield from walk(directory)


def provider_links(links: dict, files: dict, root: PurePath) -> dict:
    """The links whose target is a provider file or has provider files somewhere below it."""
    fold = str.casefold if isinstance(root, PureWindowsPath) else str
    related = set()
    for row in files.values():
        header = PurePosixPath(row["header"])
        related.update(fold(name.as_posix()) for name in (header, *header.parents))
    return {name: row for name, row in links.items() if fold(row["target"]) in related}


def search_inventory(records: list[tuple[str, dict]], directories: list[Path],
                     root: PurePath, opened: list[dict] = ()) -> dict:
    """Provider files and links the compiler meets through the search directories.

    `files` maps each path the compiler walks to the provider file behind it, opened or
    not; `opened` SDK headers are added under their own resolved path, so a header reached
    by an escaping relative include is covered too. `links` records every symlink,
    junction and re-reached directory with its resolved target. Equal link targets mean the
    content below them is the content at the target, which `files` lists under its first
    arrival path (or which lies outside the search space, where only provider files count).
    A header present on one side only, a link that leads elsewhere, or other bytes can
    decide which definition an include resolves to even with identical sources.
    Files that belong to no supported provider are not part of the identity: opening one
    is refused by the source provenance validation instead. For the same reason a link
    counts only when its target is a provider file or a directory holding provider files
    below it; links to a runner's other packages (clang, ncurses) come and go with image
    revisions and must not make two analyses incomparable.
    """
    provider = {}
    for name, row in sorted(records, key=lambda item: item[1]["algorithm"] != "symlink"):
        provider[os.path.normcase(os.path.realpath(name))] = row
    fields = ("header", "provider", "algorithm", "expected", "sha256")
    files, links, visited = {}, {}, set()
    for directory in directories:
        if os.path.normcase(os.path.realpath(directory)) in visited:
            continue
        for kind, path, *rest in walk_search_directory(directory, visited):
            if kind == "file":
                row = provider.get(os.path.normcase(os.path.realpath(path)))
                if row is not None:
                    files[relative_name(path, root)] = {key: row[key] for key in fields}
            else:
                links[relative_name(path, root)] = {
                    "kind": rest[1] if kind == "link" else "alias",
                    "target": link_target(rest[0], root)}
    for row in opened:
        files[relative_name(row["path"], root)] = {key: row[key] for key in fields}
    return {"format": "sdk-search-inventory-v1", "files": dict(sorted(files.items())),
            "links": dict(sorted(provider_links(links, files, root).items()))}


def validate_inventory(scope: dict) -> None:
    """The recorded inventory must be present, well formed and cover the opened SDK headers."""
    inventory = scope.get("sdkInventory")
    reason = "SDK search inventory missing, empty or incomplete"
    if (not isinstance(inventory, dict) or inventory.get("format") != "sdk-search-inventory-v1"
            or set(inventory) != {"format", "files", "links"}
            or not isinstance(inventory["files"], dict) or not inventory["files"]
            or not isinstance(inventory["links"], dict)):
        raise ValueError(reason)
    keys = {"header", "provider", "algorithm", "expected", "sha256"}
    for name, row in inventory["files"].items():
        if (not isinstance(name, str) or not isinstance(row, dict) or set(row) != keys
                or any(not isinstance(row[key], str) or not row[key]
                       for key in ("header", "provider", "algorithm", "expected"))
                or not (row["sha256"] is None and row["algorithm"] == "symlink"
                        or isinstance(row["sha256"], str))):
            raise ValueError(reason)
    for name, row in inventory["links"].items():
        if (not isinstance(name, str) or not isinstance(row, dict)
                or set(row) != {"kind", "target"}
                or row["kind"] not in {"dir", "file", "dangling", "alias"}
                or not isinstance(row["target"], str) or not row["target"]):
            raise ValueError(reason)
    covered = {(row["header"], row["sha256"]) for row in inventory["files"].values()}
    if any((row["header"], row["sha256"]) not in covered
           for row in scope.get("sdkHeaders", []) if row.get("category") == "sdk"):
        raise ValueError(reason)


def semantic(scope: dict) -> str:
    """Paths and package revisions are provenance; searchable and opened bytes are semantic."""
    headers = scope.get("sdkHeaders", [])
    packages = (
        msys_headers(Path(scope["diagnosticCompiler"]["path"]))
        if scope["target"].startswith("windows")
        else debian_headers()
    )
    for header in headers:
        path = Path(header["path"])
        package = packages.get(str(path))
        if package and package["algorithm"] == "symlink":
            package = None
        expected = {
            key: header[key]
            for key in ("provider", "header", "algorithm", "expected")
            if key in header
        }
        provenance = {key: value for key, value in package.items() if key != "version"} if package else None
        if digest(path) != header["sha256"] or provenance != (
            expected if "provider" in header else None
        ):
            raise ValueError("SDK source/provider identity changed during analysis")
    records = installed_records(packages)
    scope["sdkEnvironment"] = environment_digest(records)
    search = scope["compilerSemanticInputs"]["includeSearch"]
    windows = scope["target"].startswith("windows")
    root = provider_root(scope["diagnosticCompiler"]["path"] if windows else "", windows)
    directories = search_directories(search, root)
    opened = [row for row in headers if row.get("category") == "sdk"]
    scope["sdkInventory"] = search_inventory(records, directories, root, opened)
    identity = semantic_inputs(scope)
    return hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()


def semantic_inputs(scope: dict) -> dict:
    """The SDK inputs the compiler can find and opened, without the target host's filesystem."""
    headers = scope.get("sdkHeaders", [])
    keys = {row["path"]: row["key"] for row in headers}
    return {
        "inventory": scope["sdkInventory"]["files"],
        "links": scope["sdkInventory"]["links"],
        "lineDirectives": scope.get("sdkLineDirectives", False),
        "headers": sorted(
            (
                {k: v for k, v in row.items() if k not in {"path", "reason", "version"}}
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
