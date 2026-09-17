# SPDX-FileCopyrightText: 2026 Georg Rybski
# SPDX-License-Identifier: Apache-2.0
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("notices", Path(__file__).resolve().parents[2] / "scripts/licensing/cargo_notices.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class NoticeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.crate = self.root / "crate"
        self.crate.mkdir()
        (self.crate / "Cargo.toml").write_text('[package]\nname="fixture"\nversion="1.0.0"\n')
        (self.crate / "LICENSE").write_text("Copyright Original Author\nSynthetic MIT fixture\n")
        (self.crate / "NOTICE").write_text("Original attribution <not markup>\n")
        self.package = {"id": "fixture", "name": "fixture", "version": "1.0.0", "source": None,
                        "license": "MIT", "manifest_path": str(self.crate / "Cargo.toml")}
        self.report = {"licenses": [{"id": "MIT", "text": "Generic MIT fixture", "used_by": [{"crate": self.package}]}]}
        self.policy = self.root / "deny.toml"
        self.policy.write_text('[licenses]\nallow=["MIT", "Apache-2.0", "ISC"]\n')
        self.output = self.root / "output"

    def build(self):
        m.assemble(self.output, [self.package], self.report, [self.root], {"target": "fixture"})

    def test_policy_derivation(self):
        config = m.tomllib.loads(m.config(self.policy, "target"))
        self.assertEqual(config["accepted"], ["Apache-2.0", "MIT", "ISC"])
        self.assertFalse(config["ignore-build-dependencies"])
        self.assertFalse(config["ignore-transitive-dependencies"])
        self.assertFalse(config["private"]["ignore"])

    def test_empty_policy(self):
        self.policy.write_text("[licenses]\nallow=[]\n")
        with self.assertRaises(ValueError):
            m.config(self.policy, "target")

    def test_scoped_exceptions_are_not_broadened(self):
        with self.policy.open("a") as stream:
            stream.write('[[licenses.exceptions]]\nname="special"\nallow=["BSD-3-Clause"]\n')
        with self.assertRaisesRegex(ValueError, "exceptions"):
            m.config(self.policy, "target")

    def test_wildcard_exception_stays_crate_scoped(self):
        with self.policy.open("a") as stream:
            stream.write('[[licenses.exceptions]]\nname="special"\nversion="*"\nallow=["OpenSSL"]\n')
        data = m.tomllib.loads(m.config(self.policy, "target"))
        self.assertNotIn("OpenSSL", data["accepted"])
        self.assertEqual(data["special"]["accepted"], ["OpenSSL"])

    def test_clarifications_need_explicit_translation(self):
        with self.policy.open("a") as stream:
            stream.write('[[licenses.clarify]]\nname="special"\nexpression="MIT"\n')
        with self.assertRaisesRegex(ValueError, "clarifications"):
            m.config(self.policy, "target")

    def test_original_attribution_and_safe_html(self):
        self.build()
        m.verify(self.output)
        text = (self.output / "THIRD-PARTY.html").read_text()
        self.assertIn("Copyright Original Author", text)
        self.assertIn("&lt;not markup&gt;", text)
        self.assertNotIn(str(self.root), (self.output / "cargo-about.json").read_text())

    def test_generic_text_is_insufficient(self):
        (self.crate / "LICENSE").unlink()
        (self.crate / "NOTICE").unlink()
        with self.assertRaisesRegex(ValueError, "original legal"):
            self.build()
        self.assertFalse(self.output.exists())

    def test_omitted_local_crate(self):
        self.report["licenses"][0]["used_by"] = []
        with self.assertRaisesRegex(ValueError, "omitted"):
            self.build()

    def test_tampered_report(self):
        self.build()
        (self.output / "THIRD-PARTY.html").write_text("modified")
        with self.assertRaisesRegex(ValueError, "checksum"):
            m.verify(self.output)

    def test_missing_report(self):
        self.build()
        (self.output / "cargo-about.json").unlink()
        with self.assertRaisesRegex(ValueError, "missing"):
            m.verify(self.output)

    def test_no_overwrite(self):
        self.build()
        with self.assertRaisesRegex(ValueError, "overwrite"):
            self.build()

    def test_symlink_evidence(self):
        (self.crate / "NOTICE").unlink()
        (self.crate / "NOTICE").symlink_to(self.policy)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.build()

    def test_parent_license_requires_approved_root(self):
        (self.crate / "LICENSE").unlink()
        (self.crate / "NOTICE").unlink()
        (self.root / "LICENSE").write_text("unrelated")
        with self.assertRaises(ValueError):
            m.originals(self.package, [])
        self.assertIn(self.root / "LICENSE", m.originals(self.package, [self.root]))

    def test_declared_nonstandard_license(self):
        (self.crate / "legal.txt").write_text("Nonstandard legal filename")
        self.package["license_file"] = "legal.txt"
        self.assertIn(self.crate / "legal.txt", m.originals(self.package, []))

    def test_declared_license_cannot_escape(self):
        self.package["license_file"] = "/etc/passwd"
        with self.assertRaisesRegex(ValueError, "escapes"):
            m.originals(self.package, [self.root])

    def test_build_dependency_kept_dev_dependency_dropped(self):
        packages = [self.package] + [dict(self.package, id=n, name=n, manifest_path=str(self.root / n / "Cargo.toml")) for n in ("build", "dev")]
        metadata = {"packages": packages, "resolve": {"nodes": [
            {"id": "fixture", "deps": [{"pkg": "build", "dep_kinds": [{"kind": "build"}]}, {"pkg": "dev", "dep_kinds": [{"kind": "dev"}]}]},
            {"id": "build", "deps": []}, {"id": "dev", "deps": []}]}}
        report = {"crates": [{"package": p, "license": "MIT"} for p in packages[:2]]}
        self.assertEqual({p["name"] for p in m.selected(metadata, self.crate / "Cargo.toml", report)}, {"fixture", "build"})

    def test_deterministic_bundle(self):
        self.build()
        manifest = (self.output / "manifest.json").read_text()
        self.output = self.root / "second"
        self.build()
        self.assertEqual(manifest, (self.output / "manifest.json").read_text())


if __name__ == "__main__":
    unittest.main()
