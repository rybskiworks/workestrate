---
name: workflow-gleam-refactoring-02-plan
description: |
  Use only for the plan phase of the Gleam refactoring workflow.
  Identify small, incremental, independently-verifiable refactoring steps.
  Do not use for baseline, executing, verifying, or confirming.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-refactoring
  org.phase: plan
  org.phase_order: "02"
---

## Phase purpose

Identify the refactoring steps. Each step must be small, incremental, and independently verifiable. The plan is the contract that phase 03 executes one step at a time and phase 04 verifies after each.

## Steps to perform

1. Using the behavior/goal description from phase 01, decompose the refactor into the smallest independently-verifiable steps. One step should be ONE of: extract a helper function; replace a `case` chain with `result.try`/`use`; swap a collection (`list` → `dict`, etc.); split a module; rename a function/constructor for clarity; consolidate duplicated `@external` declarations; or another single structural change.
2. Order the steps so each builds on a verified-green previous step. Avoid steps that require a later step to compile.
3. For each step, note which operational skill applies (see below) and whether the public API surface is touched. If a step changes the public API, flag it as a likely behavior change — either restructure the step to avoid it, or escalate to the implementation workflow.
4. Keep each step small enough that a revert is cheap. If a step grows, split it.
5. Record scope-creep candidates (unrelated cleanups, defects, features) as follow-ups; do NOT fold them into the refactor.

## Docs to consult

- `docs/gleam/workflows/refactoring.md`
- `docs/gleam/stdlib.md`
- `docs/gleam/result-option-and-errors.md`
- `docs/gleam/conventions-patterns-antipatterns.md`
- `docs/gleam/types-records-and-patterns.md`

## Operational skills to load

Conditional by refactor area:

- `gleam-language` — language fundamentals, types, `Result`/`Option`, pattern matching, collection, naming, and convention refactors.
- `gleam-packages-ffi` — project structure, `gleam.toml`, tests, validation gates, FFI/API boundaries.
- `gleam-otp-interop` — if OTP actors, supervision, `gleam_erlang`/`gleam_otp`, or `@external` FFI code is touched.

On the **Erlang target**, refactors touching actor message shapes or
supervision structure also reference `docs/gleam/otp-actors-and-supervision.md`
plus `docs/gleam/erlang-interop.md` AND `docs/beam/supervision.md` plus
`docs/beam/processes-and-messages.md`. The `constraint-beam-*` constraints
apply. On the **JavaScript target**, BEAM docs do NOT apply; concurrency is
`gleam/javascript/promise`, and `constraint-beam-*` constraints do NOT apply.

## Constraints to apply

- Scope discipline (prose) — Plan only the structure described in phase 01; record unrelated items as follow-ups; do not fold in defect fixes, features, dependency changes, or lint-policy changes.

## Validations to run

None.

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (plan produced) | `partial` (plan incomplete, needs review)
- `files_touched`: `[]` (planning only; no code changes in this phase)
- `constraints_applied`: `["scope discipline"]`
- `assumptions`: the ordered list of planned steps, each with its applicable operational skill and a public-API-touch flag
- `risks`: steps that touch the public API or OTP/FFI code
- `tests_needed`: any characterization tests still missing for a planned step
- `next_phase`: `03-execute`
- `next_workflow`: `null`
- `blockers`: any step that cannot be made behavior-preserving (escalate to implementation workflow)
