---
name: workflow-litellm-validation-00-orchestration
description: |
  Use only to orchestrate the LiteLLM validation workflow. Use when running the
  full validation gate suite (config-check, startup, smoke, report) as the final
  gate before deploy/release or as a periodic config health gate. This workflow
  IS the validation — it runs all gates and reports; it does NOT modify config.
  Do not use as a substitute for implementation, refactoring, debugging, or
  code-review workflows; run it after one of those has produced config. Referenced
  by the other LiteLLM workflows' verify phases.
allowed-tools: Read Bash(git:*) Bash(litellm:*) Bash(curl:*) Bash(python:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: litellm-validation
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: LiteLLM Validation (Orchestration)

Run the full validation gate suite on LiteLLM `config.yaml` and the running
proxy, and report pass/fail per gate with evidence. This workflow IS the
validation — it runs all gates and reports. It does NOT modify config. If a gate
fails, hand off to the appropriate fixing workflow; do not fix in the validation
pass.

## Phase routing

Phases are flat public phase skills loaded by exact name. Run them in order:

| Order | Phase Skill | Purpose |
|-------|-------------|---------|
| 01 | `workflow-litellm-validation-01-scope` | Identify what changed (git diff); determine which gates apply (in-memory vs DB-backed; KVM available for smoke?). Record applicable vs not-applicable gates. |
| 02 | `workflow-litellm-validation-02-config-check` | Run `validation-litellm-config-check` (the checks a–i; the co-located script `.agents/skills/validation-litellm-config-check/scripts/check_config.py`). Record pass/fail + evidence per check. |
| 03 | `workflow-litellm-validation-03-startup` | Run `validation-litellm-startup` (proxy boots; "Loaded config YAML (api_key and environment_variables are not shown)"). Record pass/fail + evidence. |
| 04 | `workflow-litellm-validation-04-smoke` | Run `validation-litellm-smoke` (GET `/v1/models`, `/health/liveliness`, POST `/v1/chat/completions` with `model=<alias>`). Requires KVM; mark `not_fully_checkable` if absent. Record pass/fail + evidence. |
| 05 | `workflow-litellm-validation-05-report` | Aggregate all gate results into a per-gate table; report overall verdict; hand off to fixing workflow if any applicable gate is red. |

### Phase skipping conditions

- **01-scope**: Never skip. Determines which gates are applicable vs not-applicable (in-memory vs DB-backed; KVM availability for smoke).
- **02-config-check**: Never skip. The config-check gate runs against the on-disk corpus and config; it does not require a running proxy.
- **03-startup**: Run if phase 02 is green (a config that fails config-check will also fail to start). If phase 02 is red, still attempt startup to capture the Pydantic `ValidationError` traceback as evidence, but the overall result is already fail.
- **04-smoke**: Run only if KVM is available (determined in phase 01). If KVM is absent, mark the smoke gate `not_fully_checkable` and skip the live calls; the overall verdict reflects this gap. If phase 03 is red, skip smoke (proxy not running) and mark `not_fully_checkable`.
- **05-report**: Never skip. Produces the final verdict and handoff.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each
phase skill's "Constraints to apply" and "Validations to run" sections. **This
orchestrator does not re-enumerate them.** (Constraints/validations are owned by
phases; the orchestrator only routes and chains.)

This workflow CHECKS all 8 constraints via the config-check gate (phase 02):
`constraint-litellm-config-schema`, `constraint-litellm-in-memory-no-db`,
`constraint-litellm-secret-hygiene`, `constraint-litellm-provider-prefix`,
`constraint-litellm-openai-compatible-api-base`,
`constraint-litellm-anthropic-suffix`, `constraint-litellm-fallback-resolution`,
`constraint-litellm-deprecation-free`.

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator
hands off to the next workflow per the SDLC chain. The terminal-phase handoff's
`next_workflow` field is `null` (end of *this* workflow); the chaining decision
below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-litellm-validation-05-report` returns `outcome: pass` (all applicable gates green) | `stop` |

This workflow is referenced by the other LiteLLM workflows' verify phases; when
invoked as a verify sub-step, the calling workflow consumes the report-phase
handoff's `validations_run`, `constraints_checked`, `evidence`, `failures`, and
`not_fully_checkable` fields.

## Handoff format

Each phase returns this YAML handoff:

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-litellm-config-schema
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: 02-config-check
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

Phase 05-report additionally returns the verify-phase extra handoff fields:

```yaml
validations_run:
  - validation-litellm-config-check
  - validation-litellm-startup
  - validation-litellm-smoke
  - validation-litellm-no-hardcoded-secrets
constraints_checked:
  - constraint-litellm-config-schema
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-secret-hygiene
  - constraint-litellm-provider-prefix
  - constraint-litellm-openai-compatible-api-base
  - constraint-litellm-anthropic-suffix
  - constraint-litellm-fallback-resolution
  - constraint-litellm-deprecation-free
evidence:
  - ...
failures:
  - ...
not_fully_checkable:
  - ...
```

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is
decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation policy

**stop**: after the report (phase 05), no further action. If any applicable
gate failed, hand off to the appropriate fixing workflow — do NOT fix in the
validation pass:

- Config defect → LiteLLM implementation/harden workflow
- Runtime/startup defect → LiteLLM debugging workflow

Then re-run validation after the fix.

```yaml
continuation: stop
next_phase: null
fallback: stop
```

## Docs Consulted

- `docs/litellm/config/config-validation.md` (source validation prose)
- `docs/litellm/schemas/config-yaml.normalized.schema.md` (authoritative key reference)
- `docs/litellm/schemas/config-yaml.option-index.json` (authoritative key index)
- `docs/litellm/schemas/endpoints.index.json` (smoke endpoints)
- `docs/litellm/validation-report.md` (KVM-gating precedent for smoke)
