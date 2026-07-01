---
name: workflow-rust-implementation-05-verify
description: |
  Use only for the verify phase of the Rust implementation workflow. Run the
  full check suite (check, clippy, test, fmt, doc) and report evidence. Do not
  use for scoping, design, implementation, or test writing.
allowed-tools: Read Write Edit Bash(cargo:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-implementation
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: verify (Rust implementation)

## Phase purpose

Run the full check suite (check, clippy, test, fmt, doc) and report evidence.
This is the validation phase; it does not introduce new behavior.

## Steps to perform

1. Run the gates in this exact order, stopping and fixing at the root cause if
   any fails. After a fix, re-run from `cargo check`:
   - `cargo check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - `cargo fmt --check`
2. Document the public API per `docs/rust/documentation-guidelines.md`:
   `///` on items, `//!` at crate root, `# Examples` for non-trivial public
   functions, `# Safety` on `unsafe` items, and intra-doc links.
3. Run `cargo doc --no-deps --document-private-items`; ensure it builds without
   warnings.
4. Apply `constraint-rust-api-docs` to verify documentation completeness.
5. Report: the intended-behavior summary (from phase 01), files added/modified,
   tests added with coverage, raw output/pass-fail of each gate plus the doc
   build, and any deferred policy decisions.

## Docs to consult

- `docs/rust/documentation-guidelines.md`
- `docs/rust/style-formatting.md`
- `docs/rust/lints-clippy.md`

## Operational skills to load

- `rust-lints-and-clippy` — for clippy gate interpretation.

## Constraints to apply

- `constraint-rust-api-docs` — verify documentation completeness for the public
  API.
- `constraint-rust-scope-discipline` — fix only the new code's contribution to
  any gate failure. If a gate fails because of pre-existing code, fix only the
  new code's contribution and surface the pre-existing issue as a follow-up.

## Validations to run

Run these validation skills in gate order. Each maps to a command or build
step.

- `validation-rust-compile` — `cargo check`
- `validation-rust-clippy` — `cargo clippy --all-targets -- -D warnings`
- `validation-rust-test` — `cargo test`
- `validation-rust-format` — `cargo fmt --check`
- `validation-rust-docs` — `cargo doc --no-deps --document-private-items`

## Handoff output

Return the handoff YAML schema defined in
`workflow-rust-implementation-00-orchestration`, extended with the
verification-phase extra fields. Set:

- `outcome` to `pass` only when every gate and the doc build pass.
- `constraints_applied` to include `constraint-rust-api-docs` and
  `constraint-rust-scope-discipline`.
- `validations_run` to list each validation skill with its pass/fail.
- `constraints_checked` to list each constraint verified.
- `evidence` to include raw output or pass/fail of each gate and the doc build.
- `failures` to list any gate or doc build that failed (empty on full pass).
- `not_fully_checkable` to list anything that could not be fully validated
  automatically and why.
- `next_phase: null` and `next_workflow: null` — this is the terminal phase.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-rust-api-docs
  - constraint-rust-scope-discipline
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: ...
    covers: ...
tests_needed:
  - ...
next_phase: null
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: validation-rust-compile
    gate: cargo check
    result: pass|fail
  - validation: validation-rust-clippy
    gate: cargo clippy --all-targets -- -D warnings
    result: pass|fail
  - validation: validation-rust-test
    gate: cargo test
    result: pass|fail
  - validation: validation-rust-format
    gate: cargo fmt --check
    result: pass|fail
  - validation: validation-rust-docs
    gate: cargo doc --no-deps --document-private-items
    result: pass|fail
constraints_checked:
  - constraint-rust-api-docs
  - constraint-rust-scope-discipline
evidence:
  - gate: cargo check
    output: ...
  - gate: cargo clippy --all-targets -- -D warnings
    output: ...
  - gate: cargo test
    output: ...
  - gate: cargo fmt --check
    output: ...
  - gate: cargo doc --no-deps --document-private-items
    output: ...
failures: []
not_fully_checkable: []
```
