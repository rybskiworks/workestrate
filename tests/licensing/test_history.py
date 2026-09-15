# SPDX-FileCopyrightText: 2026 Georg Rybski
# SPDX-License-Identifier: Apache-2.0
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("history", Path(__file__).resolve().parents[2] / "scripts/licensing/audit_history.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class HistoryTests(unittest.TestCase):
    def test_deleted_document_is_reported_without_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            path = repo / "docs/nix/.crawl/example.md"
            path.parent.mkdir(parents=True)
            path.write_text("synthetic document, not upstream content")
            m.git(repo, "add", ".")
            identity = ["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false"]
            m.git(repo, *identity, "commit", "-qm", "add fixture")
            path.unlink()
            m.git(repo, "add", "-u")
            m.git(repo, *identity, "commit", "-qm", "remove fixture")
            before = m.git(repo, "show-ref")
            result = m.inventory(repo)
            self.assertTrue(any(p["path_hint"] == "docs/nix/.crawl/example.md" for p in result["objects"]))
            self.assertEqual(before, m.git(repo, "show-ref"))
            self.assertEqual("", m.git(repo, "status", "--porcelain"))
            self.assertFalse(path.exists())
            self.assertFalse(result["shallow"])


if __name__ == "__main__":
    unittest.main()
