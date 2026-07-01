---
name: workflow-elixir-implementation-01-scope
description: |
  Use only for the scope phase of the Elixir implementation workflow. Understand
  requirements, read topic docs, and record the intended behavior and public
  surface. Do not use for design, implementation, testing, verification, or for
  reviewing a diff.
allowed-tools: Read Write Edit Bash(mix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-implementation
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (Elixir implementation)

## Phase purpose

Understand requirements, read topic docs, and record the intended behavior and
public surface. This phase produces the one-paragraph summary that every later
phase is checked against.

## Steps to perform

1. Capture the intended behavior, inputs, outputs, error cases, and any public
   API implications of the work. Write this down before reading any docs.
2. Read `docs/elixir/naming-conventions.md` — casing, module/function/atom
   naming, `foo`/`foo!` pairs.
3. Read `docs/elixir/language-fundamentals.md` — pattern matching, immutability,
   pipe operator, control flow.
4. Read `docs/elixir/core-modules.md` — `Enum` vs `Stream`, `Map`/`Keyword`/
   `MapSet`, string types.
5. Read `docs/elixir/error-handling.md` — error tuples vs exceptions, bang
   variants, `raise`/`rescue` boundaries.
6. Record a one-paragraph summary of the intended behavior and the public
   surface. This summary is the scope contract for the rest of the workflow.

## Docs to consult

- `docs/elixir/naming-conventions.md`
- `docs/elixir/language-fundamentals.md`
- `docs/elixir/core-modules.md`
- `docs/elixir/error-handling.md`

## Operational skills to load

None are mandatory in the scope phase; skills load in the design and implement
phases. Optionally load `elixir-coding` if the API surface is non-trivial and
would benefit from early naming/shape guidance.

## Constraints to apply

- `constraint-elixir-style` — stay within the captured requirements. Do not fold
  in related cleanups, unrelated refactors, or speculative generalization. If a
  related cleanup appears, record it as a follow-up.

## Validations to run

None — validations run in phase 05 (workflow-elixir-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-elixir-implementation-00-orchestration`. Set:

- `outcome` to `pass` once the one-paragraph summary and public surface are
  recorded.
- `constraints_applied` to include `constraint-elixir-style`.
- `next_phase: 02-design`.
- `blockers: []` unless something prevents proceeding.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-elixir-style
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
