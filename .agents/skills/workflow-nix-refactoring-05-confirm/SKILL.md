---
name: workflow-nix-refactoring-05-confirm
description: |
  Use only for the confirm phase of the Nix refactoring workflow.
  After all steps, run the final no-regression check (full just verify-full
  + nix flake check) and report. Do not use for baseline, planning,
  executing, or verifying.
allowed-tools: Read Write Edit Bash(nix:*) Bash(git:*) Bash(just:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-refactoring
  org.phase: confirm
  org.phase_order: "05"
---

## Phase purpose

After all planned steps are complete, run the final no-regression check and report. Confirm the build output matches the baseline from phase 01, run the full `just verify-full` suite (which includes `nix build .#workestrate`), run `nix flake check`, and report what changed, why, and the evidence of no regression.

## Steps to perform

1. Run the full verification suite and confirm the output matches the baseline from phase 01 (same build output store path hash, same flake outputs, no new failures): `just verify-full`. This runs the project's complete pre-merge gate chain including `nix build .#workestrate`. If characterization checks were added in phase 01, confirm they still pass unchanged. If any check had to change, that is a behavior change — either justify it explicitly and switch to the implementation workflow, or revert.
2. Run the full flake check to confirm all outputs evaluate and all `checks` derivations build: `nix flake check`. This is broader than the per-step `nix build .#<name>` — it evaluates every output and builds every `checks.*` derivation.
3. Run the purity lint to catch any impurity violations introduced across the full refactor: `just lint-nix`.
4. Map gates to validation skills: `nix build .#<name>` / `just verify-full` → `validation-nix-build`; `nix flake check` → `validation-nix-flake-check`; `just lint-nix` → `validation-nix-lint`.
5. Aggregate evidence and report: the one-paragraph behavior/goal description from phase 01; the baseline gate results from phase 01; the list of incremental steps taken, with the gate results after each (from phase 04); the final `just verify-full` result confirming no regression; the final `nix flake check` result; the final `just lint-nix` result; the list of files changed; an explicit statement that no behavior change occurred, OR a flagged behavior change with a handoff to the implementation workflow.

## Docs to consult

- docs/nix/validation.md
- docs/nix/derivations-and-builds.md
- docs/nix/purity-and-sandboxing.md
- docs/nix/testing.md
- docs/nix/conventions-and-style.md

## Operational skills to load

- `nix-testing`
- `nix-usage`

## Constraints to apply

- `constraint-nix-scope-discipline` — Confirm only the planned refactor; flag any drift as a behavior change; do not fold in defect fixes, features, input/flake.lock changes, or lint-policy changes.

## Validations to run

Final no-regression confirmation:

- `validation-nix-build` (`just verify-full`, which includes `nix build .#workestrate`)
- `validation-nix-flake-check` (`nix flake check`)
- `validation-nix-lint` (`just lint-nix`)

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (no regression, all final gates green) | `fail` (regression or red final gate) | `partial` (gates green but a behavior change was flagged)
- `files_touched`: full list of files changed across all steps
- `constraints_applied`: `["constraint-nix-scope-discipline"]`
- `tests_run`: `["just verify-full", "nix flake check", "just lint-nix"]` with pass/fail per gate
- `validations_run`: `["validation-nix-build", "validation-nix-flake-check", "validation-nix-lint"]`
- `constraints_checked`: `["constraint-nix-scope-discipline"]`
- `evidence`: baseline comparison, per-step gate results, final gate results, files changed, explicit no-behavior-change statement
- `failures`: any red final gate or detected behavior change
- `next_phase`: `stop`
- `next_workflow`: `null` | `implementation` (if behavior change flagged) | `debugging` (if a defect surfaced)
- `blockers`: any unresolved regression
