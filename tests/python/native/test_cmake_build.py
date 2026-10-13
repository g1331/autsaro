"""The generated CMake build compiles the same closure as the sealed ecu-tool build."""
from __future__ import annotations

import json
import os
import shlex
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

from ecu_tools.build import check_object
from ecu_tools.process import OwnedProcess, ProcessSpec


def tool(variable: str, name: str) -> str:
    requested = os.environ.get(variable) or shutil.which(name)
    if not requested:
        raise unittest.SkipTest(f"{name} is required; set {variable} or add it to PATH")
    return str(Path(requested).resolve(strict=True))


class CMakeBuildTests(unittest.TestCase):
    def test_cmake_and_ecu_tool_build_the_same_translation_units(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        if not samples:
            self.fail("Generate c_analysis samples and set AUTOSAR_C_ANALYSIS_SAMPLES first")
        tools = {variable: tool(variable, name) for variable, name in
                 [("AUTOSAR_CC", "gcc"), ("AUTOSAR_OBJDUMP", "objdump"), ("AUTOSAR_GIT", "git")]}
        cmake, ninja = tool("CMAKE", "cmake"), tool("NINJA", "ninja")
        suffix = ".exe" if os.name == "nt" else ""
        with tempfile.TemporaryDirectory(prefix="autosar-cmake-") as temporary:
            directory = Path(temporary)
            env = {**os.environ, **tools}

            def execute(argv, stage, stdin=None):
                result = OwnedProcess(ProcessSpec.seconds(argv, directory, 300, directory, stage,
                                                          env=env, stdin_file=stdin)).wait()
                output = result.stdout.read_text(errors="replace") + result.stderr.read_text(errors="replace")
                self.assertTrue(result.success, f"{stage}: {output}")
                return result.stdout.read_text(errors="replace")

            project = directory / "package"
            shutil.copytree(Path(samples) / "user-application", project)
            seal = (project / "meta/files.sha256").read_bytes()
            target = json.loads((project / "target.json").read_text(encoding="utf-8"))
            build = directory / "cmake"
            execute([cmake, "-S", str(project), "-B", str(build), "-G", "Ninja", f"-DCMAKE_MAKE_PROGRAM={ninja}",
                     f"-DCMAKE_C_COMPILER={tools['AUTOSAR_CC']}", "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON"], "configure")
            execute([cmake, "--build", str(build)], "build")

            # Every compile command uses the target's flags and include paths; the
            # kernel paths name the build tree's patched copy, as in ecu-tool.
            def include(name: str) -> Path:
                return (build / name if name.startswith("kernel/") else project / name).resolve()

            expected_includes = [include(name) for name in target["includePaths"]]
            units = set()
            for command in json.loads((build / "compile_commands.json").read_text(encoding="utf-8")):
                argv = shlex.split(command["command"], posix=os.name != "nt")
                includes = [Path(value[2:]).resolve() for value in argv if value.startswith("-I")]
                flags = [value for value in argv[1:] if value.startswith("-") and value[:2] not in ("-I", "-o", "-c")]
                self.assertEqual(includes, expected_includes, command["file"])
                self.assertEqual(flags, target["compilerFlags"], command["file"])
                units.add(Path(command["file"]).resolve())
            expected_units = {(project / name).resolve() for name in [*target["sources"], *target["entries"].values()]}
            expected_units |= {(build / "kernel" / name).resolve() for name in target["kernelSources"]}
            self.assertEqual(units, expected_units)

            def behavior(output: str) -> list[str]:
                # STACK and lifecycle reports carry thread identities, addresses and timing-dependent traces.
                return [line for line in output.splitlines() if not line.startswith(("STACK ", "lifecycle="))]

            # The CMake binaries run like the ecu-tool binaries and keep the checked sections.
            stdin = directory / "batch.stdin"
            stdin.write_bytes(b"BEGIN 1\nCOMMIT\n")
            logs = directory / "logs"
            logs.mkdir(mode=0o700)
            for mode, name in [("host-batch", "ecu_host_batch"), ("probe", "ecu_probe")]:
                with self.subTest(mode=mode):
                    reference = directory / ("ecu-tool-" + mode)
                    execute([sys.executable, "-I", "-S", str(project / "tools/ecu-tool.py"), "build", "--project",
                             str(project), "--output", str(reference), "--mode", mode], "ecu-tool-" + mode)
                    binary = build / (name + suffix)
                    check_object(binary, Path(tools["AUTOSAR_OBJDUMP"]), target, project, logs)
                    batch = stdin if mode == "host-batch" else None
                    self.assertEqual(behavior(execute([str(binary)], "cmake-run-" + mode, batch)),
                                     behavior(execute([str(reference / (name + suffix))], "ecu-tool-run-" + mode, batch)))
            self.assertEqual((project / "meta/files.sha256").read_bytes(), seal)
            self.assertFalse((project / "kernel/.git").exists())
            execute([sys.executable, "-I", "-S", str(project / "tools/ecu-tool.py"), "build", "--project",
                     str(project), "--output", str(directory / "sealed-after"), "--mode", "probe"], "sealed-after")


if __name__ == "__main__":
    unittest.main()
