---
name: validation-rust-test
description: |
  Verifies all Rust tests pass (unit, integration, doctests). Load after
  compile and clippy validation pass. Does NOT cover compile or lint
  validation.
metadata:
  org.kind: validation
---

# Validation: Rust Test

This gate verifies that all tests pass, including unit tests, integration
tests, and doctests. It runs after `validation-rust-compile` and
`validation-rust-clippy` pass.

## Triggers

Load this skill when:

- After `validation-rust-compile` and `validation-rust-clippy` pass.
- As part of the standard validation suite.
- After changing code that affects behavior, fixtures, or doctests.

## Command

```bash
cargo test --all-features
cargo test --doc
```

Run both commands; the second gives a separate doctest signal.

## Pass criteria

- All tests pass.
- Exit code 0 for both commands.

## Fail criteria

- Any test fails.
- Exit code non-zero for either command.

## Evidence to report

- Exit code for each command.
- Passed / failed counts.
- Failing test names and their output (assertion message, panic location).

## Notes

- `--all-features` exercises the full feature matrix. If the workspace has
  mutually exclusive features, run `cargo test --features <f>` per feature set
  instead.
- `--doc` runs doctests separately so a doctest failure is distinguishable
  from a unit/integration failure.
- For unsafe code, also run `cargo miri test` (see
  `constraint-rust-unsafe-safety`).
- For flaky/timing tests, run with `-- --test-threads=1` or repeat with
  `-- --test-threads=1 --count=10` to surface races.
- Run inside `just shell` (see `nix-usage` skill).
