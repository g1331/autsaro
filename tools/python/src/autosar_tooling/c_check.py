"""Analyze a sealed generated C project without modifying its source package."""
from __future__ import annotations

import importlib.util
import json
import os
import platform
import re
import shutil
import sys
import xml.etree.ElementTree as ET
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from ecu_tools.build import (
    TARGETS,
    digest,
    refuse_links,
    safe_relative,
    sealed_sources,
    target_configuration,
    tool_argument,
)
from ecu_tools.process import OwnedProcess, ProcessSpec

from autosar_tooling.config import ROOT

CPPCHECK_VERSION = "2.21.0"
CPPCHECK_REVISION = "e73bf44c3e49686b7495fab352d03a6c6075516b"
ADDON_HASHES = {
    "misra.py": "d1f7947b6c2c6f3980314155d6a89f0e6a9435456fb3a02ac1ded25a99af3f44",
    "cppcheckdata.py": "8f14a656e8c753246ca86fe9ab047ebee2b1ef23ab94304cbe710423bbe8baeb",
    "misra_9.py": "2116c59e403c49c662590d4d83a9d220256c0e396fa13c20a475e43d87970c70",
}
TIMEOUT = 600


def run(argv: list[str], directory: Path, stage: str, timeout: int = TIMEOUT) -> tuple[int, str, str]:
    """Keep raw subprocess output and reclaim owned processes on timeout."""
    result = OwnedProcess(ProcessSpec.seconds(argv, directory, timeout, directory, stage)).wait()
    stdout = result.stdout.read_text(encoding="utf-8", errors="replace")
    stderr = result.stderr.read_text(encoding="utf-8", errors="replace")
    (directory / f"{stage}.stdout.log").write_text(stdout, encoding="utf-8")
    (directory / f"{stage}.stderr.log").write_text(stderr, encoding="utf-8")
    if result.status != "exited":
        raise RuntimeError(f"{stage}: {result.status}; logs retained in {directory}")
    return result.exit_code or 0, stdout, stderr


def addon_directory(executable: Path) -> Path:
    requested = os.environ.get("CPPCHECK_ADDON_DIR")
    candidates = ([Path(requested)] if requested else [
        executable.parent.parent / "share/cppcheck/addons",
        executable.parent / "addons",
        Path("/usr/share/cppcheck/addons"),
    ])
    for path in candidates:
        if all((path / name).is_file() for name in ("misra.py", "cppcheckdata.py", "misra_9.py")):
            return path.resolve()
    raise ValueError("Cppcheck 2.21.0 addons missing; set CPPCHECK_ADDON_DIR")


def tool_identity(directory: Path) -> tuple[Path, Path, dict]:
    found = os.environ.get("CPPCHECK") or shutil.which("cppcheck")
    if not found or not Path(found).is_file():
        raise ValueError("Cppcheck is missing; install the pinned 2.21.0 release")
    executable = Path(found).resolve()
    code, stdout, stderr = run([str(executable), "--version"], directory, "version", 30)
    if code or stdout.strip() != f"Cppcheck {CPPCHECK_VERSION}":
        raise ValueError(f"Cppcheck version mismatch: {stdout.strip()} {stderr.strip()}")
    addons = addon_directory(executable)
    hashes = {name: digest(addons / name) for name in ADDON_HASHES}
    if hashes != ADDON_HASHES:
        raise ValueError("Cppcheck addon hashes differ from the pinned 2.21.0 source")
    return executable, addons, {
        "version": CPPCHECK_VERSION,
        "expectedSourceRevision": CPPCHECK_REVISION,
        "binarySha256": digest(executable),
        "addons": hashes,
    }


