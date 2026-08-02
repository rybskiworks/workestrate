---
name: workflow-gleam-implementation-03-implement
description: |
  Use only for the implement phase of the Gleam implementation workflow. Write
  code following Gleam type and Result rules and the error strategy decided in
  phase 02. Do not use for scoping, design, test-only work, or final
  verification.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-implementation
  org.phase: implement
  org.phase_order: "03"
---

# Phase 03: implement (Gleam implementation)

## Phase purpose

Write the code following Gleam type and `Result` rules and the error strategy
decided in phase 02.

## Steps to perform

1. Load `gleam-language`.
2. Implement the body applying Gleam fundamentals: immutable values, annotated
   public functions, pipelines (`|>`), `use` for `result.try`, exhaustive
   `case` patterns, and custom types that make invalid states unrepresentable.
3. Apply the error strategy decided in phase 02 (`result.try` chaining,
   descriptive error types, `Result` returns).
4. If the code touches OTP actors, Erlang interop, FFI, or BEAM-process
   isolation on the Erlang target, load `gleam-otp-interop` and apply
   `constraint-beam-supervision`, `constraint-beam-failure`, and
   `constraint-beam-process-isolation`. On the JavaScript target these BEAM
   constraints do **not** apply; concurrency is `gleam/javascript/promise`.

## Docs to consult

- `docs/gleam/language-fundamentals.md`
- `docs/gleam/functions-pipelines-and-use.md`
- `docs/gleam/result-option-and-errors.md`
- IF OTP/interop on the Erlang target, also:
  - `docs/gleam/otp-actors-and-supervision.md`
  - `docs/gleam/erlang-interop.md`
  - `docs/beam/supervision.md`
  - `docs/beam/processes-and-messages.md`

## Operational skills to load

- `gleam-language`
- `gleam-packages-ffi`
- (conditional) `gleam-otp-interop` — only if an OTP/FFI/Erlang-target boundary
  is introduced.

## Constraints to apply

- `constraint-gleam-result` — use `Result` for fallible returns, `Option` only
  for optional data, chain with `result.try`, no `panic`/`let assert` for
  expected runtime conditions.
- `constraint-gleam-conventions` — qualified imports, annotated functions,
  singular module names, business-domain grouping, sans-IO boundaries.
- `constraint-beam-supervision` / `constraint-beam-failure` /
  `constraint-beam-process-isolation` — applies only on the Erlang target when
  the code touches OTP actors, supervisors, links/monitors, or shared-state
  boundaries. On the JavaScript target these do **not** apply.
- Stay within the captured requirements. Implement only the requirements
  captured in phase 01; record related cleanups as follow-ups rather than
  folding them in.

## Validations to run

None — validations run in phase 05 (workflow-gleam-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-implementation-00-orchestration`. Set:

- `outcome` to `pass` once the code is written and compiles conceptually
  against the Gleam type system and error strategy.
- `constraints_applied` to include every constraint listed above that applied
  (include the BEAM constraints only on the Erlang target when OTP/interop is
  used).
- `next_phase: 04-test`.
- `blockers: []` unless an OTP/FFI justification needs human review — in that
  case set `handoff_requires_hil: true` and record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-gleam-result
  - constraint-gleam-conventions
  - stay within captured requirements (scope discipline)
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 04-test
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
