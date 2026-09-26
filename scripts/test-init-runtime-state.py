#!/usr/bin/env python3
"""Exercise fresh runtime initialization without invoking a VM or live state."""

import os
from pathlib import Path
import shutil
import stat
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("init-runtime-state.sh")
KEY = "0123456789ab"


class RuntimeInitializationTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="wsi-", dir="/tmp")
        self.addCleanup(self.scratch.cleanup)
        self.home = Path(self.scratch.name)
        self.runtime = self.home / (KEY + "cdef0123456789abcdef-microsandbox-test")
        self.runtime.joinpath("bin").mkdir(parents=True)
        self.msb = self.runtime / "bin/msb"
        self.msb.write_text("#!/bin/sh\nexit 77\n")
        self.msb.chmod(0o755)
        self.root = self.home / ".microsandbox"

    def run_init(self, *args, **overrides):
        env = {
            "PATH": os.environ["PATH"],
            "HOME": str(self.home),
            "WORKESTRATE_INIT_MSB": str(self.msb),
            "LC_ALL": "C",
        }
        env.update(overrides)
        return subprocess.run(
            [shutil.which("bash"), str(SCRIPT), *args],
            env=env, capture_output=True, text=True, timeout=10,
        )

    def test_fresh_private_generation_without_running_runtime(self):
        result = self.run_init()
        self.assertEqual(result.returncode, 0, result.stderr)
        generation = self.root / "generations" / KEY
        self.assertEqual(os.readlink(self.root / "current"), f"generations/{KEY}")
        self.assertEqual((self.root / "current").resolve(), generation)
        for directory in (self.root, self.root / "generations", generation):
            self.assertEqual(stat.S_IMODE(directory.stat().st_mode), 0o700)
        self.assertEqual(stat.S_IMODE((self.root / ".flip.lock").stat().st_mode), 0o600)
        self.assertEqual(list(generation.iterdir()), [])

    def test_dry_run_does_not_create_state(self):
        result = self.run_init("--dry-run")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(self.root.exists())

    def test_existing_directory_is_untouched(self):
        self.root.mkdir()
        sentinel = self.root / "keep"
        sentinel.write_bytes(b"existing runtime data\n")
        result = self.run_init()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(sentinel.read_bytes(), b"existing runtime data\n")
        self.assertEqual(list(self.root.iterdir()), [sentinel])

    def test_empty_directory_and_dangling_link_are_refused(self):
        self.root.mkdir()
        self.assertNotEqual(self.run_init().returncode, 0)
        self.root.rmdir()
        self.root.symlink_to(self.home / "absent")
        self.assertNotEqual(self.run_init().returncode, 0)
        self.assertTrue(self.root.is_symlink())

    def test_explicit_runtime_home_is_untouched(self):
        custom = self.home / "custom"
        result = self.run_init(MSB_HOME=str(custom))
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(custom.exists())
        self.assertFalse(self.root.exists())

    def test_missing_runtime_or_bad_identity_creates_nothing(self):
        self.assertNotEqual(self.run_init(WORKESTRATE_INIT_MSB="/absent/msb").returncode, 0)
        bad = self.home / "bin/msb"
        bad.parent.mkdir()
        shutil.copyfile(self.msb, bad)
        bad.chmod(0o755)
        self.assertNotEqual(self.run_init(WORKESTRATE_INIT_MSB=str(bad)).returncode, 0)
        self.assertFalse(self.root.exists())

    def test_missing_or_relative_home_creates_nothing(self):
        self.assertNotEqual(self.run_init(HOME="").returncode, 0)
        self.assertNotEqual(self.run_init(HOME=".").returncode, 0)
        self.assertFalse(self.root.exists())

    def test_long_home_is_not_rejected_by_an_invented_socket_budget(self):
        long_home = self.home / ("long-home-" * 8)
        long_home.mkdir()
        result = self.run_init(HOME=str(long_home))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            os.readlink(long_home / ".microsandbox/current"), f"generations/{KEY}"
        )
        self.assertFalse(self.root.exists())

    def test_repeat_init_refuses_the_existing_generation(self):
        self.assertEqual(self.run_init().returncode, 0)
        result = self.run_init()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(os.readlink(self.root / "current"), f"generations/{KEY}")


if __name__ == "__main__":
    unittest.main()
