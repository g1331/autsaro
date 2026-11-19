"""Source provenance checks for the GCC coverage report."""

import json
import re
import sys
import tempfile
import unittest
from pathlib import Path
from subprocess import CompletedProcess
from unittest.mock import patch

from autosar_tooling import c_coverage
from autosar_tooling.c_coverage import ROOT, report_source


class CoverageSourceTests(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.scratch = Path(scratch.name)

    def copy_source(self, original: str, destination: str, suffix: bytes = b"") -> Path:
        path = self.scratch / destination
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes((ROOT / original).read_bytes() + suffix)
        return path

    def test_flat_bsw_and_host_copies_map_to_the_runtime(self):
        for original, relative, report in (
            ("runtime/bsw/src/Can.c", "src/Can.c", "runtime/bsw/Can/src/Can.c"),
            ("runtime/host/src/Can_HostLock.c", "src/Can_HostLock.c", "runtime/host/src/Can_HostLock.c"),
            ("runtime/bsw/include/Can.h", "include/Can.h", "runtime/bsw/Can/include/Can.h"),
        ):
            with self.subTest(original=original):
                path = self.copy_source(original, f"native/chain/{relative}")
                self.assertEqual(report_source(path, self.scratch), Path(report))

    def test_flat_runtime_names_do_not_admit_different_bytes(self):
        path = self.copy_source("runtime/bsw/src/Can.c", "native/chain/src/Can.c", b"\n")
        self.assertIsNone(report_source(path, self.scratch))

    def test_copies_outside_the_flat_layout_remain_excluded(self):
        for destination in (
            "native/chain/third_party/src/Can.c",
            "native/chain/mutation/src/Can.c",
            "native/chain/Can.c",
            "other/chain/src/Can.c",
        ):
            with self.subTest(destination=destination):
                path = self.copy_source("runtime/bsw/src/Can.c", destination)
                self.assertIsNone(report_source(path, self.scratch))

    def test_delivered_sources_still_require_matching_bytes(self):
        metadata = self.scratch / "native/delivery/meta/integration.json"
        metadata.parent.mkdir(parents=True)
        metadata.write_text('{"originalBswSources": {"runtime/bsw/src/Can.c": "bsw/src/Can.c"}}',
                            encoding="utf-8")
        path = self.copy_source("runtime/bsw/src/Can.c", "native/delivery/bsw/src/Can.c")
        self.assertEqual(report_source(path, self.scratch), Path("runtime/bsw/Can/src/Can.c"))
        with path.open("ab") as original:
            original.write(b"\n")
        self.assertIsNone(report_source(path, self.scratch))

    def test_generated_schm_uses_content_identity_and_excludes_the_mutation(self):
        directory = self.scratch / "native/schm"
        source = directory / "generated/src/SchM_Main.c"
        source.parent.mkdir(parents=True)
        source.write_text("void SchM_Init(void) {}\n", encoding="utf-8")
        first = report_source(source, self.scratch)
        self.assertIsNotNone(first)
        source.write_text("void SchM_Init(void) { return; }\n", encoding="utf-8")
        second = report_source(source, self.scratch)
        self.assertIsNotNone(second)
        self.assertNotEqual(first, second)
        mutation = directory / "schm-refusal.c"
        source.rename(mutation)
        self.assertIsNone(report_source(mutation, self.scratch))

    @unittest.skipUnless(sys.platform == "linux", "gcovr collection uses Linux paths")
    def test_render_collects_flat_copies_and_separates_branch_layouts(self):
        first = self.copy_source("runtime/bsw/src/Can.c", "native/chain-a/src/Can.c")
        second = self.copy_source("runtime/bsw/src/Can.c", "native/chain-b/src/Can.c")
        host_batch = self.copy_source(
            "runtime/ecu/src/Ecu_HostBatch.c", "native/delivery/integration/src/Ecu_HostBatch.c",
        )
        metadata = self.scratch / "native/delivery/meta/integration.json"
        metadata.parent.mkdir()
        metadata.write_text(json.dumps({"originalBswSources": {
            "runtime/ecu/src/Ecu_HostBatch.c": "integration/src/Ecu_HostBatch.c",
        }}), encoding="utf-8")
        generated = self.scratch / "native/delivery/generated/src/SchM_Main.c"
        generated.parent.mkdir(parents=True)
        generated.write_text("void SchM_Init(void) {}\n", encoding="utf-8")
        inputs = {
            str(self.scratch / "native/chain-a/consumer.gcda"): [
                (first, [1, 0]), (ROOT / "runtime/host/src/Can_HostLock.c", []),
                (ROOT / "runtime/os/src/Os.c", []),
            ],
            str(self.scratch / "native/chain-b/consumer.gcda"): [(second, [0, 1, 2])],
            str(self.scratch / "native/delivery/consumer.gcda"): [(host_batch, []), (generated, [])],
        }

        def collect(_scratch, root, source_filter, paths):
            entries = [
                {"file": source.relative_to(root).as_posix(), "lines": [{
                    "line_number": 677, "count": 1, "branches": [
                        {"branchno": number, "count": count} for number, count in enumerate(branches)
                    ],
                }]}
                for source, branches in inputs[paths[0]]
                if re.match(source_filter, str(source))
            ]
            return {"gcovr/format_version": "0.14", "files": entries}, []

        with (patch.object(c_coverage, "coverage_builds", return_value=[[path] for path in inputs]),
              patch.object(c_coverage, "collect", side_effect=collect),
              patch.object(c_coverage, "gcovr")):
            c_coverage.render(self.scratch, self.scratch)

        merged = json.loads((self.scratch / "merged.json").read_text(encoding="utf-8"))
        can = [entry for entry in merged["files"] if Path(entry["file"]).name == "Can.c"]
        self.assertEqual(len({entry["file"] for entry in can}), 2)
        self.assertEqual(
            [[branch["count"] for branch in entry["lines"][0]["branches"]] for entry in can],
            [[1, 0], [0, 1, 2]],
        )
        for entry in can:
            self.assertTrue(entry["file"].startswith("runtime/bsw/Can/src/Can/"))

    def test_default_entry_selects_schm_and_host_lock_fatal_tests(self):
        with patch.object(c_coverage.subprocess, "run", return_value=CompletedProcess([], 0)) as run:
            status = c_coverage.run_tests(self.scratch, self.scratch / "profiles", self.scratch / "errors")
        self.assertEqual(status, 0)
        cargo, _, host_lock = [call.args[0] for call in run.call_args_list]
        self.assertIn(
            "integration::generated_c::generated_schm_lifecycle_refuses_misordered_repeated_and_foreign_calls",
            cargo[cargo.index("--") + 1:],
        )
        self.assertEqual(host_lock[-4:], [
            "-p", "test_host_boundaries.py",
            "-k", "test_production_fatal_stops_without_signal_handler_or_crt_cleanup",
        ])
