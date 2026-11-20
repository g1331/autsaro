"""Master artifacts accelerate raw scanning only after complete input matching."""

import copy
import json
import os
import shutil
import stat
import sys
import tempfile
import unittest
import zipfile
from contextlib import ExitStack
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import test_c_comparison as comparison_tests
from autosar_tooling import c_baseline
from ecu_tools.build import digest


class BaselineReuseTests(unittest.TestCase):
    def test_cli_completion_is_separate_from_raw_diagnostics_and_preserves_errors(self):
        arguments = ["c_baseline", "--side", "current", "--target", "linux-x64-controlled-v1",
                     "--sample", "standard-ecu", "--project", "project",
                     "--output-directory", "result"]
        for raw_code in (0, 1):
            with self.subTest(raw_code=raw_code), patch.object(sys, "argv", arguments), \
                    patch.object(c_baseline, "analyze", return_value=raw_code) as analysis:
                with self.assertRaises(SystemExit) as completed:
                    c_baseline.main()
                self.assertEqual(completed.exception.code, 0)
                analysis.assert_called_once_with("current", "linux-x64-controlled-v1",
                                                 "standard-ecu", Path("project"), Path("result"))
        with patch.object(sys, "argv", arguments), \
                patch.object(c_baseline, "analyze", side_effect=ValueError("tool failure")), \
                self.assertRaisesRegex(ValueError, "tool failure"):
            c_baseline.main()

    def test_raw_receipt_refuses_acceptance_execution_error_on_both_sides(self):
        for status in ("error", None, "unknown"):
            with self.subTest(status=status), tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
                root = Path(temporary)
                summary, _project, candidate, compiler = self.fixture(root)
                self.patches(stack, root, summary, compiler)
                summary["projectAcceptance"]["status"] = status
                (candidate / "summary.json").write_text(json.dumps(summary))
                with self.assertRaisesRegex(ValueError, "project acceptance execution failed"):
                    c_baseline.record(candidate, "head", "windows-x64-controlled-v1", "standard-ecu")
                self.assertFalse((candidate / "analysis-identity.json").exists())
                self.assertFalse((candidate / "scope-input.json").exists())

    def fixture(self, root):
        helper = comparison_tests.CComparisonTests()
        summary, project, candidate, _baseline = helper.runtime_fixture(root / "fixture")
        summary["tool"]["addons"] = dict.fromkeys(c_baseline.c_check.ADDON_HASHES, "c" * 64)
        compiler = root / "gcc"
        compiler.write_bytes(b"controlled compiler")
        (root / "driver.py").write_bytes(b"current driver\n")
        (root / "sample.arxml").write_bytes(b"current sample\n")
        wrapper = root / c_baseline.WRAPPER
        wrapper.parent.mkdir(parents=True)
        wrapper.write_bytes(b"current wrapper\n")
        for path in (project, candidate / "project"):
            metadata = json.loads((path / "meta/integration.json").read_text())
            metadata["plan"].update(
                ruleSetIdentity={"release": "R24-11", "rulesVersion": "1.1.0", "sha256": "a" * 64},
                sources=[{"logicalPath": "sample.arxml", "rawSha256": "b" * 64}],
            )
            helper.seal(path, metadata)
        summary["sourceSealSha256"] = digest(project / "meta/files.sha256")
        summary["scope"]["diagnosticCompiler"] = {"path": str(compiler), "sha256": digest(compiler)}
        summary["projectAcceptance"] = {"status": "accepted", "inputIdentity": "historical"}
        (candidate / "summary.json").write_text(json.dumps(summary))
        return summary, project, candidate, compiler

    def patches(self, stack, root, summary, compiler):
        stack.enter_context(patch.object(c_baseline, "ROOT", root))
        stack.enter_context(patch.object(c_baseline, "git", side_effect=lambda *args:
                                        f"driver.py\0sample.arxml\0{c_baseline.WRAPPER}\0"
                                        if args[0] == "ls-files"
                                        else "parent"))
        stack.enter_context(patch.dict(os.environ, {
            "GITHUB_REPOSITORY": "owner/repo", "AUTOSAR_CC": str(compiler),
            "GITHUB_RUN_ID": "1", "GITHUB_RUN_ATTEMPT": "1",
        }))
        stack.enter_context(patch.object(c_baseline.c_acceptance, "_complete"))
        stack.enter_context(patch.object(c_baseline, "sealed_sources"))
        stack.enter_context(patch.object(c_baseline.c_check, "tool_identity",
                                        return_value=(compiler, root, summary["tool"])))
        stack.enter_context(patch.object(c_baseline.c_check, "coverage",
                                        return_value=summary["coverage"]))
        stack.enter_context(patch.object(c_baseline, "compiler_identity", side_effect=lambda path:
                                        {"driver": digest(path), "backend": "b" * 64}))
        semantic = stack.enter_context(patch.object(c_baseline.c_check, "recheck_semantic_inputs"))
        acceptance = stack.enter_context(patch.object(
            c_baseline.c_acceptance, "evaluate", return_value={
                "status": "blocked", "inputIdentity": "fresh-current-inputs",
            }))
        return semantic, acceptance

    def test_matching_identity_reuses_raw_scan_and_recomputes_acceptance(self):
        with tempfile.TemporaryDirectory(dir=tempfile.gettempdir()) as temporary, \
                ExitStack() as stack:
            root = Path(temporary).resolve()
            summary, project, candidate, compiler = self.fixture(root)
            semantic, acceptance = self.patches(stack, root, summary, compiler)
            c_baseline.record(candidate, "parent", "linux", "standard-ecu")
            proof = candidate / "identifier-evidence/main.c.json.gz"
            proof.parent.mkdir()
            proof.write_bytes(b"historical proof")
            receipt = c_baseline.read(candidate / "analysis-identity.json")
            receipt["identifierEvidence"] = {"main.c": digest(proof)}
            (candidate / "analysis-identity.json").write_text(json.dumps(receipt))
            archive = root / "artifact.zip"
            with zipfile.ZipFile(archive, "w") as package:
                for path in candidate.rglob("*"):
                    if path.is_file():
                        package.write(path,
                                      "standard-ecu/" + path.relative_to(candidate).as_posix())
            provenance = {"kind": "master-artifact", "tree": "parent", "run": 1,
                          "attempt": 1, "job": 3, "artifact": 2,
                          "digest": "sha256:" + digest(archive)}

            def api(_repo, path):
                if "workflows/" in path:
                    return {"workflow_runs": [{"id": 1, "event": "push", "head_branch": "master",
                                               "head_sha": "parent", "run_attempt": 1}]}
                if "/jobs?" in path:
                    return {"jobs": [{"id": 3, "conclusion": "success", "name":
                                      "c-analysis-samples (ubuntu-24.04, linux, "
                                      "standard-ecu, standard-ecu)"}]}
                return {"artifacts": [{"id": 2, "expired": False, "name":
                                       "c-analysis-raw-current-linux-standard-ecu-0",
                                       "digest": provenance["digest"]}]}

            stack.enter_context(patch.object(c_baseline, "api", side_effect=api))
            stack.enter_context(patch.object(c_baseline, "gh",
                                            return_value=SimpleNamespace(stdout=archive)))
            os.environ["GITHUB_RUN_ID"] = "9"
            os.environ["GITHUB_RUN_ATTEMPT"] = "3"
            scan = stack.enter_context(patch.object(c_baseline.c_check, "check"))
            output = root / "result"
            self.assertEqual(c_baseline.analyze(
                "baseline", "linux", "standard-ecu", project, output), 1)
            scan.assert_not_called()
            semantic.assert_called_once()
            acceptance.assert_called_once()
            result = json.loads((output / "summary.json").read_text())
            self.assertEqual(result["projectAcceptance"]["inputIdentity"], "fresh-current-inputs")
            self.assertEqual(result["projectAcceptance"]["status"], "blocked")
            self.assertEqual(result["diagnostics"], summary["diagnostics"])
            self.assertFalse(result["passed"])
            self.assertEqual(json.loads((output / "baseline-source.json").read_text()), provenance)
            manifest = json.loads((output / "analysis-identity.json").read_text())
            self.assertEqual((manifest["run"], manifest["attempt"]), (9, 3))
            self.assertEqual(manifest["summarySha256"], digest(output / "summary.json"))
            self.assertNotIn("identifierEvidence", manifest)
            self.assertFalse((output / "identifier-evidence").exists())

    def test_each_identity_field_mismatch_and_missing_artifact_execute_fresh_scan(self):
        with tempfile.TemporaryDirectory(dir=tempfile.gettempdir()) as temporary, \
                ExitStack() as stack:
            root = Path(temporary).resolve()
            summary, project, candidate, compiler = self.fixture(root)
            self.patches(stack, root, summary, compiler)
            c_baseline.record(candidate, "parent", "linux", "standard-ecu")
            original = json.loads((candidate / "analysis-identity.json").read_text())
            self.assertEqual(set(original["identity"]), {
                "tree", "target", "sample", "analyzer", "rules", "generator", "configuration",
                "inputs", "sourceSeal", "compiler", "sdk", "runner", "python",
            })
            self.assertEqual(set(original["identity"]["analyzer"]), {
                "version", "expectedSourceRevision", "binarySha256", "addons",
            })
            self.assertEqual(set(original["identity"]["analyzer"]["addons"]),
                             set(c_baseline.c_check.ADDON_HASHES))
            self.assertEqual(set(original["identity"]["compiler"]), {"driver", "backend"})
            self.assertEqual(set(original["identity"]["python"]),
                             {"implementation", "version", "binary"})
            self.assertEqual(set(original["identity"]["runner"]), {"ImageOS", "ImageVersion"})
            self.assertLessEqual({
                "tools/python/src", "core/tests", "core/resources", "Cargo.toml", "Cargo.lock",
                "core/Cargo.toml", "src-tauri/Cargo.toml",
                "pyproject.toml", "uv.lock", ".python-version", "rust-toolchain.toml", ".cargo",
                "docs/standards", "docs/adr", "docs/development/c-coding-standard.md",
                ".github/actions/native-analysis-compiler",
                ".github/actions/core-build-dependencies",
                ".agents/skills/misra-c2012/references/baseline-index.json",
            }, set(c_baseline.INPUTS))
            fields = [*original["identity"], "missing", "producer_attempt"]
            for field in fields:
                with self.subTest(field=field):
                    manifest = copy.deepcopy(original)
                    if field == "producer_attempt":
                        manifest["attempt"] = 2
                    elif field != "missing":
                        manifest["identity"][field] = "different"
                    (candidate / "analysis-identity.json").write_text(json.dumps(manifest))

                    def download(_repo, _tree, _target, sample, directory, field=field):
                        if field == "missing":
                            return None
                        shutil.copytree(candidate, directory / sample)
                        return {"kind": "master-artifact", "tree": "parent", "run": 1, "attempt": 1}

                    output = root / ("fresh-" + field)

                    def fresh(actual_project, target, result, output=output, **kwargs):
                        self.assertEqual((actual_project, target, result),
                                         (project, "linux", output))
                        result.mkdir()
                        (result / "summary.json").write_text(json.dumps(summary))
                        return 1

                    with patch.object(c_baseline, "download", side_effect=download), \
                            patch.object(c_baseline.c_check, "check", side_effect=fresh) as scan, \
                            patch.object(c_baseline, "record"):
                        self.assertEqual(c_baseline.analyze(
                            "baseline", "linux", "standard-ecu", project, output), 1)
                        scan.assert_called_once()
                        self.assertEqual(scan.call_args.kwargs, {"sdk_revision": "parent"})
                    source = json.loads((output / "baseline-source.json").read_text())
                    self.assertEqual(source["kind"], "same-run")
                    self.assertEqual(source["tree"], "parent")
                    if field == "missing":
                        self.assertEqual(source["reason"], "artifact_missing")
                    elif field == "producer_attempt":
                        self.assertIn("different run attempt", source["reason"])
                    else:
                        self.assertIn("identity differs: " + field, source["reason"])

    def test_legacy_wrapper_matches_only_exact_transition_and_other_inputs(self):
        wrapper_bytes = Path(c_baseline.__file__).read_bytes().replace(b"\r\n", b"\n")
        revisions = {
            b"\n": "86d510532958d716a7e9d821e726c535793b93c14ee9434f5166fcaa7916c969",
            b"\r\n": "6a0d29b1154f687be2cef02355fbf8a2a94e737ee652ddf1ce6fa268a180205a",
        }
        for eol, preceding_hash in revisions.items():
            with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
                root = Path(temporary).resolve()
                summary, project, candidate, compiler = self.fixture(root)
                semantic, acceptance = self.patches(stack, root, summary, compiler)
                wrapper = root / c_baseline.WRAPPER
                selector = root / c_baseline.JOB_SELECTOR
                current = wrapper_bytes.replace(b"\n", eol)
                wrapper.write_bytes(current)
                selector.write_bytes(b"Updated job labels")
                tracked = ("driver.py", "sample.arxml", c_baseline.WRAPPER, c_baseline.JOB_SELECTOR)
                c_baseline.git.side_effect = lambda *args, tracked=tracked: (
                    "\0".join(tracked) if args[0] == "ls-files" else "parent")

                c_baseline.record(candidate, "parent", "linux", "standard-ecu")
                legacy = c_baseline.read(candidate / "analysis-identity.json")
                legacy["identity"] = c_baseline.identity(
                    candidate / "project", "parent", "linux", "standard-ecu", summary["tool"],
                    c_baseline.compiler_identity(compiler), summary["semanticIdentity"]["sdk"],
                    previous=preceding_hash)
                (candidate / "analysis-identity.json").write_text(json.dumps(legacy))

                def download(_repo, _tree, _target, sample, directory,
                             root=root, candidate=candidate):
                    self.assertEqual(directory.parent, root / "analysis")
                    shutil.copytree(candidate, directory / sample)
                    return {"kind": "master-artifact", "tree": "parent", "run": 1, "attempt": 1}

                for changed in ("selector", "driver", "wrapper"):
                    with self.subTest(eol=eol, changed=changed):
                        wrapper.write_bytes(current + (b"# changed" if changed == "wrapper" else b""))
                        (root / "driver.py").write_bytes(
                            b"changed driver" if changed == "driver" else b"current driver\n")
                        output = root / "analysis" / changed
                        semantic.reset_mock()
                        acceptance.reset_mock()

                        def fresh(_project, _target, result, candidate=candidate, **_kwargs):
                            shutil.copytree(candidate, result)
                            return 1

                        with patch.object(c_baseline, "download", side_effect=download), \
                                patch.object(c_baseline.c_check, "check", side_effect=fresh) as scan:
                            c_baseline.analyze("baseline", "linux", "standard-ecu", project, output)
                            source = c_baseline.read(output / "baseline-source.json")
                            if changed == "selector":
                                scan.assert_not_called()
                                semantic.assert_called_once()
                                acceptance.assert_called_once()
                                self.assertEqual(source["kind"], "master-artifact")
                            else:
                                scan.assert_called_once()
                                self.assertEqual(source["kind"], "same-run")
                                self.assertIn("identity differs: inputs", source["reason"])

    def test_sdk_or_current_checker_changes_refuse_cached_analysis(self):
        for changed in ("sdk", "coverage", "driver", "configuration", "source",
                        "compiler", "analyzer", "receipt", "summary"):
            with self.subTest(changed=changed), \
                    tempfile.TemporaryDirectory(dir=tempfile.gettempdir()) as temporary, \
                    ExitStack() as stack:
                root = Path(temporary).resolve()
                summary, project, candidate, compiler = self.fixture(root)
                semantic, _acceptance = self.patches(stack, root, summary, compiler)
                c_baseline.record(candidate, "parent", "linux", "standard-ecu")
                if changed == "sdk":
                    semantic.side_effect = ValueError("actual SDK changed")
                elif changed == "coverage":
                    stack.enter_context(patch.object(c_baseline.c_check, "coverage",
                                                    return_value={}))
                elif changed == "driver":
                    (root / "driver.py").write_bytes(b"changed driver")
                elif changed == "configuration":
                    (project / "target.json").write_text('{"target":"changed"}')
                elif changed == "source":
                    (project / "main.c").write_text("different generated source\n")
                    metadata = json.loads((project / "meta/integration.json").read_text())
                    metadata["plan"]["runtimeSources"]["runtime/bsw/src/Example.c"] = digest(
                        project / "main.c")
                    comparison_tests.CComparisonTests().seal(project, metadata)
                elif changed == "compiler":
                    compiler.write_bytes(b"changed compiler driver")
                elif changed == "analyzer":
                    tool = {**summary["tool"], "binarySha256": "different"}
                    stack.enter_context(patch.object(c_baseline.c_check, "tool_identity",
                                                    return_value=(compiler, root, tool)))
                elif changed == "receipt":
                    summary["diagnostics"].append(copy.deepcopy(summary["diagnostics"][0]))
                    (candidate / "summary.json").write_text(json.dumps(summary))
                else:
                    (candidate / "summary.json").write_text("{}")
                with self.assertRaises((ValueError, KeyError)):
                    c_baseline.restore(candidate, project, root / "output", "parent", "linux",
                                       "standard-ecu")
                self.assertFalse((root / "output").exists())

    def test_current_tree_always_scans_and_never_queries_baseline_artifacts(self):
        with tempfile.TemporaryDirectory(dir=tempfile.gettempdir()) as temporary, \
                ExitStack() as stack:
            root = Path(temporary).resolve()
            summary, project, candidate, compiler = self.fixture(root)
            self.patches(stack, root, summary, compiler)
            c_baseline.git.side_effect = lambda *args: (
                "driver.py\0sample.arxml\0" if args[0] == "ls-files" else
                "head" if args[-1] == "HEAD" else "parent")
            output = root / "current-analysis"
            def fresh(actual_project, target, result):
                self.assertEqual((actual_project, target, result), (project, "linux", output))
                shutil.copytree(candidate, result)
                return 1
            with patch.object(c_baseline, "download", side_effect=
                              AssertionError("Current tree must scan")) as cached, \
                    patch.object(c_baseline.c_check, "check", side_effect=fresh) as scan:
                self.assertEqual(c_baseline.analyze(
                    "current", "linux", "standard-ecu", project, output), 1)
                scan.assert_called_once()
                self.assertEqual(scan.call_args.kwargs, {})
                cached.assert_not_called()
            c_baseline.git.assert_any_call("rev-parse", "HEAD")
            self.assertEqual(json.loads((output / "baseline-source.json").read_text()),
                             {"kind": "same-run", "tree": "head"})
            self.assertNotIn("identifierEvidence", c_baseline.read(output / "analysis-identity.json"))
            self.assertFalse((output / "identifier-evidence").exists())

    def test_copied_runtime_rename_candidates_match_both_trees(self):
        origin, unrelated = "runtime/bsw/src/private.c", "runtime/bsw/src/other.c"
        delivered = "bsw/src/Private.c"
        old, new = "static int storage;\n", "static int private_storage;\n"
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            (project / "meta").mkdir()
            (project / "meta/integration.json").write_text(json.dumps({
                "originalBswSources": {origin: delivered, unrelated: "bsw/src/Other.c"},
            }))
            (project / delivered).parent.mkdir(parents=True)
            for side, source in (("baseline", old), ("current", new)):
                with self.subTest(side=side):
                    (project / delivered).write_bytes(source.replace("\n", "\r\n").encode())
                    (project / "bsw/src/Other.c").write_text("static int other = 1;\n")
                    with patch.object(c_baseline, "git", side_effect=[
                            f"{origin}\0{unrelated}\0", old.strip(), new.strip(),
                            "static int other = 0;", "static int other = 1;"]):
                        self.assertEqual(c_baseline.rename_candidates(project, side), {delivered})
            with patch.object(c_baseline, "git", return_value="") as git:
                self.assertEqual(c_baseline.rename_candidates(project / "absent", "current"), set())
                git.assert_called_once()

    def test_candidate_baseline_collects_proof_during_one_fresh_scan(self):
        with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
            root = Path(temporary).resolve()
            summary, project, candidate, compiler = self.fixture(root)
            self.patches(stack, root, summary, compiler)
            stack.enter_context(patch.object(c_baseline, "rename_candidates", return_value={"main.c"}))
            cached = stack.enter_context(patch.object(c_baseline, "download"))
            output = root / "baseline-analysis"

            def fresh(actual_project, target, result, **kwargs):
                self.assertEqual((actual_project, target, result), (project, "linux", output))
                self.assertEqual(kwargs, {"sdk_revision": "parent", "rename_units": {"main.c"}})
                shutil.copytree(candidate, result)
                return 1

            with patch.object(c_baseline.c_check, "check", side_effect=fresh) as scan:
                self.assertEqual(c_baseline.analyze("baseline", "linux", "standard-ecu", project, output), 1)
            scan.assert_called_once()
            cached.assert_not_called()
            self.assertEqual(c_baseline.read(output / "baseline-source.json")["reason"],
                             "identifier_candidates_require_evidence")

    def test_download_only_uses_unique_unexpired_successful_master_push_artifacts(self):
        good_run = {"id": 1, "event": "push", "head_branch": "master", "head_sha": "parent",
                    "conclusion": None, "run_attempt": 1}
        good_job = {"id": 3, "name": "c-analysis-samples (ubuntu-24.04, linux, "
                    "standard-ecu, standard-ecu)", "conclusion": "success"}
        good_artifact = {"id": 2, "name": "c-analysis-raw-current-linux-standard-ecu-0",
                         "expired": False, "digest": "sha256:" + "a" * 64}
        for changed in ("none", "paired", "other_pair", "event", "branch", "tree", "status", "duplicate",
                        "expired", "digest", "producer_missing", "producer_duplicate",
                        "producer_skipped"):
            with self.subTest(changed=changed), \
                    tempfile.TemporaryDirectory(dir=tempfile.gettempdir()) as temporary:
                run, artifact = copy.deepcopy(good_run), copy.deepcopy(good_artifact)
                job = copy.deepcopy(good_job)
                if changed in {"paired", "other_pair"}:
                    samples = ("standard-ecu + user-application" if changed == "paired"
                               else "component-pipeline + no-diagnostic")
                    job["name"] = f"c-analysis-samples (linux, {samples})"
                archive = Path(temporary) / "raw.zip"
                with zipfile.ZipFile(archive, "w") as package:
                    package.writestr("standard-ecu/raw.txt", b"raw analysis")
                artifact["digest"] = "sha256:" + digest(archive)
                if changed in {"event", "branch", "tree"}:
                    key = {"event": "event", "branch": "head_branch", "tree": "head_sha",
                           "status": "conclusion"}[changed]
                    run[key] = "wrong"
                elif changed in {"status", "producer_skipped"}:
                    job["conclusion"] = "failure" if changed == "status" else "skipped"
                elif changed == "expired":
                    artifact["expired"] = True
                elif changed == "digest":
                    artifact["digest"] = None
                artifacts = [artifact, artifact] if changed == "duplicate" else [artifact]
                jobs = ([] if changed == "producer_missing" else
                        [job, job] if changed == "producer_duplicate" else [job])
                def api(_repo, path, run=run, artifacts=artifacts, jobs=jobs):
                    if "workflows/" in path:
                        return {"workflow_runs": [run]}
                    return {"jobs": jobs} if "/jobs?" in path else {"artifacts": artifacts}
                with patch.object(c_baseline, "api", side_effect=api), \
                        patch.object(c_baseline, "gh", return_value=
                                     SimpleNamespace(stdout=archive)) as download:
                    result = c_baseline.download("owner/repo", "parent", "linux", "standard-ecu",
                                                 Path(temporary))
                    if changed in {"none", "paired"}:
                        self.assertEqual(result["artifact"], 2)
                        download.assert_called_once()
                        self.assertEqual(download.call_args.args[0],
                                         ["api", "repos/owner/repo/actions/artifacts/2/zip"])
                    else:
                        self.assertIsNone(result)
                        download.assert_not_called()

    def test_compiler_identity_includes_the_actual_backend_bytes(self):
        with tempfile.TemporaryDirectory(dir=tempfile.gettempdir()) as temporary:
            root = Path(temporary).resolve()
            compiler, backend = root / "gcc", root / "cc1"
            compiler.write_bytes(b"compiler driver")
            backend.write_bytes(b"old backend")
            with patch.object(c_baseline.subprocess, "check_output",
                              return_value=str(backend)) as query:
                before = c_baseline.compiler_identity(compiler)
                query.assert_called_once_with([str(compiler), "-print-prog-name=cc1"],
                                              text=True, timeout=30)
                backend.write_bytes(b"new backend")
                after = c_baseline.compiler_identity(compiler)
            self.assertEqual(before["driver"], after["driver"])
            self.assertNotEqual(before["backend"], after["backend"])

    def test_archive_digest_and_member_identity_are_verified_before_extraction(self):
        for changed in ("none", "digest", "traversal", "symlink", "duplicate", "sample"):
            with self.subTest(changed=changed), \
                    tempfile.TemporaryDirectory(dir=tempfile.gettempdir()) as temporary:
                root = Path(temporary).resolve()
                archive = root / "raw.zip"
                with zipfile.ZipFile(archive, "w") as package:
                    name = ("../escape.txt" if changed == "traversal" else
                            "other/raw.txt" if changed == "sample" else "standard-ecu/raw.txt")
                    if changed == "symlink":
                        info = zipfile.ZipInfo(name)
                        info.create_system = 3
                        info.external_attr = (stat.S_IFLNK | 0o777) << 16
                        package.writestr(info, "../escape")
                    else:
                        package.writestr(name, b"raw bytes")
                    if changed == "duplicate":
                        with self.assertWarns(UserWarning):
                            package.writestr(name, b"overwritten")
                expected = "sha256:" + ("a" * 64 if changed == "digest" else digest(archive))
                output = root / "extracted"
                if changed == "none":
                    c_baseline.extract_raw(archive, expected, output, "standard-ecu")
                    self.assertEqual((output / "standard-ecu/raw.txt").read_bytes(), b"raw bytes")
                else:
                    with self.assertRaisesRegex(ValueError, "digest differs|artifact member"):
                        c_baseline.extract_raw(archive, expected, output, "standard-ecu")
                    self.assertFalse(output.exists())
