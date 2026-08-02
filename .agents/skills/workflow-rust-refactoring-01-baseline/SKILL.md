---
name: workflow-rust-refactoring-01-baseline
description: |
  Use only for the baseline phase of the Rust refactoring workflow.
  Understand current behavior and establish a green test baseline before
  any change. Do not use for planning, executing, verifying, or confirming.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-refactoring
  org.phase: baseline
  org.phase_order: "01"
---

## Phase purpose

Understand the current behavior of the code being refactored and establish a green test baseline BEFORE any change. A refactor on a red baseline cannot be verified. This phase enforces the defining constraint of the workflow: behavior must NOT change.

## Steps to perform

1. Read the code being refactored and identify the structure being improved and why.
2. Read the relevant topic docs so the refactor is grounded in the corpus rules: docs/rust/design-patterns.md; docs/rust/ownership-lifetimes.md; docs/rust/types-traits-generics.md; docs/rust/iterators-closures.md; docs/rust/smart-pointers-memory.md; docs/rust/error-handling.md.
3. Write a one-paragraph description of the current behavior and the refactoring goal (what structure is being improved and why). Later phases verify the goal was met without behavior change.
4. Run the baseline test gate: `cargo test`.
5. If tests do NOT pass: STOP. Do not refactor on a red baseline. Hand off to the debugging workflow (docs/rust/workflows/debugging.md) to fix the baseline first. Set `outcome: fail`, `next_workflow: debugging`, and record the blocker.
6. If tests pass, assess coverage in the area being refactored. If coverage is thin, add characterization tests that pin the current behavior. These tests are part of the refactor, not separate work.
7. Record the baseline test result (pass/fail counts) as evidence for the final no-regression comparison in phase 05.

## Docs to consult

- docs/rust/workflows/refactoring.md
- docs/rust/design-patterns.md
- docs/rust/ownership-lifetimes.md
- docs/rust/types-traits-generics.md
- docs/rust/iterators-closures.md
- docs/rust/smart-pointers-memory.md
- docs/rust/error-handling.md

## Operational skills to load

- `rust-ownership-borrowing` — if the refactor touches ownership/borrowing (confirm scope here).
- `rust-testing` — for characterization tests.

## Constraints to apply

- `constraint-rust-scope-discipline` — Record only the current behavior and goal; do not begin changing structure in this phase; do not fold in defect fixes, features, dependency changes, or lint-policy changes.

## Validations to run

- `validation-rust-test` — establish green baseline (tests must pass before refactoring; run `cargo test` to confirm the baseline is green).

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (baseline green) | `fail` (baseline red — hand off to debugging) | `partial` (baseline green but characterization tests still needed)
- `files_touched`: any characterization tests added
- `constraints_applied`: `["constraint-rust-scope-discipline"]`
- `assumptions`: the one-paragraph behavior/goal description
- `tests_run`: `["cargo test"]` plus baseline pass/fail counts
- `tests_needed`: characterization tests still to add, if any
- `next_phase`: `02-plan` (if pass) | `stop` (if fail)
- `next_workflow`: `null` (if pass) | `debugging` (if fail)
- `blockers`: red-baseline blocker if applicable
