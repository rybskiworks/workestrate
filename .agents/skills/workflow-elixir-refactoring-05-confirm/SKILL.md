---
name: workflow-elixir-refactoring-05-confirm
description: |
  Use only for the confirm phase of the Elixir refactoring workflow.
  After all steps, run the final no-regression check (full test suite,
  credo, mix format --check-formatted) and report. Do not use for baseline,
  planning, executing, or verifying.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-refactoring
  org.phase: confirm
  org.phase_order: "05"
---

## Phase purpose

After all planned steps are complete, run the final no-regression check and report. Confirm the output matches the baseline from phase 01, check for style drift/dead code via credo, run `mix format --check-formatted`, and report what changed, why, and the evidence of no regression.

## Steps to perform

1. Run the full test suite and confirm the output matches the baseline from phase 01 (same tests pass, same counts, no new failures): `mix test`. If characterization tests were added in phase 01, confirm they still pass unchanged. If any test had to change, that is a behavior change — either justify it explicitly and switch to the implementation workflow, or revert.
2. Run credo to catch style drift, dead code, unused imports, and complexity introduced by the refactor: `mix credo --strict`. Remove anything credo flags rather than suppressing it, unless the repo lint policy explicitly allows the suppression.
3. Run the format check: `mix format --check-formatted`.
4. Map gates to validation skills: `mix credo --strict` → `validation-elixir-credo`; `mix format --check-formatted` → `validation-elixir-format`.
5. Aggregate evidence and report: the one-paragraph behavior/goal description from phase 01; the baseline test result from phase 01; the list of incremental steps taken, with the gate results after each (from phase 04); the final test result confirming no regression; the final credo result; the final format result; the list of files changed; an explicit statement that no behavior change occurred, OR a flagged behavior change with a handoff to the implementation workflow.

## Docs to consult

- docs/elixir/workflows/refactoring.md
- docs/elixir/testing-exunit.md
- docs/elixir/static-analysis-credo.md
- docs/elixir/naming-conventions.md

## Operational skills to load

- `elixir-testing`
- `elixir-static-analysis`

## Constraints to apply

- `constraint-elixir-style` — Confirm only the planned refactor; flag any drift as a behavior change; do not fold in defect fixes, features, dependency changes, or lint-policy changes.

## Validations to run

Final no-regression confirmation:

- `validation-elixir-credo` (`mix credo --strict`)
- `validation-elixir-format` (`mix format --check-formatted`)

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (no regression, all final gates green) | `fail` (regression or red final gate) | `partial` (gates green but a behavior change was flagged)
- `files_touched`: full list of files changed across all steps
- `constraints_applied`: `["constraint-elixir-style"]`
- `tests_run`: `["mix test", "mix credo --strict", "mix format --check-formatted"]` with pass/fail per gate
- `validations_run`: `["validation-elixir-credo", "validation-elixir-format"]`
- `constraints_checked`: `["constraint-elixir-style"]`
- `evidence`: baseline comparison, per-step gate results, final gate results, files changed, explicit no-behavior-change statement
- `failures`: any red final gate or detected behavior change
- `next_phase`: `stop`
- `next_workflow`: `null` | `implementation` (if behavior change flagged) | `debugging` (if a defect surfaced)
- `blockers`: any unresolved regression
