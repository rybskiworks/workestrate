---
name: workflow-nix-validation-02-compile
description: |
  Use only for the compile phase of the Nix validation workflow.
  Run nix flake check and nix eval / nix flake show. Do not use for
  scoping, testing, formatting, or final reporting.
allowed-tools: Read Bash(nix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-validation
  org.phase: compile
  org.phase_order: "02"
---

# Nix Validation Workflow — Phase 02 — Compile & Eval

## Phase Purpose

Run the flake-check and eval gates. Record pass/fail and evidence for each. A
failure here is a blocker for the overall verdict, but continue to the
remaining gates to give a full picture. These gates require nix on the host
(HOST-GATE); if nix is absent (determined in phase 01), mark them
`not_fully_checkable`.

## Steps

1. Run the flake-check gate:
   ```sh
   nix flake check
   ```
   `nix flake check` evaluates all flake outputs and builds all `checks.<system>` derivations. Any eval error or check-build failure is a blocker. Map to `validation-nix-flake-check`.
2. Run the eval gate:
   ```sh
   nix eval .#workestrate.meta.description
   nix flake show
   ```
   `nix eval .#workestrate.meta.description` confirms the package meta evaluates; `nix flake show` lists all flake outputs and confirms each evaluates. Any eval error is a blocker. Map to `validation-nix-eval`.
3. For each gate, record pass/fail and the relevant output (eval errors or clean; check build failures or clean; evaluated value or eval error).
4. If a gate fails, continue to the remaining gates (phase 03, 04) to give a full picture; the overall result will be fail.
5. If nix is absent on the host (HOST-GATE not satisfied), mark both gates `not_fully_checkable` with the reason "nix not on host" and proceed to phase 03.
6. Do NOT modify code or flake config to make a gate pass. Record the failure and hand off to a fixing workflow in phase 05.

## Docs to Consult

- `docs/nix/validation.md`
- `docs/nix/flake-anatomy.md`
- `docs/nix/error-handling-and-debugging.md`

## Operational Skills to Load

- `nix-usage` — flake outputs, `nix flake check` / `nix eval` semantics.

## Constraints to Apply

- `constraint-nix-scope-discipline` — do not modify code or flake config to make a gate pass.

## Validations to Run

- `validation-nix-flake-check` — command: `nix flake check`
- `validation-nix-eval` — command: `nix eval .#workestrate.meta.description` and `nix flake show`

## Handoff Output

Return the handoff YAML. Required fields:
- `outcome`: pass (both gates green) | fail (any gate red) | partial (one gate not_fully_checkable).
- `files_touched`: `[]` (validation only; no changes).
- `constraints_applied`: `["constraint-nix-scope-discipline"]`
- `tests_run`: `["nix flake check", "nix eval .#workestrate.meta.description", "nix flake show"]` with pass/fail + evidence per gate (mark not_fully_checkable if nix absent).
- `risks`: any gate red; nix absent on host.
- `next_phase`: `03-lint-test`.
- `next_workflow`: `null`.
- `blockers`: the failing gate and evidence if `outcome: fail`.
