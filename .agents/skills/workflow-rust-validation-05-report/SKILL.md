---
name: workflow-rust-validation-05-report
description: |
  Use only for the report phase of the Rust validation workflow.
  Aggregate all gate results into a per-gate table and report the
  overall verdict. Do not use for scoping, compile, testing, or
  formatting.
allowed-tools: Read
metadata:
  org.kind: workflow-phase
  org.workflow: rust-validation
  org.phase: report
  org.phase_order: "05"
---

# Rust Validation Workflow — Phase 05 — Report

## Phase Purpose

Aggregate all gate results into a per-gate table. If `unsafe` code is present
(determined in phase 01), note that Miri should be run (cross-ref the debugging
workflow). Report the overall verdict: pass (all applicable gates green) or fail
(any applicable gate red). If a gate failed, hand off to the appropriate fixing
workflow — do NOT fix in the validation pass.

## Steps

1. Aggregate the gate results from phases 02-04 into a per-gate table:

   | Gate | Command | Result | Evidence |
   |---|---|---|---|
   | Compilation | `cargo check --all-targets` | pass/fail | error count or clean |
   | Lints | `cargo clippy --workspace --all-targets -- -D warnings` | pass/fail | warning count or clean |
   | Tests | `cargo test --all-features` | pass/fail | passed/failed counts |
   | Doctests | `cargo test --doc` | pass/fail | passed/failed counts |
   | Format | `cargo fmt --all -- --check` | pass/fail | diff or clean |
   | Doc build | `cargo doc --no-deps --document-private-items` | pass/fail | warning count or clean |
   | Supply chain | `cargo audit` / `cargo deny check` | pass/fail/N-A | advisory/violation count or clean |
   | Miri | `cargo +nightly miri test` | pass/fail/N-A | UB report or clean |
   | Coverage | `cargo llvm-cov --all-features` | pass/fail/N-A | percentage and threshold |

   Mark not-applicable gates "N-A" (e.g., Miri when no `unsafe` code) and not-configured gates "not configured" (e.g., coverage when no tool is configured).
2. If `unsafe` code is present, note that Miri should be run (cross-ref `docs/rust/workflows/debugging.md` and the `rust-unsafe-review` skill). Miri is not part of the standard validation gate run; flag it as a follow-up.
3. Compute the overall verdict:
   - **pass** — all applicable gates green.
   - **fail** — any applicable gate red.
4. If any applicable gate failed, hand off to the appropriate fixing workflow — do NOT fix in the validation pass:
   - Defect → `docs/rust/workflows/debugging.md`
   - Missing tests/docs → `docs/rust/workflows/implementation.md`
   - Structural cleanup → `docs/rust/workflows/refactoring.md`
   Then re-run validation after the fix.
5. Keep the report to the per-gate table and overall verdict; do not expand it into a code review or implementation summary.

## Docs to Consult

- `docs/rust/workflows/validation.md`
- `docs/rust/unsafe-security.md`
- `docs/rust/supply-chain-security.md`

## Operational Skills to Load

None. This phase aggregates results; it does not run gates or load operational skills. (If `unsafe` code is present, cross-ref the `rust-unsafe-review` skill for the Miri follow-up note only.)

## Constraints to Apply

- `constraint-rust-scope-discipline` — validation is a gate, not a fix; do not modify code; hand off failures to fixing workflows.

## Validations to Run

None. This is a reporting phase — it aggregates all gate results from phases 02-04; it does not run validation gates itself.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any applicable gate red).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-rust-scope-discipline"]`
- `tests_run`: the full list of gate commands run across phases 02-04 with pass/fail per gate.
- `validations_run`: `["validation-rust-compile", "validation-rust-clippy", "validation-rust-test", "validation-rust-format", "validation-rust-docs", "validation-rust-supply-chain"]`
- `constraints_checked`: `["constraint-rust-scope-discipline"]`
- `evidence`: the per-gate table; the overall verdict; the Miri follow-up note if `unsafe` code is present.
- `failures`: the list of failing applicable gates with evidence.
- `not_fully_checkable`: gates marked N-A or not-configured, and Miri (run separately if `unsafe` present).
- `next_phase`: stop.
- `next_workflow`: `null` (if pass) | `debugging` | `implementation` | `refactoring` (if any applicable gate failed).
- `blockers`: the failing gates if `outcome: fail`.
