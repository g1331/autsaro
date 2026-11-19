"""Reuse a private set of unchanged GCC objects during one native test shard."""

from __future__ import annotations

import hashlib
import json
import os
import runpy
import subprocess
import sys
import tempfile
from functools import cache
from pathlib import Path
from unittest.mock import patch


def content_key(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def cached_object(bucket, name, cc, flags, source, logs, dependency_name, digest):
    from autosar_tooling.c_sdk import dependency_paths

    bucket.mkdir(exist_ok=True)
    for candidate in bucket.iterdir():
        dependencies = json.loads(
            (candidate / "inputs.json").read_text(encoding="utf-8")
        )
        try:
            matches = all(
                digest(path) == expected for path, expected in dependencies.items()
            )
        except FileNotFoundError:
            # A quoted relative include can resolve outside the declared include directories.
            continue
        if matches:
            return candidate / "unit.o"

    with tempfile.TemporaryDirectory(dir=bucket.parent) as temporary:
        work = Path(temporary)
        deps = work / "unit.d"
        publish = work / "publish"
        publish.mkdir()
        subprocess.run(
            [
                cc,
                *flags,
                "-MMD",
                "-MT",
                "autsaro-tu",
                "-MF",
                str(deps),
                "-c",
                source,
                "-o",
                str(publish / "unit.o"),
            ],
            check=True,
            cwd=logs,
        )
        dependencies = {
            dependency_name(Path(path))
            for path in dependency_paths(deps.read_text(encoding="utf-8"))
        }
        if name not in dependencies:
            raise ValueError(f"GCC did not report the compiled source {source}")
        inputs = {path: digest(path) for path in dependencies}
        (publish / "inputs.json").write_text(json.dumps(inputs), encoding="utf-8")
        candidate = bucket / content_key(inputs)
        # A competing cold builder can publish the same complete object and manifest first.
        try:
            publish.rename(candidate)
        except OSError:
            if not candidate.is_dir():
                raise
        return candidate / "unit.o"


def compile_units(argv: list[str], logs: Path, fresh: list[str]) -> None:
    cc, *args = argv
    maps = [
        arg.removeprefix("-ffile-prefix-map=").rsplit("=", 1)
        for arg in args
        if arg.startswith("-ffile-prefix-map=")
    ]
    project, output = (Path(root) for root, _ in maps)
    target = json.loads((project / "target.json").read_text(encoding="utf-8"))
    end = args.index("-o") - len(target["linkLibraries"])
    first = end - len(target["sources"]) - len(target["kernelSources"]) - 1
    flags = args[:first]

    def dependency_name(path):
        if path.is_relative_to(project):
            return (Path("project") / path.relative_to(project)).as_posix()
        if path.is_relative_to(output):
            return (Path("build") / path.relative_to(output)).as_posix()
        return path.as_posix()

    @cache
    def digest(name):
        prefix, _, suffix = name.partition("/")
        path = {"project": project, "build": output}.get(prefix)
        actual = path / suffix if path is not None else Path(name)
        data = actual.read_bytes()
        value = hashlib.sha256(data)
        # Relative traversal and existence probes can see names beyond GCC's -I roots.
        logical_lines = data.replace(b"\\\r\n", b"").replace(b"\\\n", b"")
        if any(
            marker in logical_lines
            for marker in (b"../", b"..\\", b"__has_include", b"__has_embed")
        ):
            value.update(full_layout)
        return value.hexdigest()

    # The seal and patched-kernel checks have verified the file inventories. Include names
    # guard against a newly added header shadowing one that a previous build resolved.
    search_dirs = {Path(source).parent for source in args[first : end - 1]}
    normalized = []
    for index, arg in enumerate(flags):
        if arg.startswith("-ffile-prefix-map="):
            root, replacement = arg.removeprefix("-ffile-prefix-map=").rsplit("=", 1)
            arg = f"-ffile-prefix-map={dependency_name(Path(root))}={replacement}"
        elif index and flags[index - 1] == "-I":
            search_dirs.add(Path(arg))
            arg = dependency_name(Path(arg))
        normalized.append(arg)
    prefixes = tuple(dependency_name(path) + "/" for path in search_dirs)
    sealed_names = (
        (project / "meta/files.list").read_text(encoding="utf-8").splitlines()
    )
    full_layout = content_key(sealed_names).encode("ascii")
    inventory = [
        name for name in sealed_names if ("project/" + name).startswith(prefixes)
    ]
    context = content_key(
        [
            cc,
            target["toolchain"]["executable_sha256"],
            normalized,
            inventory,
            sorted(target["kernelSha256"]),
        ]
    )
    objects = Path(os.environ["AUTOSAR_TEST_OBJECTS"])
    if not objects.is_absolute():
        raise ValueError(
            "AUTOSAR_TEST_OBJECTS requires an absolute path to the job directory"
        )
    link = []
    for source in args[first : end - 1]:
        path = Path(source)
        if path.name in fresh:
            link.append(source)
        else:
            name = dependency_name(path)
            bucket = objects / content_key([context, name])
            link.append(
                str(
                    cached_object(
                        bucket, name, cc, flags, source, logs, dependency_name, digest
                    )
                )
            )
    # Keep each external control, main and explicitly varied TU in the real link command.
    subprocess.run([cc, *flags, *link, *args[end - 1 :]], check=True, cwd=logs)


def cached_query(query, fresh=()):
    def invoke(argv, logs, stage, **kwargs):
        if stage == "native-c99-build":
            plan = logs / "compilation.json"
            plan.write_text(json.dumps([argv, list(fresh)]), encoding="utf-8")
            argv = [sys.executable, "-I", "-S", "-B", __file__, "--compile", str(plan)]
        return query(argv, logs, stage, **kwargs)

    return invoke


def guarded_cli() -> None:
    # The public guard verifies the tool bytes and complete seal before invoking main.
    guard = sys.argv.pop(1)
    sys.argv[0] = guard
    load = runpy.run_path

    def standalone(path):
        module = load(path)
        main = module["main"]

        def invoke():
            from ecu_tools import build

            with patch.object(build, "query", cached_query(build.query)):
                return main()

        module["main"] = invoke
        return module

    with patch.object(runpy, "run_path", standalone):
        load(guard, run_name="__main__")


if __name__ == "__main__":
    if sys.argv[1] == "--compile":
        sys.path.insert(
            0, str(Path(__file__).resolve().parents[3] / "tools/python/src")
        )
        plan = Path(sys.argv[2])
        args, fresh = json.loads(plan.read_text(encoding="utf-8"))
        compile_units(args, plan.parent, fresh)
    else:
        guarded_cli()
