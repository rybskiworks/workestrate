---
name: workflow-beam-runtime-diagnosis-00-orchestration
description: |
  Use only to orchestrate the BEAM runtime-diagnosis workflow. Use when
  diagnosing live-runtime or operational problems in a BEAM system: memory
  leaks, scheduler pressure, runaway processes, mailbox growth, message-queue
  overload, GC pressure, and crash dumps (erl_crash.dump). This workflow is
  ops/runtime-oriented — it inspects a running or crashed system. Do not use for
  code-level defects with a known reproduction (use the project's language
  debugging workflow: workflow-elixir-debugging / workflow-gleam-debugging),
  implementing new code (use the project's language implementation workflow:
  workflow-elixir-implementation / workflow-gleam-implementation), or running
  the validation gate suite (use the project's language validation workflow).
allowed-tools: Read Write Edit Bash(rebar3:*) Bash(erl:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: beam-runtime-diagnosis
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: BEAM Runtime Diagnosis (Orchestration)

This skill orchestrates the BEAM runtime-diagnosis workflow. It is the entry point only. The actual phase work is performed by flat public phase skills at the same level as this orchestration skill (Shape A). Each phase is a loadable skill; load it by its exact skill name, execute it, and return its handoff YAML.

Source workflow doc: `docs/beam/workflows/runtime-diagnosis.md`.

This workflow is OPS/RUNTIME-oriented — it inspects a running or crashed system to identify operational issues. It is NOT a code-defect root-cause workflow; for code-level bugs (crash, hang, wrong output with a known reproduction), use the project's language debugging workflow (`workflow-elixir-debugging` / `workflow-gleam-debugging`) instead.

## Phase routing

The workflow runs five phases in order. Each phase is a flat public phase skill
loaded by exact skill name:

| Order | Phase skill name | Purpose |
|-------|------------------|---------|
| 01 | `workflow-beam-runtime-diagnosis-01-observe` | Preserve evidence (crash dump, baseline metrics); determine whether the system is running, degraded, or crashed. |
| 02 | `workflow-beam-runtime-diagnosis-02-isolate` | Identify the hotspot: reduction counting (hot loop), message_queue_len (slow consumer), heap_size (memory leak), scheduler_wall_time (scheduler saturation). |
| 03 | `workflow-beam-runtime-diagnosis-03-diagnose` | Isolate the cause via sys:trace/dbg; determine if it is a code bug (switch to debugging) or an operational issue (config, load, resource exhaustion). |
| 04 | `workflow-beam-runtime-diagnosis-04-mitigate` | Apply the fix or mitigation: code fix (switch to debugging), configuration change, or operational action. |
| 05 | `workflow-beam-runtime-diagnosis-05-verify` | Verify the symptom is resolved; baseline metrics return to normal; run validation gates if code changed. |

Load each phase skill in turn by its exact skill name. Do not inline phase work
into this orchestration file.

### Phase skipping conditions

- **`workflow-beam-runtime-diagnosis-01-observe` CANNOT be skipped.** Evidence
  preservation and baseline metrics are required before any change.
- **`workflow-beam-runtime-diagnosis-02-isolate` CANNOT be skipped.** The hotspot
  must be identified before mitigation.
- **`workflow-beam-runtime-diagnosis-03-diagnose` CANNOT be skipped.** The cause
  must be isolated and classified (code bug vs operational).
- **`workflow-beam-runtime-diagnosis-04-mitigate` MAY be skipped only if the
  diagnosis determines no action is needed (false alarm); otherwise it cannot be
  skipped.** Record the skip and reason in the handoff.
- **`workflow-beam-runtime-diagnosis-05-verify` CANNOT be skipped.** The symptom
  resolution must be confirmed.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-beam-runtime-diagnosis-03-diagnose` classifies the finding as a code-level defect | the project's language debugging workflow (`workflow-elixir-debugging-00-orchestration` or `workflow-gleam-debugging-00-orchestration`, per the project language) |
| `workflow-beam-runtime-diagnosis-05-verify` returns `outcome: pass` (operational/config-only finding, resolved) | `stop` |

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
  - constraint-beam-failure
  - ...
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: workflow-beam-runtime-diagnosis-02-isolate
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

Phase `workflow-beam-runtime-diagnosis-05-verify` additionally returns these
verification-phase fields:

- `validations_run` — list of validation skills executed.
- `constraints_checked` — list of constraint skills audited against the diff.
- `evidence` — symptom, hotspot, root cause, fix/mitigation, before/after
  metrics, per-gate pass/fail.
- `failures` — list of gates that failed (empty if all passed).
- `not_fully_checkable` — list of aspects that could not be fully validated and
  why.

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation policy

`suggest-next`: after phase `workflow-beam-runtime-diagnosis-03-diagnose`
returns its handoff, surface the classification — code bug vs operational — for
confirmation before loading `workflow-beam-runtime-diagnosis-04-mitigate`. If
the diagnosis reveals a code-level defect, hand off to the project's language
debugging workflow (`workflow-elixir-debugging` / `workflow-gleam-debugging`)
instead of proceeding to phase 04.

```yaml
continuation: suggest-next
after_phase: workflow-beam-runtime-diagnosis-03-diagnose
before_phase: workflow-beam-runtime-diagnosis-04-mitigate
reason: "Confirm root-cause classification (code bug vs operational) before applying mitigation."
```

For all other phase transitions, proceed automatically unless a phase returns
`outcome: fail` or sets `handoff_requires_hil: true`.
