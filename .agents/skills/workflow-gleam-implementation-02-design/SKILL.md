---
name: workflow-gleam-implementation-02-design
description: |
  Use only for the design phase of the Gleam implementation workflow. Make API
  design, error type design, and module structure decisions up front. Do not use
  for scoping, implementation, testing, or final verification.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-implementation
  org.phase: design
  org.phase_order: "02"
---

# Phase 02: design (Gleam implementation)

## Phase purpose

Make API design, error type design, and module structure decisions up front so
the implement phase can proceed without mid-flight architectural changes.

## Steps to perform

1. Load `gleam-language` — naming, type design, `Result`/`Option` error model,
   conventions.
2. Load `gleam-packages-ffi` — `gleam.toml` changes, dependency management,
   module structure, package surface.
3. Read `docs/gleam/types-records-and-patterns.md` — custom types, records,
   pattern matching.
4. Read `docs/gleam/functions-pipelines-and-use.md` — function definitions,
   the `|>` pipe, `use`, labelled arguments.
5. Read `docs/gleam/result-option-and-errors.md` — `Result`/`Option`, error
   handling strategy.
6. Read `docs/gleam/conventions-patterns-antipatterns.md` — naming,
   `snake_case`/`PascalCase`, type design, common anti-patterns.
7. Decide the module file convention and visibility granularity up front;
   apply it consistently across the implementation. Module names are singular
   and grouped by business domain.
8. Decide the error strategy:
   - Fallible functions return `Result(a, e)`; use `Nil` as the error type
     when there is no extra detail.
   - `Option(a)` is only for optional arguments or data-structure fields —
     never as a fallible return type.
   - Never use `panic`/`todo`/`let assert` for expected runtime conditions;
     reserve them for genuinely unreachable states and tests.
   - Design descriptive error variants in business-domain terms.
9. Conditionally load `gleam-otp-interop` if the implementation touches OTP
   actors, Erlang interop, FFI, or multi-target code.

## Docs to consult

- `docs/gleam/types-records-and-patterns.md`
- `docs/gleam/functions-pipelines-and-use.md`
- `docs/gleam/result-option-and-errors.md`
- `docs/gleam/conventions-patterns-antipatterns.md`

## Operational skills to load

- `gleam-language`
- `gleam-packages-ffi`
- (conditional) `gleam-otp-interop` — only if the implementation area touches
  OTP/FFI/Erlang-target concurrency.

## Constraints to apply

- Stay within the captured requirements. Design only what the captured
  requirements demand. Do not introduce speculative abstractions, new
  dependencies, or module boundaries beyond what the new code requires.
- `constraint-gleam-conventions` — plan naming, imports, module structure, and
  public API documentation so the implement and verify phases can produce a
  consistent package surface.

## Validations to run

None — validations run in phase 05 (workflow-gleam-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-gleam-implementation-00-orchestration`. Set:

- `outcome` to `pass` once module convention, visibility granularity, and error
  strategy are decided and recorded.
- `constraints_applied` to include the scope-discipline principle and
  `constraint-gleam-conventions`.
- `next_phase: 03-implement`.
- `blockers: []` unless a policy decision (e.g. target choice or new dependency)
  needs human input — in that case set `handoff_requires_hil: true` and record
  the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - stay within captured requirements (scope discipline)
  - constraint-gleam-conventions
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
