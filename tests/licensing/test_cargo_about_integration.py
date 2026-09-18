# SPDX-FileCopyrightText: 2026 Georg Rybski
# SPDX-License-Identifier: Apache-2.0
"""Offline, local-only fixtures for the actual cargo-about JSON/graph contract.

No VM or registry downloads. Run with Cargo, rustc and cargo-about on PATH.
Set NOTICE_REQUIRE_REAL_CARGO=1 to fail rather than skip when tools are absent.
These tests do not certify the complete Nix runtime or its dependency closure.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "real_notice_generate", Path(__file__).resolve().parents[2] / "scripts/licensing/cargo_notices.py"
)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

MIT = """MIT License

Copyright (c) 2026 Notice integration fixture authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""


class CargoAboutIntegrationTests(unittest.TestCase):
    def setUp(self):
        missing = [tool for tool in ("cargo", "rustc", "cargo-about") if not shutil.which(tool)]
        if missing:
            message = "real Cargo integration unavailable: " + ", ".join(missing)
            if os.environ.get("NOTICE_REQUIRE_REAL_CARGO") == "1":
                self.fail(message)
            self.skipTest(message)
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.environment = dict(os.environ, CARGO_NET_OFFLINE="true",
                                CARGO_TARGET_DIR=str(self.root / "target"))
        names = ("app", "other", "shared", "feature-only", "build-only", "dev-only", "windows-only")
        (self.root / "Cargo.toml").write_text(
            '[workspace]\nresolver="2"\nmembers=' + json.dumps(names) + '\ndefault-members=["app"]\n')
        for name in names:
            crate = self.root / name
            (crate / "src").mkdir(parents=True)
            (crate / "src/lib.rs").write_text("pub fn fixture() {}\n")
            (crate / "LICENSE").write_text(MIT)
            (crate / "Cargo.toml").write_text(
                f'[package]\nname="{name}"\nversion="0.1.0"\nedition="2021"\n'
                'license="MIT"\npublish=false\n')
        self.append("app", '''
[dependencies]
shared = { path = "../shared", default-features = false }
[build-dependencies]
build-only = { path = "../build-only" }
[dev-dependencies]
dev-only = { path = "../dev-only" }
[target.'cfg(windows)'.dependencies]
windows-only = { path = "../windows-only" }
[features]
extra = ["shared/extra"]
''')
        self.append("shared", '''
[dependencies]
feature-only = { path = "../feature-only", optional = true }
[features]
extra = ["dep:feature-only"]
''')
        self.append("other", '''
[dependencies]
shared = { path = "../shared", features = ["extra"] }
''')
        (self.root / "app/build.rs").write_text("fn main() {}\n")
        self.policy = self.root / "deny.toml"
        self.policy.write_text('[licenses]\nallow=["MIT"]\n')
        subprocess.run(["cargo", "generate-lockfile", "--offline", "--manifest-path",
                        str(self.root / "Cargo.toml")], cwd=self.root, env=self.environment,
                       check=True, capture_output=True, text=True, timeout=60)

    def append(self, name, text):
        with (self.root / name / "Cargo.toml").open("a") as stream:
            stream.write(text)

    def generate(self, features):
        args = argparse.Namespace(manifest=self.root / "app/Cargo.toml", policy=self.policy,
                                  source_root=[self.root], output=self.root / "notices",
                                  target="x86_64-unknown-linux-musl", features=features,
                                  no_default_features=False)
        # Run the real helper in a child with bounded duration, not mocked subprocesses.
        command = ["python3", str(Path(m.__file__).resolve()), "generate",
                   "--manifest", str(args.manifest), "--policy", str(args.policy),
                   "--source-root", str(self.root), "--output", str(args.output),
                   "--target", args.target]
        if features:
            command.extend(["--features", features])
        before = (self.root / "Cargo.lock").read_bytes()
        result = subprocess.run(command, cwd=self.root, env=self.environment,
                                capture_output=True, text=True, timeout=120)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / "Cargo.lock").read_bytes(), before)
        m.verify(args.output)
        inventory = json.loads((args.output / "inventory.json").read_text())
        self.assertTrue(all(p["legal_files"] for p in inventory["crates"]))
        return {p["name"] for p in inventory["crates"]}

    def test_real_root_graph_excludes_other_member_features_dev_and_windows(self):
        self.assertEqual(self.generate(""), {"app", "shared", "build-only"})

    def test_real_explicit_feature_adds_its_required_notice(self):
        self.assertEqual(self.generate("extra"), {"app", "shared", "build-only", "feature-only"})


if __name__ == "__main__":
    unittest.main()
