#!/usr/bin/env python3
"""Small offline repository invariants, not a substitute for actionlint or secret scanning."""
from pathlib import Path, PurePosixPath
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
RETIRED = {'NEXT-SESSION.md', 'NEXT-SESSION-fleet-tool-refactor.md', 'STATUS.md'}
LOCAL_ROOTS = {'.devenv', '.direnv', '.toolchain', '.tmp', 'target', 'node_modules', '__pycache__'}


def artifacts(paths: list[str]) -> list[str]:
    rejected = []
    for name in paths:
        path = PurePosixPath(name)
        first = path.parts[0]
        if (name in RETIRED or first in LOCAL_ROOTS or first == 'result'
                or first.startswith('result-') or path.suffix == '.pyc'):
            rejected.append(name)
    return rejected


def workflow_errors(text: str) -> list[str]:
    errors = []
    for line in text.splitlines():
        match = re.match(r'^\s*(?:-\s*)?uses:\s*([^\s#]+)', line)
        if match:
            action = match[1].strip('\"\'')
            if not action.startswith('./') and not re.fullmatch(r'[A-Za-z0-9_./-]+@[0-9a-f]{40}', action):
                errors.append(f'Remote action or reusable workflow is not SHA-pinned: {action}')
    if re.search(r'persist-credentials:\s*true\b', text):
        errors.append('Checkout must not persist repository credentials')
    if 'migration/tool-model' in text:
        errors.append('Active workflow must not target the merged migration branch')
    return errors


def check(root: Path = ROOT) -> None:
    paths = subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).decode().split('\0')
    bad = artifacts([p for p in paths if p])
    if bad:
        raise ValueError(f'Tracked machine-local or retired artifacts: {bad}')
    for path in sorted((root / '.github/workflows').glob('*.y*ml')):
        if errors := workflow_errors(path.read_text()):
            raise ValueError(f'{path.name}: {errors}')
    owners = (root / '.github/CODEOWNERS').read_text()
    active = '\n'.join(x for x in owners.splitlines() if not x.lstrip().startswith('#'))
    if re.search(r'@rybskiworks(?:\s|$)', active):
        raise ValueError('A bare organization is not a CODEOWNER; use a user or visible team')
    if 'migration/tool-model' in (root / '.github/dependabot.yml').read_text():
        raise ValueError('Dependabot must target the default branch after migration integration')
    print('Repository hygiene, active branch targets and immutable action references checked.')


if __name__ == '__main__':
    check()
