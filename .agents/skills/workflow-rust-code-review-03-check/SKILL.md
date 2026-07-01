---
name: workflow-rust-code-review-03-check
description: |
  Use only for the check phase of the Rust code-review workflow. Run the
  automated gates (cargo check, clippy, test, fmt) and record pass/fail. Do not
  use for scoping, analysis, manual review, or issuing a verdict.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-code-review
  org.phase: check
  org.phase_order: "03"
---

# Phase 03: check (Rust code review)

## Phase purpose

Run the automated gates (`cargo check`, clippy, test, fmt) and record their
pass/fail status. A clean compile is the baseline; a compile failure is a
blocker that stops the workflow.

## Steps to perform

1. Run `cargo check`. A clean compile is the baseline. If it does not compile,
   report a blocker, set `outcome: fail`, and stop — do not proceed to
   04-review against code that does not compile.
2. Run `cargo clippy --all-targets -- -D warnings`. Any warning treated as
   error is a blocker. Review any new `#[allow(...)]` or `#[expect(...)]` for
   justification against the repo lint policy in `docs/rust/lints-clippy.md`.
3. Run `cargo test`. All tests must pass. Flag deleted or weakened tests. New
   public items should have accompanying tests (see `docs/rust/testing.md`).
4. Run `cargo fmt --all -- --check`. Formatting must be clean; do not accept
   hand-formatted code that contradicts `rustfmt` output.
5. Run the validation skills: `validation-rust-compile`,
   `validation-rust-clippy`, `validation-rust-test`, `validation-rust-format`.
   Each validation skill wraps the corresponding command above and records
   structured pass/fail evidence.
6. Apply `constraint-rust-scope-discipline`: gate failures are reported only
   for the diff under review; do not request fixes for pre-existing failures
   in untouched code (record them as follow-ups).

## Docs to consult

- `docs/rust/lints-clippy.md`
- `docs/rust/style-formatting.md`
- `docs/rust/testing.md`

## Operational skills to load

- `rust-lints-and-clippy`
- `rust-testing`

## Constraints to apply

- `constraint-rust-scope-discipline` — gate failures are reported only for the
  diff under review; do not request fixes for pre-existing failures in
  untouched code (record them as follow-ups).

## Validations to run

- `validation-rust-compile` — `cargo check`
- `validation-rust-clippy` — `cargo clippy --all-targets -- -D warnings`
- `validation-rust-test` — `cargo test`
- `validation-rust-format` — `cargo fmt --all -- --check`

## Handoff output

Return the handoff YAML block per the schema in
`workflow-rust-code-review-00-orchestration`. Set:

- `outcome`: `pass` if all four gates are green; `fail` if any gate failed
  (especially compile); `partial` if a gate was skipped with attached CI
  evidence.
- `constraints_applied`: `constraint-rust-scope-discipline`.
- `tests_run`: one entry per gate with command and pass/fail, e.g.
  `cargo check: pass`, `cargo clippy --all-targets -- -D warnings: pass`,
  `cargo test: pass`, `cargo fmt --all -- --check: pass`.
- `blockers`: any gate failure, with the failing command.
- `assumptions`: any skipped gate and the attached CI evidence.
- `next_phase`: `04-review` (or `null` if a blocker stopped the workflow).
- `next_workflow`: `null`.
