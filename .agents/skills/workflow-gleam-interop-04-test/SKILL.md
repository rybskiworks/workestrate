---
name: workflow-gleam-interop-04-test
description: |
  Use only for the test phase of the Gleam interop workflow. Write
  target-specific tests and boundary round-trips. Do not use for scoping,
  design, implementation, or final verification.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-interop
  org.phase: test
  org.phase_order: "04"
---

# Phase 04: test (Gleam interop)

## Phase purpose

Write target-specific tests and boundary round-trips to confirm the interop
behaves as scoped in phase 01. The compiler cannot verify that foreign
functions exist or return their annotated types, so FFI boundaries require
extra test coverage.

## Steps to perform

1. Load `gleam-packages-ffi` (testing procedures live there).
2. Read `docs/gleam/testing.md`.
3. Write gleeunit tests under `test/` as `pub fn ..._test` functions.
4. Cover: happy path, each error branch, and edge cases at every FFI boundary
   (empty input, boundary values, type-round-trip correctness).
5. For multi-target projects, run target-specific tests:
   - `gleam test --target erlang`
   - `gleam test --target javascript`
   For single-target projects, run `gleam test` for the relevant target.
6. Write boundary round-trip tests: Gleam value → foreign function → back to
   Gleam, asserting the value survives the round-trip with correct types.
7. Fix failures at the root cause before proceeding. Do not suppress failures
   by deleting tests.

## Docs to consult

- `docs/gleam/testing.md`
- `docs/gleam/externals-and-ffi.md` (review checklist: "additional unit tests
  written at every FFI boundary")

## Operational skills to load

- `gleam-packages-ffi`

## Constraints to apply

- Stay within the captured requirements. Test the scoped interop behavior, not
  unrelated modules the new code touches only at a call site. Do not add tests
  for speculative future behavior.

## Validations to run

- `validation-gleam-test` — runs `gleam test` (and target-specific variants)
  and records structured pass/fail evidence for the test suite.

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-interop-00-orchestration`. Set:

- `outcome` to `pass` once `gleam test` passes on all relevant targets and the
  test set covers happy path, error branches, edge cases, and boundary
  round-trips.
- `constraints_applied` to include the scope-discipline principle.
- `tests_run` to list every test added (unit/integration) with what each
  covers, including which target(s) each was run on.
- `next_phase: 05-verify`.
- `blockers: []` unless a test reveals a defect that needs phase 03 to
  revisit — in that case set `outcome: fail` and record the failing test.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - stay within captured requirements (scope discipline)
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: ...
    covers: ...
    target: erlang|javascript|both
tests_needed:
  - ...
next_phase: 05-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
