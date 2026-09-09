#!/usr/bin/env python3
"""Ensure the repository gate builds checks without entering a runtime shell."""

import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


JUSTFILE = Path(__file__).resolve().parent.parent / "justfile"


class VerificationTests(unittest.TestCase):
    def run_gate(self, nix_status=0, git_status=0, recipe="verify"):
        with tempfile.TemporaryDirectory(prefix="workestrate-verify-") as directory:
            root = Path(directory)
            binary_dir = root / "bin"
            binary_dir.mkdir()
            for name in ("just", "bash", "sh", "python3"):
                executable = shutil.which(name)
                self.assertIsNotNone(executable, f"fixture tool missing: {name}")
                (binary_dir / name).symlink_to(executable)
            shutil.copyfile(JUSTFILE, root / "justfile")
            (root / "scripts").mkdir()
            shutil.copyfile(
                JUSTFILE.parent / "scripts/store-audit.py",
                root / "scripts/store-audit.py",
            )
            stub = (
                f"#!{sys.executable}\n"
                "import json, os, pathlib, sys\n"
                "name = pathlib.Path(sys.argv[0]).name\n"
                "with open(os.environ['CALL_LOG'], 'a') as log:\n"
                "    log.write(json.dumps([name, *sys.argv[1:]]) + '\\n')\n"
                "if name == 'nix' and sys.argv[1] == 'path-info':\n"
                "    print('{}')\n"
                "    sys.exit(0)\n"
                "sys.exit(int(os.environ[name.upper() + '_STATUS']))\n"
            )
            for name in ("nix", "git"):
                tool = binary_dir / name
                tool.write_text(stub)
                tool.chmod(0o755)
            log = root / "calls.jsonl"
            result = subprocess.run(
                [str(binary_dir / "just"), "--no-deps", recipe],
                cwd=root,
                env={
                    "PATH": str(binary_dir),
                    "HOME": str(root / "home"),
                    "CALL_LOG": str(log),
                    "NIX_STATUS": str(nix_status),
                    "GIT_STATUS": str(git_status),
                },
                capture_output=True,
                text=True,
                timeout=10,
            )
            calls = [json.loads(line) for line in log.read_text().splitlines()]
            self.assertFalse((root / "home").exists(), result.stderr)
            return result, calls

    def test_all_declared_repository_gates_are_built_without_develop(self):
        result, calls = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        build = calls[0]
        self.assertEqual(build[:4], ["nix", "build", "--no-link", "--no-update-lock-file"])
        self.assertEqual(
            set(build[4:]),
            {
                f".#checks.x86_64-linux.{name}"
                for name in (
                    "rust", "unit", "package", "pre-commit", "treefmt",
                    "tombiCheck", "schemaSync", "deny",
                )
            },
        )
        self.assertEqual(calls[1], ["git", "diff", "--exit-code", "HEAD", "--", "control/agentctl/Cargo.lock"])
        self.assertEqual(calls[2], ["nix", "path-info", "--all", "--json", "--json-format", "1", "--closure-size"])

    def test_failed_build_stops_the_gate(self):
        result, calls = self.run_gate(nix_status=23)
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertEqual(len(calls), 1)

    def test_standalone_dependency_check_uses_the_same_nix_gate(self):
        result, calls = self.run_gate(recipe="deny-check")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls, [[
            "nix", "build", "--no-link", "--no-update-lock-file",
            ".#checks.x86_64-linux.deny",
        ]])

    def test_lock_drift_stops_before_store_audit(self):
        result, calls = self.run_gate(git_status=17)
        self.assertEqual(result.returncode, 17, result.stderr)
        self.assertEqual(len(calls), 2)


if __name__ == "__main__":
    unittest.main()
