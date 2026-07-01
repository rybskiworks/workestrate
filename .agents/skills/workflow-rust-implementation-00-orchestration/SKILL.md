---
name: workflow-rust-implementation-00-orchestration
description: |
  Use only to orchestrate the Rust implementation workflow. Use when adding new
  Rust code — a new feature, module, crate, public function/trait, or first
  implementation of a capability — that requires scoping, design, implementation,
  testing, and verification across multiple phases. Do not use for reviewing a
  diff (use workflow-rust-code-review), refactoring without behavior change, or
  fixing a defect (use workflow-rust-debugging).
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: rust-implementation
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Rust Implementation (Orchestration)

This skill orchestrates the Rust implementation workflow. It is the entry point
for adding new Rust code that requires scoping, design, implementation, testing,
and verification. Each phase is a separate public skill (Shape A) loaded by
exact name; the orchestrator loads them in order and applies the continuation
policy between phases.

## Phase routing

The workflow runs five phases in order. Each phase is a public skill at the
same level as this orchestrator.

| Order | Phase skill | Purpose |
| --- | --- | --- |
| 01 | `workflow-rust-implementation-01-scope` | Understand requirements, read topic docs, record intended behavior and public surface. |
| 02 | `workflow-rust-implementation-02-design` | API design, error type design, module structure decisions. |
| 03 | `workflow-rust-implementation-03-implement` | Write code following ownership/borrowing rules. |
| 04 | `workflow-rust-implementation-04-test` | Write tests alongside code, run `cargo test`. |
| 05 | `workflow-rust-implementation-05-verify` | Run full check suite (check, clippy, test, fmt, doc) and report evidence. |

### Phase skipping conditions

Phases should run in full. Skipping is permitted only under the conditions
below; if a phase is skipped, record the skip and the reason in the handoff.

- `workflow-rust-implementation-01-scope` — **cannot be skipped.** Capturing
  intended behavior and public surface is required so later phases can be
  checked against it.
- `workflow-rust-implementation-02-design` — **may be skipped only for trivial
  single-function additions** with no API decisions, no new error types, and no
  module structure changes. If the addition touches a public API, introduces an
  error type, or changes module layout, the phase must run.
- `workflow-rust-implementation-03-implement` — **cannot be skipped.** This is
  the core code-writing phase.
- `workflow-rust-implementation-04-test` — **cannot be skipped.** Every
  implementation must ship with tests covering happy path, error branches, and
  edge cases.
- `workflow-rust-implementation-05-verify` — **cannot be skipped.** The full
  gate suite and documentation build are the validation evidence for the
  workflow.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-rust-implementation-05-verify` returns `outcome: pass` | `workflow-rust-code-review-00-orchestration` |

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
  - constraint-rust-ownership
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

The `workflow-rust-implementation-05-verify` phase additionally returns the
verification-phase extra fields: `validations_run`, `constraints_checked`,
`evidence`, `failures`, `not_fully_checkable`. See
`workflow-rust-implementation-05-verify`.

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
