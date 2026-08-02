---
name: workflow-rust-debugging-05-verify
description: |
  Use only for the verify phase of the Rust debugging workflow.
  Run the full check suite (plus Miri if unsafe was touched) and report
  the root cause, fix, and regression test. Do not use for reproduction,
  diagnosis, fixing, or regression testing.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-debugging
  org.phase: verify
  org.phase_order: "05"
---

## Phase purpose

Run the full check suite (plus Miri if unsafe was touched) and report the root cause, fix, and regression test. This is the final phase.

## Steps to perform

1. Run gates in this exact order; all must pass: `cargo check`; `cargo clippy --all-targets -- -D warnings`; `cargo test`; `cargo fmt --check`.
2. If the fix touched `unsafe` code, also run Miri: `MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test`.
3. Run validation skills: `validation-rust-compile`; `validation-rust-clippy`; `validation-rust-test`.
4. Report all of the following (aggregate from prior phases):
   - the reproduction command and the original error/panic output (from phase `01-reproduce`);
   - the defect category (from phase `02-diagnose`);
   - the root cause: a one-paragraph explanation of why the defect occurred;
   - the fix: what changed and why it addresses the root cause (from phase `03-fix`);
   - the regression test: location, assertion, and fails-before/passes-after confirmation (from phase `04-regression`);
   - pass/fail for each gate above, and for Miri if run.

## Docs to consult

None new. Findings reference docs cited in earlier phases (`docs/rust/ownership-lifetimes.md`, `docs/rust/async-tokio.md`, `docs/rust/unsafe-security.md`, `docs/rust/error-handling.md`, `docs/rust/testing.md`).

## Operational skills to load

None new.

## Constraints to apply

- `constraint-rust-scope-discipline` — Verify only the reported defect's fix; do not make additional changes to make a gate pass; do not refactor surrounding code; record adjacent issues as follow-ups.

## Validations to run

- `validation-rust-compile` (`cargo check`);
- `validation-rust-clippy` (`cargo clippy --all-targets -- -D warnings`);
- `validation-rust-test` (`cargo test`).

If `unsafe` was touched, also run Miri as a command: `MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test`.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md), including the verification-phase extra fields:
- `validations_run` — list of validation skills executed (`validation-rust-compile`, `validation-rust-clippy`, `validation-rust-test`);
- `constraints_checked` — list of constraint skills audited against the diff;
- `evidence` — reproduction command, original error, category, root cause, fix, regression test location/assertion, per-gate pass/fail (and Miri if run);
- `failures` — list of gates that failed (empty if all passed);
- `not_fully_checkable` — list of aspects that could not be fully validated and why (e.g., Miri unavailable without nightly).

Set `next_phase: null`, `next_workflow: null`. Unless the fix revealed a need for new public API, in which case set `next_workflow: workflow-rust-implementation` (the debugging workflow is complete; the API design work is a separate workflow).
