---
name: workflow-gleam-debugging-01-reproduce
description: |
  Use only for the reproduce phase of the Gleam debugging workflow.
  Reproduce the issue reliably and capture the exact command, input,
  environment, target, and full error or crash output. Do not use for diagnosis,
  fixing, regression testing, or final verification.
allowed-tools: Read Write Edit Bash(gleam:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: gleam-debugging
  org.phase: reproduce
  org.phase_order: "01"
---

## Phase purpose

Reproduce the issue reliably and capture the exact command, input, environment, target, and full error or crash output. This phase is reproduction only — do not begin diagnosis or fixing here.

## Steps to perform

1. Reproduce the issue reliably. Capture:
   - the exact command that triggers the defect (e.g. `gleam check`, `gleam test`, `gleam test --target erlang`, `gleam run -m <module>`);
   - the input that triggers it;
   - the environment (Gleam version, target (`erlang` or `javascript`), `gleam.toml` settings, working directory);
   - the full error message or crash output (compiler error code and message, `panic`/`let assert` stack trace, wrong-output observation, or actor exit reason).
2. Identify the target. The debugging approach differs by target:
   - **Erlang target:** runtime defects use BEAM tooling (`:observer`, `:dbg`, `:sys.get_state`, Erlang shell tracing). BEAM docs apply.
   - **JavaScript target:** use Node/browser debugging tooling (inspector, breakpoints, `console.log`); BEAM docs and `:observer` do NOT apply. Concurrency is `gleam/javascript/promise`, not BEAM processes/OTP.
3. If the issue cannot be reproduced, record what is known and what reproduction attempts were made. Do NOT proceed to a fix on an unreproducible report. Set `outcome: fail` and `blockers: ["issue not reproduced"]` in the handoff.
4. Apply scope discipline: this phase is reproduction only. Do not start fixing adjacent issues observed while reproducing; record them as follow-ups.

## Docs to consult

- `docs/gleam/result-option-and-errors.md`
- `docs/gleam/testing.md`

## Operational skills to load

None mandatory in this phase.

## Constraints to apply

- Scope discipline — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with catch-all `_` patterns, `panic`/`let assert` renames, or target-workarounds. This phase is reproduction only.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 02-diagnose`, `next_workflow: null`, `handoff_requires_hil: false`. Record the reproduction command and the original error/crash output in `evidence` (or `assumptions` if the reproduction is partial). Record the target (`erlang` or `javascript`) in `evidence`. If reproduction failed, set `outcome: fail`, `blockers: ["issue not reproduced"]`, and stop.
