---
name: workflow-rust-implementation-03-implement
description: |
  Use only for the implement phase of the Rust implementation workflow. Write
  code following ownership/borrowing rules and the error strategy decided in
  phase 02. Do not use for scoping, design, test-only work, or final
  verification.
allowed-tools: Read Write Edit Bash(cargo:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-implementation
  org.phase: implement
  org.phase_order: "03"
---

# Phase 03: implement (Rust implementation)

## Phase purpose

Write the code following ownership/borrowing rules and the error strategy
decided in phase 02.

## Steps to perform

1. Load `rust-ownership-borrowing`.
2. Implement the body applying the borrow rules: one owner, one `&mut` or many
   `&`, references must outlive borrowed data, non-`Copy` values move.
3. Choose smart pointers (`Box`, `Rc`, `Arc`, `RefCell`, `Mutex`, `RwLock`,
   `Cow`) per the skill's selection guidance rather than reflexive `clone()`.
4. Apply the error strategy decided in phase 02 (`?` propagation, error types).
5. If any `unsafe` block is needed, apply `constraint-rust-unsafe-safety`:
   justify the `unsafe`, add `// SAFETY:` comments, and flag the block for
   `rust-unsafe-review` consultation.

## Docs to consult

- `docs/rust/ownership-lifetimes.md`
- `docs/rust/smart-pointers-memory.md`
- `docs/rust/error-handling.md`

## Operational skills to load

- `rust-ownership-borrowing`
- `rust-error-handling`
- (conditional) `rust-unsafe-review` — only if an `unsafe` block is introduced.

## Constraints to apply

- `constraint-rust-ownership` — enforce borrow rules and smart pointer
  selection; no reflexive `clone()`.
- `constraint-rust-error-propagation` — propagate errors with `?`; no
  `unwrap`/`expect`/`panic!` for expected runtime conditions.
- `constraint-rust-unsafe-safety` — applies if any `unsafe` block is present:
  justify it, add `// SAFETY:` comments, and flag for review.
- `constraint-rust-scope-discipline` — implement only the requirements captured
  in phase 01; record related cleanups as follow-ups rather than folding them
  in.

## Validations to run

None — validations run in phase 05 (workflow-rust-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-rust-implementation-00-orchestration`. Set:

- `outcome` to `pass` once the code is written and compiles conceptually
  against the borrow rules and error strategy.
- `constraints_applied` to include every constraint listed above that applied
  (include `constraint-rust-unsafe-safety` only if `unsafe` was used).
- `next_phase: 04-test`.
- `blockers: []` unless an `unsafe` justification needs human review — in that
  case set `handoff_requires_hil: true` and record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-rust-ownership
  - constraint-rust-error-propagation
  - constraint-rust-scope-discipline
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
