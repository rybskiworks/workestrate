---
name: workflow-rust-debugging-04-regression
description: |
  Use only for the regression phase of the Rust debugging workflow.
  Add a regression test that fails before the fix and passes after.
  Do not use for reproduction, diagnosis, fixing, or final verification.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-debugging
  org.phase: regression
  org.phase_order: "04"
---

## Phase purpose

Add a regression test that fails before the fix and passes after. The test must encode the reproduction from phase `01-reproduce` so the same defect cannot silently return.

## Steps to perform

1. Load `rust-testing` skill.
2. Read `docs/rust/testing.md`.
3. Add a test that fails before the fix and passes after. Place it with the code it protects: unit test in `#[cfg(test)] mod tests` next to the code under test; or integration test under `tests/` per `docs/rust/testing.md`.
4. The test MUST encode the reproduction from phase `01-reproduce` (same input, same trigger, same assertion that the defect violated).
5. Confirm the test fails before the fix and passes after. Record both observations (command + result) in `evidence`.
6. Apply `constraint-rust-scope-discipline`: keep the regression test scoped to the defect; do not build a broad test suite expansion as part of a bugfix.

## Docs to consult

- `docs/rust/testing.md`.

## Operational skills to load

- `rust-testing`.

## Constraints to apply

- `constraint-rust-scope-discipline` — Keep the regression test scoped to the defect; do not build a broad test suite expansion as part of a bugfix; do not refactor surrounding code; record adjacent issues as follow-ups.

## Validations to run

- `validation-rust-test` — verify the new regression test passes (run `cargo test` to confirm the regression test passes after the fix).

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 05-verify`, `next_workflow: null`. Record the regression test location (file + test name) and its assertion in `tests_run` and `evidence`. Record the fails-before/passes-after confirmation in `evidence`.
