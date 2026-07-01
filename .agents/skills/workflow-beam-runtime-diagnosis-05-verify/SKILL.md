---
name: workflow-beam-runtime-diagnosis-05-verify
description: |
  Use only for the verify phase of the BEAM runtime-diagnosis workflow.
  Verify the symptom is resolved; baseline metrics return to normal; run
  validation gates if code changed. Do not use for observing the system,
  isolating the hotspot, diagnosing the cause, or applying mitigation.
allowed-tools: Read Write Edit Bash(erl:*) Bash(rebar3:*)
metadata:
  org.kind: workflow-phase
  org.workflow: beam-runtime-diagnosis
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: Verify (BEAM runtime diagnosis)

## Phase purpose

Verify the symptom is resolved; baseline metrics return to normal; run
validation gates if code changed.

## Steps to perform

1. Re-collect the same baseline metrics from phase 01 (`process_count`,
   `run_queue`, `memory`, `scheduler_wall_time`).
2. Confirm the symptom is resolved: memory stable, run-queue low, mailbox growth
   stopped, process count stable.
3. If a code fix was applied in phase 04, run validation gates:
   - `rebar3 compile`
   - `rebar3 ct` / `rebar3 eunit`
4. Ensure no `sys` debug/trace options left on (`sys:no_debug/1`).
5. Report:
   - symptom (from phase 01);
   - hotspot (from phase 02);
   - root cause (from phase 03);
   - fix/mitigation (from phase 04);
   - before/after metrics;
   - whether the issue requires a code fix (switch to the project's language debugging workflow: `workflow-elixir-debugging` / `workflow-gleam-debugging`).

## Docs to consult

- `docs/beam/runtime-debugging.md`
- `docs/beam/validation.md`

## Operational skills to load

- `beam-observability-debugging`

## Constraints to apply

None.

## Validations to run

BEAM validation is now language-specific; run the project's language validation
workflow if code changed (`rebar3 compile`; `rebar3 ct` / `rebar3 eunit`).

If no code changed, this phase verifies runtime metrics instead.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md), including the
verification-phase extra fields:

- `validations_run` — list of validation skills executed (the project's
  language validation skills, e.g. `validation-elixir-compile` /
  `validation-elixir-test` or the gleam equivalents) or empty if no code
  changed;
- `constraints_checked` — list of constraint skills audited against the diff;
- `evidence` — symptom, hotspot, root cause, fix/mitigation, before/after
  metrics, per-gate pass/fail;
- `failures` — list of gates that failed (empty if all passed);
- `not_fully_checkable` — list of aspects that could not be fully validated and
  why.

Set `next_phase: null`, `next_workflow: null` (terminal). If a code bug was
identified, set `next_workflow: workflow-elixir-debugging` (or
`workflow-gleam-debugging`, per the project language) (the runtime-diagnosis
workflow is complete; the code-level fix is a separate workflow).
