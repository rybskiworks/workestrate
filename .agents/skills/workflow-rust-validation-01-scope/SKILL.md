---
name: workflow-rust-validation-01-scope
description: |
  Use only for the scope phase of the Rust validation workflow.
  Identify what changed (git diff) and determine which gates apply
  (unsafe present? coverage configured? cargo-deny configured?). Do not
  use for compile, test, format, doc, supply-chain, or final reporting.
allowed-tools: Read Bash(git:*) Bash(grep:*) Bash(find:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-validation
  org.phase: scope
  org.phase_order: "01"
---

# Rust Validation Workflow — Phase 01 — Scope

## Phase Purpose

Identify what changed and determine which gates apply. Record applicable vs
not-applicable gates so phases 02-04 run only the relevant gates and mark the
rest "not applicable" or "not configured".

## Steps

1. Identify what changed:
   ```sh
   git diff --stat
   git diff --name-only
   ```
2. Determine gate applicability:
   - **Unsafe code**: search the workspace for `unsafe` blocks. If present, Miri applies (noted in phase 05; cross-ref debugging workflow). If absent, mark Miri "not applicable".
   - **Coverage**: check whether a coverage tool is configured (e.g., `cargo-llvm-cov` or `tarpaulin`). If not configured, mark coverage "not configured" and skip.
   - **cargo-deny**: check whether `deny.toml` exists. If absent, phase 04 runs `cargo audit` alone and records `cargo deny` as "not configured".
   - **Feature matrix**: check `Cargo.toml` for mutually exclusive features; if present, note that `--all-features` in phase 03 must be adjusted (consult `docs/rust/cargo-dependencies.md`).
3. Record the list of applicable gates and the list of not-applicable/not-configured gates with reasons.
4. Do NOT modify any code, config, or manifest in this phase. Scope only.

## Docs to Consult

- `docs/rust/workflows/validation.md`
- `docs/rust/cargo-dependencies.md`
- `docs/rust/unsafe-security.md`
- `docs/rust/supply-chain-security.md`

## Operational Skills to Load

- `rust-cargo-and-deps` — feature matrix, cargo-deny configuration.
- `rust-unsafe-review` (conditional) — if `unsafe` code is present.

## Constraints to Apply

- `constraint-rust-scope-discipline` — validation is a gate, not a fix; do not modify code or config to make a gate applicable/inapplicable.

## Validations to Run

None. This phase identifies which gates apply; it does not run validation gates.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (scope determined) | partial (some gates' applicability unclear).
- `files_touched`: `[]` (scope only; no changes).
- `constraints_applied`: `["constraint-rust-scope-discipline"]`
- `assumptions`: the list of applicable gates and the list of not-applicable/not-configured gates with reasons; the changed-files list.
- `risks`: mutually-exclusive features requiring `--all-features` adjustment; `unsafe` code present (Miri applies).
- `tests_run`: `["git diff --stat", "git diff --name-only"]`.
- `next_phase`: `02-compile`.
- `next_workflow`: `null`.
- `blockers`: any gate whose applicability could not be determined.