def coverage(addons: Path) -> dict:
    """Report checker availability; never infer per-rule compliance from zero findings."""
    sys.path.insert(0, str(addons))
    try:
        spec = importlib.util.spec_from_file_location("_autsaro_misra", addons / "misra.py")
        if spec is None or spec.loader is None:
            raise ValueError("Cannot load MISRA addon coverage")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        addon_rules = module.getAddonRules()
        core_rules = module.getCppcheckRules()
    finally:
        sys.path.pop(0)
    inventory = json.loads((ROOT / ".agents/skills/misra-c2012/references/baseline-index.json").read_text(encoding="utf-8"))
    # The locked repository inventory includes directives and amendment guidance.
    items = inventory["guideline_metadata"]
    available = {f"R{value}" for value in [*addon_rules, *core_rules]}
    return {
        "basis": "MISRA C:2012 + AMD1-AMD4 + TC1-TC2; C99",
        "completeCompliance": False,
        "knownCheckerLimitations": [{
            "guideline": "R21.8",
            "detail": "Pinned addon still reports getenv although AMD1 removed that prohibition; its termination-call list also omits _Exit/quick_exit. Raw diagnostics remain unchanged.",
            "basis": "https://misra.org.uk/app/uploads/2021/06/MISRA-C-2012-AMD2.pdf",
        }],
        "rules": [{"id": item["id"], "category": item["category"],
                   "automaticChecker": item["id"] in available,
                   "assessment": "not_assessed"} for item in items],
        "projectChecks": {"autsaro-dynamicAllocation": "Direct calls only; partial Dir 4.12 check",
                          "autsaro-hostThreadInBsw": "Project adapter boundary; not complete Dir 5.3"},
    }


