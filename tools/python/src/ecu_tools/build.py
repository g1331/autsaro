"""Build one pinned native target from a sealed, immutable source package."""
from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
import stat
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path, PurePosixPath

from ecu_tools.process import OwnedProcess, ProcessSpec

TARGETS = {"windows-x64-controlled-v1": "Windows", "linux-x64-controlled-v1": "Linux"}


def digest(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def safe_relative(value: str) -> bool:
    return bool(value and "\\" not in value and ":" not in value
                and not PurePosixPath(value).is_absolute()
                and all(part not in ("", ".", "..") for part in value.split("/")))


def refuse_links(path: Path) -> None:
    for entry in (path.absolute(), *path.absolute().parents):
        if entry.is_symlink() or (
                entry.exists() and getattr(entry.lstat(), "st_file_attributes", 0) & 0x400):
            raise ValueError(f"Links/reparse points are refused: {entry}")


def sealed_sources(project: Path) -> tuple[list[str], dict]:
    refuse_links(project)
    project = project.resolve(strict=True)
    names = (project / "files.list").read_text(encoding="utf-8").splitlines()
    if not names or names != sorted(set(names)) or any(not safe_relative(name) for name in names):
        raise ValueError("Invalid sealed source file list")
    if any(name in ("files.list", "files.sha256") for name in names):
        raise ValueError("Source file list cannot enumerate its own seal")
    records = (project / "files.sha256").read_text(encoding="utf-8").splitlines()
    expected = {}
    for record in records:
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", record)
        if not match or match[2] in expected:
            raise ValueError("Invalid or duplicate sealed source hash record")
        expected[match[2]] = match[1]
    if set(expected) != set(names) | {"files.list"}:
        raise ValueError("Sealed source hash closure differs")
    allowed = set(names) | {"files.list", "files.sha256"}
    for entry in project.rglob("*"):
        metadata = entry.lstat()
        relative = entry.relative_to(project).as_posix()
        if entry.is_symlink() or getattr(metadata, "st_file_attributes", 0) & 0x400:
            raise ValueError(f"Source links/reparse points are refused: {relative}")
        if stat.S_ISDIR(metadata.st_mode):
            if not any(name.startswith(relative + "/") for name in names):
                raise ValueError(f"Unlisted source directory: {relative}")
        elif not stat.S_ISREG(metadata.st_mode) or relative not in allowed:
            raise ValueError(f"Unlisted source file: {relative}")
    for name, identity in expected.items():
        if digest(project / name) != identity:
            raise ValueError(f"Source identity mismatch: {name}")
    target = json.loads((project / "target.json").read_text(encoding="utf-8"))
    if target.get("format") != "autosar-build-target-v1" or target.get("target") not in TARGETS:
        raise ValueError("Unsupported build target format or identity")
    return names, target


def tool_argument(path: Path) -> str:
    """GNU tools use DOS/UNC paths, not Win32 verbatim namespace prefixes."""
    value = path.as_posix()
    if os.name == "nt":
        if value.startswith("//?/UNC/"):
            return "//" + value[8:]
        if value.startswith("//?/"):
            return value[4:]
    return value


def target_configuration(names: list[str], target: dict) -> tuple[list[str], list[str], list[str]]:
    """Read the same validated C flags, translation units and includes for build/analysis."""
    flags = target.get("compilerFlags")
    allowed_flags = {"-std=c99", "-O1", "-g", "-Wall", "-Wextra", "-Werror",
                     "-pedantic", "-D_GNU_SOURCE", "-pthread"}
    if (not isinstance(flags, list) or "-std=c99" not in flags
            or any(not isinstance(flag, str) or flag not in allowed_flags for flag in flags)
            or len(flags) != len(set(flags))):
        raise ValueError("Pinned compiler flags are invalid or contain private test/unsafe options")
    sources = target.get("sources")
    if (not isinstance(sources, list) or not sources
            or any(not isinstance(name, str) or name not in names or not name.endswith(".c") for name in sources)
            or len(sources) != len(set(sources))):
        raise ValueError("Target's real C translation-unit inventory is absent or invalid")
    includes = target.get("includePaths")
    if (not isinstance(includes, list) or not includes
            or any(not isinstance(name, str) or (name != "." and not safe_relative(name)) for name in includes)):
        raise ValueError("Target's include-path inventory is absent or invalid")
    return list(flags), list(sources), list(includes)


def executable(name: str, variable: str) -> Path:
    requested = os.environ.get(variable)
    if not requested:
        raise ValueError(f"Missing {name}: set {variable} to its absolute executable path")
    path = Path(requested)
    if not path.is_absolute() or not path.is_file():
        raise ValueError(f"{variable} must name an existing absolute executable")
    return path.resolve(strict=True)


def query(argv: list[str], logs: Path, stage: str, *, stdin_file: Path | None = None,
          timeout: int = 180) -> str:
    result = OwnedProcess(ProcessSpec.seconds(argv, logs, timeout, logs, stage, stdin_file=stdin_file)).wait()
    stdout = result.stdout.read_text(encoding="utf-8", errors="replace")
    stderr = result.stderr.read_text(encoding="utf-8", errors="replace")
    if not result.success:
        raise RuntimeError(f"{stage}: status={result.status} exit={result.exit_code}\n{stdout}\n{stderr}\nlogs={logs}")
    return stdout


def pinned_tools(target: dict, logs: Path) -> tuple[Path, Path, Path]:
    cc = executable("gcc", "AUTOSAR_CC")
    objdump = executable("objdump", "AUTOSAR_OBJDUMP")
    git = executable("git", "AUTOSAR_GIT")
    lock = target["toolchain"]
    version = query([str(cc), "--version"], logs, "compiler-version", timeout=30).splitlines()[0]
    machine = query([str(cc), "-dumpmachine"], logs, "compiler-triple", timeout=30).strip()
    if (version.partition(" ")[2] != lock["identity"].partition(" ")[2]
            or machine != lock["target"] or digest(cc) != lock["executable_sha256"]):
        raise ValueError(f"Compiler identity differs from the fixed target: {version}; {machine}")
    if lock.get("objdump_sha256") and digest(objdump) != lock["objdump_sha256"]:
        raise ValueError("Native objdump identity differs from the fixed target")
    query([str(objdump), "--version"], logs, "objdump-version", timeout=30)
    query([str(git), "--version"], logs, "git-version", timeout=30)
    return cc, objdump, git


def check_object(binary: Path, objdump: Path, target: dict, project: Path, logs: Path) -> None:
    identity = query([str(objdump), "-f", tool_argument(binary)], logs, "object-format", timeout=30)
    linux = target["target"] == "linux-x64-controlled-v1"
    if ("elf64-x86-64" if linux else "pei-x86-64") not in identity:
        raise ValueError("Native object format differs from the selected target")
    symbols = query([str(objdump), "-t", tool_argument(binary)], logs, "object-symbols", timeout=30)
    sections = query([str(objdump), "-h", tool_argument(binary)], logs, "object-sections", timeout=30)
    if not linux and "__emutls" in symbols:
        raise ValueError("Physical stack faults require native PE TLS")
    if target["profile"] != "ecu":
        return
    section_map = {}
    for match in re.finditer(r"^\s*(\d+)\s+(\.[\w]+)\s+([0-9a-fA-F]+)[^\n]*\n([^\n]+)", sections, re.MULTILINE):
        section_map[match[2]] = (int(match[1]) + 1, int(match[3], 16), match[4])
    if any(name not in section_map for name in target["requiredSections"]):
        raise ValueError("A declared native target section is missing")
    for name, wanted in ((".os_vec", "DATA"), (".os_code", "CODE"), (".rte_code", "CODE")):
        if name not in section_map:
            raise ValueError(f"Native section is missing: {name}")
        _, size, flags = section_map[name]
        if size == 0 or wanted not in flags or "ALLOC" not in flags:
            raise ValueError(f"Native section has wrong size/flags: {name}")
        if (name == ".os_vec" and (size != 256 or "READONLY" in flags)) or (
                name != ".os_vec" and "READONLY" not in flags):
            raise ValueError(f"Native section access invariant differs: {name}")

    def owns(symbol: str, section: str) -> bool:
        if linux:
            return len(re.findall(r"^[0-9a-fA-F]+\s+g\s+\w+\s+" + re.escape(section)
                                  + r"\s+[0-9a-fA-F]+\s+" + re.escape(symbol) + r"$", symbols, re.MULTILINE)) == 1
        matches = re.findall(r"\(sec\s+(\d+)\)[^\n]*\s" + re.escape(symbol) + r"$", symbols, re.MULTILINE)
        return len(matches) == 1 and int(matches[0]) == section_map[section][0]

    if not owns("Os_InterruptVectorTable", ".os_vec"):
        raise ValueError("Interrupt vector symbol is not in .os_vec")
    for symbol in ("ErrorHook", "PreTaskHook", "PostTaskHook", "StartupHook", "ShutdownHook"):
        if not owns(symbol, ".os_code"):
            raise ValueError(f"OS hook is outside its code section: {symbol}")
    task_names = set(re.findall(r"\b(Os_TaskEntry_[A-Za-z0-9_]+)$", symbols, re.MULTILINE))
    if len(task_names) != 1 or not all(owns(symbol, ".os_code") for symbol in task_names):
        raise ValueError("Generated task is outside its code section")
    tree = ET.parse(project / "descriptions/Host_Implementation.arxml")
    ns = {"ar": "http://autosar.org/schema/r4.0"}
    owners = [node for node in tree.findall(".//ar:BSW-MODULE-DESCRIPTION", ns)
              if node.findtext("ar:SHORT-NAME", namespaces=ns) == "Rte"]
    entries = [reference.text.rsplit("/", 1)[-1] for owner in owners
               for reference in owner.findall("ar:IMPLEMENTED-ENTRYS/ar:BSW-MODULE-ENTRY-REF-CONDITIONAL/ar:BSW-MODULE-ENTRY-REF", ns)]
    if not entries or any(not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name) or not owns(name, ".rte_code") for name in entries):
        raise ValueError("Described RTE entries are outside their code section")


