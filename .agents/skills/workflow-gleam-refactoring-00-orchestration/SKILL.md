---
name: workflow-gleam-refactoring-00-orchestration
description: |
  Use only to orchestrate the Gleam refactoring workflow. Use when restructuring
  existing Gleam code without changing behavior: module splits, type redesign,
  helper extraction, pattern replacement, dead-code cleanup, consolidating
  externals. Do not use for adding features, fixing defects, reviewing a diff,
  or any change that intends behavior change (use the implementation workflow
  instead).
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: gleam-refactoring
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Gleam Refactoring (Orchestration)

Orchestrate a behavior-preserving refactor of Gleam code. The defining constraint is: **behavior must NOT change**. Every phase enforces this. The phases of this workflow are flat public phase skills loaded by exact skill name.

## Phase routing

Phases are flat public phase skills, loaded by exact skill name. Run them
in order:

| Order | Phase skill name | Purpose |
|-------|------------------|---------|
| 01 | `workflow-gleam-refactoring-01-baseline` | Understand current behavior; establish a green test baseline; add characterization tests if coverage is thin. |
| 02 | `workflow-gleam-refactoring-02-plan` | Identify small, incremental, independently-verifiable refactoring steps. |
| 03 | `workflow-gleam-refactoring-03-execute` | Make ONE incremental change; commit/checkpoint after each step. |
| 04 | `workflow-gleam-refactoring-04-verify` | After each step: `gleam check` + `gleam test`. Revert on any red gate. |
| 05 | `workflow-gleam-refactoring-05-confirm` | After all steps: full test suite, confirm no regression, `gleam format --check`, report. |

Phases 03 and 04 loop per incremental step until the plan from phase 02 is
exhausted, then phase 05 runs once.

### Phase skipping conditions

- **`workflow-gleam-refactoring-01-baseline`**: Never skip. A refactor cannot be verified without a green baseline. If tests fail at baseline, STOP and hand off to the debugging workflow (`docs/gleam/workflows/debugging.md`).
- **`workflow-gleam-refactoring-02-plan`**: Skip only if the refactor is a single trivial step that needs no plan; otherwise always run.
- **`workflow-gleam-refactoring-03-execute`** / **`workflow-gleam-refactoring-04-verify`**: Run once per planned step. Never skip — a step without verification is an unverifiable change.
- **`workflow-gleam-refactoring-05-confirm`**: Never skip. This is the final no-regression gate.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-gleam-refactoring-05-confirm` returns `outcome: pass` | `workflow-gleam-code-review-00-orchestration` |

This workflow's place in the overall SDLC: see docs/languages.md "Workflows map" for the full graph.

## Handoff format

Each phase returns this YAML handoff:

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-gleam-result
  - constraint-gleam-conventions
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: workflow-gleam-refactoring-04-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

Phase `workflow-gleam-refactoring-05-confirm` additionally returns:

```yaml
validations_run:
  - validation-gleam-format
constraints_checked:
  - constraint-gleam-result
  - constraint-gleam-conventions
evidence:
  - ...
failures:
  - ...
not_fully_checkable:
  - ...
```

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation policy

**conditional-next**: proceed to the next phase only if `outcome == pass` AND no
behavior change is detected.

- If a behavior change is detected at any phase, STOP and hand off to the
  implementation workflow (`docs/gleam/workflows/implementation.md`). Do not
  smuggle a behavior change through a refactoring pass.
- If a gate is red at phase `workflow-gleam-refactoring-04-verify`, REVERT the
  step and redo phase `workflow-gleam-refactoring-03-execute`; do not proceed
  to phase `workflow-gleam-refactoring-05-confirm` on a red gate.
- If the baseline (phase `workflow-gleam-refactoring-01-baseline`) is red, STOP
  and hand off to the debugging workflow.

```yaml
continuation: conditional-next
condition: "outcome == pass && no_behavior_change_detected"
next_phase: <next phase skill name in sequence>
fallback: stop
```

## Docs Consulted

- `docs/gleam/workflows/refactoring.md` (source workflow)
- `docs/gleam/stdlib.md`
- `docs/gleam/result-option-and-errors.md`
- `docs/gleam/conventions-patterns-antipatterns.md`
- `docs/gleam/types-records-and-patterns.md`