def prepare(project: Path, target_id: str, output: Path) -> tuple[Path, list[str], list[dict], dict]:
    names, target = sealed_sources(project)
    if target["target"] != target_id:
        raise ValueError("Requested analysis target differs from the sealed target")
    if TARGETS[target_id] != platform.system():
        raise ValueError("Analyze each target on its corresponding Windows/Linux host")
    flags, sources, includes = target_configuration(names, target)
    compiler = os.environ.get("AUTOSAR_CC") or shutil.which("gcc")
    if not compiler or not Path(compiler).is_file():
        raise ValueError("GCC is required to discover the target's system headers")
    compiler = str(Path(compiler).resolve())
    code, machine, _ = run([compiler, "-dumpmachine"], output, "compiler-target", 30)
    if code or machine.strip() != target["abi"]:
        raise ValueError("Diagnostic compiler ABI differs from the sealed target")
    code, compiler_version, _ = run([compiler, "--version"], output, "compiler-version", 30)
    if code:
        raise ValueError("Cannot identify diagnostic compiler")
    empty = output / "headers.c"
    empty.write_text("", encoding="utf-8")
    code, _, header_log = run([compiler, *flags, "-E", "-v", "-x", "c", str(empty)], output, "compiler-headers", 30)
    if code:
        raise ValueError("Cannot discover compiler include search paths")
    collecting = False
    system_includes = []
    for line in header_log.splitlines():
        if line.strip() == "#include <...> search starts here:":
            collecting = True
        elif line.strip() == "End of search list.":
            collecting = False
        elif collecting:
            path = Path(line.strip()).resolve(strict=True)
            if not path.is_dir():
                raise ValueError("Compiler reported a non-directory include path")
            system_includes.append(tool_argument(path))
    if not system_includes:
        raise ValueError("Compiler reported no system include paths")
    code, macros, _ = run([compiler, *flags, "-dM", "-E", "-x", "c", str(empty)], output, "compiler-macros", 30)
    if code or "#define __GNUC__ " not in macros:
        raise ValueError("Cannot discover compiler predefined macros")
    predefines = output / "compiler-predefines.h"
    predefines.write_text(macros, encoding="utf-8")
    root = output / "project"
    shutil.copytree(project, root)
    sealed_sources(root)
    sources = list(sources)
    if target.get("profile") == "ecu":
        kernel_sources = target.get("kernelSources")
        if (not isinstance(kernel_sources, list) or not kernel_sources
                or any(not isinstance(name, str) or not safe_relative(name) for name in kernel_sources)):
            raise ValueError("Pinned kernel translation units are missing")
        git = shutil.which("git")
        if not git:
            raise ValueError("Git is required to prepare the reviewed kernel patches")
        for index, patch in enumerate(target["kernelPatches"]):
            patch_path = patch["path"].removeprefix("runtime/")
            if patch_path not in names:
                raise ValueError("Pinned kernel patch is outside the sealed inventory")
            argv = [str(Path(git).resolve()), "apply", "--ignore-space-change",
                    *("--include=" + name for name in patch["includes"]), str(root / patch_path)]
            # Apply only inside the copied kernel, never the authoritative source.
            result = OwnedProcess(ProcessSpec.seconds(argv, root / "kernel", 30, output, f"patch-{index}")).wait()
            if not result.success:
                raise ValueError(f"Kernel patch {index} failed; raw logs retained")
        sources.extend("kernel/" + name for name in kernel_sources)
        sources.extend(["src/ecu_host_batch.c", "src/ecu_probe.c"])
    elif target.get("profile") != "host":
        raise ValueError("Unknown sealed C profile")
    if len(sources) != len(set(sources)) or any(not (root / name).is_file() for name in sources):
        raise ValueError("C translation-unit inventory is duplicated or incomplete")
    # Declared C files outside the link closure are still checked; distinguish them
    # in the report, rather than silently dropping optional/test adapters.
    supplemental = [name for name in names if name.endswith(".c") and not name.startswith("kernel/") and name not in sources]
    sources.extend(supplemental)
    if any(not (root / name).is_dir() for name in includes):
        raise ValueError("Declared project include directory is missing")
    includes = [tool_argument(root / name) for name in includes]
    builtins = ["-D" + line[8:].replace(" ", "=", 1) for line in macros.splitlines()
                if line.startswith("#define ") and line[8:].split(" ", 1)[0] in {
                    "__GNUC__", "__GNUC_MINOR__", "__GNUC_PATCHLEVEL__", "__x86_64__",
                    "_WIN32", "_WIN64", "__MINGW32__", "__MINGW64__", "_UCRT",
                    "__linux__", "__unix__", "__LP64__", "__STDC_VERSION__", "__STDC__"}]
    owned_text = "\n".join((root / name).read_text(encoding="utf-8") for name in names
                            if name.endswith((".c", ".h")))
    identifiers = set(re.findall(r"\b[A-Za-z_]\w*\b", owned_text))
    owned_macros = set(re.findall(r"^\s*#\s*define\s+(\w+)", owned_text, re.MULTILINE))
    model = None
    if target_id.startswith("linux") and identifiers.intersection({"PTHREAD_MUTEX_RECURSIVE", "SIGRTMIN", "PTHREAD_STACK_MIN"}):
        model = output / "platform-model.h"
        # Preserve anonymous-enum and dynamic-function types; never substitute
        # the glibc runtime values with invented constants.
        model.write_text("#ifndef AUTSAR_GLIBC_ANALYSIS_MODEL_H\n#define AUTSAR_GLIBC_ANALYSIS_MODEL_H\n"
                         "enum { PTHREAD_MUTEX_RECURSIVE = 1 };\n"
                         "extern int __libc_current_sigrtmin(void);\n"
                         "extern long __sysconf(int name);\n#endif\n", encoding="utf-8")
        validation = output / "platform-model-check.c"
        validation.write_text("#ifndef _GNU_SOURCE\n#define _GNU_SOURCE 1\n#endif\n"
                              "#include <pthread.h>\n#include <signal.h>\n#include <unistd.h>\n"
                              "typedef char AutsaroMutexValue[(PTHREAD_MUTEX_RECURSIVE == 1) ? 1 : -1];\n"
                              "int (*AutsaroSignalPrototype)(void) = &__libc_current_sigrtmin;\n"
                              "long (*AutsaroStackPrototype)(int) = &__sysconf;\n", encoding="utf-8")
        code, _, _ = run([compiler, *flags, *("-I" + path for path in includes), "-fsyntax-only", str(validation)], output, "platform-model-check", 30)
        if code:
            raise ValueError("Linux library analysis model differs from the actual compiler headers")
    per_unit_macros = {}
    stdio_modes = {}
    mode_names = {"_IOFBF", "_IOLBF", "_IONBF"} if "setvbuf" in identifiers else set()
    def compiler_unit(item):
        index, name = item
        unit_modes = {}
        code, _, _ = run([compiler, *flags, *("-I" + path for path in includes),
                          "-fsyntax-only", str(root / name)], output, f"compiler-unit-{index}", 60)
        if code:
            raise ValueError(f"Compiler validation failed for {name}; logs retained")
        code, definitions, _ = run([compiler, *flags, *("-I" + path for path in includes),
                                    "-dM", "-E", str(root / name)], output, f"compiler-defines-{index}", 60)
        if code:
            raise ValueError(f"Compiler preprocessing failed for {name}")
        standard_constants = {f"{prefix}{width}_C" for prefix in ("INT", "UINT")
                              for width in ("8", "16", "32", "64", "MAX")}
        function_macros = ["-D" + line[8:].replace(" ", "=", 1)
                           for line in definitions.splitlines() if line.startswith("#define ")
                           and line[8:].split("(", 1)[0] in standard_constants & identifiers - owned_macros]
        candidates = [line[8:].split(" ", 1)[0] for line in definitions.splitlines()
                      if line.startswith("#define ")
                      and line[8:].split(" ", 1)[0] in (identifiers - owned_macros) | mode_names
                      and "(" not in line[8:].split(" ", 1)[0]]
        probe = output / f"macro-probe-{index}.c"
        probe.write_text('#include "' + tool_argument(root / name) + '"\n' +
                         "".join(f"AUTSAR_VALUE_{macro} {macro}\n" for macro in candidates), encoding="utf-8")
        code, expanded, _ = run([compiler, *flags, *("-I" + path for path in includes),
                                "-E", "-P", str(probe)], output, f"compiler-values-{index}", 60)
        if code:
            raise ValueError(f"Compiler macro expansion failed for {name}")
        unit_macros = function_macros
        for line in expanded.splitlines():
            if not line.startswith("AUTSAR_VALUE_"):
                continue
            macro, _, value = line.removeprefix("AUTSAR_VALUE_").partition(" ")
            numeric = re.fullmatch(r"[()\s0-9a-fA-FxXuUlL+*/|&<>~-]+", value)
            dynamic = model is not None and (
                (macro == "SIGRTMIN" and re.fullmatch(r"[()\s]*__libc_current_sigrtmin\s*\(\s*\)[()\s]*", value))
                or (macro == "PTHREAD_STACK_MIN" and re.fullmatch(r"[()\s]*__sysconf\s*\(\s*[0-9]+\s*\)[()\s]*", value)))
            if numeric or dynamic:
                unit_macros.append(f"-D{macro}={value}")
                if macro in mode_names:
                    try:
                        unit_modes[macro] = int(value.strip("() ").rstrip("uUlL"), 0)
                    except ValueError as error:
                        raise ValueError(f"Unsupported stdio mode expansion: {macro}") from error
            elif model is not None and macro in {"SIGRTMIN", "PTHREAD_STACK_MIN"}:
                raise ValueError(f"Unsupported Linux system macro expansion: {macro}")
        return name, unit_macros, unit_modes

    # Each TU owns distinct probe/log paths; collect in declared source order.
    # Two compiler processes overlap header/owner setup without unbounded fan-out.
    with ThreadPoolExecutor(max_workers=2) as executor:
        try:
            for name, macros, modes in executor.map(compiler_unit, enumerate(sources)):
                per_unit_macros[name] = macros
                for mode, value in modes.items():
                    if mode in stdio_modes and stdio_modes[mode] != value:
                        raise ValueError(f"Inconsistent compiler stdio mode: {mode}")
                    stdio_modes[mode] = value
        except BaseException:
            executor.shutdown(wait=True, cancel_futures=True)
            raise
    library_model = None
    if target_id.startswith("windows") and mode_names:
        if set(stdio_modes) != mode_names:
            raise ValueError("Compiler did not report the complete setvbuf mode inventory")
        library_model = output / "platform-library.cfg"
        library_model.write_text('<def format="2"><function name="setvbuf"><arg nr="3"><valid>' +
                                 ",".join(str(stdio_modes[name]) for name in sorted(mode_names)) +
                                 '</valid></arg></function></def>\n', encoding="utf-8")
    database = [{"directory": tool_argument(root), "file": tool_argument(root / name),
                 "arguments": ["gcc", *flags, *builtins, *per_unit_macros[name], *("-I" + path for path in includes),
                               "-c", tool_argument(root / name)]} for name in sources]
    return root, sources, database, {"target": target_id, "profile": target["profile"],
                                   "compilerFlags": flags, "compilerWorkers": 2, "includePaths": target["includePaths"],
                                   "systemIncludePaths": system_includes, "diagnosticCompiler": {"path": compiler, "version": compiler_version, "sha256": digest(Path(compiler))},
                                   "compilerDiscoveredDefines": per_unit_macros,
                                   "platformModel": {"path": str(model), "sha256": digest(model)} if model else None,
                                   "platformLibrary": {"path": str(library_model), "sha256": digest(library_model), "stdioModes": stdio_modes} if library_model else None,
                                   "translationUnits": sources, "supplemental": supplemental,
                                   "adoptedSources": [name for name in sources if name.startswith("kernel/")],
                                   "patchedKernel": target.get("kernelPatches", [])}


