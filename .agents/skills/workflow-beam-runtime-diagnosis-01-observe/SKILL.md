---
name: workflow-beam-runtime-diagnosis-01-observe
description: |
  Use only for the observe phase of the BEAM runtime-diagnosis workflow.
  Preserve evidence and gather baseline metrics before changing anything. If the
  VM crashed, preserve erl_crash.dump. If running, collect baseline metrics and
  classify the symptom. Do not use for isolating the hotspot, diagnosing the
  cause, applying mitigation, or final verification.
allowed-tools: Read Write Edit Bash(erl:*) Bash(rebar3:*)
metadata:
  org.kind: workflow-phase
  org.workflow: beam-runtime-diagnosis
  org.phase: observe
  org.phase_order: "01"
---

# Phase 01: Observe (BEAM runtime diagnosis)

## Phase purpose

Preserve evidence and gather baseline metrics before changing anything. If the
VM crashed, preserve `erl_crash.dump`. If running, collect baseline metrics and
classify the symptom.

## Steps to perform

1. Determine whether the system is still running, partially degraded, or fully
crashed (VM dead).
2. If the VM crashed: locate and preserve `erl_crash.dump` before anything else
— it contains the full state at crash time.
3. If the system is running but degraded: connect via Erlang shell
(`erl -sname ... -remsh node@host`) or attach to the running node. Do NOT
restart the node until diagnostic data is collected.
4. Gather baseline metrics:
   - `erlang:system_info(process_count)`
   - `erlang:statistics(run_queue)`
   - `erlang:memory()`
   - `erlang:system_flag(scheduler_wall_time, true)` (enable before sampling)
5. Classify the symptom: memory growth, scheduler saturation, mailbox growth,
process leak, GC pressure, crash, or log flooding.

## Docs to consult

- `docs/beam/runtime-debugging.md`
- `docs/beam/proc-lib-and-sys.md`
- `docs/beam/processes-and-messages.md`
- `docs/beam/logger-and-config.md`

## Operational skills to load

- `beam-observability-debugging` (primary)
- `beam-processes`
- `beam-logger-config` (if log flooding)

## Constraints to apply

None.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set
`next_phase: 02-isolate`, `next_workflow: null`, `handoff_requires_hil: false`.
Record baseline metrics and symptom classification in `evidence`.
