#!/usr/bin/env python3
"""Exercise the offline pin guard against isolated repository fixtures."""

import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("check-msb-versions.sh")
REV = "a" * 40
OTHER_REV = "b" * 40
MANIFEST = """[dependencies]
microsandbox = { version = "=0.6.16" }
microsandbox-network = "=0.6.16"
[dev-dependencies]
microsandbox-image = "=0.6.16"
"""


class VersionPinsTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="workestrate-pins-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.manifest = MANIFEST
        self.versions = (
            'pub const MSB_VERSION_PIN: &str = "0.6.16";\n'
            f'pub const FORK_REV_PIN: &str = "{REV}";\n'
        )
        self.flake = (
            "{ inputs = { microsandbox-fork = {\n"
            f'  url = "github:rybskiworks/microsandbox/{REV}";\n'
            '  inputs.tooling.follows = "tooling";\n'
            "}; }; }\n"
        )
        source = {
            "type": "github",
            "owner": "rybskiworks",
            "repo": "microsandbox",
            "rev": REV,
        }
        self.lock = {
            "root": "root",
            "nodes": {
                "root": {"inputs": {"microsandbox-fork": "microsandbox-fork"}},
                "microsandbox-fork": {
                    "locked": dict(source),
                    "original": dict(source),
                },
            },
        }

    def check(self, failure=None, *, omit=(), overrides=None):
        files = {
            "control/agentctl/Cargo.toml": self.manifest,
            "control/agentctl/src/commands/versions.rs": self.versions,
            "flake.nix": self.flake,
            "flake.lock": json.dumps(self.lock),
        }
        files.update(overrides or {})
        for relative, content in files.items():
            path = self.root / relative
            if relative in omit:
                path.unlink(missing_ok=True)
                continue
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        result = subprocess.run(
            ["bash", str(SCRIPT)],
            env={**os.environ, "REPO": str(self.root)},
            text=True,
            capture_output=True,
            timeout=10,
        )
        output = result.stdout + result.stderr
        self.assertNotIn("Traceback", output)
        self.assertEqual(result.returncode, 1 if failure else 0, output)
        self.assertIn(failure or "msb-versions-check: OK", output)

    def test_matching_pins_without_local_runtime_recipes(self):
        self.check()
        self.assertFalse((self.root / "nix").exists())

    def test_string_and_table_dependency_pins(self):
        self.manifest = MANIFEST.replace(
            'microsandbox = { version = "=0.6.16" }',
            'microsandbox = "=0.6.16"',
        ).replace(
            'microsandbox-image = "=0.6.16"',
            'microsandbox-image = { version = "=0.6.16" }',
        )
        self.check()

    def test_missing_required_files(self):
        for path in (
            "control/agentctl/Cargo.toml",
            "control/agentctl/src/commands/versions.rs",
            "flake.nix",
            "flake.lock",
        ):
            with self.subTest(path=path):
                self.check("cannot read", omit=(path,))

    def test_missing_required_dependency_pins(self):
        for line in (
            'microsandbox = { version = "=0.6.16" }\n',
            'microsandbox-image = "=0.6.16"\n',
        ):
            with self.subTest(line=line):
                self.manifest = MANIFEST.replace(line, "")
                self.check("version pin missing")

    def test_nonexact_dependency_pins(self):
        for pin in ("0.6.16", "^0.6.16", "~0.6.16", ">=0.6.16", "=0.6", "=0.6.*", "="):
            with self.subTest(pin=pin):
                self.manifest = MANIFEST.replace('version = "=0.6.16"', f'version = "{pin}"')
                self.check("pin must be exact")
        self.manifest = MANIFEST.replace('version = "=0.6.16"', "version = false")
        self.check("pin must be exact")
        self.manifest = MANIFEST.replace('version = "=0.6.16"', "workspace = true")
        self.check("pin must be exact")

    def test_dependency_version_mismatch(self):
        for name in ("microsandbox-network", "microsandbox-image"):
            with self.subTest(name=name):
                self.manifest = MANIFEST.replace(f'{name} = "=0.6.16"', f'{name} = "=0.6.15"')
                self.check("msb version pins disagree")
        self.manifest = MANIFEST + '[build-dependencies]\nmicrosandbox-types = "=0.6.15"\n'
        self.check("msb version pins disagree")

    def test_reported_version_mismatch(self):
        self.versions = self.versions.replace('"0.6.16"', '"0.6.15"')
        self.check("msb version pins disagree")

    def test_missing_rust_constants(self):
        versions = self.versions
        for name in ("MSB_VERSION_PIN", "FORK_REV_PIN"):
            with self.subTest(name=name):
                self.versions = "\n".join(line for line in versions.splitlines() if name not in line)
                self.check(f"expected one {name}")

    def test_dotted_input_url(self):
        self.flake = f'{{ inputs.microsandbox-fork.url = "github:rybskiworks/microsandbox/{REV}"; }}'
        self.check()

    def test_url_must_contain_exact_full_revision(self):
        flake = self.flake
        for revision in ("main", REV[:8], REV + "a", REV + "?dir=source", ""):
            with self.subTest(revision=revision):
                self.flake = flake.replace(REV, revision) + f"# Pinned fork rev {REV}\n"
                self.check("URL must pin one full 40-hex")

    def test_missing_url_cannot_be_replaced_by_comment_or_other_input(self):
        for declaration in (
            f'# microsandbox-fork.url = "github:rybskiworks/microsandbox/{REV}";',
            f'/* microsandbox-fork.url = "github:rybskiworks/microsandbox/{REV}"; */',
            f'other.url = "github:rybskiworks/microsandbox/{REV}";',
        ):
            with self.subTest(declaration=declaration):
                self.flake = f"{{ inputs = {{ {declaration}\n }}; }}\n# Pinned fork rev {REV}\n"
                self.check("URL must pin one full 40-hex")

    def test_url_revision_mismatch(self):
        self.flake = self.flake.replace(REV, OTHER_REV)
        self.check("fork rev pins disagree")

    def test_missing_or_nonexact_lock_revisions(self):
        for key in ("locked", "original"):
            source = self.lock["nodes"]["microsandbox-fork"][key]
            for revision in (None, "main", REV[:8], REV + "a", 123):
                with self.subTest(key=key, revision=revision):
                    if revision is None:
                        source.pop("rev", None)
                    else:
                        source["rev"] = revision
                    self.check(f"{key}.rev must be a full 40-hex")
            source["rev"] = REV

    def test_lock_revision_mismatch(self):
        for key in ("locked", "original"):
            with self.subTest(key=key):
                self.lock["nodes"]["microsandbox-fork"][key]["rev"] = OTHER_REV
                self.check("fork rev pins disagree")
                self.lock["nodes"]["microsandbox-fork"][key]["rev"] = REV

    def test_lock_uses_root_input_node(self):
        nodes = self.lock["nodes"]
        nodes["selected-fork"] = copy.deepcopy(nodes["microsandbox-fork"])
        nodes["microsandbox-fork"]["locked"]["rev"] = OTHER_REV
        nodes["root"]["inputs"]["microsandbox-fork"] = "selected-fork"
        self.check()
        nodes["selected-fork"]["locked"]["rev"] = OTHER_REV
        self.check("fork rev pins disagree")

    def test_missing_lock_input(self):
        self.lock["nodes"]["root"]["inputs"].pop("microsandbox-fork")
        self.check("cannot parse microsandbox-fork rev")

    def test_lock_source_identity_mismatch(self):
        for key in ("locked", "original"):
            source = self.lock["nodes"]["microsandbox-fork"][key]
            for attribute, value in (("owner", "other"), ("repo", "other"), ("type", "git")):
                with self.subTest(key=key, attribute=attribute):
                    original = source[attribute]
                    source[attribute] = value
                    self.check("source does not match flake.nix URL")
                    source[attribute] = original

    def test_source_only_lock_is_stale(self):
        self.lock["nodes"]["microsandbox-fork"]["flake"] = False
        self.check("must provide flake packages")

    def test_reported_revision_mismatch_or_nonexact(self):
        versions = self.versions
        for revision, failure in (
            (OTHER_REV, "fork rev pins disagree"),
            (REV[:8], "FORK_REV_PIN is not a full 40-hex"),
            (REV + "a", "FORK_REV_PIN is not a full 40-hex"),
        ):
            with self.subTest(revision=revision):
                self.versions = versions.replace(REV, revision)
                self.check(failure)

    def test_malformed_manifest_and_lock(self):
        self.check("cannot parse manifest", overrides={"control/agentctl/Cargo.toml": "["})
        self.check("cannot parse microsandbox-fork rev", overrides={"flake.lock": "{"})


if __name__ == "__main__":
    unittest.main()
