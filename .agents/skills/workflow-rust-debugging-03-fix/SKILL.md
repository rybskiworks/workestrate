---
name: workflow-rust-debugging-03-fix
description: |
  Use only for the fix phase of the Rust debugging workflow.
  Implement the minimal root-cause fix. Do not use for reproduction,
  diagnosis, regression testing, or final verification.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-debugging
  org.phase: fix
  org.phase_order: "03"
---

## Phase purpose

Implement the minimal root-cause fix. Do not suppress the symptom. The fix must address the mechanism identified in phase `02-diagnose`.

## Steps to perform

1. Fix the root cause, not the symptom. Concretely by category:
   - **Borrow checker error** → restructure ownership/borrowing per the `rust-ownership-borrowing` skill (borrow, reorder, split scopes, choose a smart pointer) rather than adding `clone()` reflexively or `#[allow]`.
   - **Runtime panic** → replace `unwrap`/`expect`/indexing with proper error handling per `docs/rust/error-handling.md`.
   - **Async issue** → address the actual deadlock/`Send`/cancellation bug per `docs/rust/async-tokio.md`, not a `Box::pin` workaround.
   - **Unsafe/UB bug** → fix the invariant violation and add or strengthen the `// SAFETY:` comment; do not paper over with `unsafe` in a different place.
   - **Logic error** → fix the incorrect computation, not a caller that masks it.
2. Apply `constraint-rust-error-propagation`: ensure errors propagate correctly (`?`, proper error types) rather than being swallowed or panicked.
3. If the fix touches `unsafe` code, apply `constraint-rust-unsafe-safety`: restrict changes to the soundness fix and its `// SAFETY:` comment; do not rewrite the `unsafe` block's surrounding logic.
4. Apply `constraint-rust-scope-discipline`: fix only the reported defect; record adjacent issues as follow-ups; do not add features; do not suppress with `#[allow]`, `unwrap`→`expect` rename, or `Box::pin` workarounds.

## Docs to consult

By category:
- `docs/rust/ownership-lifetimes.md` (borrow checker errors);
- `docs/rust/error-handling.md` (runtime panics, error-handling mistakes);
- `docs/rust/async-tokio.md` (async issues);
- `docs/rust/unsafe-security.md` (unsafe/UB).

## Operational skills to load

Conditional by category:
- `rust-ownership-borrowing`;
- `rust-error-handling`;
- `rust-async-tokio`;
- `rust-unsafe-review`.

## Constraints to apply

- `constraint-rust-scope-discipline` — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with `#[allow]`, `unwrap`→`expect` rename, or `Box::pin` workarounds.
- `constraint-rust-error-propagation` — Ensure errors propagate correctly (`?`, proper error types) rather than being swallowed or panicked.
- `constraint-rust-unsafe-safety` — If `unsafe` is touched: restrict changes to the soundness fix and its `// SAFETY:` comment; do not rewrite the `unsafe` block's surrounding logic.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 04-regression`, `next_workflow: null`. Record the fix — what changed and why it addresses the root cause — in `evidence`. List every file changed in `files_touched`. List the constraints actually applied in `constraints_applied`.
