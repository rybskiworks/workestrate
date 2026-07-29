# `config.yaml` Top-Level Structure

## What it controls

The entire proxy configuration: which models are exposed, how requests are
routed, reliability/retry behavior, auth, and module-level SDK settings.

## Where it appears

`config.yaml`. In workestrate it is mounted **read-only** at `/app/config.yaml`
and the proxy is started with `--config /app/config.yaml --host 0.0.0.0`.

## Verified behavior

LiteLLM validates `config.yaml` against an internal **Pydantic schema at
startup**. On invalid config, the proxy **fails to start**. Startup log:

> "Loaded config YAML (api_key and environment_variables are not shown): { ... }"

### Top-level keys

| Key | Role |
| --- | --- |
| `environment_variables` | Inject env vars via config |
| `model_list` | List of deployments (`model_name` alias + `litellm_params` + optional `model_info`) |
| `litellm_settings` | Module-level LiteLLM SDK settings (`drop_params`, `request_timeout`, `force_ipv4`, callbacks, caching) |
| `callback_settings` | Callback-specific config (e.g. `otel.message_logging`) |
| `general_settings` | Proxy-level settings (`master_key`, `completion_model`, `database_url`, alerting, health checks, `disable_spend_logs`) |
| `router_settings` | Routing/reliability (`routing_strategy`, `fallbacks`, `num_retries`, `timeout`, `redis_*`, `retry_policy`, `cooldown_time`) |
| `credential_list` | Centralized credential reuse (paired with `litellm_params.litellm_credential_name`) |
| `include` | Merge external YAML files into the parent config (list of filenames; from the `config_management` "File Management" page) |

> Note: `credential_list` appears on the `configs` page and is **not** in the
> `config_settings` schema block, but it is a real top-level key.

> Note: `include` appears on the `config_management` ("File Management") page and
> is **not** in the `config_settings` schema block, but it is a real top-level
> key. Verbatim syntax: `include:` taking a single file or a list of files. See
> [config-management.md](config-management.md).

### Precedence rule (verbatim from config_settings Caveats)

> "Most values can also be set via `litellm_settings`. If you see overlapping
> values, settings on `router_settings` will override those on `litellm_settings`."

Overlap keys (appear under **both** `litellm_settings` and `router_settings`):
`num_retries`, `timeout`, `fallbacks`, `context_window_fallbacks`,
`content_policy_fallbacks`, `default_fallbacks`, `set_verbose` (deprecated),
`cache`/`cache_responses`. When both are set, **`router_settings` wins**.

### Env-var resolution (verbatim)

```
os.environ/<YOUR-ENV-VAR>  # runs os.getenv("YOUR-ENV-VAR")
```

Used for `api_key`, `master_key`, `api_base`, and any string config value.

### Notable verbatim defaults (from config_settings Caveats)

| Key | Default | Section |
| --- | --- | --- |
| `num_retries` | 3 | router_settings |
| `timeout` | 10 minutes | router_settings |
| `max_fallbacks` | 5 | router_settings |
| `request_timeout` | 6000 seconds | litellm_settings |
| `user_api_key_cache_ttl` | 60s | general_settings |
| `client_ttl` | 3600 | litellm_settings |
| `routing_strategy` | `simple-shuffle` | router_settings |
| `enable_pre_call_checks` | false | router_settings |
| `enable_weighted_failover` | false | router_settings |
| `cache_responses` | false | litellm_settings |
| `fail_closed_budget_enforcement` | false | general_settings |
| `cancel_on_disconnect` | false | router_settings |
| `enable_drain_endpoint` | off by default | router_settings |
| `database_connection_pool_limit` | 10 | general_settings |
| `database_connection_timeout` | 60s | general_settings |
| `health_check_interval` | 300 | general_settings |
| `alerting_threshold` | 0 | general_settings |
| `cooldown_time` | 30 (from YAML example) | router_settings |
| `allowed_fails` | 3 (from YAML example) | router_settings |
| `stream_timeout` | "If not set, the `timeout` value is used." | router_settings |

## Examples

Verbatim top-level YAML schema block from `config_settings`:

```yaml
environment_variables: {}
model_list:
  - model_name: string
    litellm_params:
      model: string
      api_base: string
      api_key: string
      api_version: string
      organization: string
      temperature: float
      max_tokens: int
      seed: int
      extra_headers: dict
      rpm: int
      tpm: int
      weight: int
      region_name: string
      base_model: string
      tags: list
    model_info:
      id: string
      mode: string
      input_cost_per_token: float
      output_cost_per_token: float
      max_tokens: int
      base_model: string
litellm_settings: {}
callback_settings:
  otel:
    message_logging: boolean
general_settings: {}
router_settings: {}
```

## Pitfalls

- **`router_settings` overrides `litellm_settings`** for overlap keys. If you set
  `num_retries` in both, only the `router_settings` value is used.
- Both `enable_pre_call_check` (singular) and `enable_pre_call_checks` (plural)
  are real, distinct keys — do not assume one is a typo.
- `redis_url` has a known performance issue (config_settings Caveats).
- `request_timeout` default is **6000 seconds** — far longer than the OpenAI SDK's
  600s. Set it explicitly when tighter bounds are needed.

## Links to local

- [model-list.md](model-list.md) — `model_list` anatomy
- [schemas/config-yaml.option-index.json](../schemas/config-yaml.option-index.json) — every config key with sources
- [schemas/config-yaml.normalized.schema.md](../schemas/config-yaml.normalized.schema.md) — normalized schema reference
- [../01-mental-model.md](../01-mental-model.md) — request flow mental model

## Sources

- https://docs.litellm.ai/docs/proxy/config_settings (PRIMARY)
- https://docs.litellm.ai/docs/proxy/configs
- Local: `extracted/p0-config_settings.md`, `extracted/p0-configs.md`

## Workestrate notes

[PROJECT CONTEXT — NOT upstream docs]

The real config (`infra/litellm/config.yaml`) uses **4 of the 7** top-level
sections:

```yaml
model_list:        # 9 aliases across coding tiers + neural
  ...
general_settings:
  master_key: os.environ/LITELLM_MASTER_KEY
  completion_model: coding
  disable_spend_logs: true
router_settings:
  fallbacks: [...]
  num_retries: 2
  timeout: 300
  stream_timeout: 300
  allowed_fails: 3
  cooldown_time: 60
  retry_policy: {...}
litellm_settings:
  drop_params: true
  request_timeout: 300
  force_ipv4: true
```

- **Not used**: `environment_variables` (secrets injected via sandbox
  `env()`/`secret_env()`), `credential_list` (keys are inline `os.environ/`),
  `callback_settings` (no OTEL/callback config).
- Config is mounted **read-only** at `/app/config.yaml`; started with
  `--config /app/config.yaml --host 0.0.0.0`, listening on `:4000`.
- `disable_spend_logs: true` compensates for the missing DB.
