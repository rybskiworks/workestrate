---
name: workflow-gleam-interop-05-verify
description: |
  Use only for the verify phase of the Gleam interop workflow. Run gleam check,
  gleam format --check, and confirm boundary safety. Do not use for scoping,
  design, implementation, or test writing.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-interop
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: verify (Gleam interop)

## Phase purpose

Run `gleam check` (type-checks the FFI boundary), `gleam format --check`, and
confirm boundary safety — no leaking `Dynamic` into internals, no `panic` in
library FFI. This is the validation phase; it does not introduce new behavior.

## Steps to perform

1. Run the gates in this exact order, stopping and fixing at the root cause if
   any fails. After a fix, re-run from `gleam check`:
   - `gleam check`
   - `gleam test`
   - `gleam format --check`
2. For multi-target projects, run `gleam check` and `gleam test` on both the
   Erlang and JavaScript targets (`--target erlang`, `--target javascript`).
3. Confirm boundary safety:
   - No `Dynamic` leaking into internal modules — `Dynamic` is decoded at the
     edge only.
   - No `panic`/`todo`/`let assert` for expected runtime conditions in library
     FFI.
   - Opaque domain types used in public APIs (not generic external types like
     `Pid` or `Reference`).
   - Every `@external` function has mandatory type annotations.
   - Each `@external` has exactly three arguments with target `erlang` or
     `javascript`.
4. On the JavaScript target, BEAM docs and constraints do NOT apply — do not
   run BEAM-specific checks or apply BEAM constraints for JS-target code.
5. Document the public API per `docs/gleam/conventions-patterns-antipatterns.md`:
   module and function documentation comments, examples for non-trivial public
   functions, and clear error-variant descriptions.
6. Apply `constraint-gleam-conventions` to verify documentation completeness
   and naming/import consistency.
7. `gleam docs build` is a release-only gate, not a standard validation skill;
   run it only if the repo's release policy requires it.
8. Report: the interop surface summary (from phase 01), files added/modified,
   tests added with coverage, raw output/pass-fail of each gate, and any
   deferred policy decisions.

## Docs to consult

- `docs/gleam/externals-and-ffi.md` (review and implementation checklist)
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
  the new code's contribution and surface the pre-existing issue as a
  follow-up.

## Validations to run

Run these validation skills in gate order. Each maps to a command or build
step.

- `validation-gleam-check` — `gleam check` (type-checks the FFI boundary)
- `validation-gleam-test` — `gleam test`
- `validation-gleam-format` — `gleam format --check`

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-interop-00-orchestration`, extended with the verification-phase
extra fields. Set:

- `outcome` to `pass` only when every gate passes and boundary safety is
  confirmed.
- `constraints_applied` to include `constraint-gleam-conventions` and the
  scope-discipline principle.
- `validations_run` to list each validation skill with its pass/fail.
- `constraints_checked` to list each constraint verified.
- `evidence` to include raw output or pass/fail of each gate.
- `failures` to list any gate that failed (empty on full pass).
- `not_fully_checkable` to list anything that could not be fully validated
  automatically and why (e.g. foreign function existence cannot be verified by
  the compiler).
- `next_phase: null` — this is the terminal phase.
- `next_workflow: workflow-gleam-code-review` — when `outcome == pass`, chain
  to code review (interop code is always reviewed).

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
    target: erlang|javascript|both
tests_needed:
  - ...
next_phase: null
next_workflow: workflow-gleam-code-review
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
not_fully_checkable:
  - "Foreign function existence and return-type correctness cannot be verified
    by the compiler; covered by boundary round-trip tests in phase 04 and human
    review in workflow-gleam-code-review."
```
