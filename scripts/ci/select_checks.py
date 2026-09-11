#!/usr/bin/env python3
"""Select CI tiers. Unknown changes select checks; no workflow-level path filters."""
from __future__ import annotations
import json
import os
from pathlib import Path
import re
import subprocess


def select(event_name: str, event: dict, paths: list[str] | None) -> dict[str, str]:
    def documentation_only(path: str) -> bool:
        return path.startswith('docs/') and not path.startswith('docs/migration/')
    code = event_name == 'workflow_dispatch' or paths is None or not paths or any(
        not documentation_only(path) for path in paths
    )
    labels = event.get('pull_request', {}).get('labels', [])
    nix = (event_name == 'workflow_dispatch' or (event_name == 'push' and code)
           or any(x.get('name') == 'nix-ci' for x in labels))
    return {key: str(value).lower() for key, value in {
        'code': code, 'nix': nix, 'build': event_name == 'workflow_dispatch'
    }.items()}


def changed_paths(event_name: str, event: dict) -> list[str] | None:
    if event_name == 'pull_request':
        pr = event['pull_request']
        base, head = pr['base']['sha'], pr['head']['sha']
        comparison = f'{base}...{head}'
    elif event_name == 'push':
        base, head = event.get('before', ''), event.get('after', '')
        comparison = f'{base}..{head}'
    else:
        return None
    if any(not re.fullmatch('[0-9a-f]{40}', ref) or set(ref) == {'0'} for ref in (base, head)):
        return None
    try:
        output = subprocess.check_output(['git', 'diff', '--name-only', '-z', comparison, '--'],
                                         stderr=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
        return None
    return [x.decode('utf-8', 'surrogateescape') for x in output.split(b'\0') if x]


if __name__ == '__main__':
    event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    name = os.environ['GITHUB_EVENT_NAME']
    result = select(name, event, changed_paths(name, event))
    with open(os.environ['GITHUB_OUTPUT'], 'a') as stream:
        stream.writelines(f'{key}={value}\n' for key, value in result.items())
    print(json.dumps(result, sort_keys=True))
