---
name: workflow-gleam-refactoring-04-verify
description: |
  Use only for the verify phase of the Gleam refactoring workflow.
  After each incremental step, run gleam check + gleam test.
  Revert on any red gate. Do not use for baseline, planning, executing, or
  confirming.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-refactoring
  org.phase: verify
  org.phase_order: "04"
---

## Phase purpose

After each incremental step (phase 03), run the verification gates. If any gate fails, REVERT the step and redo it. Do not proceed on a red gate. `gleam test` is the primary no-regression signal; `gleam check` catches type errors and exhaustiveness regressions introduced by the refactor. There is no separate lint step in Gleam.

## Steps to perform

1. Run the gates in this order: `gleam check`; `gleam test`.
2. Map each command to its validation skill: `gleam check` → `validation-gleam-check`; `gleam test` → `validation-gleam-test`.
3. For multi-target projects, also run `gleam test --target erlang` and `gleam test --target javascript` if repo policy requires both targets.
4. If ANY gate fails: REVERT the step (`git revert HEAD` or the repo's revert mechanism for the checkpoint made in phase 03); do NOT proceed to the next step on a red gate; set `outcome: fail`, record which gate failed and the evidence, and return to phase 03 to redo the step.
5. If ALL gates pass: Compare test output to the baseline from phase 01 (same tests pass, same counts, no new failures). If the test result differs from baseline, that is a behavior change — STOP and hand off to the implementation workflow. If test output matches baseline, set `outcome: pass`.
6. If more planned steps remain, set `next_phase: 03-execute`. If this was the last step, set `next_phase: 05-confirm`.

## Docs to consult

- `docs/gleam/workflows/refactoring.md`
- `docs/gleam/testing.md`
- `docs/gleam/validation.md`

## Operational skills to load

- `gleam-packages-ffi`

## Constraints to apply

- Scope discipline (prose) — Verify only this step; do not make additional changes to make a gate pass (that is a new change requiring its own plan step); do not fold in defect fixes or features.

## Validations to run

Run after EACH incremental step:

- `validation-gleam-check` (`gleam check`)
- `validation-gleam-test` (`gleam test`)

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (all gates green, test output matches baseline) | `fail` (a gate red or behavior change detected)
- `files_touched`: `[]` (verification only; revert if failed)
- `constraints_applied`: `["scope discipline"]`
- `tests_run`: `["gleam check", "gleam test"]` with pass/fail per gate (plus per-target `gleam test --target erlang` / `gleam test --target javascript` if run)
- `risks`: behavior-change flag if test output diverged from baseline
- `next_phase`: `03-execute` (more steps) | `05-confirm` (last step) | `stop` (behavior change detected — hand off to implementation)
- `next_workflow`: `null` | `implementation` (if behavior change detected)
- `blockers`: the failing gate and evidence if `outcome: fail`
