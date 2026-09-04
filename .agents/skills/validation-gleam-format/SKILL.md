---
name: validation-gleam-format
description: |
  Verifies Gleam code formatting is clean per gleam format. Load as part of the
  validation suite. Does NOT cover type-checking (see validation-gleam-check) or
  tests (see validation-gleam-test).
metadata:
  org.kind: validation
---

# Validation: Gleam Format

This gate verifies that the code is formatted per `gleam format` without producing
a diff. It does not modify files; it only checks.

## Triggers

Load this skill when:

- As part of the standard validation suite.
- After changing any `.gleam` file.
- Before committing or claiming completion.

## Command

```bash
gleam format --check
```

## Pass criteria

- Exit code 0.
- No formatting diff.

## Fail criteria

- Exit code non-zero.
- A formatting diff exists (gleam format would change the file).

## Evidence to report

- Exit code.
- List of unformatted files.

## Notes

- `--check` reports without modifying files. Do NOT run `gleam format` without
  `--check` in validation — that rewrites files and is not a validation step.
- If the gate fails, run `gleam format` (without `--check`) to fix, then re-run
  `gleam format --check` to confirm.
- The compiler enforces `snake_case`/`PascalCase`; `gleam format` enforces the
  remaining layout.
- Run inside `just shell` (see `nix-usage` skill) so the correct `gleam` is used.
