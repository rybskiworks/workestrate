#!/usr/bin/env python3
"""Resolve the exact Rust release from the locked Fenix source, without Nix.

The lock graph is the authority. Network failure is fatal, never a fallback to
stable, a runner compiler, or a second hard-coded Rust version.
"""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import re
import urllib.request

SHARED = ('nixpkgs', 'fenix', 'flake-parts', 'devenv', 'treefmt-nix', 'git-hooks')
SHA = re.compile(r'[0-9a-f]{40}')


def node_at(lock: dict, path: tuple[str, ...], seen: frozenset = frozenset()) -> str:
    if path in seen:
        raise ValueError(f'Cycle in follows graph: {path}')
    current = lock['root']
    for name in path:
        edge = lock['nodes'][current]['inputs'][name]
        if isinstance(edge, list) and all(isinstance(x, str) for x in edge):
            current = node_at(lock, tuple(edge), seen | {path})
        elif isinstance(edge, str):
            current = edge
        else:
            raise ValueError(f'Invalid input edge: {name}')
    if current not in lock['nodes']:
        raise ValueError(f'Missing lock node: {current}')
    return current


def github_pin(lock: dict, path: tuple[str, ...], owner: str, repo: str) -> str:
    node = lock['nodes'][node_at(lock, path)]
    locked, original = node['locked'], node['original']
    for item in (locked, original):
        if (item.get('type'), item.get('owner'), item.get('repo')) != ('github', owner, repo):
            raise ValueError(f'Unexpected supplier for {path}')
    rev = locked.get('rev', '')
    if not SHA.fullmatch(rev) or original.get('rev') != rev:
        raise ValueError(f'{path} must use the same immutable revision in original and locked')
    if not re.fullmatch(r'sha256-[A-Za-z0-9+/]{43}=', locked.get('narHash', '')):
        raise ValueError(f'{path} is missing its NAR integrity hash')
    return rev


def validate(lock: dict, flake: str, role: str) -> dict[str, str]:
    if lock.get('version') != 7:
        raise ValueError('Unsupported lockfile format')
    result = {}
    if role == 'consumer':
        result['tooling_rev'] = github_pin(lock, ('tooling',), 'rybskiworks', 'nix-tooling')
        declared = re.findall(r'tooling\.url\s*=\s*"github:rybskiworks/nix-tooling/([0-9a-f]{40})"\s*;', flake)
        if declared != [result['tooling_rev']]:
            raise ValueError('flake.nix and flake.lock must select the same tooling revision')
        root_inputs = lock['nodes'][lock['root']]['inputs']
        for name in SHARED:
            if root_inputs.get(name) != ['tooling', name]:
                raise ValueError(f'{name} must follow tooling/{name}, not own another pin')
            pattern = rf'{re.escape(name)}\.follows\s*=\s*"tooling/{re.escape(name)}"\s*;'
            if len(re.findall(pattern, flake)) != 1:
                raise ValueError(f'flake.nix must follow tooling/{name}')
        msb = lock['nodes'][node_at(lock, ('microsandbox-fork',))]
        if msb['inputs'].get('tooling') != ['tooling']:
            raise ValueError('Microsandbox must share the root tooling authority')
        result['microsandbox_rev'] = github_pin(lock, ('microsandbox-fork',), 'rybskiworks', 'microsandbox')
        fenix_path = ('tooling', 'fenix')
    elif role == 'supplier':
        fenix_path = ('fenix',)
    else:
        raise ValueError('Expected supplier or consumer')
    result['fenix_rev'] = github_pin(lock, fenix_path, 'nix-community', 'fenix')
    return result


def rust_version(manifest: dict) -> str:
    target = manifest['pkg']['rustc']['target']['x86_64-unknown-linux-gnu']
    if target.get('available') is not True:
        raise ValueError('The selected Linux Rust compiler is unavailable')
    match = re.fullmatch(
        r'https://static\.rust-lang\.org/dist/(\d{4}-\d{2}-\d{2})/rustc-'
        r'((?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*))'
        r'-x86_64-unknown-linux-gnu\.tar\.gz', target['url'])
    if not match or manifest.get('date') != match[1]:
        raise ValueError('Expected a dated, exact stable Rust release')
    return match[2]


def resolve_manifest(rev: str) -> dict:
    if not SHA.fullmatch(rev):
        raise ValueError('Invalid Fenix revision')
    # Fixed host, supplier, path and immutable ref. No credentials are sent.
    url = f'https://raw.githubusercontent.com/nix-community/fenix/{rev}/data/stable.json'
    with urllib.request.urlopen(url, timeout=30) as response:
        if response.geturl() != url:
            raise ValueError('Unexpected manifest redirect')
        data = response.read(4 * 1024 * 1024 + 1)
    if len(data) > 4 * 1024 * 1024:
        raise ValueError('Oversized Fenix manifest')
    return json.loads(data)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['check', 'resolve'])
    parser.add_argument('--role', required=True, choices=['supplier', 'consumer'])
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    pins = validate(json.loads((root / 'flake.lock').read_text()), (root / 'flake.nix').read_text(), args.role)
    if args.command == 'resolve':
        pins['rust_version'] = rust_version(resolve_manifest(pins['fenix_rev']))
    if output := os.environ.get('GITHUB_OUTPUT'):
        with open(output, 'a', encoding='utf-8') as stream:
            stream.writelines(f'{key}={value}\n' for key, value in pins.items())
    print(json.dumps(pins, sort_keys=True))


if __name__ == '__main__':
    main()
