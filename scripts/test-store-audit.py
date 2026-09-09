#!/usr/bin/env python3
"""Exercise the informational store report without accessing the Nix store."""

import json
from pathlib import Path
import subprocess
import sys
import unittest


SCRIPT = Path(__file__).with_name("store-audit.py")


class StoreAuditTests(unittest.TestCase):
    def report(self, data):
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--warn-if-source-over", "50"],
            input=json.dumps(data),
            text=True,
            capture_output=True,
            check=True,
        )
        return result.stdout, result.stderr

    def test_sorts_by_closure_not_nar_size(self):
        stdout, _ = self.report(
            {
                "/nix/store/first": {"closureSize": 200, "narSize": 1},
                "/nix/store/second": {"closureSize": 100, "narSize": 999},
            }
        )
        self.assertIn("200  /nix/store/first", stdout.splitlines()[0])

    def test_warns_for_oversized_named_sources(self):
        path = "/nix/store/" + "a" * 32 + "-workestrate-source"
        _, stderr = self.report({path: {"closureSize": 60_000_000}})
        self.assertIn(path, stderr)

    def test_missing_closure_size_is_not_reported_as_zero(self):
        stdout, _ = self.report({"/nix/store/example": {"narSize": 123}})
        self.assertIn("request --closure-size", stdout)
        self.assertNotIn("OK:", stdout)

    def test_invalid_entries_remain_nonblocking(self):
        for entry in [None, {}, {"closureSize": -1}, {"closureSize": True}]:
            with self.subTest(entry=entry):
                stdout, _ = self.report({"/nix/store/example": entry})
                self.assertIn("could not read", stdout)

    def test_array_output_explains_required_format(self):
        stdout, _ = self.report([])
        self.assertIn("request --json-format 1", stdout)


if __name__ == "__main__":
    unittest.main()
