#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Georg Rybski
# SPDX-License-Identifier: Apache-2.0
"""Offline Rust notice generation. Mirrors contract v1 in Microsandbox.

The accepted license set comes from deny.toml, not another hand-maintained list.
Original license/NOTICE files are retained separately from cargo-about's report.
This does not certify native dependencies or fulfill corresponding-source duties.
"""
from __future__ import annotations

import argparse
import hashlib
import html
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib

LEGAL = re.compile(r"^(licen[cs]e|copying|notice|copyright|authors|unlicense)([._-].*)?$", re.I)
SKIP = {".git", "target", "node_modules", ".venv", "__pycache__"}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command: list[str]) -> str:
    return subprocess.run(command, check=True, text=True, stdout=subprocess.PIPE).stdout


def config(policy: Path, target: str) -> str:
    licenses = tomllib.loads(policy.read_text())["licenses"]
    allowed = licenses.get("allow", [])
    if not allowed or not all(isinstance(x, str) and x for x in allowed):
        raise ValueError("licenses.allow must be nonempty")
    if licenses.get("clarify"):
        raise ValueError("translate hash-bound clarifications explicitly; do not discard them")
    allowed = list(dict.fromkeys(allowed))
    if "Apache-2.0" in allowed:
        allowed.remove("Apache-2.0")
        allowed.insert(0, "Apache-2.0")
    result = (f"accepted = {json.dumps(allowed)}\ntargets = {json.dumps([target])}\n"
            "ignore-build-dependencies = false\nignore-dev-dependencies = true\n"
            "ignore-transitive-dependencies = false\nprivate = { ignore = false }\n")
    scoped = {}
    for exception in licenses.get("exceptions", []):
        # cargo-about scopes by name, not semver. Never widen a version restriction.
        if exception.get("version") != "*":
            raise ValueError("version-scoped exceptions require explicit graph-aware review")
        name, extra = exception["name"], exception["allow"]
        if not isinstance(name, str) or not name or not extra or not all(isinstance(x, str) for x in extra):
            raise ValueError("invalid scoped license exception")
        scoped.setdefault(name, []).extend(extra)
    for name, extra in sorted(scoped.items()):
        result += f"\n[{json.dumps(name)}]\naccepted = {json.dumps(list(dict.fromkeys(extra)))}\n"
    return result


def matching_package(reported: dict, packages: dict) -> dict:
    """Resolve report identities to metadata; never trust report source paths."""
    identity = reported["id"]
    if identity not in packages:
        raise ValueError(f"report contains an unknown package identity: {identity}")
    package = packages[identity]
    if (any(reported.get(key) != package.get(key) for key in ("name", "version", "source"))
            or Path(reported["manifest_path"]).resolve() != Path(package["manifest_path"]).resolve()):
        raise ValueError(f"report package metadata mismatch: {identity}")
    return package


def selected(metadata: dict, manifest: Path, report: dict) -> list[dict]:
    # cargo metadata describes a workspace-unified graph. Walking its normal /
    # build edges does not undo features enabled by other workspace members.
    # cargo-about's krates resolver already filters the requested root, target,
    # features and dependency kinds. Use its separate `crates` inventory, NOT
    # licenses[].used_by: missing license-text coverage must still fail below.
    packages = {p["id"]: p for p in metadata["packages"]}
    roots = [p["id"] for p in packages.values()
             if Path(p["manifest_path"]).resolve() == manifest.resolve()]
    if len(roots) != 1:
        raise ValueError("select a package manifest, not a virtual workspace")
    inventory = report.get("crates")
    if not isinstance(inventory, list) or not inventory:
        raise ValueError("cargo-about report has no crate inventory")
    chosen = {}
    for item in inventory:
        license_expression = item.get("license")
        if (not isinstance(license_expression, str) or not license_expression.strip()
                or license_expression in ("Unknown", "Ignore")):
            raise ValueError("cargo-about inventory contains an unresolved or ignored license")
        package = matching_package(item["package"], packages)
        if package["id"] in chosen:
            raise ValueError(f"duplicate report package identity: {package['id']}")
        chosen[package["id"]] = package
    if roots[0] not in chosen:
        raise ValueError("cargo-about inventory omitted the selected root package")
    result = sorted(chosen.values(), key=lambda p: (p["name"], p["version"]))
    if len({(p["name"], p["version"]) for p in result}) != len(result):
        raise ValueError("duplicate name/version from different sources requires identity-aware review")
    return result


