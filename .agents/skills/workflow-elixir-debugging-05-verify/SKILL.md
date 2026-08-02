---
name: workflow-elixir-debugging-05-verify
description: |
  Use only for the verify phase of the Elixir debugging workflow.
  Run the full check suite (`mix compile --warnings-as-errors`,
  `mix credo --strict`, `mix dialyzer`, `mix test`, `mix format --check-formatted`)
  and report the root cause, fix, and regression test. Do not use for
  reproduction, diagnosis, fixing, or regression testing.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-debugging
  org.phase: verify
  org.phase_order: "05"
---

## Phase purpose

Run the full check suite (`mix compile --warnings-as-errors`, `mix credo --strict`, `mix dialyzer`, `mix test`, `mix format --check-formatted`) and report the root cause, fix, and regression test. This is the final phase.

## Steps to perform

1. Run gates in this exact order; all must pass: `mix compile --warnings-as-errors`; `mix credo --strict`; `mix dialyzer`; `mix test`; `mix format --check-formatted`.
2. Run validation skills: `validation-elixir-compile`; `validation-elixir-credo`; `validation-elixir-dialyzer`; `validation-elixir-test`.
3. Report all of the following (aggregate from prior phases):
   - the reproduction command and the original error/crash output (from phase `01-reproduce`);
   - the defect category (from phase `02-diagnose`);
   - the root cause: a one-paragraph explanation of why the defect occurred;
   - the fix: what changed and why it addresses the root cause (from phase `03-fix`);
   - the regression test: location, assertion, and fails-before/passes-after confirmation (from phase `04-regression`);
   - pass/fail for each gate above.

## Docs to consult

None new. Findings reference docs cited in earlier phases (`docs/elixir/error-handling.md`, `docs/elixir/beam-otp-internals.md`, `docs/elixir/concurrency-processes.md`, `docs/elixir/testing-exunit.md`, `docs/elixir/otp-supervision.md`, `docs/beam/supervision.md`, `docs/beam/processes-and-messages.md`, `docs/beam/links-monitors-and-exits.md`).

## Operational skills to load

None new.

## Constraints to apply

- `constraint-elixir-style` — Verify only the reported defect's fix; do not make additional changes to make a gate pass; do not refactor surrounding code; record adjacent issues as follow-ups.

## Validations to run

- `validation-elixir-compile` (`mix compile --warnings-as-errors`);
- `validation-elixir-credo` (`mix credo --strict`);
- `validation-elixir-dialyzer` (`mix dialyzer`);
- `validation-elixir-test` (`mix test`).

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md), including the verification-phase extra fields:
- `validations_run` — list of validation skills executed (`validation-elixir-compile`, `validation-elixir-credo`, `validation-elixir-dialyzer`, `validation-elixir-test`);
- `constraints_checked` — list of constraint skills audited against the diff;
- `evidence` — reproduction command, original error, category, root cause, fix, regression test location/assertion, per-gate pass/fail;
- `failures` — list of gates that failed (empty if all passed);
- `not_fully_checkable` — list of aspects that could not be fully validated and why (e.g., Dialyzer unavailable in this project).

Set `next_phase: null`, `next_workflow: null`. Unless the fix revealed a need for new public API, in which case set `next_workflow: workflow-elixir-implementation` (the debugging workflow is complete; the API design work is a separate workflow).
