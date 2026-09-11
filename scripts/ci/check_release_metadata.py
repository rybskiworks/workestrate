#!/usr/bin/env python3
"""Validate the existing Cargo authority and its Nix/lockfile consumers. No publishing."""
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def validate(root: Path = ROOT) -> str:
    cargo = tomllib.loads((root / 'control/agentctl/Cargo.toml').read_text())
    version = cargo['package']['version']
    # Repository policy intentionally disallows SemVer build metadata in release tags.
    numeric = r'(?:0|[1-9][0-9]*)'
    semver = rf'{numeric}\.{numeric}\.{numeric}(?:-(?:alpha|beta|rc)\.{numeric})?'
    if re.fullmatch(semver, version) is None:
        raise ValueError(f'Unsupported release version: {version!r}')
    lock = tomllib.loads((root / 'control/agentctl/Cargo.lock').read_text())
    own = [p for p in lock['package'] if p['name'] == cargo['package']['name'] and 'source' not in p]
    if len(own) != 1 or own[0]['version'] != version:
        raise ValueError('Cargo.lock must record exactly one matching local Workestrate version')
    nix = (root / 'nix/packages/agentctl.nix').read_text()
    if 'version = manifest.package.version;' not in nix:
        raise ValueError('The Nix package must derive its version from Cargo metadata')
    return version


if __name__ == '__main__':
    print(f'Workestrate source version: {validate()} (not evidence of a published release)')
