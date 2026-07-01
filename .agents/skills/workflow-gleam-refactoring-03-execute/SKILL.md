---
name: workflow-gleam-refactoring-03-execute
description: |
  Use only for the execute phase of the Gleam refactoring workflow.
  Make ONE incremental change from the plan and commit/checkpoint after
  each step. Do not use for baseline, planning, verifying, or confirming.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-refactoring
  org.phase: execute
  org.phase_order: "03"
---

## Phase purpose

Make ONE incremental change from the plan (phase 02). Commit or checkpoint after each step so a bad step can be reverted without losing the whole refactor. This phase runs once per planned step, looping with phase 04.

## Steps to perform

1. Take the next planned step from the phase 02 plan. Make only that one change.
2. Load the operational skill(s) identified for this step in the plan.
3. Implement the single structural change (e.g., extract helper, replace a `case` chain with `result.try`/`use`, swap a collection, split a module, rename a function/constructor, consolidate duplicated `@external` declarations).
4. Do NOT fold in unrelated cleanups, defect fixes, features, dependency changes, feature-flag changes, or lint-policy changes. Record those as follow-ups.
5. Commit or checkpoint the change so it can be reverted in isolation: `git add -A && git commit -m "refactor: <step description>"` (Or checkpoint via the repo's preferred mechanism.)
6. Hand off to phase 04 for verification of this single step.

## Docs to consult

- `docs/gleam/workflows/refactoring.md`
- The topic doc relevant to this step (e.g., `docs/gleam/stdlib.md` for a collection swap, `docs/gleam/result-option-and-errors.md` for a `Result` refactor, `docs/gleam/conventions-patterns-antipatterns.md` for a naming/type-design refactor, `docs/gleam/types-records-and-patterns.md` for a record/pattern refactor).
- If the step touches OTP/FFI: on the **Erlang target**, `docs/gleam/otp-actors-and-supervision.md` plus `docs/gleam/erlang-interop.md` AND `docs/beam/supervision.md` plus `docs/beam/processes-and-messages.md`. On the **JavaScript target**, BEAM docs do NOT apply; concurrency is `gleam/javascript/promise`, and `constraint-beam-*` constraints do NOT apply.

## Operational skills to load

Conditional by step area:

- `gleam-language` — language fundamentals, types, `Result`/`Option`, pattern matching, collection, naming, and convention steps.
- `gleam-packages-ffi` — project structure, `gleam.toml`, tests, validation gates, FFI/API boundary steps.
- `gleam-otp-interop` — OTP actors, supervision, `gleam_erlang`/`gleam_otp`, or `@external` FFI steps.

## Constraints to apply

- Scope discipline (prose) — Make only the one planned change; nothing else; do not fold in unrelated cleanups, defect fixes, features, dependency changes, or lint-policy changes.
- `constraint-gleam-result` + `constraint-gleam-conventions` — If this is a language/type refactor: apply `Result`/`Option`, exhaustiveness, naming, import, module-structure, and sans-IO rules correctly; prefer `result.try`/`use` over nested `case`; do not introduce panic/let-assert in library code; do not fragment modules or trespass namespaces.
- `constraint-beam-supervision` + `constraint-beam-failure` + `constraint-beam-process-isolation` — Erlang target only, if OTP/actor/supervision code is touched: restrict changes to the structural change; preserve child specs, restart semantics, message shapes, links/monitors, and process isolation invariants.

## Validations to run

None (verification runs in phase 04).

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (change made and committed) | `partial` (change made but not committed) | `fail` (could not complete the step)
- `files_touched`: the files changed in this step with a one-line change description each
- `constraints_applied`: `["scope discipline"]` (plus `constraint-gleam-result` and/or `constraint-gleam-conventions` if applied, and `constraint-beam-supervision` + `constraint-beam-failure` + `constraint-beam-process-isolation` on Erlang-target OTP steps)
- `assumptions`: the step description and which plan item it satisfies
- `risks`: whether the public API or OTP/FFI code was touched
- `next_phase`: `04-verify`
- `next_workflow`: `null`
- `blockers`: any issue that prevented completing the step
