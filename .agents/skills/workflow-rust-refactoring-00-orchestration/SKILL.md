---
name: workflow-rust-refactoring-00-orchestration
description: |
  Use only to orchestrate the Rust refactoring workflow. Use when restructuring
  existing Rust code without changing behavior: module splits, ownership
  simplification, pattern replacement, helper extraction, dead-code cleanup.
  Do not use for adding features, fixing defects, reviewing a diff, or any
  change that intends behavior change (use the implementation workflow instead).
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: rust-refactoring
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Rust Refactoring (Orchestration)

Orchestrate a behavior-preserving refactor of Rust code. The defining constraint is: **behavior must NOT change**. Every phase enforces this. The phases of this workflow are flat public phase skills loaded by exact skill name.

## Phase routing

Phases are flat public phase skills, loaded by exact skill name. Run them
in order:

| Order | Phase skill name | Purpose |
|-------|------------------|---------|
| 01 | `workflow-rust-refactoring-01-baseline` | Understand current behavior; establish a green test baseline; add characterization tests if coverage is thin. |
| 02 | `workflow-rust-refactoring-02-plan` | Identify small, incremental, independently-verifiable refactoring steps. |
| 03 | `workflow-rust-refactoring-03-execute` | Make ONE incremental change; commit/checkpoint after each step. |
| 04 | `workflow-rust-refactoring-04-verify` | After each step: cargo check + cargo test + cargo clippy. Revert on any red gate. |
| 05 | `workflow-rust-refactoring-05-confirm` | After all steps: full test suite, confirm no regression, clippy, cargo fmt --check, report. |

Phases 03 and 04 loop per incremental step until the plan from phase 02 is
exhausted, then phase 05 runs once.

### Phase skipping conditions

- **`workflow-rust-refactoring-01-baseline`**: Never skip. A refactor cannot be verified without a green baseline. If tests fail at baseline, STOP and hand off to the debugging workflow (`docs/rust/workflows/debugging.md`).
- **`workflow-rust-refactoring-02-plan`**: Skip only if the refactor is a single trivial step that needs no plan; otherwise always run.
- **`workflow-rust-refactoring-03-execute`** / **`workflow-rust-refactoring-04-verify`**: Run once per planned step. Never skip — a step without verification is an unverifiable change.
- **`workflow-rust-refactoring-05-confirm`**: Never skip. This is the final no-regression gate.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-rust-refactoring-05-confirm` returns `outcome: pass` | `workflow-rust-code-review-00-orchestration` |

This workflow's place in the overall SDLC: see docs/languages.md "Workflows map" for the full graph.

## Handoff format

Each phase returns this YAML handoff:

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-rust-scope-discipline
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: workflow-rust-refactoring-04-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

Phase `workflow-rust-refactoring-05-confirm` additionally returns:

```yaml
validations_run:
  - validation-rust-clippy
  - validation-rust-format
constraints_checked:
  - constraint-rust-scope-discipline
evidence:
  - ...
failures:
  - ...
not_fully_checkable:
  - ...
```

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation Policy

**conditional-next**: proceed to the next phase only if `outcome == pass` AND no
behavior change is detected.

- If a behavior change is detected at any phase, STOP and hand off to the
  implementation workflow (`docs/rust/workflows/implementation.md`). Do not
  smuggle a behavior change through a refactoring pass.
- If a gate is red at phase `workflow-rust-refactoring-04-verify`, REVERT the
  step and redo phase `workflow-rust-refactoring-03-execute`; do not proceed
  to phase `workflow-rust-refactoring-05-confirm` on a red gate.
- If the baseline (phase `workflow-rust-refactoring-01-baseline`) is red, STOP
  and hand off to the debugging workflow.

```yaml
continuation: conditional-next
condition: "outcome == pass && no_behavior_change_detected"
next_phase: <next phase skill name in sequence>
fallback: stop
```

## Docs Consulted

- `docs/rust/workflows/refactoring.md` (source workflow)
- `docs/rust/design-patterns.md`
- `docs/rust/ownership-lifetimes.md`
- `docs/rust/types-traits-generics.md`
- `docs/rust/iterators-closures.md`
- `docs/rust/smart-pointers-memory.md`
- `docs/rust/error-handling.md`
