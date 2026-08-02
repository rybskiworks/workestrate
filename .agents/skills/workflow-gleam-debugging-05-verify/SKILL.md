---
name: workflow-gleam-debugging-05-verify
description: |
  Use only for the verify phase of the Gleam debugging workflow.
  Run the full check suite and report the root cause, fix, and regression
  test. Do not use for reproduction, diagnosis, fixing, or regression testing.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-debugging
  org.phase: verify
  org.phase_order: "05"
---

## Phase purpose

Run the full check suite and report the root cause, fix, and regression test. This is the final phase.

## Steps to perform

1. Run gates in this exact order; all must pass: `gleam check`; `gleam test`; `gleam format --check`.
2. If the project supports both Erlang and JavaScript targets, also run per-target tests: `gleam test --target erlang` and `gleam test --target javascript`.
3. Run validation skills: `validation-gleam-check`; `validation-gleam-test`; `validation-gleam-format`.
4. Report all of the following (aggregate from prior phases):
   - the reproduction command and the original error/crash output (from phase `01-reproduce`);
   - the defect category (from phase `02-diagnose`);
   - the root cause: a one-paragraph explanation of why the defect occurred;
   - the fix: what changed and why it addresses the root cause (from phase `03-fix`);
   - the regression test: location, assertion, and fails-before/passes-after confirmation (from phase `04-regression`);
   - pass/fail for each gate above.

## Docs to consult

- `docs/gleam/validation.md`
- `docs/gleam/testing.md`

## Operational skills to load

None new.

## Constraints to apply

- Scope discipline — Verify only the reported defect's fix; do not make additional changes to make a gate pass; do not refactor surrounding code; record adjacent issues as follow-ups.

## Validations to run

- `validation-gleam-check` (`gleam check`);
- `validation-gleam-test` (`gleam test`, plus per-target `gleam test --target erlang` / `gleam test --target javascript` when both targets are supported);
- `validation-gleam-format` (`gleam format --check`).

(There is no Gleam equivalent to a low-level runtime checker. On the Erlang target, BEAM runtime debugging tools apply instead — see `docs/beam/runtime-debugging.md`.)

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md), including the verification-phase extra fields:
- `validations_run` — list of validation skills executed (`validation-gleam-check`, `validation-gleam-test`, `validation-gleam-format`);
- `constraints_checked` — list of constraint skills audited against the diff;
- `evidence` — reproduction command, original error, category, root cause, fix, regression test location/assertion, per-gate pass/fail;
- `failures` — list of gates that failed (empty if all passed);
- `not_fully_checkable` — list of aspects that could not be fully validated and why.

Set `next_phase: null`, `next_workflow: null`. Unless the fix revealed a need for new public API, in which case set `next_workflow: workflow-gleam-implementation` (the debugging workflow is complete; the API design work is a separate workflow).
