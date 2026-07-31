---
name: workflow-nix-refactoring-04-verify
description: |
  Use only for the verify phase of the Nix refactoring workflow.
  After each incremental step, run nix build .#<name> + just lint-nix.
  Revert on any red gate. Do not use for baseline, planning, executing, or
  confirming.
allowed-tools: Read Write Edit Bash(nix:*) Bash(git:*) Bash(just:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-refactoring
  org.phase: verify
  org.phase_order: "04"
---

## Phase purpose

After each incremental step (phase 03), run the verification gates. If any gate fails, REVERT the step and redo it. Do not proceed on a red gate. `nix build .#<name>` is the primary no-regression signal (the output store path hash should be unchanged for behavior-preserving refactors); `just lint-nix` catches purity violations introduced by the refactor.

## Steps to perform

1. Run the gates in this order: `nix build .#<name>` (for each affected output); `just lint-nix`.
2. Map each command to its validation skill: `nix build .#<name>` → `validation-nix-build`; `just lint-nix` → `validation-nix-lint`.
3. If ANY gate fails: REVERT the step (`git revert HEAD` or the repo's revert mechanism for the checkpoint made in phase 03); do NOT proceed to the next step on a red gate; set `outcome: fail`, record which gate failed and the evidence, and return to phase 03 to redo the step.
4. If ALL gates pass: Compare build output store paths to the baseline from phase 01 (same output hash, same outputs). If the build result differs from baseline (different store path hash for a behavior-preserving refactor, or a new/missing output), that is a behavior change — STOP and hand off to the implementation workflow. If build output matches baseline, set `outcome: pass`.
5. If more planned steps remain, set `next_phase: 03-execute`. If this was the last step, set `next_phase: 05-confirm`.

## Docs to consult

- docs/nix/validation.md
- docs/nix/derivations-and-builds.md
- docs/nix/purity-and-sandboxing.md
- docs/nix/testing.md

## Operational skills to load

- `nix-testing`
- `nix-usage`

## Constraints to apply

- `constraint-nix-scope-discipline` — Verify only this step; do not make additional changes to make a gate pass (that is a new change requiring its own plan step); do not fold in defect fixes or features.

## Validations to run

Run after EACH incremental step:

- `validation-nix-build` (`nix build .#<name>` for each affected output)
- `validation-nix-lint` (`just lint-nix`)

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (all gates green, build output matches baseline) | `fail` (a gate red or behavior change detected)
- `files_touched`: `[]` (verification only; revert if failed)
- `constraints_applied`: `["constraint-nix-scope-discipline"]`
- `tests_run`: `["nix build .#<name>", "just lint-nix"]` with pass/fail per gate
- `risks`: behavior-change flag if build output diverged from baseline
- `next_phase`: `03-execute` (more steps) | `05-confirm` (last step) | `stop` (behavior change detected — hand off to implementation)
- `next_workflow`: `null` | `implementation` (if behavior change detected)
- `blockers`: the failing gate and evidence if `outcome: fail`
