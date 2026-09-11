import copy
import contextlib
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / f'{name}.py')
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module
selection, gate, metadata = map(load, ['select_checks', 'check_results', 'check_release_metadata'])

class SelectionTests(unittest.TestCase):
    def test_docs_only(self):
        self.assertEqual(selection.select('pull_request', {}, ['docs/design.md']),
                         {'code': 'false', 'nix': 'false', 'build': 'false'})
    def test_unknown_missing_and_empty_changes_fail_closed(self):
        for files in [None, [], ['new-directory/test.rs'], ['.github/workflows/ci.yml'], ['docs/migration/state.md'], ['README.md']]:
            self.assertEqual(selection.select('pull_request', {}, files)['code'], 'true')
    def test_label_selects_nix_even_for_docs(self):
        event = {'pull_request': {'labels': [{'name': 'nix-ci'}]}}
        self.assertEqual(selection.select('pull_request', event, ['docs/design.md'])['nix'], 'true')
    def test_manual_runs_every_tier(self):
        self.assertEqual(set(selection.select('workflow_dispatch', {}, []).values()), {'true'})
    def test_push_runs_nix_for_code_not_normal_docs(self):
        self.assertEqual(selection.select('push', {}, ['flake.lock'])['nix'], 'true')
        self.assertEqual(selection.select('push', {}, ['docs/design.md'])['nix'], 'false')

    def renamed_paths(self, old, new):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            environment = {'PATH': os.defpath, 'HOME': str(root), 'LANG': 'C',
                           'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': os.devnull}
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=root, env=environment,
                                               stderr=subprocess.PIPE, timeout=5)
            git('init', '-q')
            git('config', 'user.name', 'CI test fixture')
            git('config', 'user.email', 'ci-fixture@example.invalid')
            git('config', 'commit.gpgsign', 'false')
            git('config', 'core.hooksPath', os.devnull)
            git('config', 'diff.renames', 'true')
            source, destination = root / old, root / new
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text('unchanged content for exact rename detection\n')
            git('add', '--', old)
            git('commit', '-qm', 'initial fixture')
            base = git('rev-parse', 'HEAD').decode().strip()
            destination.parent.mkdir(parents=True, exist_ok=True)
            git('mv', '--', old, new)
            git('commit', '-qm', 'rename fixture')
            head = git('rev-parse', 'HEAD').decode().strip()
            # Prove this fixture exercises the post-image-only rename case.
            self.assertEqual(git('diff', '--name-only', '-z', f'{base}..{head}', '--'),
                             new.encode() + b'\0')
            events = {'pull_request': {'pull_request': {'base': {'sha': base}, 'head': {'sha': head}}},
                      'push': {'before': base, 'after': head}}
            with contextlib.chdir(root), patch.dict(os.environ, environment, clear=True):
                return {name: selection.changed_paths(name, event) for name, event in events.items()}

    def test_code_and_config_renames_to_docs_preserve_source_paths(self):
        for old, new in [('control/agentctl/src/lib.rs', 'docs/lib.rs'),
                         ('flake.nix', 'docs/archived-flake.nix')]:
            with self.subTest(old=old):
                for event, paths in self.renamed_paths(old, new).items():
                    self.assertEqual(set(paths), {old, new})
                    result = selection.select(event, {}, paths)
                    self.assertEqual(result['code'], 'true')
                    self.assertEqual(result['nix'], 'true' if event == 'push' else 'false')

    def test_docs_only_rename_keeps_compilation_unselected(self):
        for event, paths in self.renamed_paths('docs/design.md', 'docs/overview.md').items():
            self.assertEqual(selection.select(event, {}, paths),
                             {'code': 'false', 'nix': 'false', 'build': 'false'})

class GateTests(unittest.TestCase):
    def needs(self, code='true', nix='false', build='false'):
        flags = dict(code=code, nix=nix, build=build)
        result = {x: {'result': 'success'} for x in gate.ALWAYS}
        result['changes']['outputs'] = flags
        result.update({x: {'result': 'success' if flags[y] == 'true' else 'skipped'} for x,y in gate.CONDITIONAL.items()})
        return result
    def test_successful_code_and_docs_contracts(self):
        for flags in [('true','false','false'), ('false','false','false'), ('true','true','true')]:
            gate.validate(self.needs(*flags))
    def test_each_failed_cancelled_missing_or_skipped_required_job_blocks(self):
        for name in gate.ALWAYS | {'gates','deny'}:
            for status in ['failure','cancelled','skipped',None]:
                needs = self.needs(); needs[name]['result'] = status
                with self.assertRaises(ValueError, msg=(name,status)): gate.validate(needs)
    def test_selected_nix_and_build_may_not_skip(self):
        for job in ['e2e-nix','heavy-build']:
            needs = self.needs('true','true','true'); needs[job]['result'] = 'skipped'
            with self.assertRaises(ValueError): gate.validate(needs)
    def test_an_unselected_but_failed_job_still_blocks(self):
        needs = self.needs(); needs['e2e-nix']['result'] = 'failure'
        with self.assertRaises(ValueError): gate.validate(needs)
    def test_missing_dependency_and_bad_flag(self):
        needs = self.needs(); del needs['zizmor']
        with self.assertRaises(ValueError): gate.validate(needs)
        needs = self.needs(); needs['changes']['outputs']['code'] = 'unknown'
        with self.assertRaises(ValueError): gate.validate(needs)

class MetadataTests(unittest.TestCase):
    def fixture(self, root, version='0.1.0', locked='0.1.0'):
        (root/'control/agentctl').mkdir(parents=True)
        (root/'nix/packages').mkdir(parents=True)
        (root/'control/agentctl/Cargo.toml').write_text(f'[package]\nname="workestrate"\nversion="{version}"\n')
        (root/'control/agentctl/Cargo.lock').write_text(f'[[package]]\nname="workestrate"\nversion="{locked}"\n')
        (root/'nix/packages/agentctl.nix').write_text('version = manifest.package.version;\n')
    def test_version_authority(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); self.fixture(root)
            self.assertEqual(metadata.validate(root),'0.1.0')
    def test_lock_mismatch(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); self.fixture(root,locked='0.0.9')
            with self.assertRaises(ValueError): metadata.validate(root)
    def test_invalid_version(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); self.fixture(root,version='01.2.3')
            with self.assertRaises(ValueError): metadata.validate(root)

if __name__ == '__main__': unittest.main()
