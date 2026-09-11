"""Offline contract tests: the supplier lock selects the compiler, not CI YAML."""
import copy
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SOURCE = HERE.parent if (HERE.parent / 'toolchain.py').exists() else HERE
spec = importlib.util.spec_from_file_location('toolchain', SOURCE / 'toolchain.py')
toolchain = importlib.util.module_from_spec(spec)
spec.loader.exec_module(toolchain)


def pin(owner, repo, revision):
    original = dict(type='github', owner=owner, repo=repo, rev=revision)
    return {'original': original, 'locked': dict(original, narHash='sha256-' + 'A' * 43 + '=')}


def fixture():
    tooling = pin('rybskiworks', 'nix-tooling', '1' * 40)
    tooling['inputs'] = {name: 'shared-' + name for name in toolchain.SHARED}
    runtime = pin('rybskiworks', 'microsandbox', '2' * 40)
    runtime['inputs'] = {'tooling': ['tooling']}
    nodes = {
        'root': {'inputs': dict({name: ['tooling', name] for name in toolchain.SHARED},
                               tooling='renamed-tooling', **{'microsandbox-fork': 'runtime'})},
        'renamed-tooling': tooling,
        'runtime': runtime,
        'shared-fenix': pin('nix-community', 'fenix', '3' * 40),
    }
    nodes.update({f'shared-{name}': {} for name in toolchain.SHARED if name != 'fenix'})
    flake = 'tooling.url = "github:rybskiworks/nix-tooling/' + '1' * 40 + '";\n'
    flake += '\n'.join(f'{name}.follows = "tooling/{name}";' for name in toolchain.SHARED)
    return {'root': 'root', 'version': 7, 'nodes': nodes}, flake


class ToolchainTests(unittest.TestCase):
    def test_consumer_and_renamed_lock_nodes(self):
        lock, flake = fixture()
        self.assertEqual(toolchain.validate(lock, flake, 'consumer'),
                         {'tooling_rev': '1' * 40, 'microsandbox_rev': '2' * 40, 'fenix_rev': '3' * 40})

    def test_supplier(self):
        lock, _ = fixture()
        lock['nodes']['root']['inputs'] = {'fenix': 'shared-fenix'}
        self.assertEqual(toolchain.validate(lock, '', 'supplier'), {'fenix_rev': '3' * 40})

    def test_follows_drift_for_each_shared_input(self):
        for name in toolchain.SHARED:
            lock, flake = fixture()
            lock['nodes']['root']['inputs'][name] = 'shared-' + name
            with self.assertRaises(ValueError):
                toolchain.validate(lock, flake, 'consumer')

    def test_runtime_must_share_tooling(self):
        lock, flake = fixture()
        lock['nodes']['runtime']['inputs']['tooling'] = 'another-tooling'
        with self.assertRaises(ValueError):
            toolchain.validate(lock, flake, 'consumer')

    def test_rejects_mismatched_flake_and_lock(self):
        lock, flake = fixture()
        with self.assertRaises(ValueError):
            toolchain.validate(lock, flake.replace('1' * 40, '4' * 40), 'consumer')

    def test_rejects_mutable_pin_wrong_supplier_and_missing_integrity(self):
        for section, key, value in [('original', 'rev', 'main'), ('locked', 'rev', 'stable'),
                                    ('locked', 'owner', 'someone-else'), ('original', 'repo', 'other'),
                                    ('locked', 'narHash', '')]:
            lock, flake = fixture()
            lock['nodes']['shared-fenix'][section][key] = value
            with self.assertRaises(ValueError):
                toolchain.validate(lock, flake, 'consumer')

    def test_cycle_is_rejected(self):
        lock, _ = fixture()
        lock['nodes']['root']['inputs']['cycle'] = ['cycle']
        with self.assertRaises(ValueError):
            toolchain.node_at(lock, ('cycle',))

    def test_missing_node_is_rejected(self):
        lock, _ = fixture()
        lock['nodes']['root']['inputs']['missing'] = 'absent'
        with self.assertRaises(ValueError):
            toolchain.node_at(lock, ('missing',))

    def manifest(self):
        return {'date': '2026-07-16', 'pkg': {'rustc': {'target': {'x86_64-unknown-linux-gnu': {
            'available': True, 'url': 'https://static.rust-lang.org/dist/2026-07-16/rustc-1.97.1-x86_64-unknown-linux-gnu.tar.gz'}}}}}

    def test_exact_patch_version_is_from_manifest(self):
        data = self.manifest()
        self.assertEqual(toolchain.rust_version(data), '1.97.1')
        data['pkg']['rustc']['target']['x86_64-unknown-linux-gnu']['url'] = data['pkg']['rustc']['target']['x86_64-unknown-linux-gnu']['url'].replace('1.97.1', '1.97.2')
        self.assertEqual(toolchain.rust_version(data), '1.97.2')

    def test_bad_manifest_fails_closed(self):
        for old, new in [('1.97.1', 'nightly'), ('https://static.rust-lang.org', 'https://example.invalid'),
                         ('2026-07-16/rustc', '2026-07-17/rustc')]:
            data = self.manifest()
            target = data['pkg']['rustc']['target']['x86_64-unknown-linux-gnu']
            target['url'] = target['url'].replace(old, new)
            with self.assertRaises(ValueError):
                toolchain.rust_version(data)
        data = self.manifest()
        data['pkg']['rustc']['target']['x86_64-unknown-linux-gnu']['available'] = False
        with self.assertRaises(ValueError):
            toolchain.rust_version(data)

    def test_network_failure_is_not_a_toolchain_fallback(self):
        with patch.object(toolchain.urllib.request, 'urlopen', side_effect=OSError('offline')):
            with self.assertRaises(OSError):
                toolchain.resolve_manifest('3' * 40)
        with self.assertRaises(ValueError):
            toolchain.resolve_manifest('main')


if __name__ == '__main__':
    unittest.main()
