"""Build-time runtime identities and pinned dependency rejection in an isolated checkout."""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from autosar_tooling.config import ROOT


class AssetBuildTests(unittest.TestCase):
    def test_runtime_edit_builds_and_generates_but_dependency_edits_fail(self):
        with tempfile.TemporaryDirectory(prefix="autosar-asset-build-") as directory:
            root = Path(directory)
            for name in ("core", "runtime", "third_party", "tools", "src-tauri"):
                shutil.copytree(ROOT / name, root / name, ignore=shutil.ignore_patterns(
                    "__pycache__", "target", "node_modules"))
            for name in ("Cargo.toml", "Cargo.lock", "LICENSE", "NOTICE"):
                shutil.copyfile(ROOT / name, root / name)
            env = {**os.environ, "CARGO_TARGET_DIR": str(root / "target"),
                   "AUTOSAR_C_ANALYSIS_SAMPLES": str(root / "samples")}
            manifest = root / "runtime/contracts/assets.json"
            before = manifest.read_bytes()
            runtime = root / "runtime/bsw/src/Det.c"
            runtime.write_bytes(runtime.read_bytes() + b"\n/* Build-time asset identity regression. */\n")
            command = ["cargo", "test", "--locked", "-p", "autosar-config-core", "--test", "it"]
            for selected in ("delivery::runtime_source_inventory", "c_analysis::"):
                result = subprocess.run([*command, "--", selected], cwd=root, env=env,
                                        capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(manifest.read_bytes(), before)
            for relative, diagnostic in (
                ("third_party/freertos/tasks.c", "asset digest differs"),
                ("third_party/freertos/portable/MSVC-MingW/port.c", "asset digest differs"),
                ("third_party/freertos/portable/ThirdParty/GCC/Posix/port.c", "asset digest differs"),
                ("runtime/os/patches/0001-controlled-host-lifecycle.patch", "controlled patch digest differs"),
            ):
                with self.subTest(path=relative):
                    path = root / relative
                    original = path.read_bytes()
                    path.write_bytes(original + b"\nchanged dependency\n")
                    try:
                        result = subprocess.run(["cargo", "check", "--locked", "-p", "autosar-config-core"], cwd=root, env=env,
                                                capture_output=True, text=True, check=False)
                        output = result.stdout + result.stderr
                        self.assertNotEqual(result.returncode, 0, output)
                        self.assertIn(diagnostic, output)
                        self.assertIn(relative, output)
                    finally:
                        path.write_bytes(original)
