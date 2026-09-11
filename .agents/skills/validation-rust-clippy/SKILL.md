---
name: validation-rust-clippy
description: |
  Verifies Rust code passes Clippy lint checks. Load after compile validation
  passes. Does NOT cover compilation errors (see validation-rust-compile) or
  formatting (see validation-rust-format).
metadata:
  org.kind: validation
---

# Validation: Rust Clippy

This gate verifies that the code passes Clippy's lint suite with warnings
denied. It runs after `validation-rust-compile` passes and before tests.

## Triggers

Load this skill when:

- After `validation-rust-compile` passes.
- As part of the standard validation suite.
- When diagnosing whether a failure is a lint violation vs. a compile/test
  failure.

## Command

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

## Pass criteria

- Exit code 0.
- No warnings (all warnings treated as errors via `-D warnings`).

## Fail criteria

- Exit code non-zero.
- Lint warnings reported, treated as errors.

## Evidence to report

- Exit code.
- Warning count.
- Specific lints triggered (lint name, file, line, message).

## Notes

- `--workspace` covers all workspace members.
- `--all-targets` covers lib, bins, tests, examples, and benches.
- `-D warnings` treats all warnings as errors; do not run clippy in validation
  without it.
- Check the `[lints]` table in `Cargo.toml` and any `#![deny(...)]` /
  `#![warn(...)]` attributes in crate roots for configured lint levels — these
  override command-line flags.
- Project-relevant lints include: `needless_clone`, `redundant_clone`,
  `clone_on_copy`, `ptr_arg`, `unwrap_used`, `expect_used`,
  `missing_errors_doc`, `missing_panics_doc`, `missing_safety_doc`,
  `rc_mutex`, `arc_with_non_send_sync`, `await_holding_lock`.
- Run inside `just shell` (see `nix-usage` skill).
