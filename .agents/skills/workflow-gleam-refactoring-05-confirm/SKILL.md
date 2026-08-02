---
name: workflow-gleam-refactoring-05-confirm
description: |
  Use only for the confirm phase of the Gleam refactoring workflow.
  After all steps, run the final no-regression check (full test suite,
  gleam format --check) and report. Do not use for baseline, planning,
  executing, or verifying.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-refactoring
  org.phase: confirm
  org.phase_order: "05"
---

## Phase purpose

After all planned steps are complete, run the final no-regression check and report. Confirm the output matches the baseline from phase 01, run `gleam format --check`, and report what changed, why, and the evidence of no regression.

## Steps to perform

1. Run the full test suite and confirm the output matches the baseline from phase 01 (same tests pass, same counts, no new failures): `gleam test`. If characterization tests were added in phase 01, confirm they still pass unchanged. If any test had to change, that is a behavior change — either justify it explicitly and switch to the implementation workflow, or revert. For multi-target projects, also run `gleam test --target erlang` and `gleam test --target javascript` if repo policy requires both targets.
2. Run the format check: `gleam format --check`.
3. Map gates to validation skills: `gleam format --check` → `validation-gleam-format`.
4. Aggregate evidence and report: the one-paragraph behavior/goal description from phase 01; the baseline test result from phase 01; the list of incremental steps taken, with the gate results after each (from phase 04); the final test result confirming no regression; the final format result; the list of files changed; an explicit statement that no behavior change occurred, OR a flagged behavior change with a handoff to the implementation workflow.

## Docs to consult

- `docs/gleam/workflows/refactoring.md`
- `docs/gleam/testing.md`
- `docs/gleam/validation.md`
- `docs/gleam/conventions-patterns-antipatterns.md`

## Operational skills to load

- `gleam-packages-ffi`

## Constraints to apply

- Scope discipline (prose) — Confirm only the planned refactor; flag any drift as a behavior change; do not fold in defect fixes, features, dependency changes, or lint-policy changes.

## Validations to run

Final no-regression confirmation:

- `validation-gleam-format` (`gleam format --check`)

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (no regression, all final gates green) | `fail` (regression or red final gate) | `partial` (gates green but a behavior change was flagged)
- `files_touched`: full list of files changed across all steps
- `constraints_applied`: `["scope discipline"]`
- `tests_run`: `["gleam test", "gleam format --check"]` with pass/fail per gate (plus per-target `gleam test --target erlang` / `gleam test --target javascript` if run)
- `validations_run`: `["validation-gleam-format"]`
- `constraints_checked`: `["scope discipline"]`
- `evidence`: baseline comparison, per-step gate results, final gate results, files changed, explicit no-behavior-change statement
- `failures`: any red final gate or detected behavior change
- `next_phase`: `stop`
- `next_workflow`: `null` | `implementation` (if behavior change flagged) | `debugging` (if a defect surfaced)
- `blockers`: any unresolved regression
