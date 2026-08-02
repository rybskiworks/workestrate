---
name: workflow-gleam-validation-02-compile
description: |
  Use only for the compile phase of the Gleam validation workflow.
  Run gleam check. Do not use for scoping, testing, formatting, or final
  reporting.
allowed-tools: Read Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-validation
  org.phase: compile
  org.phase_order: "02"
---

# Gleam Validation Workflow — Phase 02 — Type Check

## Phase Purpose

Run the type-check gate. Record pass/fail and evidence. A failure here is a
blocker for the overall verdict, but continue to the remaining gates to give a
full picture. Gleam has no separate lint step: `gleam check` is the strict,
non-configurable type+compile gate.

## Steps

1. Run the type-check gate:
   ```sh
   gleam check
   ```
   `gleam check` performs whole-program type inference and exhaustiveness checking without code generation. Map to `validation-gleam-check`.
2. There is no separate lint gate. Gleam does not have Clippy, Dialyzer, or configurable lint warnings. Any type error or exhaustiveness failure is reported by `gleam check`.
3. For the gate, record pass/fail and the relevant output (error count or clean).
4. If the gate fails, continue to the remaining gates (phase 03, 04) to give a full picture; the overall result will be fail.
5. Do NOT modify code to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/gleam/validation.md`

## Operational Skills to Load

- `gleam-language` — language fundamentals, exhaustiveness, Result/Option error model.
- `gleam-packages-ffi` — CLI and validation gate procedures.

## Constraints to Apply

- scope discipline — do not modify code or `gleam.toml` to make a gate pass.

## Validations to Run

- `validation-gleam-check` — command: `gleam check`

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (gate green) | fail (gate red).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["scope discipline"]`
- `tests_run`: `["gleam check"]` with pass/fail + evidence.
- `risks`: any gate red.
- `next_phase`: `03-lint-test`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
