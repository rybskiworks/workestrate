---
name: workflow-elixir-validation-03-lint-test
description: |
  Use only for the lint-test phase of the Elixir validation workflow.
  Run mix dialyzer and mix test. Do not use for scoping, compile,
  formatting, or final reporting.
allowed-tools: Read Bash(mix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-validation
  org.phase: lint-test
  org.phase_order: "03"
---

# Elixir Validation Workflow — Phase 03 — Dialyzer & Test

## Phase Purpose

Run the Dialyzer and test gates. Record pass/fail and evidence for each. All
tests must pass.

## Steps

1. Run the static-analysis gate:
   ```sh
   mix dialyzer
   ```
   Dialyzer checks typespecs and success typings. If the repo marks Dialyzer advisory, report warnings but do not block on them (consult `docs/elixir/typespecs-and-dialyzer.md`). Map to `validation-elixir-dialyzer`.
2. Run the test suite:
   ```sh
   mix test
   ```
   Exercises the full test corpus. Doctests are part of `mix test` in Elixir. Map to `validation-elixir-test`.
3. For each gate, record pass/fail and the passed/failed counts.
4. If a gate fails, continue to the remaining gates (phase 04) to give a full picture; the overall result will be fail.
5. Do NOT add tests or modify code to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/elixir/workflows/validation.md`
- `docs/elixir/typespecs-and-dialyzer.md`
- `docs/elixir/testing-exunit.md`

## Operational Skills to Load

- `elixir-testing` — ExUnit test policy.
- `elixir-static-analysis` — Dialyzer policy.

## Constraints to Apply

- `constraint-elixir-style` — do not add tests or modify code to make a gate pass.

## Validations to Run

- `validation-elixir-dialyzer` — command: `mix dialyzer`
- `validation-elixir-test` — command: `mix test`

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (both gates green) | fail (any gate red).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-elixir-style"]`
- `tests_run`: `["mix dialyzer", "mix test"]` with pass/fail + passed/failed counts per gate.
- `risks`: Dialyzer marked advisory; any gate red.
- `next_phase`: `04-format-doc`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
