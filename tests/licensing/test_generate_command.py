# SPDX-FileCopyrightText: 2026 Georg Rybski
# SPDX-License-Identifier: Apache-2.0
"""Exercise generate(), not just bundle assembly, with strict command fixtures.

The command contract is cargo-about 0.9.0's src/cargo-about/generate.rs.
These are unit tests with subprocess responses mocked, not a real Cargo build.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "notice_generate", Path(__file__).resolve().parents[2] / "scripts/licensing/cargo_notices.py"
)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class GenerateCommandTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.roots = [self.root / "application", self.root / "fork"]
        self.packages = []
        for root, name in zip(self.roots, ("application", "local-fork")):
            crate = root / "crates" / name
            crate.mkdir(parents=True)
            (crate / "Cargo.toml").write_text(
                f'[package]\nname="{name}"\nversion="1.0.0"\npublish=false\nlicense="MIT"\n'
            )
            (root / "LICENSE").write_text(f"Copyright Original {name} Author\nSynthetic MIT fixture\n")
            self.packages.append({"id": name, "name": name, "version": "1.0.0",
                                  "source": None, "license": "MIT",
                                  "manifest_path": str(crate / "Cargo.toml")})
        self.lock = self.roots[0] / "Cargo.lock"
        self.lock.write_text("version = 4\n")
        self.policy = self.root / "deny.toml"
        self.policy.write_text('[licenses]\nallow=["MIT"]\n')
        self.args = argparse.Namespace(
            manifest=Path(self.packages[0]["manifest_path"]),
            policy=self.policy, output=self.root / "notices", source_root=self.roots,
            target="x86_64-unknown-linux-musl", features="", no_default_features=False,
        )
        self.metadata = {"workspace_root": str(self.roots[0]), "packages": self.packages,
                         "resolve": {"nodes": [
                             {"id": "application", "deps": [
                                 {"pkg": "local-fork", "dep_kinds": [{"kind": None}]}]},
                             {"id": "local-fork", "deps": []},
                         ]}}
        self.report = {"licenses": [{"id": "MIT", "text": "Synthetic MIT report",
                                    "used_by": [{"crate": p} for p in self.packages]}]}
        # cargo-about emits a graph inventory separately from license text users.
        self.report["crates"] = [{"package": dict(p), "license": "MIT"} for p in self.packages]
        self.commands = []
        self.mutate_lock = False
        self.fail_report = False

    def run_fixture(self, command):
        self.commands.append(command)
        flags = ["--manifest-path", str(self.args.manifest), "--locked", "--offline"]
        if self.args.no_default_features:
            flags.append("--no-default-features")
        if self.args.features:
            flags += ["--features", self.args.features]
        if command[:2] == ["cargo", "metadata"]:
            self.assertEqual(command, ["cargo", "metadata", "--format-version", "1",
                                       "--filter-platform", self.args.target, *flags])
            return json.dumps(self.metadata)
        if command[:3] == ["cargo", "about", "generate"]:
            config_path = command[command.index("--config") + 1]
            self.assertEqual(command, ["cargo", "about", "generate", *flags, "--fail",
                                       "--format", "json", "--config", config_path])
            config = m.tomllib.loads(Path(config_path).read_text())
            self.assertFalse(config["private"]["ignore"])
            self.assertEqual(config["targets"], [self.args.target])
            if self.fail_report:
                raise subprocess.CalledProcessError(2, command)
            if self.mutate_lock:
                self.lock.write_text("changed\n")
            return json.dumps(self.report)
        self.assertEqual(command, ["cargo", "about", "--version"])
        return "cargo-about 0.9.0\n"

    def generate(self):
        with patch.object(m, "run", side_effect=self.run_fixture):
            m.generate(self.args)

    def test_default_musl_profile_retains_local_crates_and_parent_evidence(self):
        before = self.lock.read_bytes()
        self.generate()
        m.verify(self.args.output)
        inventory = json.loads((self.args.output / "inventory.json").read_text())
        self.assertEqual({p["name"] for p in inventory["crates"]}, {"application", "local-fork"})
        text = (self.args.output / "THIRD-PARTY.html").read_text()
        for name in ("application", "local-fork"):
            self.assertIn(f"Original {name} Author", text)
        self.assertEqual(self.lock.read_bytes(), before)
        self.assertEqual(len(self.commands), 3)

    def test_cli_feature_profile_is_preserved(self):
        self.args.target = "x86_64-unknown-linux-gnu"
        self.args.features = "net,ssh"
        self.args.no_default_features = True
        self.generate()
        inventory = json.loads((self.args.output / "inventory.json").read_text())
        self.assertEqual(inventory["profile"]["features"], "net,ssh")
        self.assertFalse(inventory["profile"]["default_features"])

    def test_omitted_local_crate_still_fails(self):
        self.report["licenses"][0]["used_by"].pop()
        with self.assertRaisesRegex(ValueError, "omitted selected crates"):
            self.generate()
        self.assertFalse(self.args.output.exists())

    def test_missing_original_evidence_still_fails(self):
        (self.roots[1] / "LICENSE").unlink()
        with self.assertRaisesRegex(ValueError, "missing original legal evidence"):
            self.generate()
        self.assertFalse(self.args.output.exists())

    def test_cargo_failure_is_not_suppressed(self):
        self.fail_report = True
        with self.assertRaises(subprocess.CalledProcessError):
            self.generate()
        self.assertFalse(self.args.output.exists())

    def test_changed_lock_still_fails(self):
        self.mutate_lock = True
        with self.assertRaisesRegex(ValueError, "Cargo.lock changed"):
            self.generate()
        self.assertFalse(self.args.output.exists())

    def test_workspace_feature_edges_do_not_expand_the_report_inventory(self):
        # Another workspace member enabled an optional dependency on local-fork.
        # Raw metadata exposes this as a normal edge even though cargo-about's
        # selected-root graph correctly excludes it. No source files are needed
        # for the excluded package, and its absence must not fail generation.
        extra = dict(self.packages[1], id="workspace-only", name="workspace-only",
                     manifest_path=str(self.root / "workspace-only" / "Cargo.toml"))
        self.metadata["packages"].append(extra)
        self.metadata["resolve"]["nodes"][1]["deps"].append(
            {"pkg": "workspace-only", "dep_kinds": [{"kind": None}]})
        self.metadata["resolve"]["nodes"].append({"id": "workspace-only", "deps": []})
        self.generate()
        inventory = json.loads((self.args.output / "inventory.json").read_text())
        self.assertEqual({p["name"] for p in inventory["crates"]}, {"application", "local-fork"})
        self.assertEqual(inventory["profile"]["graph_source"], "cargo-about.crates")

    def test_inventory_is_required_not_inferred_from_license_users(self):
        del self.report["crates"]
        with self.assertRaisesRegex(ValueError, "no crate inventory"):
            self.generate()

    def test_empty_inventory_fails(self):
        self.report["crates"] = []
        with self.assertRaisesRegex(ValueError, "no crate inventory"):
            self.generate()

    def test_root_cannot_disappear_from_inventory(self):
        self.report["crates"].pop(0)
        with self.assertRaisesRegex(ValueError, "omitted the selected root"):
            self.generate()

    def test_duplicate_inventory_identity_fails(self):
        self.report["crates"].append(self.report["crates"][0])
        with self.assertRaisesRegex(ValueError, "duplicate report package identity"):
            self.generate()

    def test_unknown_inventory_identity_fails(self):
        self.report["crates"][1]["package"]["id"] = "unrelated-source"
        with self.assertRaisesRegex(ValueError, "unknown package identity"):
            self.generate()

    def test_inventory_source_and_path_are_checked_against_metadata(self):
        original = dict(self.report["crates"][1]["package"])
        for key, value in (("source", "git+https://example.invalid/other"),
                           ("manifest_path", "/unapproved/Cargo.toml"),
                           ("name", "wrong-name"), ("version", "9.9.9")):
            with self.subTest(field=key):
                self.report["crates"][1]["package"] = dict(original, **{key: value})
                with self.assertRaisesRegex(ValueError, "metadata mismatch"):
                    self.generate()
        self.assertFalse(self.args.output.exists())

    def test_unresolved_and_ignored_inventory_licenses_fail(self):
        for expression in ("Unknown", "Ignore", "", None):
            with self.subTest(license=expression):
                self.report["crates"][1]["license"] = expression
                with self.assertRaisesRegex(ValueError, "unresolved or ignored"):
                    self.generate()

    def test_same_name_version_cannot_mask_wrong_source_in_license_users(self):
        wrong = dict(self.packages[1], id="different-package-source")
        self.report["licenses"][0]["used_by"][1] = {"crate": wrong}
        with self.assertRaisesRegex(ValueError, "unknown package identity"):
            self.generate()

    def test_uninventoried_license_user_fails(self):
        self.report["crates"].pop()
        with self.assertRaisesRegex(ValueError, "unknown package identity"):
            self.generate()


if __name__ == "__main__":
    unittest.main()
