"""Run generated mainline packages as a receiver with a selected Python."""

import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from autosar_tooling.config import ROOT, environment

SAMPLES = ("standard-ecu", "user-application", "component-pipeline", "no-diagnostic")
# The pipeline's own requests and responses, independent of the package's verify oracle.
PIPELINE_TRACE = (
    b"BEGIN 0\nRX 800 4 78563412\nRX 1792 8 0322123400000000\nCOMMIT\n"
    b"BEGIN 1\nCOMMIT\nBEGIN 10\nRX 1792 8 0322123400000000\nCOMMIT\n"
    b"BEGIN 10\nCOMMIT\nBEGIN 20\nRX 1792 8 0322123400000000\nCOMMIT\n"
)
PIPELINE_OUTPUT = [
    ("1", "1800", "8", "0762123400000000"),
    ("10", "801", "4", "78563412"),
    ("10", "1800", "8", "0762123412345678"),
    ("20", "801", "4", "78563412"),
    ("20", "1800", "8", "0762123412345678"),
]


def receiver_environment(source: dict[str, str], receiver: str) -> dict[str, str]:
    """Only the pinned tools, the receiver interpreter and the system directories."""
    tools = {name: str(Path(source[name]).resolve(strict=True))
             for name in ("AUTOSAR_CC", "AUTOSAR_OBJDUMP", "AUTOSAR_GIT")}
    if os.name == "nt":
        system = Path(os.environ["SYSTEMROOT"])
        directories = [system / "System32", system]
        kept = ("SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT", "TEMP", "TMP")
    else:
        directories = [Path(entry) for entry in os.defpath.split(os.pathsep) if entry]
        kept = ("TMPDIR", "LANG")
    directories += [Path(path).parent for path in [*tools.values(), receiver]]
    env = {name: os.environ[name] for name in kept if name in os.environ}
    env["PATH"] = os.pathsep.join(dict.fromkeys(map(str, directories)))
    return {**env, **tools}


class OfflineDeliveryTests(unittest.TestCase):
    def test_moved_packages_build_and_run_with_receiver_python_and_pinned_tools_only(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        if not samples:
            self.fail(
                "Generate c_analysis samples and set AUTOSAR_C_ANALYSIS_SAMPLES first"
            )
        receiver = os.environ.get("AUTOSAR_RECEIVER_PYTHON", sys.executable)
        env = receiver_environment(environment(), receiver)
        for absent in ("cargo", "rustc", "node", "npm", "uv"):
            self.assertIsNone(shutil.which(absent, path=env["PATH"]), absent)
        origins = {str(path.resolve()) for path in (ROOT, Path(samples))}
        origins = {variant.encode() for origin in origins
                   for variant in (origin, origin.replace("\\", "/"))}
        suffix = ".exe" if os.name == "nt" else ""
        with tempfile.TemporaryDirectory(prefix="autosar-handoff-receiver-") as temporary:
            root = Path(temporary).resolve()
            for sample in SAMPLES:
                with self.subTest(sample=sample):
                    moved = root / sample / "package"
                    moved.parent.mkdir()
                    shutil.copytree(Path(samples) / sample, moved)
                    before = snapshot(moved)

                    def run(argv, stage, stdin=None, moved=moved):
                        result = subprocess.run(argv, cwd=moved.parent, env=env, input=stdin,
                                                capture_output=True, timeout=300, check=False)
                        observed = (result.stdout + result.stderr).decode("utf-8", "replace")
                        self.assertEqual(result.returncode, 0, f"{stage}: {observed}")
                        return observed

                    tool = [receiver, "-I", "-S", str(moved / "tools/ecu-tool.py")]
                    for mode in ("host-batch", "probe"):
                        run([*tool, "build", "--project", str(moved),
                             "--output", str(moved.parent / mode), "--mode", mode], "build " + mode)
                    probe = run([str(moved.parent / "probe" / ("ecu_probe" + suffix))], "probe")
                    self.assertIn("ecu_probe completed=20", probe)
                    batch = run([str(moved.parent / "host-batch" / ("ecu_host_batch" + suffix))],
                                "host-batch", PIPELINE_TRACE if sample == "component-pipeline"
                                else b"BEGIN 1\nCOMMIT\n")
                    self.assertIn("READY HostBatchV1", batch)
                    receipts = re.findall(r"^COMMIT_OK .*?\bepoch=(\d+)", batch, re.MULTILINE)
                    if sample == "component-pipeline":
                        self.assertEqual(receipts, ["0", "1", "10", "10", "20"], batch)
                        records = [tuple(dict(field.split("=", 1) for field in line.split()[1:])[key]
                                         for key in ("epoch", "id", "dlc", "data"))
                                   for line in batch.splitlines() if line.startswith("OUT ")]
                        self.assertEqual(records, PIPELINE_OUTPUT, batch)
                    else:
                        self.assertEqual(receipts, ["1"], batch)
                    if sample == "standard-ecu":
                        run([*tool, "verify", "--project", str(moved),
                             "--build-directory", str(moved.parent / "verify")], "verify")
                    self.assertEqual(snapshot(moved), before)
                    for path in (moved.parent).rglob("*"):
                        if path.is_file():
                            content = path.read_bytes()
                            for origin in origins:
                                self.assertNotIn(origin, content, path)


def snapshot(root: Path) -> dict[str, bytes]:
    return {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob("*") if p.is_file()}
