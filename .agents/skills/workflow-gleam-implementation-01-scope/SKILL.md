---
name: workflow-gleam-implementation-01-scope
description: |
  Use only for the scope phase of the Gleam implementation workflow. Understand
  requirements, read topic docs, and record the intended behavior and public
  surface. Do not use for design, implementation, testing, verification, or for
  reviewing a diff.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-implementation
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (Gleam implementation)

## Phase purpose

Understand requirements, read topic docs, and record the intended behavior and
public surface. This phase produces the one-paragraph summary that every later
phase is checked against.

## Steps to perform

1. Capture the intended behavior, inputs, outputs, error cases, target
   (Erlang/JavaScript/both), and any public API implications of the work. Write
   this down before reading any docs.
2. Read `docs/gleam/language-fundamentals.md` — variables, immutability,
   blocks, control flow, labels.
3. Read `docs/gleam/types-records-and-patterns.md` — custom types, records,
   pattern matching, making invalid states impossible.
4. Read `docs/gleam/result-option-and-errors.md` — `Result`/`Option`, error
   handling strategy, when to panic vs return.
5. Read `docs/gleam/project-structure-and-cli.md` — where files live,
   `gleam.toml`, module naming, the `src`/`test` split.
6. Record a one-paragraph summary of the intended behavior and the public
   surface. This summary is the scope contract for the rest of the workflow.

## Docs to consult

- `docs/gleam/language-fundamentals.md`
- `docs/gleam/types-records-and-patterns.md`
- `docs/gleam/result-option-and-errors.md`
- `docs/gleam/project-structure-and-cli.md`

## Operational skills to load

None are mandatory in the scope phase; skills load in the design and implement
phases. Optionally load `gleam-language` if the API surface is non-trivial and
would benefit from early naming/type guidance.

## Constraints to apply

- Stay within the captured requirements. Do not fold in related cleanups,
  unrelated refactors, or speculative generalization. If a related cleanup
  appears, record it as a follow-up.

## Validations to run

None — validations run in phase 05 (workflow-gleam-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-implementation-00-orchestration`. Set:

- `outcome` to `pass` once the one-paragraph summary and public surface are
  recorded.
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
