---
name: workflow-beam-runtime-diagnosis-03-diagnose
description: |
  Use only for the diagnose phase of the BEAM runtime-diagnosis workflow.
  Isolate the cause via tracing; determine if it is a code bug (switch to
  debugging) or an operational issue (config, load, resource exhaustion). Do not
  use for observing the system, isolating the hotspot, applying mitigation, or
  final verification.
allowed-tools: Read Write Edit Bash(erl:*) Bash(rebar3:*)
metadata:
  org.kind: workflow-phase
  org.workflow: beam-runtime-diagnosis
  org.phase: diagnose
  org.phase_order: "03"
---

# Phase 03: Diagnose (BEAM runtime diagnosis)

## Phase purpose

Isolate the cause via tracing; determine if it is a code bug (switch to
debugging) or an operational issue (config, load, resource exhaustion).

## Steps to perform

1. Trace the hotspot process with `sys:trace/2` or `:dbg`
   (`dbg:tracer()`, `dbg:tp/2`, `dbg:p/2`) to see what it is doing.
2. Check process flags, links, and monitors.
3. Determine if it is a code bug or an operational issue:
   - If the code is correct but the system is overloaded, the fix may be
     configuration (more schedulers, `message_queue_data`, `max_heap_size`),
     load shedding, or scaling.
   - If the code has a bug (infinite loop, unbounded accumulation), switch to
      the project's language debugging workflow (`workflow-elixir-debugging` / `workflow-gleam-debugging`).
4. Classify the root cause: code bug, configuration issue, resource exhaustion,
   or load.

## Docs to consult

- `docs/beam/runtime-debugging.md`
- `docs/beam/proc-lib-and-sys.md`
- `docs/beam/processes-and-messages.md`
- `docs/beam/logger-and-config.md`
- `docs/beam/supervision.md`

## Operational skills to load

- `beam-observability-debugging`
- `beam-processes`
- `beam-logger-config` (if config issue)

## Constraints to apply

None.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md).

- If operational: set `next_phase: 04-mitigate`, `next_workflow: null`.
- If code bug: set `next_phase: null`, `next_workflow: workflow-elixir-debugging` (or `workflow-gleam-debugging`, per the project language).

Record root cause classification in `evidence`. Because continuation is
`suggest-next`, surface the classification for confirmation before phase 04.
