---
name: workflow-rust-implementation-04-test
description: |
  Use only for the test phase of the Rust implementation workflow. Write tests
  alongside the code and run cargo test. Do not use for scoping, design,
  implementation, or final verification.
allowed-tools: Read Write Edit Bash(cargo:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-implementation
  org.phase: test
  org.phase_order: "04"
---

# Phase 04: test (Rust implementation)

## Phase purpose

Write tests alongside the code and run `cargo test` to confirm the
implementation behaves as scoped in phase 01.

## Steps to perform

1. Load `rust-testing`.
2. Read `docs/rust/testing.md`.
3. Write unit tests in the same module (`#[cfg(test)] mod tests`).
4. Write integration tests under `tests/` where appropriate.
5. Cover: happy path, each error branch, and edge cases (empty input, boundary
   values, concurrent access for async code).
6. Add doctests for non-trivial public items.
7. Run `cargo test`; fix failures at the root cause before proceeding. Do not
   suppress failures with `#[allow]` or by deleting tests.

## Docs to consult

- `docs/rust/testing.md`

## Operational skills to load

- `rust-testing`

## Constraints to apply

- `constraint-rust-scope-discipline` — test the scoped behavior, not unrelated
  modules the new code touches only at a call site. Do not add tests for
  speculative future behavior.

## Validations to run

- `validation-rust-test` — runs `cargo test` and records structured pass/fail
  evidence for the test suite.

## Handoff output

Return the handoff YAML schema defined in
`workflow-rust-implementation-00-orchestration`. Set:

- `outcome` to `pass` once `cargo test` passes and the test set covers happy
  path, error branches, and edge cases.
- `constraints_applied` to include `constraint-rust-scope-discipline`.
- `tests_run` to list every test added (unit, integration, doctest) with what
  each covers.
- `next_phase: 05-verify`.
- `blockers: []` unless a test reveals a defect that needs phase 03 to revisit
  — in that case set `outcome: fail` and record the failing test.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
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
next_phase: 05-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
