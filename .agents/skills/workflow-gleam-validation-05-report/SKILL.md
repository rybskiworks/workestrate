---
name: workflow-gleam-validation-05-report
description: |
  Use only for the report phase of the Gleam validation workflow.
  Aggregate all gate results into a per-gate table and report the
  overall verdict. Do not use for scoping, compile, testing, or
  formatting.
allowed-tools: Read
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-validation
  org.phase: report
  org.phase_order: "05"
---

# Gleam Validation Workflow — Phase 05 — Report

## Phase Purpose

Aggregate all gate results into a per-gate table. If OTP/FFI/Erlang-target code
is present (determined in phase 01), note that BEAM runtime concerns may require
follow-up debugging. Report the overall verdict: pass (all applicable gates
green) or fail (any applicable gate red). If a gate failed, hand off to the
appropriate fixing workflow — do NOT fix in the validation pass.

## Steps

1. Aggregate the gate results from phases 02-04 into a per-gate table:

   | Gate | Command | Result | Evidence |
   |---|---|---|---|
   | Format | `gleam format --check` | pass/fail | diff or clean |
   | Type check | `gleam check` | pass/fail | error count or clean |
   | Tests | `gleam test` | pass/fail | passed/failed counts |
   | Per-target tests | `gleam test --target erlang` / `gleam test --target javascript` | pass/fail/N-A | passed/failed counts per target |
   | Build | `gleam build` | pass/fail/N-A | warning count or clean |
   | Package interface | `gleam docs build` | pass/fail/N-A | export errors or clean |
   | Publish | `gleam publish` | pass/fail/N-A | specific failure or clean |
   | Lints | N-A | N-A | Gleam has no separate lint step |
   | Doctests | N-A | N-A | Gleam has no doctest gate; gleeunit collects `*_test` functions under `test/` |
   | Supply chain | N-A | N-A | No Gleam supply-chain validation skill; SBoM is delegated to ORT consuming `manifest.toml` |
   | Miri | N-A | N-A | No Miri for Gleam |
   | Coverage | N-A | N-A | No coverage validation skill |

   Mark not-applicable gates "N-A" (e.g., JavaScript tests when the repo is Erlang-only) and release-only gates "release only" (e.g., `gleam publish`).
2. If OTP/FFI/Erlang-target code is present, note that BEAM runtime concerns may require follow-up (cross-ref `docs/gleam/workflows/debugging.md` and the `gleam-otp-interop` skill). On the JavaScript target, BEAM constraints do not apply.
3. Compute the overall verdict:
   - **pass** — all applicable gates green.
   - **fail** — any applicable gate red.
4. If any applicable gate failed, hand off to the appropriate fixing workflow — do NOT fix in the validation pass:
   - Defect → `docs/gleam/workflows/debugging.md`
   - Missing tests/docs → `docs/gleam/workflows/implementation.md`
   - Structural cleanup → `docs/gleam/workflows/refactoring.md`
   Then re-run validation after the fix.
5. Keep the report to the per-gate table and overall verdict; do not expand it into a code review or implementation summary.

## Docs to Consult

- `docs/gleam/workflows/validation.md`
- `docs/gleam/validation.md`

## Operational Skills to Load

None. This phase aggregates results; it does not run gates or load operational skills. (If OTP/FFI/Erlang-target code is present, cross-ref the `gleam-otp-interop` skill for the BEAM follow-up note only.)

## Constraints to Apply

- scope discipline — validation is a gate, not a fix; do not modify code; hand off failures to fixing workflows.

## Validations to Run

None. This is a reporting phase — it aggregates all gate results from phases 02-04; it does not run validation gates itself.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any applicable gate red).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["scope discipline"]`
- `tests_run`: the full list of gate commands run across phases 02-04 with pass/fail per gate.
- `validations_run`: `["validation-gleam-check", "validation-gleam-test", "validation-gleam-format"]`
- `constraints_checked`: `["scope discipline"]`
- `evidence`: the per-gate table; the overall verdict; the BEAM follow-up note if OTP/FFI/Erlang-target code is present.
- `failures`: the list of failing applicable gates with evidence.
- `not_fully_checkable`: gates marked N-A or release-only, and BEAM runtime concerns (run separately if OTP/FFI/Erlang-target code is present).
- `next_phase`: stop.
- `next_workflow`: `null` (if pass) | `debugging` | `implementation` | `refactoring` (if any applicable gate failed).
- `blockers`: the failing gates if `outcome: fail`.
