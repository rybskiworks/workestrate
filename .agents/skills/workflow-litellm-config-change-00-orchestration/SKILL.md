---
name: workflow-litellm-config-change-00-orchestration
description: |
  Use only to orchestrate the LiteLLM config-change workflow. Use when editing
  `infra/litellm/config.yaml` — adding or editing a provider deployment, model
  alias, routing/fallbacks, cache, logging, guardrail, or MCP gateway entry —
  that requires scoping, design, implementation, validation, and verification
  across multiple phases. Do not use for creating a brand-new config from
  scratch, reviewing an existing config, or debugging a running proxy.
allowed-tools: Read Grep
metadata:
  org.kind: workflow-orchestration
  org.workflow: litellm-config-change
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: LiteLLM Config Change (Orchestration)

This skill orchestrates the LiteLLM config-change workflow. It is the PRIMARY
entry point for editing `infra/litellm/config.yaml` for the workestrate
in-memory LiteLLM deployment (no Postgres, no Redis, `master_key`-only auth).
Each phase is a separate public skill (Shape A) loaded by exact name; the
orchestrator loads them in order and applies the continuation policy between
phases. This workflow sets the template that other LiteLLM workflows copy.

## Phase routing

The workflow runs five phases in order. Each phase is a public skill at the
same level as this orchestrator.

| Order | Phase skill | Purpose |
| --- | --- | --- |
| 01 | `workflow-litellm-config-change-01-scope` | Capture the intended change, target top-level section, and affected `model_name` aliases; read `docs/litellm/config/config-yaml-overview.md` + load `litellm-config-anatomy`. |
| 02 | `workflow-litellm-config-change-02-design` | Decide exact keys/section, provider prefix + model string, `api_base` rules, env vars, and fallback wiring. |
| 03 | `workflow-litellm-config-change-03-implement` | Edit `infra/litellm/config.yaml`; load the change-type regular skill; preserve `os.environ/` secret hygiene and in-memory constraints. |
| 04 | `workflow-litellm-config-change-04-validate` | Run `validation-litellm-config-check` (the 9 schema checks; optionally the co-located `scripts/check_config.py`). |
| 05 | `workflow-litellm-config-change-05-verify` | Run `validation-litellm-startup`, `validation-litellm-smoke`, and `validation-litellm-no-hardcoded-secrets`; return the verification handoff. |

### Phase skipping conditions

Phases should run in full. Skipping is permitted only under the conditions
below; if a phase is skipped, record the skip and the reason in the handoff.

- `workflow-litellm-config-change-01-scope` — **cannot be skipped.** Capturing
  the intended change, target section, and affected aliases is required so
  later phases can be checked against it.
- `workflow-litellm-config-change-02-design` — **may be skipped only for
  trivial single-key edits** with no provider prefix decision, no `api_base`
  rule, no new env var, and no fallback wiring. If the change touches a
  provider prefix, `api_base`, an env var, or `router_settings.fallbacks`, the
  phase must run.
- `workflow-litellm-config-change-03-implement` — **cannot be skipped.** This
  is the core config-editing phase.
- `workflow-litellm-config-change-04-validate` — **cannot be skipped.** The
  schema checks are the static validation gate before any proxy startup.
- `workflow-litellm-config-change-05-verify` — **cannot be skipped.** Startup
  + smoke + secret scan are the runtime validation evidence for the workflow.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-litellm-config-change-05-verify` returns `outcome: pass` | suggest `workflow-litellm-validation-00-orchestration` OR stop |

After `05-verify` passes, the config change is complete and verified. Running
`workflow-litellm-validation-00-orchestration` is optional — use it for a
full periodic re-validation gate; otherwise stop.

## Handoff format

Each phase returns a handoff in this YAML schema. The orchestrator reads
`outcome`, `blockers`, and `next_phase` to decide whether to continue.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: infra/litellm/config.yaml
    change: added coding.pro-fallback deployment
constraints_applied:
  - constraint-litellm-config-schema
  - ...
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: 04-validate
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

The `workflow-litellm-config-change-05-verify` phase additionally returns the
verification-phase extra fields: `validations_run`, `constraints_checked`,
`evidence`, `failures`, `not_fully_checkable`. See
`workflow-litellm-config-change-05-verify`.

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
