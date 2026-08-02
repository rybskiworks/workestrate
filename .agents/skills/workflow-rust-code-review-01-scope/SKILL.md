---
name: workflow-rust-code-review-01-scope
description: |
  Use only for the scope phase of the Rust code-review workflow. Understand the
  diff, capture the change's stated intent, and identify risk areas. Do not use
  for analyzing, running gates, manual review, or issuing a verdict.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: rust-code-review
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (Rust code review)

## Phase purpose

Understand the diff, capture the change's stated intent, and identify risk
areas that will drive which docs and skills to consult in later phases.

## Steps to perform

1. Read the PR description and the full diff.
2. Record: which files changed, what the stated intent is, and whether the
   change touches public API, `unsafe`, async, FFI, or dependencies.
3. Identify risk areas (e.g. `unsafe` blocks, public API surface, dependency
   additions, FFI boundaries) that will drive which docs and skills to consult
   in phases 02-analyze and 04-review.
4. Apply `constraint-rust-scope-discipline`: review only the diff; do not
   request changes to untouched code. Pre-existing issues in untouched code are
   recorded as separate follow-ups, not as review findings.

## Docs to consult

None mandatory in this phase (docs load in 02-analyze and 04-review).
Optionally consult `docs/rust/api-design.md` if the diff touches the public
API, to prime the risk assessment.

## Operational skills to load

None mandatory in this phase.

## Constraints to apply

- `constraint-rust-scope-discipline` — review only the diff; do not request
  changes to untouched code. Pre-existing issues in untouched code are recorded
  as separate follow-ups, not as review findings.

## Validations to run

None — validations run in phase 03 (workflow-rust-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-rust-code-review-00-orchestration`. Set:

- `outcome`: `pass` if the diff and intent were captured; `partial` if the PR
  description is missing and intent had to be inferred.
- `files_touched`: one entry per changed file with a short `change` summary.
- `assumptions`: any inferred intent.
- `risks`: the risk areas identified in step 3.
- `next_phase`: `02-analyze`.
- `next_workflow`: `null`.
