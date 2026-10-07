"""Keep the application release metadata consistent across its existing packages."""
import json
import tomllib
import unittest

from autosar_tooling.config import ROOT


class VersionTests(unittest.TestCase):
    def test_release_version_is_consistent(self):
        versions = []
        for relative in ("core/Cargo.toml", "src-tauri/Cargo.toml", "pyproject.toml"):
            metadata = tomllib.loads((ROOT / relative).read_text(encoding="utf-8"))
            versions.append(metadata.get("package", metadata.get("project"))["version"])
        for relative in ("ui/package.json", "src-tauri/tauri.conf.json"):
            versions.append(json.loads((ROOT / relative).read_text(encoding="utf-8"))["version"])
        for relative, name in (
            ('core/Cargo.lock', 'autosar-config-core'),
            ('src-tauri/Cargo.lock', 'autosar-config-desktop'),
            ('uv.lock', 'autosar-tooling'),
        ):
            lock = tomllib.loads((ROOT / relative).read_text(encoding='utf-8'))
            versions.append(next(package['version'] for package in lock['package'] if package['name'] == name))
        npm_lock = json.loads((ROOT / 'ui/package-lock.json').read_text(encoding='utf-8'))
        versions.extend((npm_lock['version'], npm_lock['packages']['']['version']))
        self.assertEqual(len(set(versions)), 1, versions)
