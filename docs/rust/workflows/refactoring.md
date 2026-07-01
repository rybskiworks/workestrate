# Rust Refactoring Workflow

## Purpose

Process for restructuring existing Rust code without changing behavior. The
workflow enforces a passing-test baseline before any change, incremental steps
with verification after each, and a final no-regression check. The defining
constraint is: behavior must not change.

## When to use

Use this workflow when restructuring modules, simplifying ownership, replacing
a pattern, extracting a helper, or cleaning up dead code — all without intended
behavior change. Do not use it for adding features (use `implementation.md`),
fixing a defect (use `debugging.md`), or reviewing a diff (use
`code-review.md`). If behavior must change, switch to `implementation.md` and
record the intended behavior change explicitly; do not smuggle a behavior
change through a refactoring pass.

## Order of operations

### 1. Understand current behavior

Before changing anything, read the code being refactored and the relevant topic
docs so the refactor is grounded in the corpus rules:

- `docs/rust/design-patterns.md` — pattern catalog, idioms, anti-patterns.
- `docs/rust/ownership-lifetimes.md` — ownership, borrowing, lifetimes.
- `docs/rust/types-traits-generics.md` — trait/generics modeling.
- `docs/rust/iterators-closures.md` — iterator/closure idioms.
- `docs/rust/smart-pointers-memory.md` — smart-pointer selection.
- `docs/rust/error-handling.md` — error-type design.

Record a one-paragraph description of the current behavior and the refactoring
goal (what structure is being improved and why), so later steps can verify the
goal was met without behavior change.

### 2. Ensure tests pass BEFORE refactoring (baseline)

Establish a green baseline. If tests do not pass before the refactor, stop and
fix the baseline first (use `debugging.md`) — a refactor on a red baseline
cannot be verified.

```sh
cargo test
```

If test coverage is thin in the area being refactored, add characterization
tests that pin the current behavior before refactoring. These tests are part of
the refactor, not separate work.

### 3. Load relevant skills based on refactoring area

Load skills matching the refactor's focus:

- `.agents/skills/rust-ownership-borrowing/SKILL.md` — for ownership,
  borrowing, lifetime, or smart-pointer refactors.
- `.agents/skills/rust-api-design/SKILL.md` — if the public surface is touched
  (note: a refactor that changes the public API is likely a behavior change;
  treat carefully).
- `.agents/skills/rust-error-handling/SKILL.md` — if error types are
  restructured.
- `.agents/skills/rust-async-tokio/SKILL.md` — for async refactors.
- `.agents/skills/rust-unsafe-review/SKILL.md` — if `unsafe` code is touched.

### 4. Make changes incrementally

Refactor in small steps, each independently verifiable. One step might be:
extract a helper function, replace a `clone()` with a borrow, split a module,
swap `Rc` for `Arc`, or replace a hand-rolled iterator with a combinator chain.
Commit or checkpoint after each step so a bad step can be reverted without
losing the whole refactor.

### 5. Run checks after each step

After each step, run the gates in this order:

```sh
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

If any gate fails, revert the step and redo it. Do not proceed to the next step
on a red gate. `cargo test` is the primary no-regression signal; `cargo clippy`
catches dead code and unused imports introduced by the refactor.

### 6. Verify no behavior change

After all steps are complete, run the full test suite again and confirm the
output matches the baseline from step 2 (same tests pass, same counts, no new
failures). If characterization tests were added in step 2, confirm they still
pass unchanged. If any test had to change, that is a behavior change — either
justify it explicitly and switch to `implementation.md`, or revert.

### 7. Check for dead code and unused imports

```sh
cargo clippy --all-targets -- -D warnings
```

Clippy catches dead code, unused imports, and unreachable patterns introduced
by the refactor. Remove anything Clippy flags rather than suppressing it,
unless the repo lint policy explicitly allows the suppression.

### 8. Report

Report what changed, why, and the evidence of no regression.

## Docs consulted

- `docs/rust/design-patterns.md`
- `docs/rust/ownership-lifetimes.md`
- `docs/rust/types-traits-generics.md`
- `docs/rust/iterators-closures.md`
- `docs/rust/smart-pointers-memory.md`
- `docs/rust/error-handling.md`

## Skills loaded

- `.agents/skills/rust-ownership-borrowing/SKILL.md`
- (conditional) `.agents/skills/rust-api-design/SKILL.md`
- (conditional) `.agents/skills/rust-error-handling/SKILL.md`
- (conditional) `.agents/skills/rust-async-tokio/SKILL.md`
- (conditional) `.agents/skills/rust-unsafe-review/SKILL.md`

## Commands run (in order)

Baseline:

```sh
cargo test
```

After each incremental step:

```sh
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Final:

```sh
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Evidence to report

- One-paragraph description of current behavior and the refactoring goal (from
  step 1).
- Baseline test result before refactoring (from step 2).
- List of incremental steps taken, with the gate results after each.
- Final test result confirming no regression (from step 6).
- Final Clippy result (from step 7).
- List of files changed.
- Explicit statement that no behavior change occurred, or a flagged behavior
  change with a handoff to `implementation.md`.

## When human judgment is needed

- Deciding whether a public API change is a refactor (no behavior change) or a
  behavior change requiring `implementation.md`.
- Choosing between two valid patterns from `docs/rust/design-patterns.md` when
  the tradeoff is not clear from the skill guidance.
- Deciding whether to add characterization tests before refactoring when
  coverage is thin.
- Judging whether a `clone()` removal is worth the borrow-complexity it
  introduces.
- Approving a refactor that touches `unsafe` code (hand off to the
  `rust-unsafe-review` skill).

## Avoiding scope creep

- Refactor only the structure described in step 1. If an unrelated cleanup
  appears, record it as a follow-up rather than folding it in.
- Do not fix defects encountered during the refactor; switch to
  `debugging.md` for the defect and resume the refactor afterward.
- Do not add features during a refactor; if a feature suggests itself, open a
  separate task under `implementation.md`.
- Do not change dependencies, feature flags, or lint policy as part of a
  refactor; those are separate concerns with their own policy gates.
- Keep each incremental step small enough that a revert is cheap; if a step
  grows, split it.
