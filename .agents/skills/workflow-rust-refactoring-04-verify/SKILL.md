---
name: workflow-rust-refactoring-04-verify
description: |
  Use only for the verify phase of the Rust refactoring workflow.
  After each incremental step, run cargo check + cargo test + cargo clippy.
  Revert on any red gate. Do not use for baseline, planning, executing, or
  confirming.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-refactoring
  org.phase: verify
  org.phase_order: "04"
---

## Phase purpose

After each incremental step (phase 03), run the verification gates. If any gate fails, REVERT the step and redo it. Do not proceed on a red gate. `cargo test` is the primary no-regression signal; `cargo clippy` catches dead code and unused imports introduced by the refactor.

## Steps to perform

1. Run the gates in this order: `cargo check`; `cargo test`; `cargo clippy --all-targets -- -D warnings`.
2. Map each command to its validation skill: `cargo check` → `validation-rust-compile`; `cargo test` → `validation-rust-test`; `cargo clippy --all-targets -- -D warnings` → `validation-rust-clippy`.
3. If ANY gate fails: REVERT the step (`git revert HEAD` or the repo's revert mechanism for the checkpoint made in phase 03); do NOT proceed to the next step on a red gate; set `outcome: fail`, record which gate failed and the evidence, and return to phase 03 to redo the step.
4. If ALL gates pass: Compare test output to the baseline from phase 01 (same tests pass, same counts, no new failures). If the test result differs from baseline, that is a behavior change — STOP and hand off to the implementation workflow. If test output matches baseline, set `outcome: pass`.
5. If more planned steps remain, set `next_phase: 03-execute`. If this was the last step, set `next_phase: 05-confirm`.

## Docs to consult

- docs/rust/workflows/refactoring.md
- docs/rust/testing.md
- docs/rust/lints-clippy.md

## Operational skills to load

- `rust-testing`
- `rust-lints-and-clippy`

## Constraints to apply

- `constraint-rust-scope-discipline` — Verify only this step; do not make additional changes to make a gate pass (that is a new change requiring its own plan step); do not fold in defect fixes or features.

## Validations to run

Run after EACH incremental step:

- `validation-rust-compile` (`cargo check`)
- `validation-rust-test` (`cargo test`)
- `validation-rust-clippy` (`cargo clippy --all-targets -- -D warnings`)

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (all gates green, test output matches baseline) | `fail` (a gate red or behavior change detected)
- `files_touched`: `[]` (verification only; revert if failed)
- `constraints_applied`: `["constraint-rust-scope-discipline"]`
- `tests_run`: `["cargo check", "cargo test", "cargo clippy --all-targets -- -D warnings"]` with pass/fail per gate
- `risks`: behavior-change flag if test output diverged from baseline
- `next_phase`: `03-execute` (more steps) | `05-confirm` (last step) | `stop` (behavior change detected — hand off to implementation)
- `next_workflow`: `null` | `implementation` (if behavior change detected)
- `blockers`: the failing gate and evidence if `outcome: fail`
