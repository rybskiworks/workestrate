---
name: workflow-nix-code-review-00-orchestration
description: |
  Use only to orchestrate the Nix code-review workflow. Use when reviewing a
  pull request, auditing a diff, or performing a pre-merge gate on Nix changes
  — flake.nix, flake.lock, files under nix/, derivations, modules, or overlays.
  Do not use for implementing new Nix code (use workflow-nix-implementation),
  refactoring without behavior change, or fixing a defect (use
  workflow-nix-debugging). If the review surfaces a defect, hand off to
  workflow-nix-debugging; do not fix in the review pass.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: nix-code-review
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Nix Code Review (Orchestration)

This is the orchestration entry point for the Nix code-review workflow. Each
phase is a separate public skill (Shape A) loaded by exact name. Do not
execute review work directly from this file; load the phase skill for the
current phase and follow it.

The workflow enforces a deterministic order: understand the change, load review
skills, run the automated gates, perform the human-judgment review using the
per-doc checklists, and report findings with severity levels. If the review
surfaces a defect, hand off to `workflow-nix-debugging`; do not fix in the
review pass.

## Phase routing

| Order | Phase skill | Purpose |
| --- | --- | --- |
| 01 | `workflow-nix-code-review-01-scope` | Understand the diff, capture intent, identify risk areas. |
| 02 | `workflow-nix-code-review-02-analyze` | Categorize changes by dimension; load relevant review skills. |
| 03 | `workflow-nix-code-review-03-check` | Run automated gates: `just lint-nix`, `nix flake check`, `nix build`. |
| 04 | `workflow-nix-code-review-04-review` | Manual review using per-doc checklists: purity, reproducibility, sandbox, scope, secrets, store hygiene. |
| 05 | `workflow-nix-code-review-05-verdict` | Report findings with severity levels; issue final verdict. |

Phases run strictly in order. Each phase skill returns a handoff YAML block;
the orchestrator reads `next_phase` to decide which phase skill to load next.

### Phase skipping conditions

- `workflow-nix-code-review-01-scope` — cannot be skipped. The diff must be
  understood before any judgment is applied.
- `workflow-nix-code-review-02-analyze` — cannot be skipped. Skill loading is
  driven by the categorization; skipping risks reviewing a fixed-output
  derivation without the `constraint-nix-reproducibility` skill loaded.
- `workflow-nix-code-review-03-check` — may be skipped **only if** the gates
  (`just lint-nix`, `nix flake check`, `nix build`) were already run green in
  CI and the evidence (command, commit SHA, CI run URL or log excerpt) is
  attached to the handoff. If any gate was not run or its evidence is missing,
  run the gates here. Note: `nix flake check` and `nix build` require nix on
  the host; if nix is unavailable, run `just lint-nix` (in-container) and mark
  the nix-gated checks `not_fully_checkable`.
- `workflow-nix-code-review-04-review` — cannot be skipped. Automated gates do
  not substitute for the per-doc checklist review.
- `workflow-nix-code-review-05-verdict` — cannot be skipped. A review without
  a recorded verdict and severity-classified findings is incomplete.

When a phase is skipped, record the skip reason and the attached evidence in
the handoff `assumptions` and `tests_run` fields.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-nix-code-review-05-verdict` returns verdict `approve` | `workflow-nix-validation-00-orchestration` |
| `workflow-nix-code-review-05-verdict` returns `changes-requested` (new feature/capability) | `workflow-nix-implementation-00-orchestration` |
| `workflow-nix-code-review-05-verdict` returns `changes-requested` (behavior-preserving restructure) | `workflow-nix-refactoring-00-orchestration` |
| `workflow-nix-code-review-05-verdict` returns `changes-requested` (defect fix) | `workflow-nix-debugging-00-orchestration` |

This workflow's place in the overall SDLC: see docs/languages.md "Workflows map" for the full graph.

## Handoff format

Each phase returns a handoff YAML block using this schema. The orchestrator
reads `next_phase` to advance; `next_phase: null` ends the workflow.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-nix-purity
  - ...
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: 04-review
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

Phase `workflow-nix-code-review-05-verdict` additionally returns the
verification-phase extra fields: `validations_run`, `constraints_checked`,
`evidence`, `failures`, `not_fully_checkable`.

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation policy

This workflow uses `suggest-next`: after each phase completes, the orchestrator
recommends the next phase but waits for confirmation before loading the next
phase skill.

```yaml
continuation: suggest-next
behavior: recommend next phase; wait for confirmation before proceeding
```

If a phase returns `outcome: fail` with a blocker, the orchestrator surfaces
the blocker and asks whether to stop or continue to the next phase anyway
(continuing past a blocker should be explicit, not implicit).
