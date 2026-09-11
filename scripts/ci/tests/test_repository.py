import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
SOURCE = HERE.parent if (HERE.parent / 'check_repository.py').exists() else HERE
spec = importlib.util.spec_from_file_location('check_repository', SOURCE / 'check_repository.py')
repository = importlib.util.module_from_spec(spec)
spec.loader.exec_module(repository)


class RepositoryTests(unittest.TestCase):
    def test_rejects_local_and_retired_artifacts(self):
        paths = ['NEXT-SESSION.md', 'STATUS.md', 'target/binary', '.devenv/state',
                 'result', 'result-check', 'scripts/cache.pyc']
        self.assertEqual(repository.artifacts(paths), paths)

    def test_preserves_specs_fixtures_and_tracker_exports(self):
        paths = ['SPEC.md', '.beads/issues.jsonl', '.beads/continuation.md', 'tests/fixtures/output.json',
                 'docs/validation-and-improvements/STATUS.md', 'LICENSE-MIT', 'schemas/workload.json']
        self.assertEqual(repository.artifacts(paths), [])

    def test_remote_actions_require_immutable_shas(self):
        for ref in ['main', 'v7', 'latest', '1234']:
            self.assertTrue(repository.workflow_errors(f'  - uses: actions/checkout@{ref}\n'))
        self.assertEqual(repository.workflow_errors('  - uses: actions/checkout@' + 'a' * 40 + '\n'), [])
        self.assertEqual(repository.workflow_errors('  uses: org/repo/.github/workflows/test.yml@' + 'a' * 40 + '\n'), [])
        self.assertEqual(repository.workflow_errors('  - uses: ./.github/actions/local\n'), [])

    def test_branch_and_credential_drift(self):
        self.assertTrue(repository.workflow_errors('base: migration/tool-model'))
        self.assertTrue(repository.workflow_errors('persist-credentials: true'))
        self.assertEqual(repository.workflow_errors('base: main\npersist-credentials: false'), [])


if __name__ == '__main__':
    unittest.main()
