---
name: workflow-rust-validation-04-format-doc
description: |
  Use only for the format, docs, and supply-chain phase of the Rust
  validation workflow. Run cargo fmt --check, cargo doc, cargo audit,
  and cargo deny check. Do not use for scoping, compile, or testing.
allowed-tools: Read Bash(cargo:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-validation
  org.phase: format-doc
  org.phase_order: "04"
---

# Rust Validation Workflow — Phase 04 — Format, Docs & Supply Chain

## Phase Purpose

Run the format, doc-build, and supply-chain gates. Record pass/fail and
evidence per gate. If `cargo deny` is not configured (determined in phase 01),
run `cargo audit` alone and record `cargo deny` as "not configured".

## Steps

1. Run the format gate:
   ```sh
   cargo fmt --all -- --check
   ```
   Formatting must be clean across the workspace. Map to `validation-rust-format`.
2. Run the doc-build gate:
   ```sh
   cargo doc --no-deps --document-private-items
   ```
   `--document-private-items` ensures internal docs build too. Broken intra-doc links or rustdoc warnings are failures (configure `RUSTDOCFLAGS="-D warnings"` if repo policy denies them). Map to `validation-rust-docs`.
3. Run the supply-chain gate:
   ```sh
   cargo audit
   cargo deny check
   ```
   `cargo audit` checks the RustSec advisory database; `cargo deny check` runs advisories, licenses, bans, and sources checks. Map to `validation-rust-supply-chain`. If `cargo deny` is not configured (no `deny.toml`), run `cargo audit` alone and record `cargo deny` as "not configured".
4. For each gate, record pass/fail and the relevant output (diff or clean; warning count or clean; advisory/violation count or clean).
5. If a gate fails, continue to phase 05 to give a full picture; the overall result will be fail.
6. Do NOT modify code, `Cargo.toml`, `rustfmt.toml`, or `deny.toml` to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/rust/workflows/validation.md`
- `docs/rust/style-formatting.md`
- `docs/rust/documentation-guidelines.md`
- `docs/rust/editions-tooling.md`
- `docs/rust/supply-chain-security.md`

## Operational Skills to Load

- `rust-supply-chain` — cargo audit / cargo deny.
- `rust-cargo-and-deps` — deny.toml configuration.

## Constraints to Apply

- `constraint-rust-scope-discipline` — do not modify code or config files to make a gate pass.

## Validations to Run

- `validation-rust-format` — command: `cargo fmt --all -- --check`
- `validation-rust-docs` — command: `cargo doc --no-deps --document-private-items`
- `validation-rust-supply-chain` — command: `cargo audit` and `cargo deny check`

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any gate red) | partial (a gate not-configured/not-applicable).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-rust-scope-discipline"]`
- `tests_run`: `["cargo fmt --all -- --check", "cargo doc --no-deps --document-private-items", "cargo audit", "cargo deny check"]` with pass/fail + evidence per gate (mark not-configured gates).
- `risks`: any gate red; supply-chain advisory requiring human judgment on exploitability.
- `next_phase`: `05-report`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
