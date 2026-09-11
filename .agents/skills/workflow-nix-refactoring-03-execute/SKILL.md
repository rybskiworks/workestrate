---
name: workflow-nix-refactoring-03-execute
description: |
  Use only for the execute phase of the Nix refactoring workflow.
  Make ONE incremental change from the plan and commit/checkpoint after
  each step. Do not use for baseline, planning, verifying, or confirming.
allowed-tools: Read Write Edit Bash(nix:*) Bash(git:*) Bash(just:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-refactoring
  org.phase: execute
  org.phase_order: "03"
---

## Phase purpose

Make ONE incremental change from the plan (phase 02). Commit or checkpoint after each step so a bad step can be reverted without losing the whole refactor. This phase runs once per planned step, looping with phase 04.

## Steps to perform

1. Take the next planned step from the phase 02 plan. Make only that one change.
2. Load the operational skill(s) identified for this step in the plan.
3. Implement the single structural change (e.g. extract module file, extract derivation, restructure flake output, compose overlays, replace `with` with `lib.` prefixes, replace `rec` with `let`, tighten source filter, split module).
4. Do NOT fold in unrelated cleanups, defect fixes, features, input/flake.lock changes, or lint-policy changes. Record those as follow-ups.
5. Stage new files (`git add -N` / `git add`) before any evaluation — untracked files are invisible to `.#` flake refs (classic flakes gotcha).
6. Commit or checkpoint the change so it can be reverted in isolation: `git add -A && git commit -m "refactor(nix): <step description>"` (Or checkpoint via the repo's preferred mechanism.)
7. Hand off to phase 04 for verification of this single step.

## Docs to consult

- docs/nix/validation.md
- The topic doc relevant to this step (e.g. docs/nix/modules-and-config.md for a module split, docs/nix/overlays.md for overlay composition, docs/nix/derivations-and-builds.md for a derivation extraction, docs/nix/conventions-and-style.md for scope cleanup).

## Operational skills to load

Conditional by step area:

- `nix-flake-anatomy` — flake output restructuring steps.
- `nix-modules` — module split / composition steps.
- `nix-overlays` — overlay composition steps.
- `nix-derivations` — derivation extraction / source filter steps.
- `nix-language` — scope cleanup (`with`/`rec`/`let`) steps.

## Constraints to apply

- `constraint-nix-scope-discipline` — Make only the one planned change; nothing else; do not fold in unrelated cleanups, defect fixes, features, input/flake.lock changes, or lint-policy changes.
- `constraint-nix-purity` — If this step touches source filters or path references: preserve purity (no `--impure`, no unfiltered `builtins.path`, no `<nixpkgs>` channel refs); use `cleanSourceWith` with a `filter =` field or native `.#` refs.
- `constraint-nix-sandbox-safety` — If this step touches derivation build phases or dependencies: preserve sandbox safety (no network in build, no `$HOME` writes outside `$TMPDIR`, `strictDeps` where applicable).

## Validations to run

None (verification runs in phase 04).

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (change made and committed) | `partial` (change made but not committed) | `fail` (could not complete the step)
- `files_touched`: the files changed in this step with a one-line change description each
- `constraints_applied`: `["constraint-nix-scope-discipline"]` (plus `constraint-nix-purity` and/or `constraint-nix-sandbox-safety` if applied)
- `assumptions`: the step description and which plan item it satisfies
- `risks`: whether the public flake output surface or source filters were touched
- `next_phase`: `04-verify`
- `next_workflow`: `null`
- `blockers`: any issue that prevented completing the step
