---
name: workflow-nix-validation-05-report
description: |
  Use only for the report phase of the Nix validation workflow.
  Aggregate all gate results into a per-gate table and report the
  overall verdict. Do not use for scoping, compile, testing, or
  formatting.
allowed-tools: Read
metadata:
  org.kind: workflow-phase
  org.workflow: nix-validation
  org.phase: report
  org.phase_order: "05"
---

# Nix Validation Workflow — Phase 05 — Report

## Phase Purpose

Aggregate all gate results into a per-gate table. Report the overall verdict:
pass (all applicable gates green) or fail (any applicable gate red). If a gate
failed, hand off to the appropriate fixing workflow — do NOT fix in the
validation pass. Mark HOST-GATE gates "N-A" when nix is absent and the format
gate "not configured".

## Steps

1. Aggregate the gate results from phases 02-04 into a per-gate table:

   | Gate | Command | Result | Evidence |
   |---|---|---|---|
   | Flake check | `nix flake check` | pass/fail/N-A | eval errors or clean; check build failures or clean |
   | Eval | `nix eval .#workestrate.meta.description` / `nix flake show` | pass/fail/N-A | evaluated value or eval error |
   | Purity lint | `just lint-nix` | pass/fail | violation count or clean |
   | Check builds | `nix build .#checks.x86_64-linux.<name>` | pass/fail/N-A | build log or store path |
   | Store audit | `just store-audit` | pass/fail/N-A | top-20 report; source-path threshold |
   | Format | `nix fmt --check` | pass/fail/not configured | diff or clean; "not configured" |
   | Docs cross-refs | manual / `grep` verification | pass/fail | broken refs or clean |

   Mark HOST-GATE gates "N-A" when nix is absent. Mark the format gate "not configured".
2. Compute the overall verdict:
   - **pass** — all applicable gates green.
   - **fail** — any applicable gate red.
3. If any applicable gate failed, hand off to the appropriate fixing workflow — do NOT fix in the validation pass:
   - Defect → `docs/nix/error-handling-and-debugging.md`
   - Missing tests/checks → `docs/nix/testing.md`
   - Structural cleanup → `docs/nix/conventions-and-style.md`
   Then re-run validation after the fix.
4. Keep the report to the per-gate table and overall verdict; do not expand it into a code review or implementation summary.

## Docs to Consult

- `docs/nix/validation.md`
- `docs/nix/error-handling-and-debugging.md`
- `docs/nix/supply-chain-security.md`

## Operational Skills to Load

None. This phase aggregates results; it does not run gates or load operational skills.

## Constraints to Apply

- `constraint-nix-scope-discipline` — validation is a gate, not a fix; do not modify code or config; hand off failures to fixing workflows.

## Validations to Run

None. This is a reporting phase — it aggregates all gate results from phases 02-04; it does not run validation gates itself.

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (all applicable gates green) | fail (any applicable gate red).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-nix-scope-discipline"]`
- `tests_run`: the full list of gate commands run across phases 02-04 with pass/fail per gate.
- `validations_run`: `["validation-nix-flake-check", "validation-nix-eval", "validation-nix-lint", "validation-nix-test", "validation-nix-format", "validation-nix-supply-chain"]`
- `constraints_checked`: `["constraint-nix-scope-discipline"]`
- `evidence`: the per-gate table; the overall verdict.
- `failures`: the list of failing applicable gates with evidence.
- `not_fully_checkable`: gates marked N-A (nix absent on host) or not-configured (format gate).
- `next_phase`: stop.
- `next_workflow`: `null` (if pass) | `debugging` | `implementation` | `refactoring` (if any applicable gate failed).
- `blockers`: the failing gates if `outcome: fail`.
