---
name: workflow-nix-debugging-00-orchestration
description: |
  Use only to orchestrate the Nix debugging workflow. Use when fixing a failing
  Nix build, eval error, hash mismatch, purity violation, or store issue. Do not
  use for implementing new features (use workflow-nix-implementation),
  reviewing a diff (use workflow-nix-code-review), or restructuring without
  behavior change. If the fix requires new flake outputs or derivation
  structure, follow this workflow for the fix then workflow-nix-implementation
  for the implementation steps.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: nix-debugging
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Nix Debugging (Orchestration)

This skill orchestrates the Nix debugging workflow. It is the entry point only.
The actual phase work is performed by flat public phase skills at the same level
as this orchestration skill (Shape A). Each phase is a loadable skill; load it by
its exact skill name, execute it, and return its handoff YAML.

Source workflow doc: `docs/nix/error-handling-and-debugging.md`.

## Phase routing

The workflow runs five phases in order. Each phase is a flat public phase skill
loaded by exact skill name:

| Order | Phase skill name | Purpose |
|-------|------------------|---------|
| 01 | `workflow-nix-debugging-01-reproduce` | Reproduce the issue reliably; capture the exact command, input, environment, and full eval/build error output. |
| 02 | `workflow-nix-debugging-02-diagnose` | Categorize the defect (eval error / build failure / hash mismatch / purity violation / store issue) and identify the root cause. |
| 03 | `workflow-nix-debugging-03-fix` | Implement the minimal root-cause fix; do not suppress symptoms. |
| 04 | `workflow-nix-debugging-04-regression` | Add a check that would have caught the defect (regression test or guard). |
| 05 | `workflow-nix-debugging-05-verify` | Run the full check suite (`nix flake check`, `nix build`, `just verify`) and report. |

Load each phase skill in turn by its exact skill name. Do not inline phase work
into this orchestration file.

### Phase skipping conditions

- **`workflow-nix-debugging-01-reproduce` CANNOT be skipped.** Do not proceed
  to a fix on an unreproducible report. If the issue cannot be reproduced, set
  `outcome: fail`, `blockers: ["issue not reproduced"]`, and stop.
- **`workflow-nix-debugging-02-diagnose` CANNOT be skipped.** A fix without an
  identified category and root cause is symptom suppression. If the category
  cannot be determined, set `outcome: fail` and stop.
- **`workflow-nix-debugging-03-fix` CANNOT be skipped.** The workflow exists to
  apply a fix.
- **`workflow-nix-debugging-04-regression` CANNOT be skipped.** Every fix must
  encode its reproduction as a regression check so the defect cannot silently
  return.
- **`workflow-nix-debugging-05-verify` CANNOT be skipped.** All gates must pass
  before the workflow is declared complete.

No phase may be skipped. If a phase cannot complete, it must fail its handoff
rather than be omitted.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-nix-debugging-05-verify` returns `outcome: pass` | `workflow-nix-validation-00-orchestration` |

This workflow's place in the overall SDLC: see docs/languages.md "Workflows map" for the full graph.

## Handoff format

Each phase returns a handoff in this YAML schema. The next phase consumes the
prior phase's handoff.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-nix-scope-discipline
  - ...
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: workflow-nix-debugging-02-diagnose
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

Phase `workflow-nix-debugging-05-verify` additionally returns these
verification-phase fields:

- `validations_run` — list of validation skills executed.
- `constraints_checked` — list of constraint skills audited against the diff.
- `evidence` — reproduction command, original error, category, root cause, fix,
  regression check location/assertion, per-gate pass/fail.
- `failures` — list of gates that failed (empty if all passed).
- `not_fully_checkable` — list of aspects that could not be fully validated and
  why.

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation policy

`suggest-next`: each diagnosis phase benefits from confirmation before
proceeding to the fix. After `workflow-nix-debugging-02-diagnose` returns its
handoff, surface the category and root cause for confirmation before loading
`workflow-nix-debugging-03-fix`.

```yaml
continuation: suggest-next
after_phase: workflow-nix-debugging-02-diagnose
before_phase: workflow-nix-debugging-03-fix
reason: "Confirm defect category and root cause before applying the fix."
```

For all other phase transitions, proceed automatically unless a phase returns
`outcome: fail` or sets `handoff_requires_hil: true`.
