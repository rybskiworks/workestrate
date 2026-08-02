---
name: workflow-elixir-validation-05-report
description: |
  Use only for the report phase of the Elixir validation workflow.
  Aggregate all gate results into a per-gate table and report the
  overall verdict. Do not use for scoping, compile, testing, or
  formatting.
allowed-tools: Read
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-validation
  org.phase: report
  org.phase_order: "05"
---

# Elixir Validation Workflow — Phase 05 — Report

## Phase Purpose

Aggregate all gate results into a per-gate table. If OTP code is present
(determined in phase 01), note that runtime validation hooks from
`docs/beam/validation.md` should be cross-checked (cross-ref the debugging
workflow). Report the overall verdict: pass (all applicable gates green) or fail
(any applicable gate red). If a gate failed, hand off to the appropriate fixing
workflow — do NOT fix in the validation pass.

## Steps

1. Aggregate the gate results from phases 02-04 into a per-gate table:

   | Gate | Command | Result | Evidence |
   |---|---|---|---|
   | Compilation | `mix compile --warnings-as-errors` | pass/fail | error/warning count or clean |
   | Credo | `mix credo --strict` | pass/fail | issue count or clean |
   | Dialyzer | `mix dialyzer` | pass/fail | warning count or clean |
   | Tests | `mix test` | pass/fail | passed/failed counts |
   | Format | `mix format --check-formatted` | pass/fail | diff or clean |
   | Coverage | `mix test --cover` | pass/fail/N-A | percentage and threshold |
   | Deps audit | `mix deps.audit` | pass/fail/N-A | advisory count or clean |
   | Runtime validation | see `docs/beam/validation.md` | pass/fail/N-A | sys/trace/ETS/timer findings |

   Mark not-applicable gates "N-A" (e.g., runtime validation when no OTP code) and not-configured gates "not configured" (e.g., coverage when not configured).
2. If OTP code is present, note that runtime validation hooks from `docs/beam/validation.md` should be cross-checked (cross-ref `docs/elixir/workflows/debugging.md` and the `beam-observability-debugging` skill). Runtime validation is not part of the standard validation gate run; flag it as a follow-up.
3. Compute the overall verdict:
   - **pass** — all applicable gates green.
   - **fail** — any applicable gate red.
4. If any applicable gate failed, hand off to the appropriate fixing workflow — do NOT fix in the validation pass:
   - Defect → `docs/elixir/workflows/debugging.md`
   - Missing tests/docs → `docs/elixir/workflows/implementation.md`
   - Structural cleanup → `docs/elixir/workflows/refactoring.md`
   Then re-run validation after the fix.
5. Keep the report to the per-gate table and overall verdict; do not expand it into a code review or implementation summary.

## Docs to Consult

- `docs/elixir/workflows/validation.md`
- `docs/beam/validation.md`
- `docs/elixir/dependencies-and-packages.md`

## Operational Skills to Load

None. This phase aggregates results; it does not run gates or load operational skills. (If OTP code is present, cross-ref the `beam-observability-debugging` skill for the runtime-validation follow-up note only.)

## Constraints to Apply

- `constraint-elixir-style` — validation is a gate, not a fix; do not modify code; hand off failures to fixing workflows.

## Validations to Run

None. This is a reporting phase — it aggregates all gate results from phases 02-04; it does not run validation gates itself.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any applicable gate red).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-elixir-style"]`
- `tests_run`: the full list of gate commands run across phases 02-04 with pass/fail per gate.
- `validations_run`: `["validation-elixir-compile", "validation-elixir-credo", "validation-elixir-dialyzer", "validation-elixir-test", "validation-elixir-format"]`
- `constraints_checked`: `["constraint-elixir-style"]`
- `evidence`: the per-gate table; the overall verdict; the runtime-validation follow-up note if OTP code is present.
- `failures`: the list of failing applicable gates with evidence.
- `not_fully_checkable`: gates marked N-A or not-configured, and runtime validation (cross-checked separately if OTP code is present).
- `next_phase`: stop.
- `next_workflow`: `null` (if pass) | `debugging` | `implementation` | `refactoring` (if any applicable gate failed).
- `blockers`: the failing gates if `outcome: fail`.
