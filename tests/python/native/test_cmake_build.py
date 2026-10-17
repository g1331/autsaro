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
    def test_cmake_refuses_one_byte_corruption_after_patching(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        self.assertTrue(samples, "Generate c_analysis samples first")
        with tempfile.TemporaryDirectory(prefix="autosar-cmake-corruption-") as temporary:
            directory = Path(temporary)
            project = directory / "package"
            shutil.copytree(Path(samples) / "standard-ecu", project)
            # Inject at the actual boundary between patching and verification.
            corrupt = directory / "corrupt.py"
            corrupt.write_text("import pathlib, sys\np = pathlib.Path(sys.argv[1])\n"
                               "b = p.read_bytes()\ni = b.index(b'FreeRTOS Kernel')\n"
                               "p.write_bytes(b[:i] + bytes([b[i] ^ 1]) + b[i + 1:])\n",
                               encoding="utf-8")
            cmake_file = project / "CMakeLists.txt"
            source = cmake_file.read_text(encoding="utf-8")
            anchor = "file(GLOB_RECURSE actual_files"
            self.assertEqual(source.count(anchor), 1)
            injection = (f'execute_process(COMMAND "{Path(sys.executable).as_posix()}" '
                         f'"{corrupt.as_posix()}" "${{AUTSARO_KERNEL}}/tasks.c" COMMAND_ERROR_IS_FATAL ANY)\n')
            cmake_file.write_text(source.replace(anchor, injection + anchor), encoding="utf-8", newline="\n")
            result = OwnedProcess(ProcessSpec.seconds(
                [tool("CMAKE", "cmake"), "-S", str(project), "-B", str(directory / "build"),
                 "-G", "Ninja", f'-DCMAKE_MAKE_PROGRAM={tool("NINJA", "ninja")}',
                 f'-DCMAKE_C_COMPILER={tool("AUTOSAR_CC", "gcc")}'],
                directory, 60, directory, "corrupt-configure")).wait()
            output = result.stdout.read_text() + result.stderr.read_text()
            self.assertEqual(result.status, "exited", output)
            self.assertNotEqual(result.exit_code, 0, output)
            self.assertIn("Kernel identity mismatch: kernel/tasks.c", output)

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
