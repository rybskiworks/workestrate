---
name: validation-gleam-test
description: |
  Verifies all Gleam tests pass (gleeunit). Load after check validation passes.
  Does NOT cover type-checking (see validation-gleam-check) or formatting (see
  validation-gleam-format).
metadata:
  org.kind: validation
---

# Validation: Gleam Test

This gate verifies that all Gleam tests pass. It runs after
`validation-gleam-check` passes. Gleam tests use `assert` (bool, for test code)
and live under `test/` as `*_test` functions (via `gleeunit`).

## Triggers

Load this skill when:

- After `validation-gleam-check` passes.
- As part of the standard validation suite.
- After changing code that affects behavior.

## Command

```bash
gleam test
```

To validate a specific target, add `--target erlang` or `--target javascript`.

## Pass criteria

- All tests pass.
- Exit code 0.

## Fail criteria

- Any test fails.
- Exit code non-zero.

## Evidence to report

- Exit code.
- Passed / failed counts.
- Failing test names and their output (assertion message, file, line).

## Notes

- Gleam tests use `assert` (bool) — this is the TEST-only crash boundary; do not
  use `assert` in library/application code (see `constraint-gleam-result`).
- If the project targets both Erlang and JavaScript, run
  `gleam test --target erlang` and `gleam test --target javascript` separately
  to validate both targets.
- `gleam test` runs `gleeunit` by default; ensure `gleeunit` is a dev dependency
  in `gleam.toml`.
- Run inside `just shell` (see `nix-usage` skill) so the correct Gleam toolchain
  is used.
