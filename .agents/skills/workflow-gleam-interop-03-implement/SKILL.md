---
name: workflow-gleam-interop-03-implement
description: |
  Use only for the implement phase of the Gleam interop workflow. Write
  @external attributes, target-specific modules, and gleam_erlang /
  gleam_javascript wrappers. Do not use for scoping, design, test-only work,
  or final verification.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-interop
  org.phase: implement
  org.phase_order: "03"
---

# Phase 03: implement (Gleam interop)

## Phase purpose

Write the interop code: `@external` attributes, target-specific modules, and
`gleam_erlang` / `gleam_javascript` typed wrappers, following the boundary
design decided in phase 02.

## Steps to perform

1. Load `gleam-language` and `gleam-packages-ffi`.
2. Implement external functions with `@external` attributes. Each attribute
   takes exactly three arguments: target (`erlang` or `javascript`), module,
   and function name. Type annotations are **mandatory** on every external
   function.
   - Erlang: `@external(erlang, "lists", "reverse")`
   - JavaScript: `@external(javascript, "./project_ffi.mjs", "reverse_list")`
   - Elixir: same `erlang` target with `Elixir.` prefix:
     `@external(erlang, "Elixir.Pokemon", "badge_count")`
3. For multi-target support, place multiple `@external` attributes on one
   function, or provide a Gleam body fallback (the external is used when
   compiling to its target; otherwise the Gleam body runs). A function with
   `@external` for only one target is usable only on that target — using it on
   the other is a compile error.
4. Use target-specific modules with `if erlang { }` / `if javascript { }`
   blocks for target-specific code paths.
5. For the Erlang target, load `gleam-otp-interop` and use `gleam_erlang`
   v1.3.0:
   - `gleam/erlang/process` — `send`, `call`, `call_forever`, `monitor`,
     `demonitor_process`, `link`, `unlink`, `trap_exits`, `send_exit`,
     `send_abnormal_exit`, `kill`, `spawn` (linked), `spawn_unlinked`,
     `self`, `sleep`, `Subject`, `Selector`, `new_subject`, `new_selector`,
     `select`, `selector_receive`, `selector_receive_forever`.
   - Note: v1.3.0 has **no** `try_send` or `monitor_process` — use `send` and
     `monitor`. `send`/`call` panic on failure (no result-returning variants).
   - `gleam/erlang/atom`, `gleam/erlang/charlist`, `gleam/erlang/node`,
     `gleam/erlang/reference`, `gleam/erlang/application` (`priv_directory`,
     `StartType`).
6. For the JavaScript target, use `gleam_javascript`:
   - `gleam/javascript/array` — typed wrapper around JS `Array` (mutable,
     index-based; distinct from Gleam's immutable `List`).
   - `gleam/javascript/promise` — typed wrapper around JS `Promise`; the
     JS-target concurrency primitive (where Erlang target uses processes/OTP).
   - `gleam/javascript/symbol` — typed wrapper around JS `Symbol`.
7. Respect Gleam↔foreign data mappings:
   - Erlang: `String` is UTF-8 binary (not `string()`/charlist); `Ok(x)`/
     `Error(x)` are `{ok, x}`/`{error, x}` tagged tuples (not bare atoms);
     lists must be proper.
   - JavaScript: tuples are arrays `[a, b]` (never mutate); `Int` must be
     whole (no `Infinity`/`NaN`); `Nil` is `undefined`.
8. Keep `Dynamic` at the edge — decode at the boundary, never let it
   propagate into internal modules. Use opaque domain types over generic
   external types in public APIs.
9. If the Erlang-target boundary touches NIFs, apply
   `constraint-beam-nif-safety`. On the JavaScript target, BEAM docs and
   constraints do NOT apply.

## Docs to consult

- `docs/gleam/externals-and-ffi.md`
- `docs/gleam/erlang-interop.md` (Erlang target)
- `docs/gleam/javascript-target.md` (JavaScript target)
- IF NIFs on the Erlang target, also:
  - `docs/beam/nifs.md`
- IF ports on the Erlang target, also:
  - `docs/beam/ports-io.md`

## Operational skills to load

- `gleam-language`
- `gleam-packages-ffi`
- `gleam-otp-interop` (Erlang target — process primitives, OTP actors,
  multi-target concurrency)

## Constraints to apply

- `constraint-gleam-result` — use `Result` for fallible FFI returns, `Option`
  only for optional data, chain with `result.try`, no `panic`/`let assert` for
  expected runtime conditions in library FFI.
- `constraint-gleam-conventions` — qualified imports, annotated functions,
  singular module names, business-domain grouping, opaque domain types.
- (Erlang target, NIFs only) `constraint-beam-nif-safety` — applies when the
  boundary touches NIFs. On the JavaScript target, BEAM constraints do NOT
  apply.
- Stay within the captured requirements. Implement only the requirements
  captured in phase 01; record related cleanups as follow-ups rather than
  folding them in.

## Validations to run

None — validations run in phase 05 (workflow-gleam-interop-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-interop-00-orchestration`. Set:

- `outcome` to `pass` once the interop code is written and compiles
  conceptually against the Gleam type system and boundary design.
- `constraints_applied` to include every constraint listed above that applied
  (include `constraint-beam-nif-safety` only on the Erlang target when NIFs
  are used).
- `next_phase: 04-test`.
- `blockers: []` unless an FFI justification needs human review — in that
  case set `handoff_requires_hil: true` and record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-gleam-result
  - constraint-gleam-conventions
  - constraint-beam-nif-safety
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
