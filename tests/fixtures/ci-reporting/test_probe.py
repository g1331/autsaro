import unittest


class DiagnosticProbe(unittest.TestCase):
    def test_deliberate_reporting_failure(self):
        self.assertEqual(1, 2, "Issue 192 temporary failing-test probe")
