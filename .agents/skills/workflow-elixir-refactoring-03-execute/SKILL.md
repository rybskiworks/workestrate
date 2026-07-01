---
name: workflow-elixir-refactoring-03-execute
description: |
  Use only for the execute phase of the Elixir refactoring workflow.
  Make ONE incremental change from the plan and commit/checkpoint after
  each step. Do not use for baseline, planning, verifying, or confirming.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-refactoring
  org.phase: execute
  org.phase_order: "03"
---

## Phase purpose

Make ONE incremental change from the plan (phase 02). Commit or checkpoint after each step so a bad step can be reverted without losing the whole refactor. This phase runs once per planned step, looping with phase 04.

## Steps to perform

1. Take the next planned step from the phase 02 plan. Make only that one change.
2. Load the operational skill(s) identified for this step in the plan.
3. Implement the single structural change (e.g., extract helper, swap `Enum` for `Stream`, consolidate clauses, split module, restructure supervision child spec, introduce `@type t`).
4. Do NOT fold in unrelated cleanups, defect fixes, features, dependency changes, feature-flag changes, or lint-policy changes. Record those as follow-ups.
5. Commit or checkpoint the change so it can be reverted in isolation: `git add -A && git commit -m "refactor: <step description>"` (Or checkpoint via the repo's preferred mechanism.)
6. Hand off to phase 04 for verification of this single step.

## Docs to consult

- docs/elixir/workflows/refactoring.md
- The topic doc relevant to this step (e.g., docs/elixir/core-modules.md for an `Enum` → `Stream` swap, docs/elixir/otp-supervision.md and docs/beam/supervision.md for a supervision restructure, docs/elixir/language-fundamentals.md for a pattern-matching refactor).

## Operational skills to load

Conditional by step area:

- `elixir-coding` — naming, function-shape, pattern-matching, collection, and module-structure steps.
- `elixir-static-analysis` — Credo policy and style consistency.
- `elixir-otp` — OTP and GenServer/supervisor steps.
- `beam-supervision` — if the supervision tree is restructured.
- `beam-gen-server` — if a GenServer contract is refactored.
- `elixir-error-handling` — if error types or result shapes are restructured.
- `elixir-testing` — if a step requires new characterization tests.

## Constraints to apply

- `constraint-elixir-style` — Make only the one planned change; nothing else; do not fold in unrelated cleanups, defect fixes, features, dependency changes, or lint-policy changes.
- `constraint-beam-supervision` — If OTP supervision is touched: apply supervision/child-spec/restart-strategy rules correctly; preserve failure semantics and child ordering; do not weaken isolation guarantees.
- `constraint-beam-process-isolation` — If NIFs or raw processes are touched: restrict changes to the isolation-preserving structural change; do not rewrite surrounding process or NIF invariants.

## Validations to run

None (verification runs in phase 04).

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (change made and committed) | `partial` (change made but not committed) | `fail` (could not complete the step)
- `files_touched`: the files changed in this step with a one-line change description each
- `constraints_applied`: `["constraint-elixir-style"]` (plus `constraint-beam-supervision` and/or `constraint-beam-process-isolation` if applied)
- `assumptions`: the step description and which plan item it satisfies
- `risks`: whether the public API, OTP supervision, or process isolation was touched
- `next_phase`: `04-verify`
- `next_workflow`: `null`
- `blockers`: any issue that prevented completing the step
