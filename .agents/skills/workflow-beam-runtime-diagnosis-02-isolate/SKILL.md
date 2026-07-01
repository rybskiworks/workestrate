---
name: workflow-beam-runtime-diagnosis-02-isolate
description: |
  Use only for the isolate phase of the BEAM runtime-diagnosis workflow.
  Identify the hotspot: which process(es) or resource(s) are the bottleneck.
  Use reduction counting, message_queue_len, heap_size, scheduler_wall_time,
  and crash-dump analysis. Do not use for observing the system, diagnosing the
  cause, applying mitigation, or final verification.
allowed-tools: Read Write Edit Bash(erl:*) Bash(rebar3:*)
metadata:
  org.kind: workflow-phase
  org.workflow: beam-runtime-diagnosis
  org.phase: isolate
  org.phase_order: "02"
---

# Phase 02: Isolate (BEAM runtime diagnosis)

## Phase purpose

Identify the hotspot — which process(es) or resource(s) are the bottleneck.

## Steps to perform

1. Find the busiest processes by reductions (hot loop):
   ```erlang
   [{P, erlang:process_info(P, reductions)} || P <- erlang:processes()].
   ```
2. Find processes with high `message_queue_len` (slow consumer):
   ```erlang
   [{P, erlang:process_info(P, message_queue_len)} || P <- erlang:processes()].
   ```
3. Find processes with large `heap_size` / `total_heap_size` (memory leak).
4. Sample `scheduler_wall_time` twice (1 s apart) for scheduler saturation.
5. Inspect specific processes:
   ```erlang
   erlang:process_info(Pid, [status, current_function, heap_size, total_heap_size, message_queue_len, garbage_collection]).
   ```
6. If the VM crashed, analyze `erl_crash.dump` with
   `crashdump_viewer:start()`.
7. Inspect ETS tables if relevant: `ets:info/1,2`, `ets:i/0`.

## Docs to consult

- `docs/beam/runtime-debugging.md`
- `docs/beam/processes-and-messages.md`
- `docs/beam/ets-data.md`
- `docs/beam/distribution.md`
- `docs/beam/nifs.md`
- `docs/beam/timers.md`

## Operational skills to load

- `beam-observability-debugging`
- `beam-processes`

## Constraints to apply

None.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set
`next_phase: 03-diagnose`, `next_workflow: null`, `handoff_requires_hil: false`.
Record the identified hotspot(s) in `evidence`.
