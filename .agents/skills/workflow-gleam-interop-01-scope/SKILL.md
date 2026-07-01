---
name: workflow-gleam-interop-01-scope
description: |
  Use only for the scope phase of the Gleam interop workflow. Identify the
  interop surface — which target (Erlang vs JavaScript), what external
  functions/types are needed, and the boundary shape. Do not use for design,
  implementation, testing, verification, or reviewing a diff.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-interop
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (Gleam interop)

## Phase purpose

Identify the interop surface: which target(s) are involved (Erlang,
JavaScript, or both), what external functions and external types are needed,
and the shape of the FFI boundary. This phase produces the boundary contract
that every later phase is checked against.

## Steps to perform

1. Capture the intended interop behavior: which target(s), what foreign
   functions are called, what data crosses the boundary, and whether the code
   must work on both targets. Write this down before reading any docs.
2. Read `docs/gleam/externals-and-ffi.md` — `@external` attribute syntax,
   external types, multi-target externals, Gleam fallbacks, the review
   checklist.
3. For the Erlang target, read `docs/gleam/erlang-interop.md` —
   `gleam_erlang` v1.3.0 API surface (`gleam/erlang/process`, `atom`,
   `charlist`, `node`, `reference`, `application`).
4. For the JavaScript target, read `docs/gleam/javascript-target.md` —
   `gleam_javascript` typed wrappers (`array`, `promise`, `symbol`), JS
   externals, `[javascript]` config, prelude API.
5. Record a one-paragraph summary of the interop surface: target(s), external
   functions/types, boundary data shapes, and whether multi-target support is
   required. This summary is the scope contract for the rest of the workflow.

## Docs to consult

- `docs/gleam/externals-and-ffi.md`
- `docs/gleam/erlang-interop.md` (Erlang target)
- `docs/gleam/javascript-target.md` (JavaScript target)

## Operational skills to load

None are mandatory in the scope phase; skills load in the design and implement
phases. Optionally load `gleam-language` if the boundary type design is
non-trivial and would benefit from early naming/type guidance.

## Constraints to apply

- Stay within the captured requirements. Do not fold in related cleanups,
  unrelated refactors, or speculative generalization. If a related cleanup
  appears, record it as a follow-up.

## Validations to run

None — validations run in phase 05 (workflow-gleam-interop-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-interop-00-orchestration`. Set:

- `outcome` to `pass` once the interop surface summary and boundary contract
  are recorded.
- `constraints_applied` to include the scope-discipline principle (no
  back-ticked skill name; there is no dedicated Gleam scope-discipline
  constraint skill).
- `next_phase: 02-design`.
- `blockers: []` unless something prevents proceeding.

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
tests_run: []
tests_needed:
  - ...
next_phase: 02-design
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
