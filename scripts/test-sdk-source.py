#!/usr/bin/env python3
"""Disposable SDK-link and recipe fixtures; no real Nix, Cargo, or VM calls."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts/sdk-source.sh"


class SdkSourceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="workestrate-sdk-source-")
        self.addCleanup(self.temporary.cleanup)
        self.outer = Path(self.temporary.name)
        self.root = self.outer / 'checkout with spaces; $(false)'
        self.root.mkdir()
        for name in ("flake.nix", "control/agentctl/Cargo.toml", "config.reference/workestrate.toml"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("fixture\n")
        self.source = self.workspace('SDK with spaces; `false`')
        self.vendor = self.root / "control/agentctl/vendor"
        self.link = self.vendor / "microsandbox-fork"
        self.env = {"PATH": os.environ["PATH"], "HOME": str(self.outer / "private home"), "LC_ALL": "C"}

    def workspace(self, name):
        root = self.outer / name
        for name in ("Cargo.toml", "sdk/rust/Cargo.toml", "crates/protocol/Cargo.toml"):
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("workspace fixture\n")
        return root

    def run_helper(self, *, root=None, source=None, arguments=(), success=True):
        result = subprocess.run(
            ["bash", str(SCRIPT), str(self.root if root is None else root),
             str(self.source if source is None else source), *arguments],
            env=self.env, cwd=self.outer, stdin=subprocess.DEVNULL,
            capture_output=True, text=True, timeout=5,
        )
        self.assertEqual(result.returncode == 0, success, result.stderr)
        return result

    def test_missing_link_selects_only_the_complete_source(self):
        result = self.run_helper()
        self.assertEqual(result.stdout, str(self.source) + "\n")
        self.assertEqual(self.link.resolve(), self.source)
        self.assertFalse(Path(self.env["HOME"]).exists())
        self.assertFalse((self.root / "target").exists())

    def test_correct_link_is_idempotent(self):
        self.run_helper()
        inode = self.link.lstat().st_ino
        self.run_helper()
        self.assertEqual(self.link.lstat().st_ino, inode)

    def test_old_link_refresh_preserves_old_workspace(self):
        old = self.workspace("old source")
        self.vendor.mkdir()
        self.link.symlink_to(old, target_is_directory=True)
        before = (old / "Cargo.toml").read_bytes()
        self.run_helper()
        self.assertEqual(self.link.resolve(), self.source)
        self.assertEqual((old / "Cargo.toml").read_bytes(), before)

    def test_broken_link_is_replaced_without_touching_its_target(self):
        self.vendor.mkdir()
        missing = self.outer / "missing source"
        self.link.symlink_to(missing)
        self.run_helper()
        self.assertEqual(self.link.resolve(), self.source)
        self.assertFalse(missing.exists())

    def test_source_and_checkout_aliases_are_canonicalized(self):
        alias = self.outer / "source alias"
        alias.symlink_to(self.source)
        checkout = self.outer / "checkout alias"
        checkout.symlink_to(self.root)
        self.run_helper(root=checkout, source=alias)
        self.assertEqual(os.readlink(self.link), str(self.source))

    def test_real_vendor_directory_is_preserved_and_refused(self):
        self.link.mkdir(parents=True)
        marker = self.link / "local edits"
        marker.write_text("do not replace")
        self.run_helper(success=False)
        self.assertFalse(self.link.is_symlink())
        self.assertEqual(marker.read_text(), "do not replace")

    def test_default_shell_can_explicitly_preserve_unlocked_directory(self):
        self.link.mkdir(parents=True)
        marker = self.link / "local edits"
        marker.write_text("preserved")
        result = self.run_helper(arguments=("--allow-unlocked",))
        self.assertEqual(result.stdout, "")
        self.assertIn("unlocked", result.stderr)
        self.assertEqual(marker.read_text(), "preserved")

    def test_regular_file_is_never_replaced_even_in_unlocked_mode(self):
        self.vendor.mkdir()
        self.link.write_text("not a link")
        for arguments in ((), ("--allow-unlocked",)):
            self.run_helper(arguments=arguments, success=False)
            self.assertEqual(self.link.read_text(), "not a link")

    def test_vendor_parent_alias_is_refused_without_external_writes(self):
        elsewhere = self.outer / "other project"
        elsewhere.mkdir()
        self.vendor.symlink_to(elsewhere)
        self.run_helper(success=False)
        self.assertEqual(list(elsewhere.iterdir()), [])

    def test_broken_vendor_parent_and_regular_parent_are_refused(self):
        self.vendor.symlink_to(self.outer / "missing parent")
        self.run_helper(success=False)
        self.vendor.unlink()
        self.vendor.write_text("parent file")
        self.run_helper(success=False)
        self.assertEqual(self.vendor.read_text(), "parent file")

    def test_relative_or_multiline_arguments_are_refused_before_writes(self):
        for root, source in (("relative", self.source), (self.root, "relative"),
                             (str(self.root) + "\n", self.source), (self.root, str(self.source) + "\r")):
            self.run_helper(root=root, source=source, success=False)
            self.assertFalse(self.vendor.exists())

    def test_incomplete_source_and_wrong_checkout_are_refused(self):
        bad = self.outer / "incomplete"
        bad.mkdir()
        self.run_helper(source=bad, success=False)
        self.run_helper(root=bad, success=False)
        self.assertFalse(self.vendor.exists())

    def test_source_inside_checkout_is_refused(self):
        source = self.root / "unlocked SDK"
        shutil.copytree(self.source, source)
        self.run_helper(source=source, success=False)
        self.assertFalse(self.vendor.exists())

    def test_unknown_option_is_refused_before_writes(self):
        self.run_helper(arguments=("--force",), success=False)
        self.assertFalse(self.vendor.exists())

    def setup_recipe(self):
        binary = self.outer / "fixture bin"
        binary.mkdir()
        for name in ("bash", "sh", "realpath", "mkdir", "ln"):
            tool = shutil.which(name)
            self.assertIsNotNone(tool)
            (binary / name).symlink_to(tool)
        for name, body in {
            "git": "import os; print(os.environ['FIXTURE_ROOT'])",
            "nix": ("import json,os,pathlib,sys; "
                    "pathlib.Path(os.environ['NIX_LOG']).write_text(json.dumps(sys.argv[1:])); "
                    "print(os.environ['FIXTURE_SOURCE']); sys.exit(int(os.environ.get('NIX_EXIT', '0')))"),
        }.items():
            tool = binary / name
            tool.write_text(f"#!{sys.executable}\n{body}\n")
            tool.chmod(0o755)
        shutil.copyfile(ROOT / "justfile", self.root / "justfile")
        (self.root / "scripts").mkdir()
        for name in ("sdk-source.sh", "cargo-target.sh"):
            shutil.copyfile(ROOT / "scripts" / name, self.root / "scripts" / name)
        self.env.update(PATH=str(binary), FIXTURE_ROOT=str(self.root), FIXTURE_SOURCE=str(self.source),
                        NIX_LOG=str(self.outer / "nix argv.json"))
        return shutil.which("just")

    def test_recipe_uses_only_the_source_alias_and_preserves_paths(self):
        just = self.setup_recipe()
        self.assertIsNotNone(just)
        result = subprocess.run([just, "sdk-prepare"], cwd=self.root, env=self.env,
                                stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(Path(self.env["NIX_LOG"]).read_text()), [
            "build", ".#microsandbox-filesystem-patched", "--no-update-lock-file", "--no-link", "--print-out-paths",
        ])
        self.assertEqual(self.link.resolve(), self.source)
        self.assertFalse((self.root / "control/agentctl/Cargo.lock").exists())

    def test_recipe_nix_failure_does_not_touch_the_vendor_link(self):
        just = self.setup_recipe()
        self.assertIsNotNone(just)
        self.env["NIX_EXIT"] = "23"
        result = subprocess.run([just, "sdk-prepare"], cwd=self.root, env=self.env,
                                stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.vendor.exists())


if __name__ == "__main__":
    unittest.main()