def build(project: Path, output: Path, mode: str, control_source: Path | None = None) -> Path:
    names, target = sealed_sources(project)
    project = project.resolve(strict=True)
    linux = target["target"] == "linux-x64-controlled-v1"
    if platform.system() != TARGETS[target["target"]] or platform.machine().lower() not in ("x86_64", "amd64"):
        raise ValueError("Native execution requires the selected target's original host")
    profile = target.get("profile")
    if profile not in ("ecu", "host") or (profile == "host") != (mode == "host"):
        raise ValueError("Build mode and sealed source profile differ")
    if mode not in ("host", "probe", "test", "host-batch") or (control_source is not None and mode not in ("probe", "test")):
        raise ValueError("Production HostBatch/legacy mode refuses test or external control sources")
    output = output.absolute()
    refuse_links(output)
    output = output.resolve()
    if output == project or output.is_relative_to(project) or project.is_relative_to(output):
        raise ValueError("Build output must be separate from the sealed source project")
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise ValueError("Build output must be new or empty; owner files are preserved")
    if control_source:
        refuse_links(control_source)
        control_source = control_source.resolve(strict=True)
        if not control_source.is_file() or control_source.is_relative_to(project):
            raise ValueError("Independent control source must be a regular file outside the sealed project")
    flags, source_map, include_paths = target_configuration(names, target)
    output.mkdir(parents=True, exist_ok=True)
    logs = output / "logs"
    logs.mkdir(mode=0o700)
    cc, objdump, git = pinned_tools(target, logs)
    kernel = output / "kernel"
    if profile == "ecu":
        shutil.copytree(project / "kernel", kernel)
        for patch in target["kernelPatches"]:
            relative = patch["path"].removeprefix("runtime/")
            if relative not in names:
                raise ValueError(f"Pinned patch is missing: {relative}")
            patch_flags = ["--include=" + name for name in patch["includes"]]
            for checked in (True, False):
                argv = [str(git), "apply", "--ignore-space-change", *patch_flags,
                        *(["--check"] if checked else []), tool_argument(project / relative)]
                result = OwnedProcess(ProcessSpec.seconds(argv, kernel, 30, logs, "kernel-patch")).wait()
                if not result.success:
                    raise RuntimeError(f"Kernel patch failed: {relative}; logs={logs}")
    sources = [tool_argument(project / name) for name in source_map]
    if profile == "ecu":
        kernel_sources = target.get("kernelSources")
        if not kernel_sources or any(not safe_relative(name) or not (kernel / name).is_file() for name in kernel_sources):
            raise ValueError("Pinned kernel translation units are missing")
        sources += [tool_argument(kernel / name) for name in kernel_sources]
        entry = control_source if control_source else project / ("src/ecu_host_batch.c" if mode == "host-batch" else "src/ecu_probe.c")
        if entry != control_source and entry.relative_to(project).as_posix() not in names:
            raise ValueError("Selected native main entry is not sealed")
        sources.append(tool_argument(entry))
    suffix = "" if linux else ".exe"
    filename = ("ecu_host" if profile == "host" else "ecu_host_batch" if mode == "host-batch" else "ecu_probe") + suffix
    private = Path(tempfile.mkdtemp(prefix="compile-", dir=output))
    staged = private / filename
    argv = [str(cc), *flags, f"-ffile-prefix-map={tool_argument(project)}=.", f"-ffile-prefix-map={tool_argument(output)}=build"]
    if mode == "test":
        argv += ["-DECU_TARGET_TESTS", "-DOS_HOST_FAILURE_TESTS"]
    for name in include_paths:
        include = kernel / name.removeprefix("kernel/") if name.startswith("kernel/") else project / name
        argv += ["-I", tool_argument(include)]
    argv += sources + ["-l" + name for name in target["linkLibraries"]] + ["-o", tool_argument(staged)]
    query(argv, logs, "native-c99-build")
    check_object(staged, objdump, target, project, logs)
    sealed_sources(project)
    binary = output / filename
    os.link(staged, binary)
    staged.unlink()
    private.rmdir()
    print(f"Built ECU native entry: {binary}")
    return binary
