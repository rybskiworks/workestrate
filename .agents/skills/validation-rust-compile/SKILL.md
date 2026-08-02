---
name: validation-rust-compile
description: |
  Verifies Rust code compiles without errors. Load as the minimum validation
  gate after any code change. Does NOT cover lint, test, format, or doc
  validation.
metadata:
  org.kind: validation
---

# Validation: Rust Compile

This is the minimum validation gate. It verifies that the code compiles across
all targets without errors. It does not run lints, tests, formatting, or doc
builds — those are separate gates.

## Triggers

Load this skill when:

- After any code change, before claiming completion.
- As the first gate in a validation suite (before clippy, tests, format, docs).
- When diagnosing whether a failure is a compile error vs. a lint/test failure.

## Command

```bash
cargo check --all-targets
```

## Pass criteria

- Exit code 0.
- No compiler errors.
- (Warnings are not failures at this gate; clippy handles lint policy.)

## Fail criteria

- Exit code non-zero.
- Compiler errors reported (E0xxx).

## Evidence to report

- Exit code.
- Error count.
- First few error messages (error code, file, line, message).

## Notes

- `--all-targets` compiles lib, bins, tests, examples, and benches — not just
  the default target.
- This is the minimum gate; it does not replace clippy, tests, format, or docs.
- Run inside `nix develop` (see `nix-usage` skill) so the correct toolchain is
  used.
- `cargo check` does not generate final artifacts; use `cargo build` when a
  binary artifact is required.
- If the workspace has mutually exclusive features, run
  `cargo check --all-targets --features <f>` per feature set instead of
  `--all-features`.