def parse_diagnostics(xml: str, root: Path) -> list[dict]:
    tree = ET.fromstring(xml)
    if tree.tag != "results" or tree.find("errors") is None:
        raise ValueError("Cppcheck did not produce a complete XML result")
    diagnostics = []
    for error in tree.findall("errors/error"):
        locations = []
        for location in error.findall("location"):
            file = location.get("file", "")
            path = Path(file)
            if path.is_absolute() and path.is_relative_to(root):
                file = path.relative_to(root).as_posix()
            locations.append({"file": file, "line": location.get("line"), "column": location.get("column")})
        diagnostics.append({"id": error.get("id"), "severity": error.get("severity"),
                            "message": error.get("msg"), "locations": locations,
                            "adopted": bool(locations) and all(row["file"].startswith("kernel/") for row in locations)})
    return diagnostics


def analyze_program(executable: Path, addons: Path, root: Path, sources: list[str], database: list[dict],
                    scope: dict, target_id: str, output: Path) -> tuple[int, bool, list[dict], list[str]]:
    (output / "compile_commands.json").write_text(json.dumps(database, indent=2) + "\n", encoding="utf-8")
    for name, config in {
        "misra": {"script": str(addons / "misra.py"), "ctu": True},
        "project": {"script": str(Path(__file__).with_name("c_addon.py")),
                    "args": [f"--data-directory={addons}", f"--source-root={root}",
                             f"--receipt-directory={output / 'addon-receipts'}"]},
    }.items():
        (output / f"{name}.json").write_text(json.dumps(config) + "\n", encoding="utf-8")
    (output / "cache").mkdir()
    platform_name = "win64" if target_id.startswith("windows") else "unix64"
    argv = [str(executable), "--std=c99", "--language=c", f"--platform={platform_name}",
            "--check-level=exhaustive", "--library=" + ("windows" if target_id.startswith("windows") else "posix"),
            "--enable=warning,style,performance,portability,information", "--error-exitcode=1",
            "--xml", "--xml-version=2", "--addon-python=" + sys.executable,
            "--project=" + str(output / "compile_commands.json"),
            "--cppcheck-build-dir=" + str(output / "cache"),
            "--checkers-report=" + str(output / "checkers.txt"),
            "--addon=" + str(output / "misra.json"), "--addon=" + str(output / "project.json")]
    if scope.get("platformModel"):
        argv.append("--include=" + scope["platformModel"]["path"])
    if scope.get("platformLibrary"):
        argv.append("--library=" + scope["platformLibrary"]["path"])
    command = argv
    code, _, stderr = run(argv, output, "cppcheck")
    (output / "diagnostics.xml").write_text(stderr, encoding="utf-8")
    diagnostics = parse_diagnostics(stderr, root)
    passed = code == 0 and not any(row["id"] != "checkersReport" for row in diagnostics)
    inventory = (output / "cache/files.txt").read_text(encoding="utf-8").splitlines()
    parsed = {Path(line.split(":::" , 1)[-1]).resolve() for line in inventory}
    if parsed != {(root / name).resolve() for name in sources}:
        raise ValueError("Cppcheck translation-unit inventory differs from requested sources")
    receipts = list((output / "addon-receipts").glob("*.json"))
    addon_units = set()
    for receipt in receipts:
        data = json.loads(receipt.read_text(encoding="utf-8"))
        if (not isinstance(data, dict) or not isinstance(data.get("units"), list)
                or not data["units"] or any(not isinstance(unit, str) or not unit for unit in data["units"])
                or type(data.get("configurations")) is not int or data["configurations"] < 1):
            raise ValueError("Invalid project addon receipt structure")
        addon_units.update(Path(unit).resolve() for unit in data["units"])
    if addon_units != parsed or len(receipts) != len(sources):
        raise ValueError("Project addon did not analyze every translation unit")
    incomplete = {"misra-config", "syntaxError", "preprocessorErrorDirective", "internalError",
                  "cppcheckError", "addonError", "missingInclude", "missingIncludeSystem", "toomanyconfigs"}
    if any(row["id"] in incomplete for row in diagnostics):
        raise ValueError("Analyzer configuration or parsing is incomplete; raw diagnostics retained")
    return code, passed, diagnostics, command


