---
name: workflow-elixir-implementation-00-orchestration
description: |
  Use only to orchestrate the Elixir implementation workflow. Use when adding new
  Elixir code — a new feature, module, function, or first implementation of a
  capability — that requires scoping, design, implementation, testing, and
  verification across multiple phases. Do not use for reviewing a diff (use
  workflow-elixir-code-review), refactoring without behavior change, or fixing a
  defect (use workflow-elixir-debugging).
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: elixir-implementation
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Elixir Implementation (Orchestration)

This skill orchestrates the Elixir implementation workflow. It is the entry point for adding new Elixir code that requires scoping, design, implementation, testing, and verification. Each phase is a separate public skill (Shape A) loaded by exact name; the orchestrator loads them in order and applies the continuation policy between phases.

## Phase routing

The workflow runs five phases in order. Each phase is a public skill at the
same level as this orchestrator.

| Order | Phase skill | Purpose |
| --- | --- | --- |
| 01 | `workflow-elixir-implementation-01-scope` | Understand requirements, read topic docs, record intended behavior and public surface. |
| 02 | `workflow-elixir-implementation-02-design` | Module structure, function shape, error strategy, OTP design decisions. |
| 03 | `workflow-elixir-implementation-03-implement` | Write code following idiomatic Elixir patterns and the error strategy from phase 02. |
| 04 | `workflow-elixir-implementation-04-test` | Write ExUnit tests alongside code, run `mix test`. |
| 05 | `workflow-elixir-implementation-05-verify` | Run full check suite (compile, credo, dialyzer, test, format) and report evidence. |

### Phase skipping conditions

Phases should run in full. Skipping is permitted only under the conditions
below; if a phase is skipped, record the skip and the reason in the handoff.

- `workflow-elixir-implementation-01-scope` — **cannot be skipped.** Capturing
  intended behavior and public surface is required so later phases can be
  checked against it.
- `workflow-elixir-implementation-02-design` — **may be skipped only for trivial
  single-function additions** with no API decisions, no error-strategy choices,
  and no module structure changes. If the addition touches a public API,
  introduces a new process or supervision concern, or changes module layout,
  the phase must run.
- `workflow-elixir-implementation-03-implement` — **cannot be skipped.** This is
  the core code-writing phase.
- `workflow-elixir-implementation-04-test` — **cannot be skipped.** Every
  implementation must ship with tests covering happy path, error branches, and
  edge cases.
- `workflow-elixir-implementation-05-verify` — **cannot be skipped.** The full
  gate suite and documentation completeness are the validation evidence for the
  workflow.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-elixir-implementation-05-verify` returns `outcome: pass` | `workflow-elixir-code-review-00-orchestration` |

This workflow's place in the overall SDLC: see docs/languages.md "Workflows map" for the full graph.

## Handoff format

Each phase returns a handoff in this YAML schema. The orchestrator reads
`outcome`, `blockers`, and `next_phase` to decide whether to continue.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-elixir-style
  - ...
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: 04-test
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

The `workflow-elixir-implementation-05-verify` phase additionally returns the
verification-phase extra fields: `validations_run`, `constraints_checked`,
`evidence`, `failures`, `not_fully_checkable`. See
`workflow-elixir-implementation-05-verify`.

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation policy

The orchestrator uses `conditional-next` between phases. After a phase returns
its handoff, proceed to `next_phase` only when the outcome is `pass` and there
are no blockers; otherwise stop and surface the handoff for human review.

```yaml
continuation_policy: conditional-next
condition: outcome == pass && blockers == []
on_pass: proceed_to_next_phase
on_fail: stop
fallback: stop
```
