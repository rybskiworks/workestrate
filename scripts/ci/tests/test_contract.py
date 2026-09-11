import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

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
