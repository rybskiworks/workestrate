---
name: workflow-gleam-code-review-02-analyze
description: |
  Use only for the analyze phase of the Gleam code-review workflow. Categorize
  the diff by dimension and load the operational review skills that match. Do
  not use for scoping, running gates, manual review, or issuing a verdict.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-code-review
  org.phase: analyze
  org.phase_order: "02"
---

# Phase 02: analyze (Gleam code review)

## Phase purpose

Categorize the diff by dimension and load the operational review skills that
match the categories, so phase 04-review has the right skills and docs ready.

## Steps to perform

1. Categorize the diff by dimension: does it touch Result/Option/error handling,
   externals/FFI, OTP/actors/supervision, public API, or dependencies? Record
   the categories in `risks`.
2. Load `gleam-language` — Result/Option error model, naming, type design,
   conventions, anti-patterns.
3. Load `gleam-otp-interop` — OTP actors/supervision, Erlang interop,
   `@external`/FFI, JavaScript target. This is **mandatory if the diff touches
   OTP/actors/supervision, Erlang interop, or externals/FFI**.
4. Load `gleam-packages-ffi` — project structure, `gleam.toml`, dependencies,
   validation gates, publishing. Load if the diff touches project config,
   dependencies, or package surface.
5. Apply scope discipline: categorization covers only the diff; do not expand
   scope to untouched code.

## Docs to consult

- `docs/gleam/conventions-patterns-antipatterns.md`
- `docs/gleam/result-option-and-errors.md`
- `docs/gleam/externals-and-ffi.md`
- `docs/gleam/types-records-and-patterns.md`

On the JavaScript target, BEAM/OTP docs do not apply; concurrency is
`gleam/javascript/promise`, not BEAM processes/OTP.

## Operational skills to load

- `gleam-language`
- `gleam-otp-interop` (if OTP/FFI/Erlang interop present)
- `gleam-packages-ffi` (conditional: project/deps/config touched)

## Constraints to apply

- Scope discipline — categorization covers only the diff; do not expand scope
  to untouched code.

## Validations to run

None — validations run in phase 03 (`workflow-gleam-code-review-03-check`).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-gleam-code-review-00-orchestration`. Set:

- `outcome`: `pass` if categorization completed and skills loaded; `partial`
  if a conditional skill was deliberately not loaded (record why in
  `assumptions`).
- `constraints_applied`: scope discipline.
- `risks`: the dimensions the diff touches.
- `assumptions`: any conditional-load decisions and their rationale.
- `next_phase`: `03-check`.
- `next_workflow`: `null`.
