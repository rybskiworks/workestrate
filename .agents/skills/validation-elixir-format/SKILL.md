---
name: validation-elixir-format
description: |
  Verifies Elixir code formatting is clean per mix format. Load as part of the
  validation suite. Does NOT cover lint or compile validation.
metadata:
  org.kind: validation
---

# Validation: Elixir Format

This gate verifies that the code is formatted per `mix format` without producing
a diff. It does not modify files; it only checks.

## Triggers

Load this skill when:

- As part of the standard validation suite.
- After changing any `.ex`/`.exs` file.
- Before committing or claiming completion.

## Command

```bash
mix format --check-formatted
```

## Pass criteria

- Exit code 0.
- No formatting diff.

## Fail criteria

- Exit code non-zero.
- A formatting diff exists (mix format would change the file).

## Evidence to report

- Exit code.
- List of unformatted files.

## Notes

- `--check-formatted` reports without modifying files. Do NOT run `mix format`
  without `--check-formatted` in validation — that modifies files and is not a
  validation step.
- If the gate fails, run `mix format` (without `--check-formatted`) to fix, then
  re-run `mix format --check-formatted` to confirm.
- Check `.formatter.exs` for non-default formatting rules (e.g. import_deps,
  inputs, locals_without_parens).
- Run inside `nix develop` (see `nix-usage` skill) so the correct `mix` is used.
