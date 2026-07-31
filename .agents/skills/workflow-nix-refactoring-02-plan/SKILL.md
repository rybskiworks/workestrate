---
name: workflow-nix-refactoring-02-plan
description: |
  Use only for the plan phase of the Nix refactoring workflow.
  Identify small, incremental, independently-verifiable refactoring steps.
  Do not use for baseline, executing, verifying, or confirming.
allowed-tools: Read Write Edit Bash(nix:*) Bash(git:*) Bash(just:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-refactoring
  org.phase: plan
  org.phase_order: "02"
---

## Phase purpose

Identify the refactoring steps. Each step must be small, incremental, and independently verifiable. The plan is the contract that phase 03 executes one step at a time and phase 04 verifies after each.

## Steps to perform

1. Using the behavior/goal description from phase 01, decompose the refactor into the smallest independently-verifiable steps. One step should be ONE of: extract a module file from an inline `imports` block; extract a derivation into its own file; restructure a flake output (e.g. move a package into `nix/packages/`); compose overlays via `lib.composeExtensions`; replace `with` with explicit `lib.` prefixes; replace `rec` with `let ... in`; tighten a `cleanSourceWith` filter; split a large module into sub-modules; or another single structural change.
2. Order the steps so each builds on a verified-green previous step. Avoid steps that require a later step to evaluate (e.g. do not split a module and update its import in two separate steps — combine or order so each step leaves the flake evaluable).
3. For each step, note which operational skill applies (see below) and whether the public flake output surface is touched (new/removed/renamed `packages.*`, `devShells.*`, `checks.*`, `overlays.*`). If a step changes the public output surface, flag it as a likely behavior change — either restructure the step to avoid it, or escalate to the implementation workflow.
4. Keep each step small enough that a revert is cheap. If a step grows, split it.
5. Record scope-creep candidates (unrelated cleanups, defects, features, input/flake.lock updates) as follow-ups; do NOT fold them into the refactor.

## Docs to consult

- docs/nix/validation.md
- docs/nix/flake-anatomy.md
- docs/nix/modules-and-config.md
- docs/nix/overlays.md
- docs/nix/derivations-and-builds.md
- docs/nix/conventions-and-style.md
- docs/nix/purity-and-sandboxing.md

## Operational skills to load

Conditional by refactor area:

- `nix-flake-anatomy` — flake output restructuring, input changes, system keying.
- `nix-modules` — module splits, option refactors, module composition.
- `nix-overlays` — overlay composition, `composeExtensions`, overlay chaining.
- `nix-derivations` — derivation extraction, `mkDerivation` restructuring, source filters.
- `nix-language` — scope cleanup (`with`/`rec`/`let`), expression simplification.

## Constraints to apply

- `constraint-nix-scope-discipline` — Plan only the structure described in phase 01; record unrelated items as follow-ups; do not fold in defect fixes, features, input/flake.lock changes, or lint-policy changes.

## Validations to run

None.

## Handoff output

Return the handoff YAML. Required fields:

- `outcome`: `pass` (plan produced) | `partial` (plan incomplete, needs review)
- `files_touched`: `[]` (planning only; no code changes in this phase)
- `constraints_applied`: `["constraint-nix-scope-discipline"]`
- `assumptions`: the ordered list of planned steps, each with its applicable operational skill and a public-output-surface-touch flag
- `risks`: steps that touch the public flake output surface or source filters
- `tests_needed`: any characterization checks still missing for a planned step
- `next_phase`: `03-execute`
- `next_workflow`: `null`
- `blockers`: any step that cannot be made behavior-preserving (escalate to implementation workflow)
