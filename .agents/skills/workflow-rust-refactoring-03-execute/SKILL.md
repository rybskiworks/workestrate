---
name: workflow-rust-refactoring-03-execute
description: |
  Use only for the execute phase of the Rust refactoring workflow.
  Make ONE incremental change from the plan and commit/checkpoint after
  each step. Do not use for baseline, planning, verifying, or confirming.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-refactoring
  org.phase: execute
  org.phase_order: "03"
---

## Phase purpose

Make ONE incremental change from the plan (phase 02). Commit or checkpoint after each step so a bad step can be reverted without losing the whole refactor. This phase runs once per planned step, looping with phase 04.

## Steps to perform

1. Take the next planned step from the phase 02 plan. Make only that one change.
2. Load the operational skill(s) identified for this step in the plan.
3. Implement the single structural change (e.g., extract helper, replace `clone` with borrow, split module, swap `Rc` for `Arc`, replace hand-rolled iterator with combinator chain).
4. Do NOT fold in unrelated cleanups, defect fixes, features, dependency changes, feature-flag changes, or lint-policy changes. Record those as follow-ups.
5. Commit or checkpoint the change so it can be reverted in isolation: `git add -A && git commit -m "refactor: <step description>"` (Or checkpoint via the repo's preferred mechanism.)
6. Hand off to phase 04 for verification of this single step.

## Docs to consult

- docs/rust/workflows/refactoring.md
- The topic doc relevant to this step (e.g., docs/rust/ownership-lifetimes.md for a borrow refactor, docs/rust/iterators-closures.md for a combinator refactor).

## Operational skills to load

Conditional by step area:

- `rust-ownership-borrowing` — ownership/borrowing/lifetime/smart-pointer steps.
- `rust-api-design` — if the public surface is touched.
- `rust-error-handling` — if error types are restructured.
- `rust-async-tokio` — for async refactors.
- `rust-unsafe-review` — if unsafe code is touched.

## Constraints to apply

- `constraint-rust-scope-discipline` — Make only the one planned change; nothing else; do not fold in unrelated cleanups, defect fixes, features, dependency changes, or lint-policy changes.
- `constraint-rust-ownership` — If this is an ownership refactor: apply ownership/borrowing/lifetime/smart-pointer rules correctly; prefer borrows over reflexive `clone()`; choose the right smart pointer; do not introduce unsound ownership patterns.
- `constraint-rust-unsafe-safety` — If unsafe code is touched: restrict changes to the soundness-preserving structural change and its `// SAFETY:` comment; do not rewrite the unsafe block's surrounding logic; preserve all invariants.

## Validations to run

None (verification runs in phase 04).

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (change made and committed) | `partial` (change made but not committed) | `fail` (could not complete the step)
- `files_touched`: the files changed in this step with a one-line change description each
- `constraints_applied`: `["constraint-rust-scope-discipline"]` (plus `constraint-rust-ownership` and/or `constraint-rust-unsafe-safety` if applied)
- `assumptions`: the step description and which plan item it satisfies
- `risks`: whether the public API or unsafe code was touched
- `next_phase`: `04-verify`
- `next_workflow`: `null`
- `blockers`: any issue that prevented completing the step
