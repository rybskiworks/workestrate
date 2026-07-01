---
name: workflow-litellm-config-change-01-scope
description: |
  Use only for the scope phase of the LiteLLM config-change workflow. Capture
  the intended change (add/edit provider, model alias, routing/fallbacks,
  cache, logging, guardrail, or MCP gateway entry), the target top-level
  section, and the affected `model_name` aliases. Do not use for design
  decisions, implementation, validation, or verification.
allowed-tools: Read Grep
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-change
  org.phase: scope
  org.phase_order: "01"
---

# Workflow: LiteLLM Config Change — 01 Scope

Capture the intended config change so later phases can be checked against it.
This phase is read-only: it reads docs and the current config; it does not edit.

## Required source files to read first

- `docs/litellm/config/config-yaml-overview.md` — the 7 top-level sections, precedence rule, env-var syntax, in-memory mapping.
- `infra/litellm/config.yaml` — the real, working in-memory config (canonical example).
- Load regular skill `litellm-config-anatomy` for the `model_list` / `litellm_params` / `model_info` structure.

## Required inputs

- The change type (one of): add/edit provider deployment, model alias, routing/fallbacks, cache, logging/observability, guardrail, MCP gateway entry.
- The target top-level section (`model_list`, `general_settings`, `router_settings`, `litellm_settings`, `callback_settings`, `environment_variables`, `credential_list`).
- The affected `model_name` aliases (for `model_list`/fallback changes).

## Exact procedure

1. **Classify the change type.** Record which of the change types above applies. The change type determines which regular skill `03-implement` loads.
2. **Record the target top-level section.** Per `config-yaml-overview.md`, the valid top-level keys are: `environment_variables`, `model_list`, `litellm_settings`, `callback_settings`, `general_settings`, `router_settings`, plus `credential_list` and `include`. Do not invent any other top-level key. The real config uses 4: `model_list`, `general_settings`, `router_settings`, `litellm_settings`.
3. **Record affected `model_name` aliases.** For `model_list`/fallback changes, list every alias touched (e.g. `coding`, `coding-fallback`). Note that the `model` field in a client request maps to the `model_name` alias, NOT to `litellm_params.model`.
4. **Confirm deployment mode.** The workestrator deployment is in-memory (no Postgres, no Redis, `master_key`-only auth). Record this assumption — it activates the in-memory constraints in later phases.
5. **Read the current config.** Read `infra/litellm/config.yaml` and record the current state of the target section so the diff is unambiguous.

## Constraints to apply

- `constraint-litellm-config-schema` — every key cited must trace to `docs/litellm/schemas/config-yaml.option-index.json` (check `key` + `section`). Do not assert a key exists without this trace.

## Validations to run

None (read-only phase). Static validation runs in `04-validate`; runtime validation in `05-verify`.

## Handoff

Return the handoff YAML. Set `next_phase: 02-design`.

```yaml
outcome: pass|fail|partial
files_touched: []
constraints_applied:
  - constraint-litellm-config-schema
assumptions:
  - deployment is in-memory (no Postgres, no Redis)
risks: []
tests_run: []
tests_needed:
  - validation-litellm-config-check (04-validate)
  - validation-litellm-startup (05-verify)
next_phase: 02-design
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```

## Anti-hallucination

Every top-level key and `model_name` alias cited must trace to
`docs/litellm/config/config-yaml-overview.md` or `infra/litellm/config.yaml`.
Mark any inferred fact.
