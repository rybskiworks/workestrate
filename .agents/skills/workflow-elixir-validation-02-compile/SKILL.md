---
name: workflow-elixir-validation-02-compile
description: |
  Use only for the compile phase of the Elixir validation workflow.
  Run mix compile --warnings-as-errors and mix credo --strict. Do not
  use for scoping, testing, formatting, or final reporting.
allowed-tools: Read Bash(mix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-validation
  org.phase: compile
  org.phase_order: "02"
---

# Elixir Validation Workflow — Phase 02 — Compile & Lint

## Phase Purpose

Run the compilation and lint gates. Record pass/fail and evidence for each. A
failure here is a blocker for the overall verdict, but continue to the
remaining gates to give a full picture.

## Steps

1. Run the compilation gate:
   ```sh
   mix compile --warnings-as-errors
   ```
   Treats all compiler warnings as errors. Map to `validation-elixir-compile`.
2. Run the lint gate:
   ```sh
   mix credo --strict
   ```
   Strict mode treats every issue as a blocker. Map to `validation-elixir-credo`.
3. For each gate, record pass/fail and the relevant output (error count or clean; issue count or clean).
4. If a gate fails, continue to the remaining gates (phase 03, 04) to give a full picture; the overall result will be fail.
5. Do NOT modify code to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/elixir/workflows/validation.md`
- `docs/elixir/static-analysis-credo.md`
- `docs/elixir/mix-project-structure.md`

## Operational Skills to Load

- `elixir-static-analysis` — Credo and Dialyzer policy.

## Constraints to Apply

- `constraint-elixir-style` — do not modify code or `.credo.exs` to make a gate pass.

## Validations to Run

- `validation-elixir-compile` — command: `mix compile --warnings-as-errors`
- `validation-elixir-credo` — command: `mix credo --strict`

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (both gates green) | fail (any gate red) | partial (one gate not-applicable).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-elixir-style"]`
- `tests_run`: `["mix compile --warnings-as-errors", "mix credo --strict"]` with pass/fail + evidence per gate.
- `risks`: any gate red.
- `next_phase`: `03-lint-test`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
