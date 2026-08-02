---
name: workflow-gleam-code-review-03-check
description: |
  Use only for the check phase of the Gleam code-review workflow. Run the
  automated gates (gleam check, gleam test, gleam format --check) and record
  pass/fail. Do not use for scoping, analysis, manual review, or issuing a
  verdict.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-code-review
  org.phase: check
  org.phase_order: "03"
---

# Phase 03: check (Gleam code review)

## Phase purpose

Run the automated gates (`gleam check`, `gleam test`, `gleam format --check`)
and record their pass/fail status. A clean type-check is the baseline; a
compile failure is a blocker that stops the workflow.

## Steps to perform

1. Run `gleam check`. A clean type-check is the baseline. If it fails, report a
   blocker, set `outcome: fail`, and stop — do not proceed to 04-review against
   code that does not type-check.
2. Run `gleam test`. All tests must pass. Flag deleted or weakened tests. New
   public items should have accompanying tests (see `docs/gleam/testing.md`).
   For multi-target projects also run `gleam test --target erlang` and
   `gleam test --target javascript` if repo policy requires it.
3. Run `gleam format --check`. Formatting must be clean; do not accept
   hand-formatted code that contradicts `gleam format` output.
4. Run the validation skills: `validation-gleam-check`,
   `validation-gleam-test`, `validation-gleam-format`. Each validation skill
   wraps the corresponding command above and records structured pass/fail
   evidence.
5. Apply scope discipline: gate failures are reported only for the diff under
   review; do not request fixes for pre-existing failures in untouched code
   (record them as follow-ups).

## Docs to consult

- `docs/gleam/validation.md`
- `docs/gleam/testing.md`

## Operational skills to load

- `gleam-packages-ffi`
- `gleam-language`

## Constraints to apply

- Scope discipline — gate failures are reported only for the diff under review;
  do not request fixes for pre-existing failures in untouched code (record them
  as follow-ups).

## Validations to run

- `validation-gleam-check` — `gleam check`
- `validation-gleam-test` — `gleam test`
- `validation-gleam-format` — `gleam format --check`

## Handoff output

Return the handoff YAML block per the schema in
`workflow-gleam-code-review-00-orchestration`. Set:

- `outcome`: `pass` if all gates are green; `fail` if any gate failed
  (especially type-check); `partial` if a gate was skipped with attached CI
  evidence.
- `constraints_applied`: scope discipline.
- `tests_run`: one entry per gate with command and pass/fail, e.g.
  `gleam check: pass`, `gleam test: pass`, `gleam format --check: pass`.
- `blockers`: any gate failure, with the failing command.
- `assumptions`: any skipped gate and the attached CI evidence.
- `next_phase`: `04-review` (or `null` if a blocker stopped the workflow).
- `next_workflow`: `null`.
