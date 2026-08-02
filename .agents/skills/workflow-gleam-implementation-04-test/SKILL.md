---
name: workflow-gleam-implementation-04-test
description: |
  Use only for the test phase of the Gleam implementation workflow. Write tests
  alongside the code and run gleam test. Do not use for scoping, design,
  implementation, or final verification.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-implementation
  org.phase: test
  org.phase_order: "04"
---

# Phase 04: test (Gleam implementation)

## Phase purpose

Write tests alongside the code and run `gleam test` to confirm the
implementation behaves as scoped in phase 01.

## Steps to perform

1. Load `gleam-packages-ffi` (testing procedures live there).
2. Read `docs/gleam/testing.md`.
3. Write gleeunit tests under `test/` as `pub fn ..._test` functions.
4. Ensure the test entrypoint calls `gleeunit.main()`.
5. Cover: happy path, each error branch, and edge cases (empty input, boundary
   values, both targets for multi-target code).
6. Run `gleam test`; for multi-target projects also run
   `gleam test --target erlang` and `gleam test --target javascript`. Fix
   failures at the root cause before proceeding. Do not suppress failures by
   deleting tests.

## Docs to consult

- `docs/gleam/testing.md`

## Operational skills to load

- `gleam-packages-ffi`

## Constraints to apply

- Stay within the captured requirements. Test the scoped behavior, not
  unrelated modules the new code touches only at a call site. Do not add tests
  for speculative future behavior.

## Validations to run

- `validation-gleam-test` — runs `gleam test` and records structured pass/fail
  evidence for the test suite.

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-implementation-00-orchestration`. Set:

- `outcome` to `pass` once `gleam test` passes and the test set covers happy
  path, error branches, and edge cases.
- `constraints_applied` to include the scope-discipline principle.
- `tests_run` to list every test added (unit/integration) with what each covers.
- `next_phase: 05-verify`.
- `blockers: []` unless a test reveals a defect that needs phase 03 to revisit
  — in that case set `outcome: fail` and record the failing test.

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
tests_needed:
  - ...
next_phase: 05-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
