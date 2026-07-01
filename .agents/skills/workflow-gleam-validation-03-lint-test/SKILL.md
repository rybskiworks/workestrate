---
name: workflow-gleam-validation-03-lint-test
description: |
  Use only for the test phase of the Gleam validation workflow.
  Run gleam test and per-target test runs. Do not use for scoping,
  compile, formatting, or final reporting.
allowed-tools: Read Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-validation
  org.phase: lint-test
  org.phase_order: "03"
---

# Gleam Validation Workflow — Phase 03 — Test

## Phase Purpose

Run the test gate and per-target test gates. Record pass/fail and evidence for
each. All tests must pass. Per-target runs are important because the type system
does not catch target-specific runtime differences.

## Steps

1. Run the test suite:
   ```sh
   gleam test
   ```
   Map to `validation-gleam-test`.
2. Run per-target tests if the repo supports multiple targets (determined in phase 01):
   ```sh
   gleam test --target erlang
   gleam test --target javascript
   ```
   The JavaScript target uses `gleam/javascript/promise` for concurrency, not BEAM processes/OTP; BEAM constraints do not apply there. Map each per-target run to `validation-gleam-test` (target facet).
3. For each gate, record pass/fail and the passed/failed counts.
4. If a gate fails, continue to the remaining gates (phase 04) to give a full picture; the overall result will be fail.
5. Do NOT add tests or modify code to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/gleam/testing.md`
- `docs/gleam/validation.md`

## Operational Skills to Load

- `gleam-packages-ffi` — gleeunit test procedures, target matrix.

## Constraints to Apply

- scope discipline — do not add tests or modify code to make a gate pass.

## Validations to Run

- `validation-gleam-test` — command: `gleam test` (and `gleam test --target erlang` / `gleam test --target javascript` as target facets)

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all gates green) | fail (any gate red).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["scope discipline"]`
- `tests_run`: `["gleam test", "gleam test --target erlang", "gleam test --target javascript"]` with pass/fail + passed/failed counts per gate (mark not-applicable targets).
- `risks`: any gate red; target-specific runtime failures.
- `next_phase`: `04-format-doc`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
