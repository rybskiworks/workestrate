---
name: validation-rust-format
description: |
  Verifies Rust code formatting is clean per rustfmt. Load as part of the
  validation suite. Does NOT cover lint or compile validation.
metadata:
  org.kind: validation
---

# Validation: Rust Format

This gate verifies that the code is formatted per `rustfmt` without producing
a diff. It does not modify files; it only checks.

## Triggers

Load this skill when:

- As part of the standard validation suite.
- After changing any `.rs` file.
- Before committing or claiming completion.

## Command

```bash
cargo fmt --all -- --check
```

## Pass criteria

- Exit code 0.
- No formatting diff.

## Fail criteria

- Exit code non-zero.
- A formatting diff exists (rustfmt would change the file).

## Evidence to report

- Exit code.
- Diff output, if any (file, line, the suggested change).

## Notes

- `--all` formats the entire workspace.
- `--check` reports without modifying files. Do NOT run `cargo fmt` without
  `--check` in validation — that modifies files and is not a validation step.
- If the gate fails, run `cargo fmt --all` (without `--check`) to fix, then
  re-run `cargo fmt --all -- --check` to confirm.
- Check for a `rustfmt.toml` / `.rustfmt.toml` in the project root for
  non-default style settings.
- Run inside `just shell` (see `nix-usage` skill) so the correct `rustfmt`
  is used.
