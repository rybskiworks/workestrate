---
name: workflow-gleam-debugging-04-regression
description: |
  Use only for the regression phase of the Gleam debugging workflow.
  Add a regression test that fails before the fix and passes after.
  Do not use for reproduction, diagnosis, fixing, or final verification.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-debugging
  org.phase: regression
  org.phase_order: "04"
---

## Phase purpose

Add a regression test that fails before the fix and passes after. The test must encode the reproduction from phase `01-reproduce` so the same defect cannot silently return.

## Steps to perform

1. Load `gleam-packages-ffi` skill.
2. Read `docs/gleam/testing.md`.
3. Add a test that fails before the fix and passes after. Place it with the code it protects: unit test in the same module's `test/` counterpart next to the code under test, or integration test under `test/` per `docs/gleam/testing.md`.
4. The test MUST encode the reproduction from phase `01-reproduce` (same input, same trigger, same assertion that the defect violated).
5. Confirm the test fails before the fix and passes after. Record both observations (command + result) in `evidence`.
6. Run the regression test on the target(s) where the defect was reproduced; if the project supports both Erlang and JavaScript targets, prefer running on both (`gleam test --target erlang` and `gleam test --target javascript`).
7. Apply scope discipline: keep the regression test scoped to the defect; do not build a broad test suite expansion as part of a bugfix.

## Docs to consult

- `docs/gleam/testing.md`.

## Operational skills to load

- `gleam-packages-ffi`.

## Constraints to apply

- Scope discipline — Keep the regression test scoped to the defect; do not build a broad test suite expansion as part of a bugfix; do not refactor surrounding code; record adjacent issues as follow-ups.

## Validations to run

- `validation-gleam-test` — verify the new regression test passes (run `gleam test` to confirm the regression test passes after the fix).

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 05-verify`, `next_workflow: null`. Record the regression test location (file + test name) and its assertion in `tests_run` and `evidence`. Record the fails-before/passes-after confirmation in `evidence`. Record the target(s) on which the test was run.
