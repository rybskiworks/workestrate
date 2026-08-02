---
name: workflow-elixir-refactoring-01-baseline
description: |
  Use only for the baseline phase of the Elixir refactoring workflow.
  Understand current behavior and establish a green test baseline before
  any change. Do not use for planning, executing, verifying, or confirming.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-refactoring
  org.phase: baseline
  org.phase_order: "01"
---

## Phase purpose

Understand the current behavior of the code being refactored and establish a green test baseline BEFORE any change. A refactor on a red baseline cannot be verified. This phase enforces the defining constraint of the workflow: behavior must NOT change.

## Steps to perform

1. Read the code being refactored and identify the structure being improved and why.
2. Read the relevant topic docs so the refactor is grounded in the corpus rules: docs/elixir/workflows/refactoring.md; docs/elixir/core-modules.md; docs/elixir/otp-supervision.md; docs/elixir/language-fundamentals.md; docs/beam/supervision.md; docs/beam/proc-lib-and-sys.md.
3. Write a one-paragraph description of the current behavior and the refactoring goal (what structure is being improved and why). Later phases verify the goal was met without behavior change.
4. Run the baseline test gate: `mix test`.
5. If tests do NOT pass: STOP. Do not refactor on a red baseline. Hand off to the debugging workflow (docs/elixir/workflows/debugging.md) to fix the baseline first. Set `outcome: fail`, `next_workflow: debugging`, and record the blocker.
6. If tests pass, assess coverage in the area being refactored. If coverage is thin, add characterization tests that pin the current behavior. These tests are part of the refactor, not separate work.
7. Record the baseline test result (pass/fail counts) as evidence for the final no-regression comparison in phase 05.

## Docs to consult

- docs/elixir/workflows/refactoring.md
- docs/elixir/core-modules.md
- docs/elixir/otp-supervision.md
- docs/elixir/language-fundamentals.md
- docs/beam/supervision.md
- docs/beam/proc-lib-and-sys.md

## Operational skills to load

- `elixir-coding` — if the refactor touches naming, function shape, pattern matching, or module structure (confirm scope here).
- `elixir-testing` — for characterization tests.

## Constraints to apply

- `constraint-elixir-style` — Record only the current behavior and goal; do not begin changing structure in this phase; do not fold in defect fixes, features, dependency changes, or lint-policy changes.

## Validations to run

- `validation-elixir-test` — establish green baseline (tests must pass before refactoring; run `mix test` to confirm the baseline is green).

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (baseline green) | `fail` (baseline red — hand off to debugging) | `partial` (baseline green but characterization tests still needed)
- `files_touched`: any characterization tests added
- `constraints_applied`: `["constraint-elixir-style"]`
- `assumptions`: the one-paragraph behavior/goal description
- `tests_run`: `["mix test"]` plus baseline pass/fail counts
- `tests_needed`: characterization tests still to add, if any
- `next_phase`: `02-plan` (if pass) | `stop` (if fail)
- `next_workflow`: `null` (if pass) | `debugging` (if fail)
- `blockers`: red-baseline blocker if applicable
