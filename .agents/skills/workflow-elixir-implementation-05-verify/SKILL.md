---
name: workflow-elixir-implementation-05-verify
description: |
  Use only for the verify phase of the Elixir implementation workflow. Run the
  full check suite (compile, credo, dialyzer, test, format) and report evidence.
  Do not use for scoping, design, implementation, or test writing.
allowed-tools: Read Write Edit Bash(mix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-implementation
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: verify (Elixir implementation)

## Phase purpose

Run the full check suite (compile, credo, dialyzer, test, format) and report
evidence. This is the validation phase; it does not introduce new behavior.

## Steps to perform

1. Run the gates in this exact order, stopping and fixing at the root cause if
   any fails. After a fix, re-run from `mix compile --warnings-as-errors`:
   - `mix compile --warnings-as-errors`
   - `mix credo --strict`
   - `mix dialyzer`
   - `mix test`
   - `mix format --check-formatted`
2. Document the public API per `docs/elixir/documentation-and-publishing.md`:
   `@moduledoc` at module top, `@doc` on items, `@spec` on public functions, and
   doctests for non-trivial public functions.
3. Apply `constraint-elixir-otp-api` to verify documentation completeness.
4. Report: the intended-behavior summary (from phase 01), files added/modified,
   tests added with coverage, raw output/pass-fail of each gate, and any
   deferred policy decisions.

## Docs to consult

- `docs/elixir/documentation-and-publishing.md`
- `docs/elixir/static-analysis-credo.md`
- `docs/elixir/typespecs-and-dialyzer.md`

## Operational skills to load

- `elixir-static-analysis` — for credo and dialyzer gate interpretation.

## Constraints to apply

- `constraint-elixir-otp-api` — verify documentation completeness for the public
  API.
- `constraint-elixir-style` — fix only the new code's contribution to any gate
  failure. If a gate fails because of pre-existing code, fix only the new code's
  contribution and surface the pre-existing issue as a follow-up.

## Validations to run

Run these validation skills in gate order. Each maps to a command or build
step.

- `validation-elixir-compile` — `mix compile --warnings-as-errors`
- `validation-elixir-credo` — `mix credo --strict`
- `validation-elixir-dialyzer` — `mix dialyzer`
- `validation-elixir-test` — `mix test`
- `validation-elixir-format` — `mix format --check-formatted`

## Handoff output

Return the handoff YAML schema defined in
`workflow-elixir-implementation-00-orchestration`, extended with the
verification-phase extra fields. Set:

- `outcome` to `pass` only when every gate passes.
- `constraints_applied` to include `constraint-elixir-otp-api` and
  `constraint-elixir-style`.
- `validations_run` to list each validation skill with its pass/fail.
- `constraints_checked` to list each constraint verified.
- `evidence` to include raw output or pass/fail of each gate.
- `failures` to list any gate that failed (empty on full pass).
- `not_fully_checkable` to list anything that could not be fully validated
  automatically and why.
- `next_phase: null` and `next_workflow: null` — this is the terminal phase.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-elixir-otp-api
  - constraint-elixir-style
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: ...
    covers: ...
tests_needed:
  - ...
next_phase: null
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: validation-elixir-compile
    gate: mix compile --warnings-as-errors
    result: pass|fail
  - validation: validation-elixir-credo
    gate: mix credo --strict
    result: pass|fail
  - validation: validation-elixir-dialyzer
    gate: mix dialyzer
    result: pass|fail
  - validation: validation-elixir-test
    gate: mix test
    result: pass|fail
  - validation: validation-elixir-format
    gate: mix format --check-formatted
    result: pass|fail
constraints_checked:
  - constraint-elixir-otp-api
  - constraint-elixir-style
evidence:
  - gate: mix compile --warnings-as-errors
    output: ...
  - gate: mix credo --strict
    output: ...
  - gate: mix dialyzer
    output: ...
  - gate: mix test
    output: ...
  - gate: mix format --check-formatted
    output: ...
failures: []
not_fully_checkable: []
```
