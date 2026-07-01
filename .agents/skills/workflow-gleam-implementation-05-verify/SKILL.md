---
name: workflow-gleam-implementation-05-verify
description: |
  Use only for the verify phase of the Gleam implementation workflow. Run the
  full check suite (check, test, format) and report evidence. Do not use for
  scoping, design, implementation, or test writing.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-implementation
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: verify (Gleam implementation)

## Phase purpose

Run the full check suite (check, test, format) and report evidence. This is
the validation phase; it does not introduce new behavior.

## Steps to perform

1. Run the gates in this exact order, stopping and fixing at the root cause if
   any fails. After a fix, re-run from `gleam check`:
   - `gleam check`
   - `gleam test`
   - `gleam format --check`
2. For multi-target projects, run `gleam check` and `gleam test` on both the
   Erlang and JavaScript targets (`--target erlang`, `--target javascript`).
3. Document the public API per `docs/gleam/conventions-patterns-antipatterns.md`:
   module and function documentation comments, examples for non-trivial public
   functions, and clear error-variant descriptions.
4. Apply `constraint-gleam-conventions` to verify documentation completeness
   and naming/import consistency.
5. `gleam docs build` is a release-only gate, not a standard validation skill;
   run it only if the repo's release policy requires it.
6. Report: the intended-behavior summary (from phase 01), files added/modified,
   tests added with coverage, raw output/pass-fail of each gate, and any
   deferred policy decisions.

## Docs to consult

- `docs/gleam/validation.md`
- `docs/gleam/testing.md`

## Operational skills to load

None required; the validation skills execute the gates. Load `gleam-packages-ffi`
if interpretation of `gleam.toml` targets or validation policy is needed.

## Constraints to apply

- `constraint-gleam-conventions` — verify documentation completeness and
  convention consistency for the public API.
- Stay within the captured requirements. Fix only the new code's contribution
  to any gate failure. If a gate fails because of pre-existing code, fix only
  the new code's contribution and surface the pre-existing issue as a follow-up.

## Validations to run

Run these validation skills in gate order. Each maps to a command or build
step.

- `validation-gleam-check` — `gleam check`
- `validation-gleam-test` — `gleam test`
- `validation-gleam-format` — `gleam format --check`

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-implementation-00-orchestration`, extended with the
verification-phase extra fields. Set:

- `outcome` to `pass` only when every gate passes.
- `constraints_applied` to include `constraint-gleam-conventions` and the
  scope-discipline principle.
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
  - constraint-gleam-conventions
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
next_phase: null
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: validation-gleam-check
    gate: gleam check
    result: pass|fail
  - validation: validation-gleam-test
    gate: gleam test
    result: pass|fail
  - validation: validation-gleam-format
    gate: gleam format --check
    result: pass|fail
constraints_checked:
  - constraint-gleam-conventions
  - stay within captured requirements (scope discipline)
evidence:
  - gate: gleam check
    output: ...
  - gate: gleam test
    output: ...
  - gate: gleam format --check
    output: ...
failures: []
not_fully_checkable: []
```
