---
name: workflow-gleam-refactoring-01-baseline
description: |
  Use only for the baseline phase of the Gleam refactoring workflow.
  Understand current behavior and establish a green test baseline before
  any change. Do not use for planning, executing, verifying, or confirming.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-refactoring
  org.phase: baseline
  org.phase_order: "01"
---

## Phase purpose

Understand the current behavior of the code being refactored and establish a green test baseline BEFORE any change. A refactor on a red baseline cannot be verified. This phase enforces the defining constraint of the workflow: behavior must NOT change.

## Steps to perform

1. Read the code being refactored and identify the structure being improved and why.
2. Read the relevant topic docs so the refactor is grounded in the corpus rules: `docs/gleam/stdlib.md`; `docs/gleam/result-option-and-errors.md`; `docs/gleam/conventions-patterns-antipatterns.md`; `docs/gleam/types-records-and-patterns.md`.
3. Write a one-paragraph description of the current behavior and the refactoring goal (what structure is being improved and why). Later phases verify the goal was met without behavior change.
4. Run the baseline test gate: `gleam test`.
5. If tests do NOT pass: STOP. Do not refactor on a red baseline. Hand off to the debugging workflow (`docs/gleam/workflows/debugging.md`) to fix the baseline first. Set `outcome: fail`, `next_workflow: debugging`, and record the blocker.
6. If tests pass, assess coverage in the area being refactored. If coverage is thin, add characterization tests that pin the current behavior. These tests are part of the refactor, not separate work.
7. Record the baseline test result (pass/fail counts) as evidence for the final no-regression comparison in phase 05.

## Docs to consult

- `docs/gleam/workflows/refactoring.md`
- `docs/gleam/testing.md`
- `docs/gleam/stdlib.md`
- `docs/gleam/result-option-and-errors.md`
- `docs/gleam/conventions-patterns-antipatterns.md`
- `docs/gleam/types-records-and-patterns.md`

## Operational skills to load

- `gleam-language` — if the refactor touches language fundamentals, types, `Result`/`Option`, pattern matching, or collections.
- `gleam-packages-ffi` — for characterization tests and project/test layout.

## Constraints to apply

- Scope discipline (prose) — Record only the current behavior and goal; do not begin changing structure in this phase; do not fold in defect fixes, features, dependency changes, or lint-policy changes.

## Validations to run

- `validation-gleam-test` — establish green baseline (tests must pass before refactoring; run `gleam test` to confirm the baseline is green).

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (baseline green) | `fail` (baseline red — hand off to debugging) | `partial` (baseline green but characterization tests still needed)
- `files_touched`: any characterization tests added
- `constraints_applied`: `["scope discipline"]`
- `assumptions`: the one-paragraph behavior/goal description
- `tests_run`: `["gleam test"]` plus baseline pass/fail counts
- `tests_needed`: characterization tests still to add, if any
- `next_phase`: `02-plan` (if pass) | `stop` (if fail)
- `next_workflow`: `null` (if pass) | `debugging` (if fail)
- `blockers`: red-baseline blocker if applicable
