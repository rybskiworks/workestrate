---
name: workflow-nix-code-review-01-scope
description: |
  Use only for the scope phase of the Nix code-review workflow. Understand the
  diff, capture the change's stated intent, and identify risk areas. Do not use
  for analyzing, running gates, manual review, or issuing a verdict.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-code-review
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (Nix code review)

## Phase purpose

Understand the diff, capture the change's stated intent, and identify risk
areas that will drive which docs and skills to consult in later phases.

## Steps to perform

1. Read the PR description and the full diff.
2. Record: which files changed, what the stated intent is, and whether the
   change touches `flake.nix`, `flake.lock`, files under `nix/`, derivations
   (`mkDerivation`), fixed-output derivations (FODs), modules, overlays,
   devShells, or the justfile's nix-adjacent recipes.
3. Identify risk areas (e.g. `flake.lock` input changes, FOD hash updates,
   source-filter changes, `--impure` introductions, secret references, sandbox
   escapes, new store-path-producing commands) that will drive which docs and
   skills to consult in phases 02-analyze and 04-review.
4. Apply `constraint-nix-scope-discipline`: review only the diff; do not
   request changes to untouched code. Pre-existing issues in untouched code are
   recorded as separate follow-ups, not as review findings.

## Docs to consult

None mandatory in this phase (docs load in 02-analyze and 04-review).
Optionally consult `docs/nix/flake-anatomy.md` if the diff touches `flake.nix`,
or `docs/nix/purity-and-sandboxing.md` if the diff touches source filters or
derivations, to prime the risk assessment.

## Operational skills to load

None mandatory in this phase.

## Constraints to apply

- `constraint-nix-scope-discipline` — review only the diff; do not request
  changes to untouched code. Pre-existing issues in untouched code are recorded
  as separate follow-ups, not as review findings.

## Validations to run

None — validations run in phase 03 (workflow-nix-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-nix-code-review-00-orchestration`. Set:

- `outcome`: `pass` if the diff and intent were captured; `partial` if the PR
  description is missing and intent had to be inferred.
- `files_touched`: one entry per changed file with a short `change` summary.
- `assumptions`: any inferred intent.
- `risks`: the risk areas identified in step 3.
- `next_phase`: `02-analyze`.
- `next_workflow`: `null`.