def supplemental_originals(package: dict, directory: Path, lockfile: Path | None) -> list[Path]:
    """Use reviewed upstream files only for an exact registry archive/manifest."""
    name, version = package["name"], package["version"]
    if not re.fullmatch(r"[A-Za-z0-9_-]+", name) or not re.fullmatch(r"[A-Za-z0-9.+-]+", version):
        raise ValueError("unsafe supplemental package identity")
    evidence_root = Path(__file__).resolve().parent / "evidence"
    evidence = evidence_root / f"{name}-{version}"
    if evidence_root.is_symlink() or evidence.is_symlink():
        raise ValueError("supplemental evidence contains a symlink")
    if not evidence.exists():
        return []
    provenance = evidence / "NOTICE-provenance.json"
    if provenance.is_symlink():
        raise ValueError("supplemental provenance is a symlink")
    record = json.loads(provenance.read_text())
    if record.get("schema") != 1:
        raise ValueError("unknown supplemental evidence schema")
    approved = record["package"]
    if any(package.get(k) != approved[k] for k in ("name", "version", "source")):
        raise ValueError("supplemental evidence package identity mismatch")
    if lockfile is None:
        raise ValueError("supplemental evidence requires the exact Cargo.lock")
    locked = [entry for entry in tomllib.loads(lockfile.read_text()).get("package", [])
              if all(entry.get(k) == approved[k] for k in ("name", "version", "source"))]
    if len(locked) != 1 or locked[0].get("checksum") != approved["checksum"]:
        raise ValueError("supplemental evidence registry checksum mismatch")
    original_manifest = directory / "Cargo.toml.orig"
    if original_manifest.is_symlink() or not original_manifest.is_file():
        raise ValueError("supplemental evidence requires a regular Cargo.toml.orig")
    if digest(original_manifest) != record["manifest_sha256"]:
        raise ValueError("supplemental evidence original manifest mismatch")
    documents = record["documents"]
    if not isinstance(documents, dict) or not documents:
        raise ValueError("supplemental evidence has no original documents")
    files = [provenance]
    for filename, checksum in sorted(documents.items()):
        if (filename in (".", "..", provenance.name) or "/" in filename or "\\" in filename
                or not LEGAL.fullmatch(filename)):
            raise ValueError("unsafe supplemental evidence filename")
        path = evidence / filename
        if path.is_symlink() or not path.is_file() or not path.stat().st_size:
            raise ValueError("missing, empty or symlink supplemental evidence")
        if digest(path) != checksum:
            raise ValueError("supplemental evidence document checksum mismatch")
        files.append(path)
    return files


def originals(package: dict, roots: list[Path], lockfile: Path | None = None) -> list[Path]:
    directory = Path(package["manifest_path"]).resolve().parent
    candidates = set()
    for parent, dirs, files in os.walk(directory, followlinks=False):
        dirs[:] = sorted(d for d in dirs if d not in SKIP)
        candidates.update(Path(parent) / name for name in files if LEGAL.match(name))
    if package.get("license_file"):
        declared = Path(package["license_file"])
        declared = declared if declared.is_absolute() else directory / declared
        if not any(declared.resolve().is_relative_to(r) for r in [directory, *roots]):
            raise ValueError("declared license_file escapes approved source roots")
        candidates.add(declared)
    for parent in directory.parents:
        manifest = parent / "Cargo.toml"
        workspace = manifest.is_file() and "workspace" in tomllib.loads(manifest.read_text())
        if parent in roots or workspace:
            candidates.update(p for p in parent.iterdir() if LEGAL.match(p.name) and not p.is_dir())
        if parent in roots:
            break
    for path in candidates:
        if path.is_symlink() or not path.is_file() or not path.stat().st_size:
            raise ValueError(f"missing, empty or symlink legal evidence: {path}")
    if not candidates:
        candidates.update(supplemental_originals(package, directory, lockfile))
    if not candidates:
        raise ValueError(f"missing original legal evidence: {package['name']} {package['version']}")
    return sorted(candidates)


def verify(bundle: Path) -> None:
    manifest = json.loads((bundle / "manifest.json").read_text())
    if manifest.get("schema") != 1:
        raise ValueError("unknown notice manifest version")
    expected = manifest["files"]
    actual = set()
    for path in bundle.rglob("*"):
        if path.is_symlink():
            raise ValueError("notice bundle contains symlinks")
        if path.is_file():
            actual.add(path.relative_to(bundle).as_posix())
    if actual != set(expected) | {"manifest.json"}:
        raise ValueError("missing or unexpected notice files")
    if not {"THIRD-PARTY.html", "inventory.json", "cargo-about.json"} <= expected.keys():
        raise ValueError("required reports are missing")
    for name, checksum in expected.items():
        if Path(name).is_absolute() or ".." in Path(name).parts or "\\" in name:
            raise ValueError("unsafe notice path")
        if digest(bundle / name) != checksum:
            raise ValueError(f"notice checksum mismatch: {name}")
    crates = json.loads((bundle / "inventory.json").read_text())["crates"]
    if not crates or any(not p["legal_files"] or any(f not in expected for f in p["legal_files"]) for p in crates):
        raise ValueError("missing crate attribution")


