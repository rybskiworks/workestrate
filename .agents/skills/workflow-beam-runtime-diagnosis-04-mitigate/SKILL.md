---
name: workflow-beam-runtime-diagnosis-04-mitigate
description: |
  Use only for the mitigate phase of the BEAM runtime-diagnosis workflow.
  Apply the fix or mitigation: code fix (switch to debugging), configuration
  change, or operational action. Do not use for observing the system, isolating
  the hotspot, diagnosing the cause, or final verification.
allowed-tools: Read Write Edit Bash(rebar3:*) Bash(erl:*)
metadata:
  org.kind: workflow-phase
  org.workflow: beam-runtime-diagnosis
  org.phase: mitigate
  org.phase_order: "04"
---

# Phase 04: Mitigate (BEAM runtime diagnosis)

## Phase purpose

Apply the fix or mitigation: code fix (switch to debugging), configuration
change, or operational action.

## Steps to perform

1. If the root cause is a code bug, hand off to the project's language debugging workflow (`workflow-elixir-debugging` / `workflow-gleam-debugging`) (do
   not fix here).
2. If the root cause is configuration: adjust process flags, GC tuning
   (`fullsweep_after`, hibernation), logger level, `message_queue_data`,
   `max_heap_size`.
3. If the root cause is load/capacity: restart the process, scale out, or shed
   load.
4. If applying a code fix, apply the relevant constraints:
   - `constraint-beam-failure` — let-it-crash, correct `trap_exit` / links /
     monitors.
   - `constraint-beam-supervision` — if supervision is touched.
   - `constraint-beam-process-isolation` — if concurrent code is touched.
   - `constraint-beam-nif-safety` — if NIFs are touched.
5. Do NOT restart the node before collecting evidence (already done in phase
   01).
6. Do NOT enable tracing without filtering (already done in phase 03).
7. Do NOT leave `sys` debug on — always `sys:no_debug/1` when done.

## Docs to consult

- `docs/beam/runtime-debugging.md`
- `docs/beam/logger-and-config.md`
- `docs/beam/supervision.md`
- `docs/beam/processes-and-messages.md`

## Operational skills to load

- `beam-observability-debugging`
- `beam-logger-config` (if config change)
- `beam-errors-failures` (if code fix)

## Constraints to apply

- `constraint-beam-failure` (if code fix)
- `constraint-beam-supervision` (if supervision touched)
- `constraint-beam-process-isolation` (if concurrent code touched)
- `constraint-beam-nif-safety` (if NIFs touched)

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set
`next_phase: 05-verify`, `next_workflow: null`, `handoff_requires_hil: false`.
Record the applied mitigation and any constraints applied in `evidence`.
