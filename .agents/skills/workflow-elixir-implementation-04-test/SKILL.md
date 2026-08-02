---
name: workflow-elixir-implementation-04-test
description: |
  Use only for the test phase of the Elixir implementation workflow. Write
  ExUnit tests alongside code and run mix test. Do not use for scoping, design,
  implementation, or final verification.
allowed-tools: Read Write Edit Bash(mix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-implementation
  org.phase: test
  org.phase_order: "04"
---

# Phase 04: test (Elixir implementation)

## Phase purpose

Write ExUnit tests alongside the code and run `mix test` to confirm the
implementation behaves as scoped in phase 01.

## Steps to perform

1. Load `elixir-testing`.
2. Read `docs/elixir/testing-exunit.md`.
3. Write unit tests in the same module (`defmodule MyModuleTest do
   use ExUnit.Case` in `test/`).
4. Cover: happy path, each error branch, and edge cases (empty input, boundary
   values, concurrent access for OTP code).
5. Add doctests for non-trivial public functions.
6. Run `mix test`; fix failures at the root cause before proceeding. Do not
   suppress failures with `@tag :skip` or by deleting tests unless the skip is
   explicitly justified and temporary.

## Docs to consult

- `docs/elixir/testing-exunit.md`

## Operational skills to load

- `elixir-testing`

## Constraints to apply

- `constraint-elixir-style` — test the scoped behavior, not unrelated modules
  the new code touches only at a call site. Do not add tests for speculative
  future behavior.

## Validations to run

- `validation-elixir-test` — runs `mix test` and records structured pass/fail
  evidence for the test suite.

## Handoff output

Return the handoff YAML schema defined in
`workflow-elixir-implementation-00-orchestration`. Set:

- `outcome` to `pass` once `mix test` passes and the test set covers happy
  path, error branches, and edge cases.
- `constraints_applied` to include `constraint-elixir-style`.
- `tests_run` to list every test added (unit, doctest) with what each covers.
- `next_phase: 05-verify`.
- `blockers: []` unless a test reveals a defect that needs phase 03 to revisit
  — in that case set `outcome: fail` and record the failing test.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
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
next_phase: 05-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
