#!/usr/bin/env python3
"""Exercise development-shell argument forwarding with an isolated Nix stub."""

import json
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest


JUSTFILE = Path(__file__).resolve().parent.parent / "justfile"


class ShellArgumentsTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="workestrate-shell-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        for name in ("just", "bash", "sh", "mkdir", "sha256sum", "cut"):
            executable = shutil.which(name)
            self.assertIsNotNone(executable, f"required fixture tool missing: {name}")
            (self.bin / name).symlink_to(executable)
        shutil.copyfile(JUSTFILE, self.root / "justfile")
        # Exercise the non-Git root fallback without inspecting any real checkout.
        self.write_tool("git", "#!/bin/sh\nexit 1\n")
        self.write_tool(
            "nix",
            f"#!{sys.executable}\n"
            "import json, os, pathlib, sys\n"
            "pathlib.Path(os.environ['NIX_ARGV_LOG']).write_text(json.dumps(sys.argv[1:]))\n"
            "sys.exit(int(os.environ.get('NIX_EXIT_CODE', '0')))\n",
        )
        self.env = {
            "PATH": str(self.bin),
            "HOME": str(self.root / "home with spaces"),
            "LC_ALL": "C",
            "NIX_ARGV_LOG": str(self.root / "nix-argv.json"),
        }

    def write_tool(self, name, text):
        path = self.bin / name
        path.write_text(text)
        path.chmod(0o755)

    def check_recipe(self, recipe, arguments, *, exit_code=0):
        result = subprocess.run(
            [str(self.bin / "just"), recipe, *arguments],
            cwd=self.root,
            env={**self.env, "NIX_EXIT_CODE": str(exit_code)},
            text=True,
            capture_output=True,
            timeout=10,
        )
        self.assertEqual(result.returncode, exit_code, result.stderr)
        forwarded = json.loads((self.root / "nix-argv.json").read_text())
        self.assertEqual(forwarded[:3], ["develop", "--override-input", "devenv-root"])
        prefix = "file+file://"
        self.assertTrue(forwarded[3].startswith(prefix))
        root_file = Path(forwarded[3].removeprefix(prefix))
        self.assertTrue(root_file.is_relative_to(self.env["HOME"]))
        self.assertEqual(root_file.read_bytes(), str(self.root).encode())
        expected = ([".#bootstrap"] if recipe == "bootstrap" else []) + arguments
        self.assertEqual(forwarded[4:], expected)

    def test_no_command_arguments(self):
        for recipe in ("shell", "bootstrap"):
            with self.subTest(recipe=recipe):
                self.check_recipe(recipe, [])

    def test_simple_command_arguments(self):
        for recipe in ("shell", "bootstrap"):
            with self.subTest(recipe=recipe):
                self.check_recipe(recipe, ["-c", "rustc", "--version"])

    def test_shell_text_remains_literal_and_never_runs_on_host(self):
        marker = self.root / "must not be created"
        arguments = [
            "-c",
            "bash",
            "-c",
            f"printf '%s\\n' 'inside shell'; printf escaped > {shlex.quote(str(marker))}",
            "",
            "argument with spaces",
            '"double quotes" and \'single quotes\'',
            "$(printf expanded)",
            "`printf expanded`",
            "*",
        ]
        for recipe in ("shell", "bootstrap"):
            with self.subTest(recipe=recipe):
                self.check_recipe(recipe, arguments)
                self.assertFalse(marker.exists(), "command text escaped into the host shell")

    def test_nix_failure_is_propagated(self):
        for recipe in ("shell", "bootstrap"):
            with self.subTest(recipe=recipe):
                self.check_recipe(recipe, ["-c", "false"], exit_code=23)

    def test_beads_uses_pinned_app_and_preserves_arguments(self):
        arguments = ["create", "title with spaces", "--description", "$(do not expand)"]
        result = subprocess.run(
            [str(self.bin / "just"), "beads", *arguments],
            cwd=self.root,
            env=self.env,
            text=True,
            capture_output=True,
            timeout=10,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        forwarded = json.loads((self.root / "nix-argv.json").read_text())
        self.assertEqual(
            forwarded,
            [
                "run", "--no-update-lock-file", ".#beads", "--", "--sandbox",
                "-C", str(self.root), *arguments,
            ],
        )
        self.assertFalse((self.root / ".beads").exists())


if __name__ == "__main__":
    unittest.main()
