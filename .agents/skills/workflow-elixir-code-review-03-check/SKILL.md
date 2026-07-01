---
name: workflow-elixir-code-review-03-check
description: |
  Use only for the check phase of the Elixir code-review workflow. Run the
  automated gates (mix compile --warnings-as-errors, mix credo --strict, mix
  dialyzer, mix test, mix format --check-formatted) and record pass/fail. Do not
  use for scoping, analysis, manual review, or issuing a verdict.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-code-review
  org.phase: check
  org.phase_order: "03"
---

# Phase 03: check (Elixir code review)

## Phase purpose

Run the automated gates (`mix compile --warnings-as-errors`, `mix credo --strict`,
`mix dialyzer`, `mix test`, `mix format --check-formatted`) and record their
pass/fail status. A clean compile is the baseline; a compile failure is a
blocker that stops the workflow.

## Steps to perform

1. Run `mix compile --warnings-as-errors`. A clean compile is the baseline. If
   it does not compile, report a blocker, set `outcome: fail`, and stop — do
   not proceed to 04-review against code that does not compile.
2. Run `mix credo --strict`. Any Credo issue is a blocker. Review any new
   `# credo:disable-for-next-line` for justification against the repo lint
   policy in `docs/elixir/static-analysis-credo.md`.
3. Run `mix dialyzer`. Review its output even if the repo marks it advisory;
   flag new type discrepancies and missing `@spec` annotations that surface.
4. Run `mix test`. All tests must pass. Flag deleted or weakened tests. New
   public items should have accompanying tests (see
   `docs/elixir/testing-exunit.md`).
5. Run `mix format --check-formatted`. Formatting must be clean; do not accept
   hand-formatted code that contradicts `mix format` output.
6. Run the validation skills: `validation-elixir-compile`,
   `validation-elixir-credo`, `validation-elixir-dialyzer`,
   `validation-elixir-test`, `validation-elixir-format`. Each validation skill
   wraps the corresponding command above and records structured pass/fail
   evidence.
7. Apply `constraint-elixir-style`: gate failures are reported only for the
   diff under review; do not request fixes for pre-existing failures in
   untouched code (record them as follow-ups).

## Docs to consult

- `docs/elixir/static-analysis-credo.md`
- `docs/elixir/typespecs-and-dialyzer.md`
- `docs/elixir/testing-exunit.md`

## Operational skills to load

- `elixir-static-analysis`
- `elixir-testing`

## Constraints to apply

- `constraint-elixir-style` — gate failures are reported only for the diff
  under review; do not request fixes for pre-existing failures in untouched
  code (record them as follow-ups).

## Validations to run

- `validation-elixir-compile` — `mix compile --warnings-as-errors`
- `validation-elixir-credo` — `mix credo --strict`
- `validation-elixir-dialyzer` — `mix dialyzer`
- `validation-elixir-test` — `mix test`
- `validation-elixir-format` — `mix format --check-formatted`

## Handoff output

Return the handoff YAML block per the schema in
`workflow-elixir-code-review-00-orchestration`. Set:

- `outcome`: `pass` if all five gates are green; `fail` if any gate failed
  (especially compile); `partial` if a gate was skipped with attached CI
  evidence.
- `constraints_applied`: `constraint-elixir-style`.
- `tests_run`: one entry per gate with command and pass/fail, e.g.
  `mix compile --warnings-as-errors: pass`,
  `mix credo --strict: pass`, `mix dialyzer: pass`, `mix test: pass`,
  `mix format --check-formatted: pass`.
- `blockers`: any gate failure, with the failing command.
- `assumptions`: any skipped gate and the attached CI evidence.
- `next_phase`: `04-review` (or `null` if a blocker stopped the workflow).
- `next_workflow`: `null`.
