---
name: workflow-elixir-implementation-03-implement
description: |
  Use only for the implement phase of the Elixir implementation workflow. Write
  code following idiomatic Elixir patterns and the error strategy decided in
  phase 02. Do not use for scoping, design, test-only work, or final
  verification.
allowed-tools: Read Write Edit Bash(mix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-implementation
  org.phase: implement
  org.phase_order: "03"
---

# Phase 03: implement (Elixir implementation)

## Phase purpose

Write the code following idiomatic Elixir patterns and the error strategy
decided in phase 02.

## Steps to perform

1. Load `elixir-coding`.
2. Implement the body applying pattern matching, immutability, and the pipe
   operator: prefer multi-clause functions over nested conditionals, keep
   transformations explicit with `|>`, and avoid reassignment.
3. Choose standard-library modules (`Enum`, `Stream`, `Map`, `Keyword`,
   `MapSet`, `String`) per `docs/elixir/core-modules.md` rather than adding a
   dependency.
4. Apply the error strategy decided in phase 02 (`{:ok, _}` / `{:error, _}`
   tuples at boundaries, bang variants where failure is exceptional).
5. If the implementation involves OTP, load `elixir-otp` and apply
   `constraint-beam-supervision` for supervisors, child specs, and restart
   strategies; consult `docs/elixir/otp-supervision.md` and
   `docs/beam/supervision.md`.
6. If implementing a GenServer, load `beam-gen-server` and follow the callback
   contract from `docs/beam/gen-server.md`.
7. If the implementation touches exit signals or error semantics, load
   `beam-errors-failures` and apply `constraint-beam-failure`.
8. If NIFs are needed, apply `constraint-beam-process-isolation`: isolate NIF
   work, consult `docs/beam/nifs.md`, and flag the change for review.

## Docs to consult

- `docs/elixir/language-fundamentals.md`
- `docs/elixir/core-modules.md`
- `docs/elixir/error-handling.md`
- `docs/elixir/otp-supervision.md` (if OTP)
- `docs/beam/supervision.md` (if OTP)

## Operational skills to load

- `elixir-coding`
- `elixir-error-handling`
- (conditional) `elixir-otp` — only if the implementation involves OTP
  processes or supervision.
- (conditional) `beam-supervision` — only when adding or modifying supervisors,
  child specs, or restart strategies.
- (conditional) `beam-gen-server` — only when implementing a GenServer.
- (conditional) `beam-errors-failures` — only when the implementation touches
  exit signals or error semantics.

## Constraints to apply

- `constraint-elixir-style` — enforce pattern matching, pipe usage, and naming
  conventions; avoid reflexive `if/else` where multi-clause functions fit.
- `constraint-beam-failure` — return error tuples for expected runtime
  conditions; reserve `raise`/`throw` for genuine invariants and tests.
- `constraint-beam-supervision` — applies if OTP is present: justify process
  boundaries, child specs, and restart strategies.
- `constraint-beam-process-isolation` — applies if NIFs are present: isolate
  native work and flag for review.
- `constraint-elixir-otp-api` — implement `@moduledoc`, `@doc`, `@spec`, and
  doctests for the public API as planned in phase 02.

## Validations to run

None — validations run in phase 05 (workflow-elixir-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-elixir-implementation-00-orchestration`. Set:

- `outcome` to `pass` once the code is written and follows the idiomatic
  patterns and error strategy.
- `constraints_applied` to include every constraint listed above that applied
  (include `constraint-beam-supervision` only if OTP was used, and
  `constraint-beam-process-isolation` only if NIFs were used).
- `next_phase: 04-test`.
- `blockers: []` unless a NIF or supervision justification needs human review —
  in that case set `handoff_requires_hil: true` and record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-elixir-style
  - constraint-beam-failure
  - constraint-elixir-otp-api
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
