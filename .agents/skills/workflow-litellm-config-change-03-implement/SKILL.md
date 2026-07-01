---
name: workflow-litellm-config-change-03-implement
description: |
  Use only for the implement phase of the LiteLLM config-change workflow. Edit
  `infra/litellm/config.yaml` following the design from 02-design, loading the
  change-type regular skill and preserving `os.environ/` secret hygiene and
  in-memory constraints. Do not use for scoping, design, validation, or
  verification.
allowed-tools: Read Write Edit Bash
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-change
  org.phase: implement
  org.phase_order: "03"
---

# Workflow: LiteLLM Config Change — 03 Implement

Edit `infra/litellm/config.yaml` following the design from `02-design`. This is
the only phase that modifies files.

## Required source files to read first

- `infra/litellm/config.yaml` — the file being edited.
- `docs/litellm/schemas/config-yaml.option-index.json` — confirm each key's `section`, `requires_db`, `requires_redis`, `deprecated`.
- Load the change-type regular skill (by change type):
  - add/edit provider deployment → `litellm-providers` (or `litellm-openai-compatible` for OpenAI-compatible endpoints)
  - routing/fallbacks → `litellm-routing-fallbacks`
  - cache → `litellm-caching`
  - logging/observability → `litellm-logging-observability`
  - guardrail → `litellm-guardrails`
  - MCP gateway entry → `litellm-gateway-mcp`

## Exact procedure

1. **Apply the design.** Edit only the keys/section decided in `02-design`. Stay within the assigned scope; do not refactor unrelated sections.
2. **Preserve `os.environ/` secret hygiene.** Every `api_key`, `master_key`, and secret-bearing value MUST use `os.environ/<VAR>`. NEVER hardcode a secret literal. `master_key` MUST resolve to `os.environ/LITELLM_MASTER_KEY`.
3. **Enforce in-memory constraints.** The workestrator deployment runs LiteLLM in-memory in a microsandbox (no Postgres, no Redis):
   - NO `database_url` (requires_db=true).
   - NO `redis_*` keys (`redis_host`, `redis_password`, `redis_port`, `redis_db`, `redis_url`, `enable_redis_auth_cache`, `use_redis_transaction_buffer`) — all requires_redis=true.
   - NO virtual-key/team/user/budget keys (`store_model_in_db`, `custom_key_generate`, `key_generation_settings`, `default_key_generate_params`, `upperbound_key_generate_params`, `max_budget`, `budget_duration`, `default_team_params`, `prometheus_initialize_budget_metrics`) — all requires_db=true.
   - Keep `general_settings.disable_spend_logs: true` (compensating control for the absent DB; marked requires_db=true in the schema because it suppresses DB spend writes — its presence is intentional, not a violation).
   - Auth = `master_key` only. No virtual keys, teams, or users.
   - For cache, only `cache_params.type` values `local`/`disk`/`s3`/`gcs` are in-memory-safe (no `redis`/`redis-semantic`/`valkey-semantic`/`qdrant-semantic`).
4. **Avoid deprecated keys.** Do not use `set_verbose` (→ `LITELLM_LOG` / `--debug`), `LITELLM_SET_VERBOSE` / `SET_VERBOSE` env vars (→ `LITELLM_LOG`), or `disable_copilot_system_to_assistant`. Use `LITELLM_LOG` env var or `litellm_settings.json_logs` instead.
5. **Use correct sub-key enums.** `retry_policy` sub-keys (all int): `AuthenticationErrorRetries`, `TimeoutErrorRetries`, `RateLimitErrorRetries`, `ContentPolicyViolationErrorRetries`, `InternalServerErrorRetries`. `routing_strategy` enum: `simple-shuffle` (default), `least-busy`, `usage-based-routing`, `latency-based-routing`.
6. **Verify fallback resolution.** Every `router_settings.fallbacks` source and target must exist as a `model_name` in `model_list`. A dangling fallback target makes the proxy 500 on failover.

## Constraints to apply

- `constraint-litellm-config-schema` — every key traces to `config-yaml.option-index.json`.
- `constraint-litellm-in-memory-no-db` — no requires_db=true key (except `disable_spend_logs: true`) and no requires_redis=true key.
- `constraint-litellm-secret-hygiene` — all secrets use `os.environ/<VAR>`; no hardcoded literals.
- `constraint-litellm-fallback-resolution` — every fallback source/target exists in `model_list`.
- `constraint-litellm-deprecation-free` — no deprecated key (`set_verbose`, etc.).
- (Provider-prefix / api_base / anthropic-suffix constraints were satisfied in `02-design`; re-confirm if the edit diverges from the design.)

## Validations to run

None in this phase. Static validation runs in `04-validate`.

## Handoff

Return the handoff YAML. Set `next_phase: 04-validate`.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: infra/litellm/config.yaml
    change: <one-line description of the edit>
constraints_applied:
  - constraint-litellm-config-schema
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-secret-hygiene
  - constraint-litellm-fallback-resolution
  - constraint-litellm-deprecation-free
assumptions: []
risks: []
tests_run: []
tests_needed:
  - validation-litellm-config-check (04-validate)
next_phase: 04-validate
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```

## Anti-hallucination

Before adding any config key, confirm it exists in `config-yaml.option-index.json` (check `key` + `section`). If absent, do not invent it — mark TODO and cite the closest corpus file.
