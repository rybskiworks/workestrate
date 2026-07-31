---
name: workflow-nix-validation-03-lint-test
description: |
  Use only for the lint and test phase of the Nix validation workflow.
  Run just lint-nix, nix build .#checks.x86_64-linux.<name>, and
  just store-audit. Do not use for scoping, compile, formatting, or
  final reporting.
allowed-tools: Read Bash(nix:*) Bash(just:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-validation
  org.phase: lint-test
  org.phase_order: "03"
---

# Nix Validation Workflow — Phase 03 — Lint, Test & Supply Chain

## Phase Purpose

Run the purity-lint, check-build, and store-audit gates. Record pass/fail and
evidence for each. All applicable gates must pass. Note that `just lint-nix`
runs IN-CONTAINER and does NOT require nix on the host; the other gates do
require nix (HOST-GATE).

## Steps

1. Run the purity-lint gate:
   ```sh
   just lint-nix
   ```
   `just lint-nix` is a purity guard that runs in-container and requires only `bash`, `awk`, `find` — no nix needed. It checks `.nix` files, `.sh` scripts, `justfile`, and `docs/**/*.md` for purity violations. Any violation is a blocker. Map to `validation-nix-lint`.
2. Run the check-build gate:
   ```sh
   nix build .#checks.x86_64-linux.<name>
   ```
   Build each `checks.x86_64-linux.<name>` derivation (e.g., `validateConfig`). Any build failure is a blocker. This gate requires nix on the host (HOST-GATE); if nix is absent, mark `not_fully_checkable`. Map to `validation-nix-test`.
3. Run the supply-chain gate:
   ```sh
   just store-audit
   ```
   `just store-audit` is a supply-chain/store-hygiene probe producing a top-20 store report and source-path threshold check. This gate requires nix on the host (HOST-GATE); if nix is absent, mark `not_fully_checkable`. Map to `validation-nix-supply-chain`.
4. For each gate, record pass/fail and the relevant output (violation count or clean; build log or store path; top-20 report and source-path threshold).
5. If a gate fails, continue to the remaining gates (phase 04) to give a full picture; the overall result will be fail.
6. Do NOT add tests, modify scripts, or change flake config to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/nix/validation.md`
- `docs/nix/testing.md`
- `docs/nix-purity.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/supply-chain-security.md`

## Operational Skills to Load

- `nix-usage` — check derivations, store-audit context.

## Constraints to Apply

- `constraint-nix-scope-discipline` — do not add tests, modify scripts, or change flake config to make a gate pass.

## Validations to Run

- `validation-nix-lint` — command: `just lint-nix`
- `validation-nix-test` — command: `nix build .#checks.x86_64-linux.<name>`
- `validation-nix-supply-chain` — command: `just store-audit`

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any gate red) | partial (a gate not_fully_checkable).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-nix-scope-discipline"]`
- `tests_run`: `["just lint-nix", "nix build .#checks.x86_64-linux.<name>", "just store-audit"]` with pass/fail + evidence per gate (mark not_fully_checkable if nix absent).
- `risks`: any gate red; nix absent on host (check-build and store-audit not_fully_checkable).
- `next_phase`: `04-format-doc`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
