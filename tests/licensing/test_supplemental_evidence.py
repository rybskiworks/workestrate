# SPDX-FileCopyrightText: 2026 Georg Rybski
# SPDX-License-Identifier: Apache-2.0
"""Offline evidence and failure-reporting tests, not a real Nix/Cargo build."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("supplemental_notices", ROOT / "scripts/licensing/cargo_notices.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

ORIGINAL_MANIFEST = '''[package]
name = "asn1-rs-impl"
version = "0.2.0"
authors = ["Pierre Chifflier <chifflier@wzdftpd.net>"]
description = "Implementation details for the `asn1-rs` crate"
license = "MIT/Apache-2.0"
homepage = "https://github.com/rusticata/asn1-rs"
repository = "https://github.com/rusticata/asn1-rs.git"
edition = "2018"

[lib]
proc-macro = true

[dependencies]
proc-macro2 = "1"
quote = "1"
syn = "2.0"
'''


class SupplementalEvidenceTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.crate = self.root / "crate"
        self.crate.mkdir()
        (self.crate / "Cargo.toml").write_text(ORIGINAL_MANIFEST)
        (self.crate / "Cargo.toml.orig").write_text(ORIGINAL_MANIFEST)
        self.helper = self.root / "helper"
        self.helper.mkdir()
        shutil.copytree(ROOT / "scripts/licensing/evidence", self.helper / "evidence")
        file_patch = patch.object(m, "__file__", str(self.helper / "cargo_notices.py"))
        file_patch.start()
        self.addCleanup(file_patch.stop)
        self.evidence = self.helper / "evidence/asn1-rs-impl-0.2.0"
        self.provenance = self.evidence / "NOTICE-provenance.json"
        self.record = json.loads(self.provenance.read_text())
        self.package = dict(self.record["package"], id="registry-asn1-rs-impl-0.2.0",
                            license="MIT/Apache-2.0", manifest_path=str(self.crate / "Cargo.toml"))
        self.lock = self.root / "Cargo.lock"
        self.lock.write_text('version = 4\n[[package]]\n' + ''.join(
            f'{key} = {json.dumps(value)}\n' for key, value in self.record["package"].items()))
        self.output = self.root / "notices"
        self.report = {"licenses": [{"id": "MIT", "text": "Synthetic report text",
                                    "used_by": [{"crate": self.package}]}]}

    def originals(self):
        return m.originals(self.package, [], self.lock)

    def assemble(self, packages=None, report=None):
        m.assemble(self.output, packages or [self.package], report or self.report, [], {}, self.lock)

    def test_reviewed_documents_match_and_preserve_attribution(self):
        files = self.originals()
        self.assertEqual({p.name for p in files}, {"LICENSE-MIT", "LICENSE-APACHE", "NOTICE-provenance.json"})
        self.assertIn("Copyright (c) 2017 Pierre Chifflier", (self.evidence / "LICENSE-MIT").read_text())
        self.assertEqual(m.digest(self.crate / "Cargo.toml.orig"), self.record["manifest_sha256"])

    def test_bundle_contains_originals_and_provenance(self):
        self.assemble()
        m.verify(self.output)
        inventory = json.loads((self.output / "inventory.json").read_text())["crates"][0]
        self.assertEqual(len(inventory["legal_files"]), 3)
        self.assertIn("Copyright (c) 2017 Pierre Chifflier", (self.output / "THIRD-PARTY.html").read_text())
        self.assertIn(self.record["package"]["checksum"], (self.output / "THIRD-PARTY.html").read_text())

    def test_unreviewed_version_still_fails(self):
        self.package["version"] = "0.2.1"
        with self.assertRaisesRegex(ValueError, "missing original legal evidence"):
            self.originals()

    def test_other_source_is_rejected(self):
        self.package["source"] = "git+https://example.invalid/other"
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            self.originals()

    def test_lock_checksum_mismatch_is_rejected(self):
        self.lock.write_text(self.lock.read_text().replace(self.record["package"]["checksum"], "0" * 64))
        with self.assertRaisesRegex(ValueError, "registry checksum mismatch"):
            self.originals()

    def test_missing_lock_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "exact Cargo.lock"):
            m.originals(self.package, [])

    def test_duplicate_lock_entry_is_rejected(self):
        self.lock.write_text(self.lock.read_text() + self.lock.read_text().split('version = 4\n')[1])
        with self.assertRaisesRegex(ValueError, "registry checksum mismatch"):
            self.originals()

    def test_changed_original_manifest_is_rejected(self):
        (self.crate / "Cargo.toml.orig").write_text(ORIGINAL_MANIFEST + "# changed\n")
        with self.assertRaisesRegex(ValueError, "original manifest mismatch"):
            self.originals()

    def test_missing_original_manifest_is_rejected(self):
        (self.crate / "Cargo.toml.orig").unlink()
        with self.assertRaisesRegex(ValueError, "regular Cargo.toml.orig"):
            self.originals()

    def test_modified_license_is_rejected(self):
        (self.evidence / "LICENSE-MIT").write_text("not the upstream license")
        with self.assertRaisesRegex(ValueError, "document checksum mismatch"):
            self.originals()

    def test_symlinked_license_is_rejected(self):
        (self.evidence / "LICENSE-MIT").unlink()
        (self.evidence / "LICENSE-MIT").symlink_to(self.evidence / "LICENSE-APACHE")
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.originals()

    def test_symlinked_evidence_directory_is_rejected(self):
        target = self.root / "redirected"
        self.evidence.rename(target)
        self.evidence.symlink_to(target, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.originals()

    def test_path_traversal_is_rejected(self):
        self.record["documents"]["../LICENSE"] = "0" * 64
        self.provenance.write_text(json.dumps(self.record))
        with self.assertRaisesRegex(ValueError, "unsafe supplemental"):
            self.originals()

    def test_missing_document_is_rejected(self):
        (self.evidence / "LICENSE-APACHE").unlink()
        with self.assertRaisesRegex(ValueError, "missing, empty or symlink"):
            self.originals()

    def test_existing_originals_do_not_need_supplement(self):
        (self.crate / "LICENSE").write_text("Existing upstream evidence")
        self.assertEqual(m.originals(self.package, []), [self.crate / "LICENSE"])

    def test_bad_existing_evidence_is_not_hidden_by_supplement(self):
        (self.crate / "LICENSE").symlink_to(self.crate / "Cargo.toml")
        with self.assertRaisesRegex(ValueError, "symlink legal evidence"):
            self.originals()

    def test_report_coverage_remains_required(self):
        self.report["licenses"][0]["used_by"] = []
        with self.assertRaisesRegex(ValueError, "report omitted selected crates"):
            self.assemble()

    def test_reports_all_missing_packages_without_publishing(self):
        packages = []
        for name in ("missing-one", "missing-two"):
            directory = self.root / name
            directory.mkdir()
            manifest = directory / "Cargo.toml"
            manifest.write_text(f'[package]\nname="{name}"\nversion="1.0.0"\n')
            packages.append(dict(self.package, id=name, name=name, version="1.0.0", manifest_path=str(manifest)))
        report = {"licenses": [{"id": "MIT", "text": "Synthetic", "used_by": [{"crate": p} for p in packages]}]}
        with self.assertRaises(ValueError) as caught:
            self.assemble(packages, report)
        self.assertIn("missing-one 1.0.0", str(caught.exception))
        self.assertIn("missing-two 1.0.0", str(caught.exception))
        self.assertFalse(self.output.exists())


GIT_MANIFEST = '''[package]
name = "synthetic-git"
version = "0.1.0"
license = "Apache-2.0"
'''

GIT_SOURCE = "git+https://example.invalid/repo?rev={0}#{0}".format("a" * 40)


class GitSupplementalEvidenceTests(unittest.TestCase):
    """Git checkouts are never rewritten: Cargo.toml itself is the original."""
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.crate = self.root / "crate"
        self.crate.mkdir()
        (self.crate / "Cargo.toml").write_text(GIT_MANIFEST)
        self.helper = self.root / "helper"
        self.helper.mkdir()
        shutil.copytree(ROOT / "scripts/licensing/evidence", self.helper / "evidence")
        file_patch = patch.object(m, "__file__", str(self.helper / "cargo_notices.py"))
        file_patch.start()
        self.addCleanup(file_patch.stop)
        self.evidence = self.helper / "evidence/synthetic-git-0.1.0"
        self.evidence.mkdir()
        (self.evidence / "LICENSE").write_text("Synthetic upstream Apache text")
        self.record = {
            "schema": 1,
            "package": {"name": "synthetic-git", "version": "0.1.0",
                        "source": GIT_SOURCE, "checksum": None},
            "manifest_sha256": m.digest(self.crate / "Cargo.toml"),
            "documents": {"LICENSE": m.digest(self.evidence / "LICENSE")},
        }
        self.provenance = self.evidence / "NOTICE-provenance.json"
        self.provenance.write_text(json.dumps(self.record))
        self.package = dict(self.record["package"], id="git-synthetic-git-0.1.0",
                            license="Apache-2.0", manifest_path=str(self.crate / "Cargo.toml"))
        self.lock = self.root / "Cargo.lock"
        # Git lock entries carry no checksum key at all; keep it absent even
        # when a provenance record wrongly declares one.
        self.lock.write_text('version = 4\n[[package]]\n' + ''.join(
            f'{key} = {json.dumps(value)}\n' for key, value in self.record["package"].items()
            if key != "checksum"))

    def originals(self):
        return m.originals(self.package, [], self.lock)

    def test_git_manifest_is_the_original(self):
        files = self.originals()
        self.assertEqual({p.name for p in files}, {"LICENSE", "NOTICE-provenance.json"})

    def test_git_manifest_mismatch_is_rejected(self):
        (self.crate / "Cargo.toml").write_text(GIT_MANIFEST + "# changed\n")
        with self.assertRaisesRegex(ValueError, "original manifest mismatch"):
            self.originals()

    def test_git_missing_manifest_is_rejected(self):
        (self.crate / "Cargo.toml").unlink()
        with self.assertRaisesRegex(ValueError, "regular Cargo.toml"):
            self.originals()

    def test_git_orig_file_is_not_a_substitute(self):
        (self.crate / "Cargo.toml").rename(self.crate / "Cargo.toml.orig")
        with self.assertRaisesRegex(ValueError, "regular Cargo.toml"):
            self.originals()

    def test_git_provenance_checksum_must_stay_absent(self):
        self.record["package"]["checksum"] = "0" * 64
        self.provenance.write_text(json.dumps(self.record))
        with self.assertRaisesRegex(ValueError, "registry checksum mismatch"):
            self.originals()

    def test_git_unreviewed_version_still_fails(self):
        self.package["version"] = "0.1.1"
        with self.assertRaisesRegex(ValueError, "missing original legal evidence"):
            self.originals()

    def test_git_recorded_vendored_normalization_is_accepted(self):
        normalized = GIT_MANIFEST.replace('license = "Apache-2.0"',
                                          '[package.metadata.note]\nresolved = true\nlicense = "Apache-2.0"')
        (self.crate / "Cargo.toml").write_text(normalized)
        self.record["vendored_manifest_sha256"] = m.digest(self.crate / "Cargo.toml")
        self.provenance.write_text(json.dumps(self.record))
        files = self.originals()
        self.assertEqual({p.name for p in files}, {"LICENSE", "NOTICE-provenance.json"})

    def test_git_unrecorded_third_manifest_form_is_rejected(self):
        self.record["vendored_manifest_sha256"] = m.digest(self.crate / "Cargo.toml")
        self.provenance.write_text(json.dumps(self.record))
        (self.crate / "Cargo.toml").write_text(GIT_MANIFEST + "# third form\n")
        with self.assertRaisesRegex(ValueError, "original manifest mismatch"):
            self.originals()


if __name__ == "__main__":
    unittest.main()
