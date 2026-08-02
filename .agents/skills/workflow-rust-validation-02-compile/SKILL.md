---
name: workflow-rust-validation-02-compile
description: |
  Use only for the compile phase of the Rust validation workflow.
  Run cargo check and cargo clippy. Do not use for scoping, testing,
  formatting, or final reporting.
allowed-tools: Read Bash(cargo:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-validation
  org.phase: compile
  org.phase_order: "02"
---

# Rust Validation Workflow — Phase 02 — Compile & Lint

## Phase Purpose

Run the compilation and lint gates. Record pass/fail and evidence for each. A
failure here is a blocker for the overall verdict, but continue to the
remaining gates to give a full picture.

## Steps

1. Run the compilation gate:
   ```sh
   cargo check --all-targets
   ```
   `--all-targets` covers lib, bins, tests, examples, and benches. Map to `validation-rust-compile`.
2. Run the lint gate:
   ```sh
   cargo clippy --workspace --all-targets -- -D warnings
   ```
   `--workspace` covers all members. Any warning treated as error is a blocker. Map to `validation-rust-clippy`.
3. For each gate, record pass/fail and the relevant output (error count or clean; warning count or clean).
4. If a gate fails, continue to the remaining gates (phase 03, 04) to give a full picture; the overall result will be fail.
5. Do NOT modify code to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/rust/workflows/validation.md`
- `docs/rust/lints-clippy.md`
- `docs/rust/editions-tooling.md`

## Operational Skills to Load

- `rust-lints-and-clippy` — lint policy.

## Constraints to Apply

- `constraint-rust-scope-discipline` — do not modify code or `clippy.toml` to make a gate pass.

## Validations to Run

- `validation-rust-compile` — command: `cargo check --all-targets`
- `validation-rust-clippy` — command: `cargo clippy --workspace --all-targets -- -D warnings`

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (both gates green) | fail (any gate red) | partial (one gate not-applicable).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-rust-scope-discipline"]`
- `tests_run`: `["cargo check --all-targets", "cargo clippy --workspace --all-targets -- -D warnings"]` with pass/fail + evidence per gate.
- `risks`: any gate red.
- `next_phase`: `03-lint-test`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
