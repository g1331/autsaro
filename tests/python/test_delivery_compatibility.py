"""The shipped CLI accepts supported Python versions without patch-level gates."""
import sys
import unittest
from unittest.mock import patch

from ecu_tools import build, cli


class DeliveryCompatibilityTests(unittest.TestCase):
    def invoke(self, version):
        arguments = ["ecu-tool.py", "build", "--project", "source", "--output", "output", "--mode", "host"]
        with patch.object(sys, "version_info", version), patch.object(sys, "argv", arguments), patch.object(build, "build") as run:
            result = cli.main()
            return result, run.call_count

    def test_supported_minors_and_patches_reach_the_build(self):
        for version in ((3, 11, 0), (3, 12, 0), (3, 12, 10), (3, 13, 0), (3, 14, 0)):
            with self.subTest(version=version):
                self.assertEqual(self.invoke(version), (0, 1))

    def test_older_python_is_rejected_before_building(self):
        with self.assertRaises(SystemExit) as failure:
            self.invoke((3, 10, 16))
        self.assertEqual(failure.exception.code, 2)
