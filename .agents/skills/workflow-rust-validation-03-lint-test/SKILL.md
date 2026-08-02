---
name: workflow-rust-validation-03-lint-test
description: |
  Use only for the test phase of the Rust validation workflow.
  Run cargo test --all-features and cargo test --doc. Do not use for
  scoping, compile, formatting, or final reporting.
allowed-tools: Read Bash(cargo:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-validation
  org.phase: lint-test
  org.phase_order: "03"
---

# Rust Validation Workflow — Phase 03 — Test & Doctest

## Phase Purpose

Run the test and doctest gates. Record pass/fail and evidence for each. All
tests must pass.

## Steps

1. Run the test suite:
   ```sh
   cargo test --all-features
   ```
   `--all-features` exercises the full feature matrix. If the repo uses mutually exclusive features (identified in phase 01), adjust accordingly — consult `docs/rust/cargo-dependencies.md`. Map to `validation-rust-test`.
2. Run the doctest gate:
   ```sh
   cargo test --doc
   ```
   Doctests are part of `cargo test` but running `--doc` separately gives a clearer pass/fail signal for documentation examples. Map to `validation-rust-test` (doctest facet).
3. For each gate, record pass/fail and the passed/failed counts.
4. If a gate fails, continue to the remaining gates (phase 04) to give a full picture; the overall result will be fail.
5. Do NOT add tests or modify code to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/rust/workflows/validation.md`
- `docs/rust/testing.md`
- `docs/rust/cargo-dependencies.md`
- `docs/rust/documentation-guidelines.md`

## Operational Skills to Load

- `rust-testing` — test policy.
- `rust-cargo-and-deps` — feature matrix.

## Constraints to Apply

- `constraint-rust-scope-discipline` — do not add tests or modify code to make a gate pass.

## Validations to Run

- `validation-rust-test` — command: `cargo test --all-features` (and `cargo test --doc` as the doctest facet)

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (both gates green) | fail (any gate red).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-rust-scope-discipline"]`
- `tests_run`: `["cargo test --all-features", "cargo test --doc"]` with pass/fail + passed/failed counts per gate.
- `risks`: mutually-exclusive features requiring `--all-features` adjustment; any gate red.
- `next_phase`: `04-format-doc`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
