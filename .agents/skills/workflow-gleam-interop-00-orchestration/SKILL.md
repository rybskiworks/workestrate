---
name: workflow-gleam-interop-00-orchestration
description: |
  Use when adding Gleam↔Erlang or Gleam↔JavaScript interop/FFI — externals,
  gleam_erlang, gleam_javascript, typed FFI boundaries. Do NOT use for ordinary
  Gleam implementation.
allowed-tools: Read Write Edit Bash(gleam:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: gleam-interop
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Gleam Interop (Orchestration)

This skill orchestrates the Gleam interop workflow. It is the entry point for
adding Gleam↔Erlang or Gleam↔JavaScript interop and FFI boundaries — external
functions, external types, `gleam_erlang` / `gleam_javascript` typed wrappers,
and target-specific modules. Each phase is a separate public skill (Shape A)
loaded by exact name; the orchestrator loads them in order and applies the
continuation policy between phases.

## Phase routing

The workflow runs five phases in order. Each phase is a public skill at the
same level as this orchestrator.

| Order | Phase skill | Purpose |
| --- | --- | --- |
| 01 | `workflow-gleam-interop-01-scope` | Identify the interop surface: target (Erlang/JavaScript/both), external functions/types needed, boundary shape. |
| 02 | `workflow-gleam-interop-02-design` | Design the FFI boundary: typed wrappers, `Dynamic` decoding at the edge, opaque external types, type safety. |
| 03 | `workflow-gleam-interop-03-implement` | Write `@external` attributes, target-specific modules, `gleam_erlang` / `gleam_javascript` wrappers. |
| 04 | `workflow-gleam-interop-04-test` | Target-specific tests and boundary round-trips. |
| 05 | `workflow-gleam-interop-05-verify` | `gleam check`, `gleam format --check`, confirm boundary safety; report evidence. |

## Per-phase constraint and validation ownership

Each phase owns its own constraints and validations. The orchestrator does not
enumerate them here — consult each phase skill for the constraints it applies
and the validations it runs. Scope discipline (stay within the requirements
captured in phase 01) applies to all phases.

## Workflow chaining

When phase 05-verify returns `outcome == pass`, chain to
`workflow-gleam-code-review`. Interop and FFI code is always reviewed — the
compiler cannot verify that foreign functions exist or return their annotated
types, so human review of the boundary is mandatory. Run the full validation
suite after review completes.

```yaml
chaining_policy: conditional-next-workflow
condition: phase_05_verify.outcome == pass && blockers == []
on_pass: chain_to workflow-gleam-code-review
on_fail: stop
```

## Handoff format

Each phase returns a handoff in this YAML schema. The orchestrator reads
`outcome`, `blockers`, and `next_phase` to decide whether to continue.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
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

The `workflow-gleam-interop-05-verify` phase additionally returns the
verification-phase extra fields: `validations_run`, `constraints_checked`,
`evidence`, `failures`, `not_fully_checkable`. See
`workflow-gleam-interop-05-verify`.

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
