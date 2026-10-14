"""Keep the required CI's ui/-only selection safe: the jobs it skips must never read ui/."""

import re
import subprocess
import unittest

from autosar_tooling.config import ROOT

WORKFLOW = ROOT / ".github/workflows/checks.yml"
# What the skipped jobs build, run or analyse. Tooling modules that no skipped job relies on
# for ui/ content are exempt: quality runs in full (--scope all) in the always-run quality job,
# and verify, doctor and the CLI argument lists are not run by these jobs.
INPUTS = ("core/", "runtime/", "tools/python/src/", "tests/python/native/", "tests/python/process/")
EXEMPT = {
    "tools/python/src/autosar_tooling/quality.py",
    "tools/python/src/autosar_tooling/verify.py",
    "tools/python/src/autosar_tooling/diagnostics.py",
    "tools/python/src/autosar_tooling/cli.py",
}
UI_PATH = re.compile(r"""["'/\\]ui[/\\"']""")


def jobs() -> dict[str, dict[str, object]]:
    found: dict[str, dict[str, object]] = {}
    current = None
    for line in WORKFLOW.read_text(encoding="utf-8").splitlines():
        if header := re.fullmatch(r"  ([a-z-]+):", line):
            current = found.setdefault(header.group(1), {"needs": set(), "if": ""})
        elif current is not None and (needs := re.fullmatch(r"    needs: \[?([^\]]+)\]?", line)):
            current["needs"] = {name.strip() for name in needs.group(1).split(",")}
        elif current is not None and (condition := re.fullmatch(r"    if: (.+)", line)):
            current["if"] = condition.group(1)
    return found


class CiSelectionTests(unittest.TestCase):
    def test_skipped_jobs_match_the_required_gate(self):
        workflow = jobs()
        skipped = {name for name, job in workflow.items() if "outputs.native" in job["if"]}
        # A job that needs a skipped job is skipped with it.
        while extra := {name for name, job in workflow.items() if job["needs"] & skipped} - skipped:
            skipped |= extra
        gate = re.search(r"UI_INDEPENDENT = \{([^}]+)\}", WORKFLOW.read_text(encoding="utf-8"))
        self.assertIsNotNone(gate)
        listed = set(re.findall(r"'([a-z-]+)'", gate.group(1)))
        self.assertEqual(listed, skipped & workflow["required"]["needs"])
        self.assertIn("core", skipped)

    def test_skipped_job_inputs_do_not_read_ui(self):
        tracked = subprocess.run(
            ["git", "ls-files", "-z", "--", *INPUTS], cwd=ROOT, capture_output=True, check=True
        ).stdout.decode("utf-8").split("\0")
        readers = []
        for path in filter(None, tracked):
            if path in EXEMPT or path.startswith("core/tests/fixtures/"):
                continue
            try:
                text = (ROOT / path).read_text(encoding="utf-8")
            except (UnicodeDecodeError, FileNotFoundError):
                continue
            readers.extend(f"{path}:{number}" for number, line in enumerate(text.splitlines(), 1) if UI_PATH.search(line))
        self.assertEqual(readers, [], "Jobs skipped for ui/-only pull requests read ui/; update checks.yml")


if __name__ == "__main__":
    unittest.main()
