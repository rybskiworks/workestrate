---
name: workflow-gleam-interop-02-design
description: |
  Use only for the design phase of the Gleam interop workflow. Design the FFI
  boundary — typed wrappers, Dynamic decoding at boundaries, opaque external
  types, and type safety. Do not use for scoping, implementation, testing, or
  final verification.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-interop
  org.phase: design
  org.phase_order: "02"
---

# Phase 02: design (Gleam interop)

## Phase purpose

Design the FFI boundary so the implement phase can proceed without mid-flight
architectural changes: typed wrappers, `Dynamic` decoding at the edge, opaque
external types, and type safety that keeps `Dynamic` out of internals.

## Steps to perform

1. Load `gleam-language` — naming, type design, `Result`/`Option` error model,
   conventions.
2. Load `gleam-packages-ffi` — `gleam.toml` changes, dependency management,
   module structure, package surface.
3. Read `docs/gleam/externals-and-ffi.md` — external types (no variants,
   opaque), the "Dynamic at FFI boundaries" convention (do not use `Dynamic`
   to represent FFI types; define a precise domain type instead), and the
   review/implementation checklist.
4. For the Erlang target, read `docs/gleam/erlang-interop.md` — prefer
   `gleam_erlang` typed wrappers over raw external types; define precise
   domain types (e.g. `TransactionId`) rather than exposing generic types
   (e.g. `Reference`).
5. For the JavaScript target, read `docs/gleam/javascript-target.md` — use
   `gleam_javascript` typed wrappers (`array`/`promise`/`symbol`) instead of
   untyped `Dynamic`; design idiomatic Gleam APIs, not mirrors of the external
   JS API.
6. Decide the boundary type strategy:
   - External types are opaque (no variants); only manipulated via external
     functions.
   - Keep `Dynamic` at the edge — decode at the boundary, never let it
     propagate into internal modules.
   - Use opaque domain types (e.g. `ZipHandle`) over generic external types
     (e.g. `Pid`) in public APIs.
7. Decide the error strategy at the boundary:
   - Fallible FFI functions return `Result(a, e)`; use `Nil` as the error
     type when there is no extra detail.
   - Never use `panic`/`todo`/`let assert` for expected runtime conditions in
     library FFI; reserve them for genuinely unreachable states and tests.
   - On the Erlang target, `gleam_erlang` v1.3.0 `send`/`call` panic on
     failure — design callers that tolerate this or wrap with monitors.
8. Conditionally load `gleam-otp-interop` if the boundary touches OTP actors,
   Erlang process primitives, or multi-target concurrency.
9. If the Erlang-target boundary touches NIFs, read `docs/beam/nifs.md` and
   apply `constraint-beam-nif-safety`. If it touches ports, read
   `docs/beam/ports-io.md`.

## Docs to consult

- `docs/gleam/externals-and-ffi.md`
- `docs/gleam/erlang-interop.md` (Erlang target)
- `docs/gleam/javascript-target.md` (JavaScript target)
- `docs/beam/nifs.md` (Erlang target, NIFs only)
- `docs/beam/ports-io.md` (Erlang target, ports only)

## Operational skills to load

- `gleam-language`
- `gleam-packages-ffi`
- (conditional) `gleam-otp-interop` — only if the boundary touches OTP / Erlang
  process primitives / multi-target concurrency.

## Constraints to apply

- Stay within the captured requirements. Design only what the captured
  requirements demand. Do not introduce speculative abstractions, new
  dependencies, or module boundaries beyond what the new code requires.
- `constraint-gleam-conventions` — plan naming, imports, module structure, and
  public API documentation so the implement and verify phases can produce a
  consistent package surface.
- `constraint-gleam-result` — design `Result` returns for fallible FFI
  functions; `Option` only for optional data; no `panic`/`let assert` for
  expected runtime conditions.
- (Erlang target, NIFs only) `constraint-beam-nif-safety` — applies when the
  boundary touches NIFs. On the JavaScript target, BEAM constraints do NOT
  apply.

## Validations to run

None — validations run in phase 05 (workflow-gleam-interop-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-interop-00-orchestration`. Set:

- `outcome` to `pass` once boundary type strategy, error strategy, and
  target-specific design decisions are decided and recorded.
- `constraints_applied` to include the scope-discipline principle,
  `constraint-gleam-conventions`, `constraint-gleam-result`, and
  `constraint-beam-nif-safety` (Erlang/NIFs only).
- `next_phase: 03-implement`.
- `blockers: []` unless a policy decision (e.g. target choice, new dependency,
  or NIF approval) needs human input — in that case set
  `handoff_requires_hil: true` and record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - stay within captured requirements (scope discipline)
  - constraint-gleam-conventions
  - constraint-gleam-result
  - constraint-beam-nif-safety
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 03-implement
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
