# Rust Debugging Workflow

## Purpose

Process for diagnosing and fixing a Rust defect by category — compile error,
runtime panic, borrow-checker error, or logic error — then adding a regression
test and running the full check suite. The workflow enforces root-cause fixes
over symptom suppression.

## When to use

Use this workflow when fixing a failing build, a runtime panic, a
borrow-checker rejection, or an incorrect runtime result. Do not use it for
implementing new features (use `implementation.md`), reviewing a diff (use
`code-review.md`), or restructuring without behavior change (use
`refactoring.md`). If the fix requires new public API, follow this workflow for
the fix then `implementation.md` for the API design steps.

## Order of operations

### 1. Reproduce the issue

Before diagnosing, reproduce the issue reliably. Capture the exact command, the
input, the environment, and the full error message or panic output. If the
issue cannot be reproduced, record what is known and what reproduction attempts
were made; do not proceed to a fix on an unreproduced report.

### 2. Categorize

Classify the defect into one of:

- **Compile error** — `cargo check` fails; read the compiler error code (for
  example, E0277, E0432) and message.
- **Runtime panic** — code compiles but panics at runtime (`unwrap`, index out
  of bounds, `assert!`, explicit `panic!`, arithmetic overflow).
- **Borrow checker error** — a subset of compile errors (E0382, E0502, E0505,
  E0507, E0596, E0597, E0106, etc.) specifically about ownership, borrowing, or
  lifetimes.
- **Logic error** — code compiles and runs without panics but produces wrong
  output.

The category determines which skills and docs to load in step 3.

### 3. Load relevant skills

Load skills and docs matching the category:

- **Borrow checker errors** →
  `.agents/skills/rust-ownership-borrowing/SKILL.md` and
  `docs/rust/ownership-lifetimes.md`. Use the skill's error-to-fix table
  (E0382, E0502, E0505, E0507, E0596, E0597, E0106, etc.).
- **Async issues** (deadlock, hang, `Send` violation, cancellation bug) →
  `.agents/skills/rust-async-tokio/SKILL.md` and `docs/rust/async-tokio.md`.
- **Unsafe / undefined behavior** (soundness bug, miscompilation, Miri failure)
  → `.agents/skills/rust-unsafe-review/SKILL.md` and
  `docs/rust/unsafe-security.md`. Use Miri to confirm UB:
  ```sh
  MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test
  ```
- **Logic errors** → add tests that pin the wrong behavior (these become the
  regression test in step 5), and use debugging tools (`dbg!`, `eprintln!`,
  `RUST_BACKTRACE=1`, a debugger). Consult `docs/rust/testing.md` for test
  structure.

Also load `.agents/skills/rust-error-handling/SKILL.md` if the defect is an
error-handling mistake (swallowed error, wrong error type, `unwrap` on a
recoverable condition).

### 4. Fix the root cause

Fix the root cause, not the symptom. Concretely:

- For a borrow checker error, restructure ownership/borrowing per the skill's
  guidance (borrow, reorder, split scopes, choose a smart pointer) rather than
  adding `clone()` reflexively or `#[allow]`.
- For a runtime panic, replace the `unwrap`/`expect`/indexing with proper
  error handling per `docs/rust/error-handling.md`.
- For an async issue, address the actual deadlock/`Send`/cancellation bug per
  `docs/rust/async-tokio.md`, not a `Box::pin` workaround.
- For an unsafe/UB bug, fix the invariant violation and add or strengthen the
  `// SAFETY:` comment; do not paper over with `unsafe` in a different place.
- For a logic error, fix the incorrect computation, not a caller that masks
  it.

### 5. Add a regression test

Add a test that fails before the fix and passes after. Place it with the code
it protects (unit test in `#[cfg(test)] mod tests`, or integration test under
`tests/` per `docs/rust/testing.md`). The test must encode the reproduction
from step 1 so the same defect cannot silently return.

### 6. Run the full check suite

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --check
```

If the fix touched `unsafe` code, also run Miri (see step 3). All gates must
pass.

### 7. Report

Report the root cause, the fix, and the regression test.

## Docs consulted

- `docs/rust/ownership-lifetimes.md`
- `docs/rust/async-tokio.md`
- `docs/rust/unsafe-security.md`
- `docs/rust/error-handling.md`
- `docs/rust/testing.md`

## Skills loaded

- (conditional) `.agents/skills/rust-ownership-borrowing/SKILL.md`
- (conditional) `.agents/skills/rust-async-tokio/SKILL.md`
- (conditional) `.agents/skills/rust-unsafe-review/SKILL.md`
- (conditional) `.agents/skills/rust-error-handling/SKILL.md`
- `.agents/skills/rust-testing/SKILL.md`

## Commands run (in order)

Reproduce:

```sh
<the exact command that reproduces the issue>
```

Fix verification:

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --check
```

If `unsafe` is involved:

```sh
MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test
```

## Evidence to report

- The reproduction command and the original error/panic output (from step 1).
- The defect category (from step 2).
- The root cause: a one-paragraph explanation of why the defect occurred.
- The fix: what changed and why it addresses the root cause.
- The regression test: its location, what it asserts, and confirmation it
  fails before the fix and passes after.
- Pass/fail for each command in step 6 (and Miri if run).

## When human judgment is needed

- Deciding whether a borrow-checker fix should restructure ownership vs. accept
  a `clone()` (the skill gives guidance, but hot-path tradeoffs need judgment).
- Judging whether an `unsafe` soundness fix is complete or needs a human
  `unsafe` reviewer (consult the `rust-unsafe-review` skill).
- Choosing between fixing a logic error at the source vs. adding validation at
  a boundary (depends on where the invariant truly lives).
- Deciding whether a panic indicates a genuine invariant violation (keep the
  panic, fix the caller) or a recoverable error (replace with `Result`).
- Escalating when the root cause is in a dependency rather than the repo's
  own code.

## Avoiding scope creep

- Fix only the reported defect. If adjacent code has issues, record them as
  follow-ups; do not refactor the surrounding code in the same change.
- Do not add features while fixing a bug; if the fix reveals a needed feature,
  open a separate task under `implementation.md`.
- Do not suppress the symptom with `#[allow]`, `unwrap`→`expect` rename, or
  `Box::pin` workarounds; fix the root cause.
- Keep the regression test scoped to the defect; do not build a broad test
  suite expansion as part of a bugfix.
- If the fix touches `unsafe` code, restrict changes to the soundness fix and
  its `// SAFETY:` comment; do not rewrite the `unsafe` block's surrounding
  logic.
