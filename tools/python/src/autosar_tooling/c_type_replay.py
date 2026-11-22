"""Observe scalar operands when the pinned MISRA addon reports R10.3 or R10.4."""

from __future__ import annotations

from collections import defaultdict
from pathlib import Path


def scalar(token, misra, root):
    """Limit the proof to integral literals and direct integral object reads."""
    value = token.valueType if token else None
    if (not value or not value.isIntegral() or value.pointer or value.typeScope
            or token.astOperand1 or token.astOperand2):
        return None
    declaration = None
    var = token.variable
    if var:
        name = var.nameToken
        if (not name or name.str != token.str or not Path(name.file).is_relative_to(root)
                or var.isArray or var.isPointer or var.isReference):
            return None
        declaration = (Path(name.file).relative_to(root).as_posix(), name.linenr, name.column,
                       name.str, var.access, var.isStatic, var.isExtern)
    elif not token.isInt:
        return None
    essential = misra.getEssentialType(token)
    return {
        "token": token.str, "macro": token.macroName, "originalName": token.originalName,
        "declaration": declaration, "type": value.type, "sign": value.sign, "bits": value.bits,
        "constness": value.constness, "originalType": value.originalTypeName,
        "knownInt": token.getKnownIntValue(), "essentialType": essential,
        "essentialBits": misra.bitsOfEssentialType(essential),
        "category": misra.getEssentialTypeCategory(token),
    }


def reports_for(config, checker, misra, root, unit):
    sites = defaultdict(list)
    tokens = {}

    def record(token, major, minor):
        if Path(token.file) != root / unit:
            return
        site = (f"misra-c2012-{major}.{minor}", token.linenr, token.column)
        operands = [scalar(arg, misra, root) for arg in (token.astOperand1, token.astOperand2)]
        if (None in operands or tokens.setdefault(site, token) is not token
                or token.valueType and token.valueType.pointer):
            sites[site].append(None)
        else:
            sites[site].append({"operator": token.str, "originalName": token.originalName,
                                "macro": token.macroName, "operands": operands})

    checker.reportError = record
    checker.misra_10_3(config)
    checker.misra_10_4(config)
    return sites


def one_local(config, source, wanted):
    declarations, seen = set(), set()
    for token in config.tokenlist:
        position = (token.linenr, token.column, token.str)
        if position not in wanted or Path(token.file) != source:
            continue
        var = token.variable
        if not var or not var.isLocal or var.isStatic or var.isExtern or not var.nameToken:
            return False
        declaration = var.nameToken
        if Path(declaration.file) != source:
            return False
        declarations.add((declaration.linenr, declaration.column, declaration.str))
        seen.add(position)
    if seen != wanted or declarations != {min(wanted)}:
        return False
    for token in config.tokenlist:
        declaration = token.variable.nameToken if token.variable else None
        if not declaration or Path(declaration.file) != source:
            continue
        belongs = (declaration.linenr, declaration.column, declaration.str) in declarations
        if belongs and (Path(token.file) != source or (token.linenr, token.column, token.str) not in wanted):
            return False
    return True


def read_dump(dump, addons, root, unit, expected, model, edits):
    """Only retain unambiguous reports present in every original checker configuration."""
    import sys

    sys.path.insert(0, str(addons))
    import cppcheckdata
    import misra

    data = cppcheckdata.parsedump(str(dump))
    if Path(data.files[0]) != root / unit:
        raise ValueError("Replay dump primary input differs")
    for key in ("CHAR", "SHORT", "INT", "LONG", "LONG_LONG", "POINTER"):
        misra.typeBits[key] = getattr(data.platform, key.lower() + "_bit")
    checker = misra.MisraChecker(None)
    files = set(data.files) | {token.file for token in data.rawTokens}
    configurations = []
    wanted = {tuple(row) for row in edits["local"]}
    for config in data.iterconfigurations():
        if any(scope.type in ("Struct", "Union") and scope.className in edits["tags"] for scope in config.scopes):
            return {}
        if wanted and not one_local(config, root / unit, wanted):
            return {}
        files.update(directive.file for directive in config.directives)
        files.update(token.file for token in config.tokenlist)
        sites = reports_for(config, checker, misra, root, unit)
        configurations.append((config.name, config.standards.c, sites))
    actual = {Path(name).resolve() for name in files if name}
    if model:
        actual.discard(Path(model))
    if actual != expected or not configurations:
        raise ValueError("Replay did not consume the original Cppcheck input set")
    common = set.intersection(*(set(sites) for _, _, sites in configurations))
    return {site: [(name, standard, sites[site]) for name, standard, sites in configurations]
            for site in common if all(None not in sites[site] for _, _, sites in configurations)}


if __name__ == '__main__':
    import json
    import sys

    request = json.loads(Path(sys.argv[1]).read_bytes())
    output = {}
    for unit, files in request['units'].items():
        root = Path(request['root'])
        result = read_dump(Path(str(root / unit) + '.dump'), Path(request['addons']), root, unit,
                           {Path(name) for name in files}, request['model'], request['local'][unit])
        output[unit] = [[*key, value] for key, value in result.items()]
    Path(sys.argv[2]).write_text(json.dumps(output), encoding='utf-8')
