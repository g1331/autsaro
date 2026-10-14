"""Generated C samples follow the repository .clang-format (#129)."""

import os
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
# Third-party FreeRTOS sources and user application sources keep their own style.
FOREIGN = {"kernel", "app"}


class GeneratedFormatTests(unittest.TestCase):
    def test_generated_samples_match_clang_format(self):
        samples = os.environ.get("AUTOSAR_C_ANALYSIS_SAMPLES")
        if not samples:
            self.fail(
                "Generate c_analysis samples and set AUTOSAR_C_ANALYSIS_SAMPLES first"
            )
        formatter = shutil.which("clang-format")
        if formatter is None:
            self.fail("clang-format is missing; run uv sync --locked")
        projects = sorted(p for p in Path(samples).iterdir() if p.is_dir())
        self.assertTrue(projects, "No generated samples")
        for project in projects:
            files = sorted(
                path.relative_to(project).as_posix()
                for path in project.rglob("*")
                if path.suffix in {".c", ".h"}
                and path.relative_to(project).parts[0] not in FOREIGN
            )
            self.assertIn("generated/src/Rte.c", files, project.name)
            result = subprocess.run(
                [
                    formatter,
                    f"--style=file:{ROOT / '.clang-format'}",
                    "--dry-run",
                    "--Werror",
                    *files,
                ],
                cwd=project,
                capture_output=True,
                text=True,
                check=False,
            )
            with self.subTest(project=project.name):
                self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