def check(project: Path, target_id: str, output: Path) -> int:
    refuse_links(project)
    project = project.resolve(strict=True)
    refuse_links(output)
    output = output.absolute().resolve()
    if output == project or output.is_relative_to(project) or project.is_relative_to(output):
        raise ValueError("C analysis output must be separate from the sealed source project")
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise ValueError("C analysis output must be new or empty")
    output.mkdir(parents=True, exist_ok=True, mode=0o700)
    summary = {"passed": False, "error": None}
    try:
        executable, addons, identity = tool_identity(output)
        summary["tool"] = identity
        summary["coverage"] = coverage(addons)
        summary["sourceSealSha256"] = digest(project / "files.sha256")
        root, sources, database, scope = prepare(project, target_id, output)
        summary["scope"] = scope
        entries = {"host-batch": "src/ecu_host_batch.c", "legacy-probe": "src/ecu_probe.c"}
        # build.py selects exactly one ECU main for each native executable.
        # Keep cross-TU checking inside that same link boundary.
        programs = entries if scope.get("profile") == "ecu" and all(name in sources for name in entries.values()) else {"target": None}
        summary["programs"] = []
        summary["diagnostics"] = []
        seen = {}
        all_passed = True
        for program, entry in programs.items():
            units = [name for name in sources if entry is None or name not in entries.values() or name == entry]
            directory = output if len(programs) == 1 else output / program
            directory.mkdir(exist_ok=True, mode=0o700)
            commands = [row for row in database if Path(row["file"]).resolve() in {(root / name).resolve() for name in units}]
            code, passed, diagnostics, command = analyze_program(executable, addons, root, units, commands, scope, target_id, directory)
            summary["programs"].append({"name": program, "entry": entry, "translationUnits": units,
                                        "exitCode": code, "passed": passed, "command": command})
            all_passed = all_passed and passed
            for row in diagnostics:
                key = json.dumps(row, sort_keys=True)
                if key not in seen:
                    row["programs"] = []
                    seen[key] = row
                    summary["diagnostics"].append(row)
                seen[key]["programs"].append(program)
        diagnostics = summary["diagnostics"]
        sealed_sources(project)
        if digest(project / "files.sha256") != summary["sourceSealSha256"]:
            raise ValueError("Source seal changed during analysis")
        summary["passed"] = all_passed
        print(f"C analysis {target_id}: {len(sources)} units, {len(diagnostics)} diagnostics; passed={summary['passed']}")
        return 0 if summary["passed"] else 1
    except (ValueError, RuntimeError, OSError, ET.ParseError, KeyError, ImportError) as error:
        summary["passed"] = False
        summary["error"] = str(error)
        print(f"C analysis failed: {error}", file=sys.stderr)
        return 1
    finally:
        (output / "summary.json").write_text(json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
