"""The delivered tool refuses a sealed package that is not exactly its seal, naming the entry."""
from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

from ecu_tools.build import sealed_sources

SOURCE = "bsw/src/Com.c"


def link_directory(link: Path, target: Path) -> None:
    """A directory symbolic link on POSIX, an NTFS junction on Windows."""
    if os.name == "nt":
        subprocess.run(["cmd", "/C", "mklink", "/J", str(link), str(target)],
                       check=True, capture_output=True)
    else:
        link.symlink_to(target, target_is_directory=True)


def seal(project: Path, names: list[str]) -> None:
    listing = "".join(name + "\n" for name in names)
    (project / "meta").mkdir(exist_ok=True)
    (project / "meta/files.list").write_text(listing, encoding="utf-8", newline="\n")
    records = [(name, (project / name).read_bytes()) for name in names if (project / name).is_file()]
    records.append(("meta/files.list", listing.encode()))
    (project / "meta/files.sha256").write_text(
        "".join(f"{hashlib.sha256(data).hexdigest()}  {name}\n" for name, data in records),
        encoding="utf-8", newline="\n")


class SealedSourceTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve(strict=True)
        self.project = self.root / "package"
        (self.project / "bsw/src").mkdir(parents=True)
        (self.project / SOURCE).write_bytes(b"/* sealed */\n")
        (self.project / "target.json").write_text(json.dumps(
            {"format": "autosar-build-target-v3", "target": "linux-x64-controlled-v1"}), encoding="utf-8")
        self.names = [SOURCE, "target.json"]
        seal(self.project, self.names)
        self.assertEqual(sealed_sources(self.project)[0], self.names)

    def refused(self, message: str) -> None:
        with self.assertRaisesRegex(ValueError, "^" + re.escape(message) + "$"):
            sealed_sources(self.project)

    def test_retired_build_manifest_is_refused(self):
        target = self.project / "target.json"
        target.write_text(json.dumps({"format": "autosar-build-target-v2", "target": "linux-x64-controlled-v1"}),
                          encoding="utf-8")
        seal(self.project, self.names)
        self.refused("Unsupported build target format or identity")

    def test_tampered_bytes_name_the_file(self):
        with (self.project / SOURCE).open("ab") as output:
            output.write(b"\n")
        self.refused(f"Source identity mismatch: {SOURCE}")

    def test_unlisted_file_and_directory_are_named(self):
        (self.project / "bsw/src/Extra.c").write_bytes(b"")
        self.refused("Unlisted source file: bsw/src/Extra.c")
        (self.project / "bsw/src/Extra.c").unlink()
        (self.project / "user").mkdir()
        self.refused("Unlisted source directory: user")

    def test_conflicting_listed_paths_are_refused(self):
        # Each vector is otherwise sorted, so only the named conflict refuses it.
        for names in (["../outside.c", SOURCE, "target.json"], [SOURCE, SOURCE, "target.json"],
                      ["target.json", SOURCE], [SOURCE, "bsw\\src\\Com.c", "target.json"]):
            with self.subTest(names=names):
                (self.project / "meta/files.list").write_text("".join(n + "\n" for n in names), encoding="utf-8")
                self.refused("Invalid sealed source file list")

    def test_file_link_inside_the_package_is_named(self):
        external = self.root / "external.c"
        external.write_bytes((self.project / SOURCE).read_bytes())
        (self.project / SOURCE).unlink()
        (self.project / SOURCE).symlink_to(external)
        self.refused(f"Source links/reparse points are refused: {SOURCE}")

    def test_directory_link_or_junction_inside_the_package_is_named(self):
        external = self.root / "external-bsw"
        (self.project / "bsw").rename(external)
        link_directory(self.project / "bsw", external)
        self.refused("Source links/reparse points are refused: bsw")

    def test_package_below_a_host_alias_is_accepted(self):
        alias = self.root / "alias"
        link_directory(alias, self.root)
        self.assertEqual(sealed_sources(alias / "package")[0], self.names)

    def test_package_reached_through_a_directory_link_or_junction_is_refused(self):
        alias = self.root / "alias"
        link_directory(alias, self.project)
        with self.assertRaisesRegex(ValueError, "^" + re.escape(f"Links/reparse points are refused: {alias}") + "$"):
            sealed_sources(alias)


if __name__ == "__main__":
    unittest.main()
