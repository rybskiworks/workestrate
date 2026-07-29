# `config.yaml` Reference

This section documents the LiteLLM Proxy `config.yaml` file — the top-level YAML
structure, the `model_list` anatomy, and per-section references.

## Documents

| Document | What's there | Status |
| --- | --- | --- |
| [config-yaml-overview.md](config-yaml-overview.md) | Top-level YAML structure: the 7 top-level keys, precedence rule, env-var resolution, config validation | exists |
| [model-list.md](model-list.md) | `model_list` anatomy: `model_name` + `litellm_params` + `model_info`, load balancing, all 18 verified `litellm_params` keys, `credential_list` | exists |
| `litellm-settings.md` | Module-level SDK settings (`drop_params`, `request_timeout`, `force_ipv4`, callbacks, caching) | exists |
| `router-settings.md` | Routing/reliability (`routing_strategy`, `fallbacks`, `num_retries`, `timeout`, `retry_policy`, `cooldown_time`) | exists |
| `general-settings.md` | Proxy-level settings (`master_key`, `completion_model`, `database_url`, `disable_spend_logs`, health checks) | exists |
| `environment-variables.md` | `environment_variables` injection via config | exists |
| `config-validation.md` | Pydantic schema validation at startup | exists |
| [config-management.md](config-management.md) | `include` / split-config: merging external YAML files, hot-reload behavior, child-file shape | exists |

## Schema indexes

- [schemas/config-yaml.option-index.json](../schemas/config-yaml.option-index.json) — every config key with `source_urls`, `type`, `default`, `requires_db`, `requires_redis`, `enterprise`, `deprecated`, `workestrate_recommendation`.
- [schemas/config-yaml.normalized.schema.md](../schemas/config-yaml.normalized.schema.md) — normalized human-readable schema reference (includes "Workestrate in-memory deployment mapping").

## Sources

- https://docs.litellm.ai/docs/proxy/config_settings (PRIMARY — top-level YAML schema + Caveats)
- https://docs.litellm.ai/docs/proxy/configs (Overview — model_list, providers, credentials)
- Local: `extracted/p0-config_settings.md`, `extracted/p0-configs.md`

## Workestrate notes

[PROJECT CONTEXT — NOT upstream docs]

The real config at `infra/litellm/config.yaml` uses **4 of the 7** top-level
sections:

| Section | Used? | Notes |
| --- | --- | --- |
| `model_list` | yes | 9 aliases across coding tiers + `neural` |
| `general_settings` | yes | `master_key`, `completion_model: coding`, `disable_spend_logs: true` |
| `router_settings` | yes | `fallbacks`, `num_retries: 2`, `timeout: 300`, `stream_timeout: 300`, `allowed_fails: 3`, `cooldown_time: 60`, `retry_policy` |
| `litellm_settings` | yes | `drop_params: true`, `request_timeout: 300`, `force_ipv4: true` |
| `environment_variables` | no | Secrets injected via sandbox `env()`/`secret_env()`, not via config |
| `credential_list` | no | Not used; keys are inline `os.environ/` references |
| `callback_settings` | no | No OTEL/callback config |

The config is mounted **read-only** at `/app/config.yaml` and started with
`--config /app/config.yaml --host 0.0.0.0`. See [00-index.md](../00-index.md) →
Workestrate notes for the full in-memory constraints.
