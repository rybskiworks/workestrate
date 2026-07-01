---
name: litellm-config-anatomy
description: |
  Operational reference for the LiteLLM Proxy `config.yaml` anatomy: the
  top-level sections, `model_list` sub-keys, the `router_settings`-overrides-
  `litellm_settings` precedence rule, `os.environ/X` resolution, and startup
  Pydantic validation. Load when authoring, reviewing, or debugging a
  `config.yaml`, adding a `model_list` entry, or diagnosing a startup
  validation failure. Does NOT cover provider prefix selection (see
  litellm-providers), routing/fallback semantics (see litellm-routing-
  fallbacks), or deployment/runtime flags (see litellm-deployment). Full
  detail lives in docs/litellm/config/*.md.
---

# LiteLLM Config Anatomy

Distilled operational reference for the LiteLLM Proxy `config.yaml` file.
The proxy validates `config.yaml` against an internal **Pydantic schema at
startup**; on invalid config it **fails to start**. Full detail lives in:

- `docs/litellm/config/config-yaml-overview.md` — top-level structure,
  precedence rule, env-var resolution, defaults.
- `docs/litellm/config/model-list.md` — `model_list` anatomy, all 18
  verified `litellm_params` keys, `model_info`, `credential_list`.
- `docs/litellm/config/config-validation.md` — startup validation behavior.
- `docs/litellm/config/config-management.md` — `include` / split-config.
- `docs/litellm/schemas/config-yaml.option-index.json` — every config key
  with `source_urls`, `type`, `default`, `requires_db`, `requires_redis`,
  `enterprise`, `deprecated`, `workestrator_recommendation`.

> **Do not hallucinate.** Every key/default below traces to
> `docs/litellm/config/*.md` or `docs/litellm/schemas/config-yaml.option-index.json`.
> Inferred items are marked **(inferred)**.

## Triggers

Load this skill when:

- Authoring or reviewing a `config.yaml` (top-level section placement).
- Adding a `model_list` entry (`model_name` + `litellm_params` + `model_info`).
- Diagnosing a startup `ValidationError` or "Loaded config YAML" log.
- Deciding whether a key belongs under `litellm_settings` vs `router_settings`.
- Resolving `os.environ/<VAR>` indirection or empty-resolution failures.

## Top-level sections

| Key | Role | DB? | Redis? |
|-----|------|-----|--------|
| `environment_variables` | Inject env vars via config (dict). | no | no |
| `model_list` | List of deployments: `model_name` alias + `litellm_params` + optional `model_info`. | no | no |
| `litellm_settings` | Module-level SDK settings (`drop_params`, `request_timeout`, `force_ipv4`, callbacks, caching). | some | some |
| `callback_settings` | Callback-specific config (e.g. `otel.message_logging`). | no | no |
| `general_settings` | Proxy-level (`master_key`, `completion_model`, `database_url`, alerting, health, `disable_spend_logs`). | some | no |
| `router_settings` | Routing/reliability (`routing_strategy`, `fallbacks`, `num_retries`, `timeout`, `retry_policy`, `cooldown_time`, `redis_*`). | no | redis_* only |
| `credential_list` | Centralized credential reuse (paired with `litellm_params.litellm_credential_name`). | no | no |
| `include` | Merge external YAML files (single file or list); resolved by the loader from the parent `--config` file. | no | no |

> `credential_list` and `include` are real top-level keys but are NOT in the
> `config_settings` schema block (documented on the `configs` / File
> Management pages).

## `model_list` entry shape

```yaml
model_list:
  - model_name: <alias>          # clients request THIS, not litellm_params.model
    litellm_params:              # required
      model: <provider>/<model>  # provider prefix selects the upstream
      api_base: <url>            # provider-specific rules (see litellm-providers)
      api_key: os.environ/<VAR> # or literal
      # ... 18 verified keys total (see model-list.md)
    model_info:                  # optional metadata
      id: string
      mode: embedding
      input_cost_per_token: float
      output_cost_per_token: float
      max_tokens: int
      base_model: string
```

- **Multiple entries with the same `model_name` form a load-balancing group.**
  With `tpm`/`rpm` set AND `routing_strategy: simple-shuffle`, LiteLLM uses a
  weighted pick.
- The request `model` field maps to the alias (`model_name`), **not**
  `litellm_params.model`.

### `litellm_params` keys (18 verified)

`model`, `api_base`, `api_key`, `api_version`, `organization`,
`temperature`, `max_tokens`, `seed`, `extra_headers`, `rpm`, `tpm`, `weight`,
`region_name`, `base_model`, `tags`, `aws_region_name`, `azure_ad_token`,
`litellm_credential_name`. (Source: `schemas/config-yaml.option-index.json`
→ `model_list.litellm_params`.)

### `model_info` sub-keys

`id`, `mode`, `input_cost_per_token`, `output_cost_per_token`, `max_tokens`,
`base_model`, `version`, `supported_environments`, `access_groups`,
`custom_tokenizer` (`identifier`/`revision`/`auth_token`).

## Precedence rule (verbatim)

> "Most values can also be set via `litellm_settings`. If you see overlapping
> values, settings on `router_settings` will override those on
> `litellm_settings`."

Overlap keys (appear under BOTH sections): `num_retries`, `timeout`,
`fallbacks`, `context_window_fallbacks`, `content_policy_fallbacks`,
`default_fallbacks`, `set_verbose` (deprecated), `cache`/`cache_responses`.
When both are set, **`router_settings` wins** silently (not a validation
error). Place reliability knobs under `router_settings` to avoid ambiguity.

## `os.environ/X` resolution

```
os.environ/<YOUR-ENV-VAR>   # runs os.getenv("YOUR-ENV-VAR")
```

Used for `api_key`, `master_key`, `api_base`, and any string config value.

- The env var must be set in the proxy process environment **before startup**.
- An unset var resolves to empty/`None` — typically causes a downstream
  **auth failure**, NOT a config validation failure (the value type is still
  a string).
- Env vars can ALSO be injected via the top-level `environment_variables:`
  dict (workestrator does NOT use this — secrets come from the sandbox
  `env()`/`secret_env()`).

## Startup validation

- LiteLLM validates `config.yaml` against an internal Pydantic schema at
  startup. On invalid config, the proxy **fails to start**.
- Success log: `"Loaded config YAML (api_key and environment_variables are
  not shown): { ... }"` — `api_key` and `environment_variables` are
  **redacted** in the startup log.
- No separate `--validate` flag exists. To validate without serving traffic,
  start with `litellm --config <path>` and observe whether it reaches the
  "Loaded config YAML" log or emits a Pydantic `ValidationError` traceback.
- Debug flags: `--debug`, `--detailed_debug`, `export LITELLM_LOG="DEBUG"`,
  `export JSON_LOGS="True"`.

## Notable defaults

| Key | Default | Section |
|-----|---------|---------|
| `num_retries` | 3 | router_settings |
| `timeout` | 10 minutes | router_settings |
| `max_fallbacks` | 5 | router_settings |
| `routing_strategy` | `simple-shuffle` | router_settings |
| `enable_pre_call_checks` | false | router_settings |
| `enable_weighted_failover` | false | router_settings |
| `cooldown_time` | 30 (from YAML example) | router_settings |
| `allowed_fails` | 3 (from YAML example) | router_settings |
| `stream_timeout` | uses `timeout` if unset | router_settings |
| `request_timeout` | 6000 seconds | litellm_settings |
| `client_ttl` | 3600 | litellm_settings |
| `cache_responses` | false | litellm_settings |
| `user_api_key_cache_ttl` | 60s | general_settings |
| `database_connection_pool_limit` | 10 | general_settings |
| `health_check_interval` | 300 | general_settings |
| `enable_drain_endpoint` | off | general_settings |

## `include` / split-config

- `include:` takes a single file or a list of files, merged into the parent
  at load time. Start with `--config <parent>`; the loader resolves `include`.
- A child file contains **only a top-level section** (e.g. `model_list:`)
  with no wrapper.
- **No hot-reload is documented.** Config (including `include`d files) is
  resolved and validated at startup; treat changes as requiring a restart.

## Common Mistakes

| Mistake | Cause | Fix |
|---------|-------|-----|
| Unknown/misspelled key rejected | Typo like `num_retry` vs `num_retries` | Match the option-index spelling exactly. |
| Wrong type for `fallbacks` | Passing a list where a list-of-dicts is expected | Each entry is `{model: [fallbacks]}`. |
| Overlap key misread | Set in both `litellm_settings` and `router_settings` | `router_settings` wins; remove from one. |
| `enable_pre_call_check` vs `enable_pre_call_checks` | Both are valid, distinct spellings | Do not assume one is a typo. |
| `os.environ/<VAR>` empty | VAR unset at startup | Set in proxy env before startup; failure is auth, not validation. |
| `database_url` set but DB unreachable | Valid config shape, bad connection | This is a runtime error, not a validation error. |
| `set_verbose` used | Deprecated | Use `LITELLM_LOG` / `--debug`. |
| `request_timeout` assumed 600s | OpenAI SDK default confusion | LiteLLM default is 6000s; set explicitly. |
| Request `model` set to `litellm_params.model` | Confusing alias with provider model | Request `model` = `model_name` alias. |
| `api_base` missing `/v1` for `openai/` | OpenAI-compatible gotcha | Include `/v1` postfix (see litellm-openai-compatible). |

## Related Docs

- `docs/litellm/config/config-yaml-overview.md`
- `docs/litellm/config/model-list.md`
- `docs/litellm/config/config-validation.md`
- `docs/litellm/config/config-management.md`
- `docs/litellm/config/litellm-settings.md`
- `docs/litellm/config/router-settings.md`
- `docs/litellm/config/general-settings.md`
- `docs/litellm/config/environment-variables.md`
- `docs/litellm/schemas/config-yaml.option-index.json`
- `docs/litellm/schemas/config-yaml.normalized.schema.md`
