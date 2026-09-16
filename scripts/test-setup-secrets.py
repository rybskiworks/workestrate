#!/usr/bin/env python3
"""Test secrets selection and encrypted updates using disposable homes and keys.

Exercises the real `workestrate secrets` CLI (init/update) — target selection
precedence, per-repo overrides, and the no-secret-values-in-output contract.
"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class SecretsTargetTest(unittest.TestCase):
    def setUp(self):
        for tool in ("workestrate", "age-keygen", "sops"):
            self.assertIsNotNone(shutil.which(tool), f"required test tool missing: {tool}")
        self.scratch = tempfile.TemporaryDirectory(prefix="workestrate-secrets-target-")
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.home = self.root / "operator home"
        self.fleet = self.home / "config-repos/personal"
        self.key = self.root / "private keys/age.txt"
        self.key.parent.mkdir()
        self.env = {
            "PATH": os.environ["PATH"],
            "HOME": str(self.root / "user home"),
            "XDG_CONFIG_HOME": str(self.root / "unrelated xdg config"),
            "XDG_DATA_HOME": str(self.root / "unrelated xdg data"),
            "XDG_STATE_HOME": str(self.root / "unrelated xdg state"),
            "SOPS_AGE_KEY_FILE": str(self.key),
            "TMPDIR": str(self.root),
            "LITELLM_MASTER_KEY": "sk-test-initial-target-selection",
            # Fail deterministically if a flow unexpectedly goes interactive.
            "EDITOR": "false",
            "LC_ALL": "C",
        }
        subprocess.run(
            ["age-keygen", "-o", str(self.key)], env=self.env,
            check=True, capture_output=True, text=True,
        )
        self.recipient = subprocess.check_output(
            ["age-keygen", "-y", str(self.key)], env=self.env, text=True,
        ).strip()
        self.make_fleet(self.fleet)
        self.write_registry()

    def make_fleet(self, path):
        path.mkdir(parents=True)
        (path / ".sops.yaml").write_text(
            f"keys:\n  - &fixture {self.recipient}\n"
            "creation_rules:\n  - path_regex: .*\\.enc$\n"
            "    key_groups:\n      - age:\n          - *fixture\n"
        )
        (path / ".env.example").write_text("LITELLM_MASTER_KEY=\n")
        (path / "workestrate.toml").write_text(
            'schema_version = 1\n[secrets.LITELLM_MASTER_KEY]\n'
            'env_var = "LITELLM_MASTER_KEY"\nrequired = false\n'
        )

    def write_registry(self, *, settings="", overrides=""):
        (self.home / "config.toml").write_text(
            f'layers = ["personal"]\n{settings}\n'
            '[configs.personal]\nurl = "config-repos/personal"\n'
            f"{overrides}\n"
        )

    def run_helper(self, command, *args, extra_env=None, success=True):
        # `workestrate secrets <init|update> [opts]`: the target-selector
        # flags belong to the verb; --home is a global CLI flag and is
        # accepted at any position.
        result = subprocess.run(
            [shutil.which("workestrate"), "secrets", command, *map(str, args)],
            cwd=self.root, env={**self.env, **(extra_env or {})},
            stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30,
        )
        if success:
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertNotIn(self.env["LITELLM_MASTER_KEY"], result.stdout + result.stderr)
        return result

    def run_cli(self, *args, extra_env=None):
        """Raw CLI invocation for invalid-argument cases (any argv shape)."""
        return subprocess.run(
            [shutil.which("workestrate"), *map(str, args)],
            cwd=self.root, env={**self.env, **(extra_env or {})},
            stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30,
        )

    def decrypt(self, path):
        return subprocess.check_output(
            ["sops", "decrypt", "--input-type", "dotenv", "--output-type", "dotenv", str(path)],
            cwd=self.root, env=self.env, text=True, stderr=subprocess.PIPE,
        )

    def test_named_config_initializes_and_updates_only_selected_fleet(self):
        other = self.home / "config-repos/other"
        self.make_fleet(other)
        (other / ".env.enc").write_text("untouched ciphertext fixture")
        args = ["--home", self.home, "--config", "personal"]
        self.run_helper("init", *args)
        self.assertIn(self.env["LITELLM_MASTER_KEY"], self.decrypt(self.fleet / ".env.enc"))
        replacement = "sk-test-updated-target-selection"
        result = self.run_helper("update", *args, extra_env={"LITELLM_MASTER_KEY": replacement})
        self.assertIn(replacement, self.decrypt(self.fleet / ".env.enc"))
        self.assertNotIn(replacement, result.stdout + result.stderr)
        self.assertEqual((other / ".env.enc").read_text(), "untouched ciphertext fixture")
        self.assertFalse((self.root / "unrelated xdg data").exists())

    def test_relative_home_and_equals_options_after_command(self):
        self.run_helper("init", "--home=operator home", "--config=personal")
        self.assertTrue((self.fleet / ".env.enc").is_file())

    def test_home_environment_and_explicit_name_override_direct_environment(self):
        other = self.root / "unselected fleet"
        self.make_fleet(other)
        self.run_helper("init", "--config", "personal", extra_env={
            "WORKESTRATE_HOME": str(self.home),
            "WORKESTRATE_CONFIG_DIR": str(other),
        })
        self.assertTrue((self.fleet / ".env.enc").is_file())
        self.assertFalse((other / ".env.enc").exists())

    def test_home_flag_overrides_home_environment(self):
        self.run_helper("init", "--home", self.home, "--config", "personal", extra_env={
            "WORKESTRATE_HOME": str(self.root / "wrong home"),
        })
        self.assertTrue((self.fleet / ".env.enc").is_file())

    def test_registered_store_file_and_key_overrides(self):
        store = self.root / 'custom "store"'
        selected = store / "config-repos/personal"
        self.make_fleet(selected)
        self.write_registry(
            settings=f"[settings]\nstore_dir = {json.dumps(str(store))}\n",
            overrides=f'secrets_file = "custom secrets.enc"\nage_key_file = {json.dumps(str(self.key))}',
        )
        self.run_helper("init", "--home", self.home, "--config", "personal", extra_env={
            "SOPS_AGE_KEY_FILE": str(self.root / "wrong key"),
            "SECRET_FILE": "wrong-file.enc",
        })
        self.assertIn(self.env["LITELLM_MASTER_KEY"], self.decrypt(selected / "custom secrets.enc"))
        self.assertFalse((selected / "wrong-file.enc").exists())
        self.assertFalse((self.fleet / ".env.enc").exists())

    def test_direct_directory_handles_relative_paths_and_shell_metacharacters(self):
        name = 'fleet with spaces "quoted" $(touch INJECTED)'
        selected = self.root / name
        self.make_fleet(selected)
        self.run_helper("init", "--config-dir", name, extra_env={
            "WORKESTRATE_CONFIG_DIR": str(self.fleet),
        })
        self.assertIn(self.env["LITELLM_MASTER_KEY"], self.decrypt(selected / ".env.enc"))
        self.assertFalse((self.fleet / ".env.enc").exists())
        self.assertFalse((self.root / "INJECTED").exists())

    def test_relative_registry_key_is_resolved_before_changing_directory(self):
        self.write_registry(overrides='age_key_file = "private keys/age.txt"')
        self.run_helper("init", "--home", self.home, "--config", "personal")
        self.assertIn(self.env["LITELLM_MASTER_KEY"], self.decrypt(self.fleet / ".env.enc"))

    def test_direct_directory_equals(self):
        self.run_helper("init", f"--config-dir={self.fleet}")
        self.assertTrue((self.fleet / ".env.enc").exists())

    def test_unknown_name_does_not_fall_back_to_environment(self):
        result = self.run_helper("init", "--home", self.home, "--config", "missing",
                                 extra_env={"WORKESTRATE_CONFIG_DIR": str(self.fleet)}, success=False)
        self.assertIn("could not resolve config 'missing'", result.stderr)
        self.assertFalse((self.fleet / ".env.enc").exists())

    def test_missing_directory_is_not_created(self):
        missing = self.root / "missing directory"
        self.run_helper("update", "--config-dir", missing, success=False)
        self.assertFalse(missing.exists())

    def test_invalid_arguments_fail_before_writes(self):
        for verb in ("init", "update"):
            for args in (
                ["--config"], ["--config="], ["--home"], ["--home="],
                ["--config-dir"], ["--config-dir="], ["--config", "--global"],
                ["--config", "personal", "--global"],
                ["--config-dir", self.fleet, "--global"],
                ["--config-dir", self.fleet, "--config", "personal"],
                ["--unknown"],
            ):
                with self.subTest(verb=verb, args=args):
                    result = self.run_cli("secrets", verb, *args)
                    self.assertNotEqual(result.returncode, 0, result.stdout)
                    self.assertFalse((self.fleet / ".env.enc").exists())
        for args in (["secrets", "init", "update"], ["secrets", "typo"]):
            with self.subTest(args=args):
                result = self.run_cli(*args)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertFalse((self.fleet / ".env.enc").exists())


if __name__ == "__main__":
    unittest.main()
