#!/usr/bin/env python3
"""Fail closed on missing, failed, cancelled, or unexpectedly skipped CI jobs."""
from __future__ import annotations
import json
import os

ALWAYS = {'changes', 'policy', 'actionlint', 'zizmor'}
CONDITIONAL = {'gates': 'code', 'deny': 'code', 'e2e-nix': 'nix', 'heavy-build': 'build'}


def validate(needs: dict) -> None:
    if set(needs) != ALWAYS | set(CONDITIONAL):
        raise ValueError('Missing or unknown CI dependency; update the explicit gate contract')
    outputs = needs['changes'].get('outputs', {})
    if set(outputs) != {'code', 'nix', 'build'} or any(x not in {'true', 'false'} for x in outputs.values()):
        raise ValueError('Missing or invalid CI selection outputs')
    for name, job in needs.items():
        required = name in ALWAYS or outputs[CONDITIONAL[name]] == 'true'
        status = job.get('result')
        if status != 'success' and not (status == 'skipped' and not required):
            raise ValueError(f'{name}: required={required}, result={status!r}')


if __name__ == '__main__':
    try:
        validate(json.loads(os.environ['NEEDS_JSON']))
    except (ValueError, KeyError, TypeError) as error:
        raise SystemExit(f'CI gate failed: {error}')
    print('All required checks succeeded; only explicitly unselected tiers may be skipped.')
