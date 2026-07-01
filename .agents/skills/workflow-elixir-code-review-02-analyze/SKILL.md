---
name: workflow-elixir-code-review-02-analyze
description: |
  Use only for the analyze phase of the Elixir code-review workflow. Categorize
  the diff by dimension and load the operational review skills that match. Do
  not use for scoping, running gates, manual review, or issuing a verdict.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-code-review
  org.phase: analyze
  org.phase_order: "02"
---

# Phase 02: analyze (Elixir code review)

## Phase purpose

Categorize the diff by dimension and load the operational review skills that
match the categories, so phase 04-review has the right skills and docs ready.

## Steps to perform

1. Categorize the diff by dimension: does it touch naming/style, OTP,
   error handling, typespecs, documentation, or dependencies? Record the
   categories in `risks`.
2. Load `elixir-coding` — naming, pattern matching, pipe usage, function-shape
   conventions.
3. Load `elixir-static-analysis` — Credo interpretation, `# credo:disable-for-next-line`
   justification, Dialyzer warning triage. This is **mandatory** for every
   Elixir review.
4. Conditionally load `elixir-otp` — child specs, restart strategies, `GenServer`
   callbacks. Load if the diff touches OTP code.
5. Conditionally load `elixir-error-handling` — error tuples vs exceptions,
   bang/non-bang pairs, swallowed errors. Load if the diff touches error paths.
6. Conditionally load `beam-supervision`, `beam-gen-server`, and
   `beam-errors-failures` if the diff touches OTP supervision, `GenServer`
   callbacks, or exit-signal handling. When in doubt, load them.
7. Apply `constraint-elixir-style`: categorization covers only the diff; do not
   expand scope to untouched code.

## Docs to consult

- `docs/elixir/static-analysis-credo.md`
- `docs/elixir/typespecs-and-dialyzer.md`
- `docs/elixir/naming-conventions.md`

## Operational skills to load

- `elixir-coding`
- `elixir-static-analysis` (mandatory)
- `elixir-otp` (if OTP code present)
- `elixir-error-handling` (conditional)
- `beam-supervision` (if OTP supervision present)
- `beam-gen-server` (if `GenServer` callbacks present)
- `beam-errors-failures` (if error/exit paths present)

## Constraints to apply

- `constraint-elixir-style` — categorization covers only the diff; do not
  expand scope to untouched code.

## Validations to run

None — validations run in phase 03 (workflow-elixir-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-elixir-code-review-00-orchestration`. Set:

- `outcome`: `pass` if categorization completed and skills loaded; `partial`
  if a conditional skill was deliberately not loaded (record why in
  `assumptions`).
- `constraints_applied`: `constraint-elixir-style`.
- `risks`: the dimensions the diff touches.
- `assumptions`: any conditional-load decisions and their rationale.
- `next_phase`: `03-check`.
- `next_workflow`: `null`.
