---
name: workflow-elixir-refactoring-02-plan
description: |
  Use only for the plan phase of the Elixir refactoring workflow.
  Identify small, incremental, independently-verifiable refactoring steps.
  Do not use for baseline, executing, verifying, or confirming.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-refactoring
  org.phase: plan
  org.phase_order: "02"
---

## Phase purpose

Identify the refactoring steps. Each step must be small, incremental, and independently verifiable. The plan is the contract that phase 03 executes one step at a time and phase 04 verifies after each.

## Steps to perform

1. Using the behavior/goal description from phase 01, decompose the refactor into the smallest independently-verifiable steps. One step should be ONE of: extract a helper function; swap `Enum` for `Stream`; consolidate `case`/`cond` clauses into multi-clause function heads; split a module; restructure a supervision child spec; introduce `@type t`; or another single structural change.
2. Order the steps so each builds on a verified-green previous step. Avoid steps that require a later step to compile.
3. For each step, note which operational skill applies (see below) and whether the public API surface is touched. If a step changes the public API, flag it as a likely behavior change — either restructure the step to avoid it, or escalate to the implementation workflow.
4. Keep each step small enough that a revert is cheap. If a step grows, split it.
5. Record scope-creep candidates (unrelated cleanups, defects, features) as follow-ups; do NOT fold them into the refactor.

## Docs to consult

- docs/elixir/workflows/refactoring.md
- docs/elixir/core-modules.md
- docs/elixir/otp-supervision.md
- docs/elixir/language-fundamentals.md
- docs/beam/supervision.md
- docs/beam/proc-lib-and-sys.md

## Operational skills to load

Conditional by refactor area:

- `elixir-coding` — naming, function-shape, pattern-matching, collection, and module-structure refactors.
- `elixir-static-analysis` — Credo policy and style consistency.
- `elixir-otp` — OTP and GenServer/supervisor refactors.
- `beam-supervision` — if the supervision tree is restructured.
- `beam-gen-server` — if a GenServer contract is refactored.
- `elixir-error-handling` — if error types or result shapes are restructured.
- `elixir-testing` — if a step requires new characterization tests.

## Constraints to apply

- `constraint-elixir-style` — Plan only the structure described in phase 01; record unrelated items as follow-ups; do not fold in defect fixes, features, dependency changes, or lint-policy changes.

## Validations to run

None.

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (plan produced) | `partial` (plan incomplete, needs review)
- `files_touched`: `[]` (planning only; no code changes in this phase)
- `constraints_applied`: `["constraint-elixir-style"]`
- `assumptions`: the ordered list of planned steps, each with its applicable operational skill and a public-API-touch flag
- `risks`: steps that touch the public API, OTP supervision, or process isolation
- `tests_needed`: any characterization tests still missing for a planned step
- `next_phase`: `03-execute`
- `next_workflow`: `null`
- `blockers`: any step that cannot be made behavior-preserving (escalate to implementation workflow)
