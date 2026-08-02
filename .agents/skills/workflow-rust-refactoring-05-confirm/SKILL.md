---
name: workflow-rust-refactoring-05-confirm
description: |
  Use only for the confirm phase of the Rust refactoring workflow.
  After all steps, run the final no-regression check (full test suite,
  clippy, cargo fmt --check) and report. Do not use for baseline, planning,
  executing, or verifying.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-refactoring
  org.phase: confirm
  org.phase_order: "05"
---

## Phase purpose

After all planned steps are complete, run the final no-regression check and report. Confirm the output matches the baseline from phase 01, check for dead code/unused imports via clippy, run `cargo fmt --check`, and report what changed, why, and the evidence of no regression.

## Steps to perform

1. Run the full test suite and confirm the output matches the baseline from phase 01 (same tests pass, same counts, no new failures): `cargo test`. If characterization tests were added in phase 01, confirm they still pass unchanged. If any test had to change, that is a behavior change — either justify it explicitly and switch to the implementation workflow, or revert.
2. Run clippy to catch dead code, unused imports, and unreachable patterns introduced by the refactor: `cargo clippy --all-targets -- -D warnings`. Remove anything clippy flags rather than suppressing it, unless the repo lint policy explicitly allows the suppression.
3. Run the format check: `cargo fmt --all -- --check`.
4. Map gates to validation skills: `cargo clippy --all-targets -- -D warnings` → `validation-rust-clippy`; `cargo fmt --all -- --check` → `validation-rust-format`.
5. Aggregate evidence and report: the one-paragraph behavior/goal description from phase 01; the baseline test result from phase 01; the list of incremental steps taken, with the gate results after each (from phase 04); the final test result confirming no regression; the final clippy result; the list of files changed; an explicit statement that no behavior change occurred, OR a flagged behavior change with a handoff to the implementation workflow.

## Docs to consult

- docs/rust/workflows/refactoring.md
- docs/rust/testing.md
- docs/rust/lints-clippy.md
- docs/rust/style-formatting.md

## Operational skills to load

- `rust-testing`
- `rust-lints-and-clippy`

## Constraints to apply

- `constraint-rust-scope-discipline` — Confirm only the planned refactor; flag any drift as a behavior change; do not fold in defect fixes, features, dependency changes, or lint-policy changes.

## Validations to run

Final no-regression confirmation:

- `validation-rust-clippy` (`cargo clippy --all-targets -- -D warnings`)
- `validation-rust-format` (`cargo fmt --all -- --check`)

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (no regression, all final gates green) | `fail` (regression or red final gate) | `partial` (gates green but a behavior change was flagged)
- `files_touched`: full list of files changed across all steps
- `constraints_applied`: `["constraint-rust-scope-discipline"]`
- `tests_run`: `["cargo test", "cargo clippy --all-targets -- -D warnings", "cargo fmt --all -- --check"]` with pass/fail per gate
- `validations_run`: `["validation-rust-clippy", "validation-rust-format"]`
- `constraints_checked`: `["constraint-rust-scope-discipline"]`
- `evidence`: baseline comparison, per-step gate results, final gate results, files changed, explicit no-behavior-change statement
- `failures`: any red final gate or detected behavior change
- `next_phase`: `stop`
- `next_workflow`: `null` | `implementation` (if behavior change flagged) | `debugging` (if a defect surfaced)
- `blockers`: any unresolved regression
