#!/usr/bin/env python3
"""Check fallback hook chaining and mandatory gates in isolated Git fixtures."""

from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("git-hooks") / "pre-commit.sh"


class PreCommitHookTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="workestrate-hook-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        for name in ("sh", "git", "basename", "dirname", "grep", "awk", "tr"):
            executable = shutil.which(name)
            self.assertIsNotNone(executable, f"required fixture tool missing: {name}")
            (self.bin / name).symlink_to(executable)
        self.env = {
            "PATH": str(self.bin),
            "HOME": str(self.root),
            "LC_ALL": "C",
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": "/dev/null",
            "PREK_LOG": str(self.root / "prek.log"),
            "RUSTFMT_LOG": str(self.root / "rustfmt.log"),
        }
        self.git("init", "--quiet")
        (self.root / ".pre-commit-config.yaml").write_text("repos: []\n")
        self.write_tool("prek", 'printf "%s\\n" "$@" > "$PREK_LOG"\nexit 23\n')

    def git(self, *args):
        return subprocess.run(
            [str(self.bin / "git"), *args],
            cwd=self.root,
            env=self.env,
            text=True,
            capture_output=True,
            check=True,
            timeout=10,
        )

    def write_tool(self, name, body):
        path = self.bin / name
        path.write_text("#!/bin/sh\n" + body)
        path.chmod(0o755)

    def stage(self, name, content=""):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        self.git("add", "--", name)

    def run_hook(self, *, legacy=None):
        env = dict(self.env)
        if legacy is not None:
            env["PRE_COMMIT_RUNNING_LEGACY"] = legacy
        return subprocess.run(
            [str(self.bin / "sh"), str(SCRIPT), "fixture-argument"],
            cwd=self.root,
            env=env,
            text=True,
            capture_output=True,
            timeout=10,
        )

    def test_direct_hook_delegates_and_propagates_failure(self):
        result = self.run_hook()
        self.assertEqual(result.returncode, 1, result.stderr)
        arguments = (self.root / "prek.log").read_text().splitlines()
        self.assertEqual(arguments[0], "hook-impl")
        self.assertEqual(arguments[-1], "fixture-argument")

    def test_empty_legacy_marker_still_delegates(self):
        result = self.run_hook(legacy="")
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertTrue((self.root / "prek.log").exists())

    def test_legacy_hook_does_not_delegate_back_to_runner(self):
        result = self.run_hook(legacy="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.root / "prek.log").exists())

    def test_legacy_hook_still_rejects_secret_paths(self):
        self.stage(".env")
        result = self.run_hook(legacy="1")
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("refusing secret material", result.stderr)
        self.assertFalse((self.root / "prek.log").exists())

    def test_legacy_hook_still_enforces_rust_formatting(self):
        self.stage("Cargo.toml", '[package]\nname = "fixture"\n')
        self.stage("src/main.rs", "fn main() {}\n")
        self.write_tool(
            "rustfmt", 'printf "%s\\n" "$@" > "$RUSTFMT_LOG"\nexit 24\n'
        )
        result = self.run_hook(legacy="1")
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("rustfmt check failed", result.stderr)
        self.assertEqual(
            (self.root / "rustfmt.log").read_text().splitlines(),
            ["--edition", "2024", "--check", "--", "src/main.rs"],
        )
        self.assertFalse((self.root / "prek.log").exists())


if __name__ == "__main__":
    unittest.main()
