# Rust Workflow Index

## Purpose

This directory contains order-of-operations guides that future AI coding agents
follow for each SDLC activity in Rust: implementation, code review, refactoring,
debugging, and validation. Each workflow doc describes what happens first, what
context is gathered, which `docs/rust/` topic docs are consulted, which
`.agents/skills/` are loaded, what checks/commands are run in order, what
evidence is reported, when human judgment is needed, and how to avoid mixing
unrelated cleanup into the task.

The workflows are deliberately sequential and deterministic. They exist so that
any agent picking up a Rust task executes the same gates in the same order and
reports the same evidence shape, regardless of which model or session is
running.

## When to use

Use this index to choose a workflow before touching Rust code. Pick the workflow
that matches the task type; do not improvise a hybrid. If a task spans multiple
types (for example, fixing a defect that also requires new feature code), run
the primary workflow first and note the secondary workflow in the report.

## Workflow family

### `docs/rust/workflows/implementation.md`

- **Purpose:** Implement new Rust code (features, modules, functions) from
  requirements to validated, documented, tested code.
- **When to use:** Adding a new feature, module, function, or public API
  surface; writing the first Rust code for a new capability.
- **Docs referenced:** `api-design.md`, `types-traits-generics.md`,
  `error-handling.md`, `modules-visibility.md`, `documentation-guidelines.md`,
  `testing.md`.
- **Skills referenced:** `rust-ownership-borrowing`, `rust-error-handling`,
  `rust-api-design`, `rust-cargo-and-deps`, `rust-testing`.

### `docs/rust/workflows/code-review.md`

- **Purpose:** Review a Rust PR/diff for correctness, safety, idioms, and
  documentation quality.
- **When to use:** Reviewing a pull request, auditing a diff, or performing a
  pre-merge gate on someone else's Rust changes.
- **Docs referenced:** `api-design.md`, `error-handling.md`,
  `unsafe-security.md`, `lints-clippy.md`, `style-formatting.md`,
  `testing.md`, `documentation-guidelines.md`.
- **Skills referenced:** `rust-api-design`, `rust-unsafe-review`,
  `rust-lints-and-clippy`.

### `docs/rust/workflows/refactoring.md`

- **Purpose:** Restructure existing Rust code without changing behavior, with a
  passing-test baseline and incremental verification.
- **When to use:** Restructuring modules, simplifying ownership, replacing a
  pattern, or cleaning up dead code — all without intended behavior change.
- **Docs referenced:** `design-patterns.md`, `ownership-lifetimes.md`,
  `types-traits-generics.md`, `iterators-closures.md`,
  `smart-pointers-memory.md`, `error-handling.md`.
- **Skills referenced:** `rust-ownership-borrowing`, `rust-api-design` (when
  public surface changes), plus area-specific skills as needed.

### `docs/rust/workflows/debugging.md`

- **Purpose:** Diagnose and fix a Rust defect by category (compile error,
  runtime panic, borrow-checker error, logic error), then add a regression test.
- **When to use:** Fixing a failing build, a panic, a borrow-checker rejection,
  or an incorrect runtime result.
- **Docs referenced:** `ownership-lifetimes.md`, `async-tokio.md`,
  `unsafe-security.md`, `error-handling.md`, `testing.md`.
- **Skills referenced:** `rust-ownership-borrowing`, `rust-async-tokio`,
  `rust-unsafe-review`, `rust-testing`.

### `docs/rust/workflows/validation.md`

- **Purpose:** Run the full validation gate suite (compile, lint, test, doctest,
  format, doc build, supply chain, Miri, coverage) and report pass/fail per
  gate.
- **When to use:** Final validation before merge/release, or as a periodic
  health gate on the whole workspace.
- **Docs referenced:** `lints-clippy.md`, `style-formatting.md`, `testing.md`,
  `cargo-dependencies.md`, `supply-chain-security.md`, `editions-tooling.md`,
  `unsafe-security.md`.
- **Skills referenced:** `rust-lints-and-clippy`, `rust-testing`,
  `rust-cargo-and-deps`, `rust-supply-chain`, `rust-unsafe-review`.

## Decision tree

Choose a workflow by matching the task type:

- **Task: write new feature / module / function / public API**
  - Use `docs/rust/workflows/implementation.md`
  - If the new code is a fix for a known defect, also run
    `docs/rust/workflows/debugging.md` step 5 (regression test) and step 7
    (report root cause).
- **Task: review a PR / diff / someone else's changes**
  - Use `docs/rust/workflows/code-review.md`
  - If the review surfaces a defect, hand off to
    `docs/rust/workflows/debugging.md` for the fix (do not fix in the review
    pass).
- **Task: restructure existing code without behavior change**
  - Use `docs/rust/workflows/refactoring.md`
  - If behavior must change, switch to `implementation.md` and record the
    intended behavior change explicitly.
- **Task: fix a defect (compile error, panic, borrow-checker error, logic
  error)**
  - Use `docs/rust/workflows/debugging.md`
  - If the fix requires new public API, follow with `implementation.md` for
    the API design steps.
- **Task: final validation gate before merge/release**
  - Use `docs/rust/workflows/validation.md`
  - Run it after one of the above workflows has produced code, not instead of
    them.

## Workflow map

| Workflow | File path |
|---|---|
| Implementation | `docs/rust/workflows/implementation.md` |
| Code review | `docs/rust/workflows/code-review.md` |
| Refactoring | `docs/rust/workflows/refactoring.md` |
| Debugging | `docs/rust/workflows/debugging.md` |
| Validation | `docs/rust/workflows/validation.md` |

## Relationship to skills and topic docs

Workflows are the orchestration layer. They do not restate the rules that live in
`docs/rust/` topic docs or the operational recipes that live in
`.agents/skills/`. A workflow tells an agent which docs to read, which skills to
load, and which commands to run in order; the docs and skills supply the actual
rules. Always consult the referenced doc and load the referenced skill rather
than relying on memory.