def assemble(output: Path, packages: list[dict], report: dict, roots: list[Path], profile: dict,
             lockfile: Path | None = None) -> None:
    expected = {p["id"]: p for p in packages}
    covered, normalized = set(), []
    for item in report.get("licenses", []):
        if not item.get("text", "").strip():
            raise ValueError("empty license text")
        users = []
        for user in item["used_by"]:
            package = matching_package(user["crate"], expected)
            covered.add(package["id"])
            users.append({"name": package["name"], "version": str(package["version"])})
        normalized.append({"id": item["id"], "text": item["text"], "used_by": users})
    missing = set(expected) - covered
    if not normalized or missing:
        names = sorted((expected[i]["name"], str(expected[i]["version"])) for i in missing)
        raise ValueError(f"report omitted selected crates: {names}")
    if output.exists() or output.is_symlink():
        raise ValueError("refusing to overwrite notice bundle")
    # Collect all failures before publishing a bundle; do not make the operator
    # repeat a package build merely to discover the next missing notice.
    evidence, failures = {}, []
    for package in packages:
        try:
            evidence[package["id"]] = originals(package, roots, lockfile)
        except (OSError, ValueError, KeyError, TypeError) as error:
            failures.append(f"{package['name']} {package['version']}: {error}")
    if failures:
        raise ValueError("original legal evidence failures:\n  " + "\n  ".join(failures))
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=output.parent) as temporary:
        staging = Path(temporary) / "notices"
        staging.mkdir()
        inventory, sections = [], ["<h1>Third-party notices</h1>"]
        for package in packages:
            key = f"{package['name']}@{package['version']}#{package.get('source') or 'local'}"
            slug = hashlib.sha256(key.encode()).hexdigest()[:16]
            files = set()
            sections.append(f"<h2>{html.escape(key)}</h2>")
            for path in evidence[package["id"]]:
                name = f"evidence/{slug}/{digest(path)}-{path.name}"
                destination = staging / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(path.read_bytes())
                files.add(name)
                sections.append(f"<h3>{html.escape(path.name)}</h3><pre>{html.escape(path.read_text(errors='replace'))}</pre>")
            inventory.append({"name": package["name"], "version": package["version"],
                              "source": package.get("source"), "license": package.get("license"),
                              "legal_files": sorted(files)})
        for item in normalized:
            sections.append(f"<h2>{html.escape(item['id'])}</h2><pre>{html.escape(item['text'])}</pre>")
        (staging / "THIRD-PARTY.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>Notices</title><body>\n' + "\n".join(sections) + "\n</body></html>\n")
        (staging / "cargo-about.json").write_text(json.dumps({"licenses": normalized}, indent=2, sort_keys=True) + "\n")
        (staging / "inventory.json").write_text(json.dumps({"profile": profile, "crates": inventory}, indent=2, sort_keys=True) + "\n")
        files = {p.relative_to(staging).as_posix(): digest(p) for p in sorted(staging.rglob("*")) if p.is_file()}
        (staging / "manifest.json").write_text(json.dumps({"schema": 1, "files": files}, indent=2, sort_keys=True) + "\n")
        verify(staging)
        staging.rename(output)


def generate(args: argparse.Namespace) -> None:
    manifest = args.manifest.resolve(strict=True)
    roots = [p.resolve(strict=True) for p in args.source_root]
    flags = ["--manifest-path", str(manifest), "--locked", "--offline"]
    if args.no_default_features:
        flags.append("--no-default-features")
    if args.features:
        flags.extend(["--features", args.features])
    metadata = json.loads(run(["cargo", "metadata", "--format-version", "1", "--filter-platform", args.target, *flags]))
    lock = Path(metadata["workspace_root"]) / "Cargo.lock"
    before = digest(lock)
    with tempfile.TemporaryDirectory() as temporary:
        policy = Path(temporary) / "about.toml"
        policy.write_text(config(args.policy, args.target))
        command = ["cargo", "about", "generate", *flags, "--fail", "--format", "json", "--config", str(policy)]
        # cargo-about 0.9 includes local path/workspace crates in the graph.
        # private.ignore=false keeps unpublished crates; --include-local is not
        # a supported flag. Keep roots for original-file collection below, and
        # let assemble() reject any selected crate missing from the report.
        report = json.loads(run(command))
    if digest(lock) != before:
        raise ValueError("Cargo.lock changed during notice generation")
    profile = {"target": args.target, "features": args.features, "default_features": not args.no_default_features,
               "lock_sha256": before, "deny_sha256": digest(args.policy),
               "generator": run(["cargo", "about", "--version"]).strip(),
               "graph_source": "cargo-about.crates"}
    assemble(args.output, selected(metadata, manifest, report), report, roots, profile, lock)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    build = sub.add_parser("generate")
    for name in ("manifest", "policy", "output"):
        build.add_argument("--" + name, type=Path, required=True)
    build.add_argument("--target", required=True)
    build.add_argument("--source-root", type=Path, action="append", required=True)
    build.add_argument("--features", default="")
    build.add_argument("--no-default-features", action="store_true")
    sub.add_parser("verify").add_argument("bundle", type=Path)
    args = parser.parse_args()
    try:
        generate(args) if args.command == "generate" else verify(args.bundle)
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"licensing: {error}\n")


if __name__ == "__main__":
    main()
