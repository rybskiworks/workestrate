---
name: workflow-nix-refactoring-01-baseline
description: |
  Use only for the baseline phase of the Nix refactoring workflow.
  Understand current behavior and establish a green build/flake-check/lint
  baseline before any change. Do not use for planning, executing, verifying,
  or confirming.
allowed-tools: Read Write Edit Bash(nix:*) Bash(git:*) Bash(just:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-refactoring
  org.phase: baseline
  org.phase_order: "01"
---

## Phase purpose

Understand the current behavior of the Nix code being refactored and establish a green build/flake-check/lint baseline BEFORE any change. A refactor on a red baseline cannot be verified. This phase enforces the defining constraint of the workflow: behavior must NOT change.

## Steps to perform

1. Read the Nix code being refactored (`flake.nix`, `nix/` modules, overlays, derivations) and identify the structure being improved and why.
2. Read the relevant topic docs so the refactor is grounded in the corpus rules: docs/nix/flake-anatomy.md; docs/nix/modules-and-config.md; docs/nix/overlays.md; docs/nix/derivations-and-builds.md; docs/nix/conventions-and-style.md; docs/nix/purity-and-sandboxing.md.
3. Write a one-paragraph description of the current behavior and the refactoring goal (what structure is being improved and why). Later phases verify the goal was met without behavior change.
4. Run the baseline build gate: `nix build .#<name>` for each affected flake output. If the refactor targets the flake itself (not a specific package), run `nix flake check --no-build` at minimum.
5. Run the baseline flake-check gate: `nix flake check --no-build` (or full `nix flake check` if a build is feasible and desired).
6. Run the baseline purity-lint gate: `just lint-nix`.
7. If any gate does NOT pass: STOP. Do not refactor on a red baseline. Hand off to the debugging workflow (docs/nix/error-handling-and-debugging.md) to fix the baseline first. Set `outcome: fail`, `next_workflow: debugging`, and record the blocker.
8. If all gates pass, assess coverage in the area being refactored. If coverage is thin (no `checks` exercising the module/derivation being refactored), add characterization checks (`nixosTests`, `runCommand`, or `testers`) that pin the current behavior. These checks are part of the refactor, not separate work.
9. Record the baseline gate results (pass/fail per output, flake-check status, lint status) as evidence for the final no-regression comparison in phase 05.

## Docs to consult

- docs/nix/validation.md
- docs/nix/flake-anatomy.md
- docs/nix/modules-and-config.md
- docs/nix/overlays.md
- docs/nix/derivations-and-builds.md
- docs/nix/conventions-and-style.md
- docs/nix/purity-and-sandboxing.md
- docs/nix/testing.md

## Operational skills to load

- `nix-flake-anatomy` — if the refactor touches flake outputs or inputs.
- `nix-testing` — for characterization checks.
- `nix-usage` — for project-specific flake context and command conventions.

## Constraints to apply

- `constraint-nix-scope-discipline` — Record only the current behavior and goal; do not begin changing structure in this phase; do not fold in defect fixes, features, dependency (input) changes, or lint-policy changes.

## Validations to run

- `validation-nix-build` — establish green baseline (affected outputs must build before refactoring; run `nix build .#<name>` to confirm the baseline is green).
- `validation-nix-flake-check` — establish green flake evaluation baseline (`nix flake check --no-build`).
- `validation-nix-lint` — establish green purity-lint baseline (`just lint-nix`).

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (baseline green) | `fail` (baseline red — hand off to debugging) | `partial` (baseline green but characterization checks still needed)
- `files_touched`: any characterization checks added
- `constraints_applied`: `["constraint-nix-scope-discipline"]`
- `assumptions`: the one-paragraph behavior/goal description
- `tests_run`: `["nix build .#<name>", "nix flake check --no-build", "just lint-nix"]` plus baseline pass/fail per gate
- `tests_needed`: characterization checks still to add, if any
- `next_phase`: `02-plan` (if pass) | `stop` (if fail)
- `next_workflow`: `null` (if pass) | `debugging` (if fail)
- `blockers`: red-baseline blocker if applicable
