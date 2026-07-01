---
name: workflow-rust-debugging-02-diagnose
description: |
  Use only for the diagnose phase of the Rust debugging workflow.
  Categorize the defect (compile / borrow / async / unsafe / logic) and
  identify the root cause. Do not use for reproduction, fixing, regression
  testing, or final verification.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-debugging
  org.phase: diagnose
  org.phase_order: "02"
---

## Phase purpose

Categorize the defect (compile / borrow / async / unsafe / logic) and identify the root cause. The category determines which operational skills and docs to load.

## Steps to perform

1. Classify the defect into exactly one of:
   - **Compile error** (`cargo check` fails; read the compiler error code e.g. `E0277`, `E0432` and message);
   - **Runtime panic** (code compiles but panics at runtime — `unwrap`, index out of bounds, `assert!`, explicit `panic!`, arithmetic overflow);
   - **Borrow checker error** (subset of compile errors `E0382`, `E0502`, `E0505`, `E0507`, `E0596`, `E0597`, `E0106`, etc. about ownership, borrowing, or lifetimes);
   - **Logic error** (code compiles and runs without panics but produces wrong output).
2. Load skills and docs matching the category:
   - Borrow checker errors → `rust-ownership-borrowing` + `docs/rust/ownership-lifetimes.md` (use the skill's error-to-fix table);
   - Async issues (deadlock, hang, `Send` violation, cancellation bug) → `rust-async-tokio` + `docs/rust/async-tokio.md`;
   - Unsafe/UB (soundness bug, miscompilation, Miri failure) → `rust-unsafe-review` + `docs/rust/unsafe-security.md`, confirm UB with `MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test`;
   - Logic errors → add tests pinning the wrong behavior (these become the regression test in phase `04-regression`); use `dbg!`, `eprintln!`, `RUST_BACKTRACE=1`, a debugger; consult `docs/rust/testing.md`.
3. Load `rust-error-handling` if the defect is an error-handling mistake (swallowed error, wrong error type, `unwrap` on a recoverable condition).
4. Identify the root cause: write a one-paragraph explanation of why the defect occurred. The root cause must explain the mechanism, not just restate the symptom.

## Docs to consult

By category:
- `docs/rust/ownership-lifetimes.md` (borrow checker errors);
- `docs/rust/async-tokio.md` (async issues);
- `docs/rust/unsafe-security.md` (unsafe/UB);
- `docs/rust/error-handling.md` (error-handling mistakes);
- `docs/rust/testing.md` (logic errors; test structure).

## Operational skills to load

Conditional by category:
- `rust-ownership-borrowing` (borrow checker errors);
- `rust-async-tokio` (async issues);
- `rust-unsafe-review` (unsafe/UB);
- `rust-error-handling` (error-handling mistakes).

## Constraints to apply

- `constraint-rust-scope-discipline` — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with `#[allow]`, `unwrap`→`expect` rename, or `Box::pin` workarounds. Diagnosis only; do not begin fixing.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 03-fix`, `next_workflow: null`. Record the defect category and the root-cause paragraph in `evidence` (or `assumptions` if the root cause is provisional). Because the continuation policy is `suggest-next`, surface the category and root cause for confirmation before phase `03-fix` is loaded.
