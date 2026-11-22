"""Replay unmatched scalar R10.3/R10.4 reports for the pull-request comparison."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

from ecu_tools.build import digest, safe_relative, tool_argument

from autosar_tooling import c_check, c_fingerprint, c_rename
from autosar_tooling.config import ROOT

RULES = c_rename.TYPE_RULES


def same_observations(observations: dict) -> bool:
    if set(observations) != {'baseline', 'current'}:
        return False
    configs = observations['current']
    if not isinstance(configs, list) or not configs or configs != observations['baseline']:
        return False
    for config in configs:
        if not isinstance(config, list) or len(config) != 3:
            return False
        name, standard, reports = config
        if not isinstance(name, str) or standard != 'c99' or not isinstance(reports, list) or not reports:
            return False
        for report in reports:
            if not isinstance(report, dict) or not report.get('operator'):
                return False
            operands = report.get('operands')
            if not isinstance(operands, list) or len(operands) != 2 or any(
                    not isinstance(operand, dict) or not operand.get('token')
                    or 'essentialType' not in operand or 'category' not in operand
                    or operand.get('type') not in {'bool', 'char', 'short', 'int', 'long', 'long long'}
                    for operand in operands):
                return False
    return True


def candidates(summaries, inventories, comparison):
    """A physical position selects work; only the two replayed observations can match it."""
    sites = {}
    for side, field in (('current', 'unacceptedNew'), ('baseline', 'removed')):
        wanted = set(comparison[field])
        groups = defaultdict(list)
        for row in inventories[side]:
            diagnostic = summaries[side]['diagnostics'][row['diagnostic']]
            if diagnostic['id'] not in RULES or len(diagnostic['locations']) != 1:
                continue
            loc, = diagnostic['locations']
            unit = loc['file']
            if not safe_relative(unit) or not unit.endswith('.c'):
                continue
            site = (row['program'], unit, diagnostic['id'], int(loc['line'] or 0),
                    int(loc.get('column') or 0))
            groups[site].append((row['fingerprint'], c_fingerprint.sha(diagnostic | {'programs': []})))
        sites[side] = {key: values[0] for key, values in groups.items()
                       if len(values) == 1 and values[0][0] in wanted}
    return {site: (current, sites['baseline'][site][0])
            for site, (current, signature) in sites['current'].items()
            if site in sites['baseline'] and sites['baseline'][site][1] == signature}


def renamed_inputs(unit, summaries, outputs):
    """Require the complete analyzer/project closure to differ only by identifier substitutions."""
    paths = {}
    for side, summary in summaries.items():
        scope = summary['scope']
        actual = scope['cppcheckInputs'][unit]
        if actual['unbound'] or actual['sdk'] or any(
            unit in other['project'] for name, other in scope['cppcheckInputs'].items() if name != unit
        ):
            return False
        paths[side] = set(scope['projectTranslationUnits'][unit]) | set(actual['project'])
    if (paths['baseline'] != paths['current']
            or summaries['baseline']['scope']['cppcheckInputs'][unit] != summaries['current']['scope']['cppcheckInputs'][unit]):
        return False
    changes = set()
    local = {'baseline': [], 'current': []}
    tags = set()
    for path in paths['current']:
        if not safe_relative(path):
            return False
        before, after = [(outputs[side] / 'project' / path).read_bytes() for side in ('baseline', 'current')]
        if before == after:
            continue
        texts = [value.decode('utf-8') for value in (before, after)]
        substitutions = c_rename._physical_changes(*texts)
        if not substitutions:
            return False
        if (c_fingerprint.analyzer_header_inputs(texts[0], True)['directives']
                != c_fingerprint.analyzer_header_inputs(texts[1], True)['directives']):
            return False
        for line, pairs in substitutions.items():
            for left, right in pairs:
                changes.add((left[2], right[2]))
                if path == unit:
                    local['baseline'].append(left)
                    local['current'].append(right)
                elif not all(re.fullmatch(r"\s*typedef\s+struct\s+" + re.escape(token[2])
                                          + r"\s*\*\s*[A-Za-z_]\w*\s*;\s*", text.splitlines()[line - 1])
                             for token, text in zip((left, right), texts)):
                    return False
                else:
                    tags.update((left[2], right[2]))
    if len(changes) != 1:
        return False
    return {side: {'local': positions, 'tags': sorted(tags)} for side, positions in local.items()}


def program_input(output, summary, program, unit, database_digests):
    scope = summary['scope']
    record, = (p for p in summary['programs'] if p['name'] == program and unit in p['translationUnits'])
    if program not in ('target', 'host-batch', 'probe'):
        raise ValueError('Unknown replay program')
    directory = output / program if len(summary['programs']) > 1 else output
    database = directory / 'compile_commands.json'
    contents = database.read_bytes()
    if hashlib.sha256(contents).hexdigest() != database_digests.get(program):
        raise ValueError('Original compilation database lacks its producer digest')
    rows = json.loads(contents)
    row, = (row for row in rows if row['file'] == row['directory'].rstrip('/') + '/' + unit)
    base = row['directory'].rstrip('/')
    builtins = scope['compilerSemanticInputs']['builtins']
    arguments = row['arguments']
    start = 1 + len(scope['compilerFlags'])
    actual_builtins = arguments[start:start + len(builtins)]
    expected = ['gcc', *scope['compilerFlags'], *actual_builtins, *scope['compilerDiscoveredDefines'][unit],
                *('-I' + base + '/' + path for path in scope['includePaths']), '-c', row['file']]
    if sorted(actual_builtins) != sorted(builtins) or arguments != expected:
        raise ValueError('Original compilation database differs from the recorded TU configuration')
    command = record['command']
    platform_name = 'win64' if scope['target'].startswith('windows') else 'unix64'
    library_names = ('std', 'windows' if platform_name == 'win64' else 'posix')
    prefix = ['--std=c99', '--language=c', '--platform=' + platform_name, '--check-level=exhaustive',
              *('--library=' + scope['cppcheckLibraries'][name] for name in library_names),
              '--enable=warning,style,performance,portability,information', '--error-exitcode=1',
              '--xml', '--xml-version=2']
    # Only paths owned by the CLI output directory may be relocated.
    paths = command[1 + len(prefix):]
    keys = ['--addon-python', '--project', '--cppcheck-build-dir', '--checkers-report', '--addon', '--addon']
    filenames = ['compile_commands.json', 'cache', 'checkers.txt', 'misra.json', 'project.json']
    for name, switch, filename in (('platformModel', '--include', 'platform-model.h'),
                                   ('platformLibrary', '--library', 'platform-library.cfg')):
        if scope[name]:
            keys.append(switch)
            filenames.append(filename)
    parts = [arg.partition('=') for arg in paths]
    if (command[1:1 + len(prefix)] != prefix or [key for key, _, _ in parts] != keys
            or any(not sep or not path for _, sep, path in parts)
            or [path.replace('\\', '/').rsplit('/', 1)[-1] for _, _, path in parts[1:]] != filenames):
        raise ValueError('Unrecognized original Cppcheck command')
    return (*scope['compilerFlags'], *actual_builtins, *scope['compilerDiscoveredDefines'][unit])


def replay_side(summary, output, selected, edits, tool, python, work):
    exe, addons, tool_info = tool
    scope = summary['scope']
    if any(tool_info[key] != summary['tool'][key] for key in ('binarySha256', 'addons', 'version', 'expectedSourceRevision')):
        raise ValueError('Replay checker differs from the original analysis')
    config_dirs = [exe.parent] + ([Path(tool_info['filesDirectory'])] if tool_info['filesDirectory'] else [])
    if scope['cppcheckConfig'] or any((path / 'cppcheck.cfg').exists() for path in config_dirs):
        raise ValueError('Implicit replay configuration requires additional evidence')
    work.mkdir(mode=0o700)
    root = work / 'project'
    shutil.copytree(output / 'project', root)
    lib = exe.parent / 'cfg'
    def text_hash(path):
        return hashlib.sha256(path.read_text(encoding='utf-8').encode()).hexdigest()
    libraries = ['std', 'windows' if scope['target'].startswith('windows') else 'posix']
    semantic = summary['semanticIdentity']
    implicit = next(path for path in (exe.parent / 'std.cfg', lib / 'std.cfg') if path.is_file())
    if (any(text_hash(lib / (name + '.cfg')) != semantic['libraries'][name] for name in libraries)
            or text_hash(implicit) != semantic['libraries']['implicitStd']
            or any(text_hash(addons / name) != value for name, value in semantic['addonRuntime'].items())):
        raise ValueError('Replay library or addon runtime differs')
    argv = [str(exe), '--dump', '--std=c99', '--language=c',
            '--platform=' + ('win64' if scope['target'].startswith('windows') else 'unix64'),
            '--check-level=exhaustive', *('--library=' + str(lib / (name + '.cfg')) for name in libraries),
            '--enable=warning,style,performance,portability,information']
    model = None
    for key, name, switch in (('platformModel', 'platform-model.h', '--include='),
                              ('platformLibrary', 'platform-library.cfg', '--library=')):
        if scope[key]:
            file = output / name
            if digest(file) != scope[key]['sha256'] or text_hash(file) != semantic['models'][key]:
                raise ValueError('Replay platform model differs')
            argv.append(switch + str(file))
            if key == 'platformModel':
                model = file
    units, commands = {}, []
    for unit, flags in selected.items():
        if Path(str(root / unit) + '.dump').exists():
            raise ValueError('Replay input already contains a candidate dump')
        file = tool_argument(root / unit)
        commands.append({'directory': tool_argument(root), 'file': file, 'arguments': [
            'gcc', *flags, *('-I' + tool_argument(root / path) for path in scope['includePaths']), '-c', file]})
        units[unit] = [str(root / name) for name in scope['cppcheckInputs'][unit]['project']]
    db = work / 'compile_commands.json'
    db.write_text(json.dumps(commands), encoding='utf-8')
    cache = work / 'cache'
    cache.mkdir()
    c_check.run([*argv, '--project=' + str(db), '--cppcheck-build-dir=' + str(cache)], work, 'type-replay')
    request = work / 'request.json'
    request.write_text(json.dumps({'root': str(root), 'addons': str(addons), 'units': units,
                                   'local': edits, 'model': str(model) if model else None}), encoding='utf-8')
    result = work / 'types.json'
    c_check.run([str(python), '-I', str(Path(__file__).with_name('c_type_replay.py')),
                 str(request), str(result)], work, 'type-operands')
    return {unit: {(rule, line, column): evidence for rule, line, column, evidence in rows}
            for unit, rows in json.loads(result.read_bytes()).items()}


def replay(summaries, inputs, pairs, excluded):
    """Return dual-tree evidence only for original, unbound, uniquely located reports."""
    if c_check.TARGETS.get(summaries['current']['scope']['target']) != platform.system():
        return {}
    pairs = {site: pair for site, pair in pairs.items() if not excluded.intersection(pair)}
    if (c_fingerprint.canonical_config(summaries['current'], {}) !=
            c_fingerprint.canonical_config(summaries['baseline'], {})
            or summaries['current']['semanticIdentity']['sdk'] != summaries['baseline']['semanticIdentity']['sdk']):
        return {}
    if not pairs:
        return {}
    outputs = {side: value[0] for side, value in inputs.items()}
    databases = {side: value[1].get('compileCommandsSha256', {}) for side, value in inputs.items()}
    eligible = {unit for _, unit, *_ in pairs if unit in summaries['baseline']['scope']['cppcheckInputs']
                and unit in summaries['current']['scope']['cppcheckInputs']}
    edits = {unit: change for unit in eligible if (change := renamed_inputs(unit, summaries, outputs))}
    anchors = {unit: [c_fingerprint.SourceContext((outputs[side] / 'project' / unit).read_text(encoding='utf-8'))
                      for side in ('baseline', 'current')] for unit in edits}
    selected, comparable, configurations = {}, {}, {}
    for site, pair in pairs.items():
        program, unit, *_ = site
        if unit not in edits:
            continue
        before, after = (source.anchor(*site[-2:]) for source in anchors[unit])
        if before is None or before != after:
            continue
        if (program, unit) not in configurations:
            configurations[program, unit] = [program_input(outputs[side], summaries[side], program, unit, databases[side])
                                             for side in ('baseline', 'current')]
        flags = configurations[program, unit]
        if flags[0] != flags[1]:
            continue
        if unit in selected and selected[unit] != flags[0]:
            raise ValueError('Candidate TU has differing program configurations')
        selected[unit] = flags[0]
        comparable[site] = pair
    if not selected:
        return {}
    with tempfile.TemporaryDirectory(prefix='c-type-replay-') as directory:
        work = Path(directory)
        executable = os.environ.get("CPPCHECK") or shutil.which("cppcheck")
        if not executable or digest(Path(executable)) != summaries['current']['tool']['binarySha256']:
            raise ValueError("Original analyzer executable unavailable for replay")
        original = {value[1]['identity']['python']['binary'] for value in inputs.values()}
        interpreters = (sys.executable, ROOT / '.venv' / ('Scripts/python.exe' if os.name == 'nt' else 'bin/python'),
                        shutil.which('python3'))
        python = next((Path(path) for path in interpreters if path and Path(path).is_file()
                       and {digest(Path(path))} == original), None)
        if python is None:
            raise ValueError("Original addon Python executable unavailable for replay")
        tool = c_check.tool_identity(work)
        results = {side: replay_side(summaries[side], outputs[side], selected, {unit: edits[unit][side] for unit in selected}, tool, python, work / side)
                   for side in ('baseline', 'current')}
    matches = {}
    for site, (current, baseline) in comparable.items():
        _, unit, *position = site
        facts = [results[side][unit].get(tuple(position)) for side in ('baseline', 'current')]
        if facts[0] and facts[0] == facts[1]:
            matches[current] = {'baseline': baseline, 'observations': dict(zip(('baseline', 'current'), facts))}
    return matches
