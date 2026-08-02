---
name: workflow-elixir-refactoring-04-verify
description: |
  Use only for the verify phase of the Elixir refactoring workflow.
  After each incremental step, run mix compile --warnings-as-errors +
  mix test + mix credo --strict. Revert on any red gate. Do not use for
  baseline, planning, executing, or confirming.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-refactoring
  org.phase: verify
  org.phase_order: "04"
---

## Phase purpose

After each incremental step (phase 03), run the verification gates. If any gate fails, REVERT the step and redo it. Do not proceed on a red gate. `mix test` is the primary no-regression signal; `mix credo --strict` catches style drift, dead code, and complexity introduced by the refactor.

## Steps to perform

1. Run the gates in this order: `mix compile --warnings-as-errors`; `mix test`; `mix credo --strict`.
2. Map each command to its validation skill: `mix compile --warnings-as-errors` → `validation-elixir-compile`; `mix test` → `validation-elixir-test`; `mix credo --strict` → `validation-elixir-credo`.
3. If ANY gate fails: REVERT the step (`git revert HEAD` or the repo's revert mechanism for the checkpoint made in phase 03); do NOT proceed to the next step on a red gate; set `outcome: fail`, record which gate failed and the evidence, and return to phase 03 to redo the step.
4. If ALL gates pass: Compare test output to the baseline from phase 01 (same tests pass, same counts, no new failures). If the test result differs from baseline, that is a behavior change — STOP and hand off to the implementation workflow. If test output matches baseline, set `outcome: pass`.
5. If more planned steps remain, set `next_phase: 03-execute`. If this was the last step, set `next_phase: 05-confirm`.

## Docs to consult

- docs/elixir/workflows/refactoring.md
- docs/elixir/testing-exunit.md
- docs/elixir/static-analysis-credo.md

## Operational skills to load

- `elixir-testing`
- `elixir-static-analysis`

## Constraints to apply

- `constraint-elixir-style` — Verify only this step; do not make additional changes to make a gate pass (that is a new change requiring its own plan step); do not fold in defect fixes or features.

## Validations to run

Run after EACH incremental step:

- `validation-elixir-compile` (`mix compile --warnings-as-errors`)
- `validation-elixir-test` (`mix test`)
- `validation-elixir-credo` (`mix credo --strict`)

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (all gates green, test output matches baseline) | `fail` (a gate red or behavior change detected)
- `files_touched`: `[]` (verification only; revert if failed)
- `constraints_applied`: `["constraint-elixir-style"]`
- `tests_run`: `["mix compile --warnings-as-errors", "mix test", "mix credo --strict"]` with pass/fail per gate
- `risks`: behavior-change flag if test output diverged from baseline
- `next_phase`: `03-execute` (more steps) | `05-confirm` (last step) | `stop` (behavior change detected — hand off to implementation)
- `next_workflow`: `null` | `implementation` (if behavior change detected)
- `blockers`: the failing gate and evidence if `outcome: fail`
