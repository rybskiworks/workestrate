---
name: workflow-elixir-debugging-01-reproduce
description: |
  Use only for the reproduce phase of the Elixir debugging workflow.
  Reproduce the issue reliably and capture the exact command, input,
  environment, and full error/crash output. Do not use for diagnosis,
  fixing, regression testing, or final verification.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: elixir-debugging
  org.phase: reproduce
  org.phase_order: "01"
---

## Phase purpose

Reproduce the issue reliably and capture the exact command, input, environment, and full error or crash output. This phase is reproduction only — do not begin diagnosis or fixing here.

## Steps to perform

1. Reproduce the issue reliably. Capture:
   - the exact command that triggers the defect;
   - the input that triggers it;
   - the environment (Elixir/Erlang version, `MIX_ENV`, config, working directory);
   - the full error message or crash output (compiler warning/error, stack trace, exit reason, crash report, SASL report, or wrong-output observation).
2. If the issue cannot be reproduced, record what is known and what reproduction attempts were made. Do NOT proceed to a fix on an unreproducible report. Set `outcome: fail` and `blockers: ["issue not reproduced"]` in the handoff.
3. Apply `constraint-elixir-style`: this phase is reproduction only. Do not start fixing adjacent issues observed while reproducing; record them as follow-ups.

## Docs to consult

None mandatory in this phase. Docs are loaded in phase `02-diagnose` based on the defect category.

## Operational skills to load

None mandatory in this phase.

## Constraints to apply

- `constraint-elixir-style` — Fix only the reported defect; do not refactor surrounding code; do not add features; record adjacent issues as follow-ups; do not suppress with `# credo:disable-for-next-line` or catching exceptions to silence them. This phase is reproduction only.

## Validations to run

None.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 02-diagnose`, `next_workflow: null`, `handoff_requires_hil: false`. Record the reproduction command and the original error/crash output in `evidence` (or `assumptions` if the reproduction is partial). If reproduction failed, set `outcome: fail`, `blockers: ["issue not reproduced"]`, and stop.
