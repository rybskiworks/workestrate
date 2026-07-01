---
name: workflow-rust-implementation-01-scope
description: |
  Use only for the scope phase of the Rust implementation workflow. Understand
  requirements, read topic docs, and record the intended behavior and public
  surface. Do not use for design, implementation, testing, verification, or for
  reviewing a diff.
allowed-tools: Read Write Edit Bash(cargo:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-implementation
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (Rust implementation)

## Phase purpose

Understand requirements, read topic docs, and record the intended behavior and
public surface. This phase produces the one-paragraph summary that every later
phase is checked against.

## Steps to perform

1. Capture the intended behavior, inputs, outputs, error cases, and any public
   API implications of the work. Write this down before reading any docs.
2. Read `docs/rust/api-design.md` — naming, trait exposure,
   `#[non_exhaustive]`, builder/newtype choices.
3. Read `docs/rust/types-traits-generics.md` — struct/enum/trait/generics
   modeling, `dyn Trait` vs generics, conversions.
4. Read `docs/rust/error-handling.md` — `Option`/`Result`, panic vs recoverable,
   error-type design.
5. Record a one-paragraph summary of the intended behavior and the public
   surface. This summary is the scope contract for the rest of the workflow.

## Docs to consult

- `docs/rust/api-design.md`
- `docs/rust/types-traits-generics.md`
- `docs/rust/error-handling.md`

## Operational skills to load

None are mandatory in the scope phase; skills load in the design and implement
phases. Optionally load `rust-api-design` if the API surface is non-trivial and
would benefit from early naming/trait guidance.

## Constraints to apply

- `constraint-rust-scope-discipline` — stay within the captured requirements.
  Do not fold in related cleanups, unrelated refactors, or speculative
  generalization. If a related cleanup appears, record it as a follow-up.

## Validations to run

None — validations run in phase 05 (workflow-rust-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-rust-implementation-00-orchestration`. Set:

- `outcome` to `pass` once the one-paragraph summary and public surface are
  recorded.
- `constraints_applied` to include `constraint-rust-scope-discipline`.
- `next_phase: 02-design`.
- `blockers: []` unless something prevents proceeding.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-rust-scope-discipline
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
