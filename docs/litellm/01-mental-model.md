# LiteLLM Proxy Mental Model

## What it controls

The request flow: a client sends an OpenAI-shaped request whose `model` field is
a **`model_name` alias**; LiteLLM resolves that alias through `litellm_params`
to an upstream provider SDK/protocol.

```
client POST /v1/chat/completions {model: "coding"}
        -> model_name alias "coding"
        -> litellm_params {model: "anthropic/kimi-for-coding", api_base, api_key}
        -> upstream provider (Kimi, Anthropic Messages API)
```

## Where it appears

- **Config**: `config.yaml` top-level `model_list` (alias + `litellm_params`).
- **Runtime**: `POST /v1/chat/completions` — the request `model` field maps to a
  `model_name` alias, **not** to `litellm_params.model`.

## Verified behavior

- LiteLLM Proxy is an **OpenAI-compatible gateway** listening on `:4000`.
  Clients send OpenAI-shaped requests (e.g. `POST /v1/chat/completions`).
- The `model` field in the **request** maps to a `model_name` alias defined in
  `config.yaml` — it is **not** the `litellm_params.model` (provider model) string.
- The `model` field in `litellm_params` uses a **provider prefix** (e.g.
  `anthropic/`, `openai/`, `openrouter/`) to select the upstream SDK/protocol.
- Top-level config sections and their roles:

| Section | Role |
| --- | --- |
| `model_list` | Defines deployments: `model_name` alias + `litellm_params` + optional `model_info` |
| `litellm_settings` | Module-level LiteLLM SDK settings (`drop_params`, `request_timeout`, `force_ipv4`, callbacks, caching) |
| `general_settings` | Proxy-level settings (`master_key`, `completion_model`, `database_url`, alerting, health checks, `disable_spend_logs`) |
| `router_settings` | Routing/reliability (`routing_strategy`, `fallbacks`, `num_retries`, `timeout`, `redis_*`, `retry_policy`, `cooldown_time`) |
| `callback_settings` | Callback-specific config (`otel.message_logging`) |
| `environment_variables` | Inject env vars via config |
| `credential_list` | Centralized credential reuse (paired with `litellm_params.litellm_credential_name`) |

### Precedence rule (verbatim from config_settings Caveats)

> "Most values can also be set via `litellm_settings`. If you see overlapping
> values, settings on `router_settings` will override those on `litellm_settings`."

Overlap keys (appear under **both** sections): `num_retries`, `timeout`,
`fallbacks`, `context_window_fallbacks`, `content_policy_fallbacks`,
`default_fallbacks`, `set_verbose` (deprecated), `cache`/`cache_responses`.
When both are set, **`router_settings` wins**.

### Env-var resolution (verbatim)

```
os.environ/<YOUR-ENV-VAR>  # runs os.getenv("YOUR-ENV-VAR")
```

Used for `api_key`, `master_key`, `api_base`, and any string config value.
Confirmed on `configs`, `deploy`, `docker_quick_start`, `config_settings` pages.

### In-memory vs DB-gated features

- **Virtual keys / budgets / teams / users / spend tracking** require
  `database_url` (Postgres via Prisma).
- **Redis** is required for cross-instance rate-limit tracking, redis cache, and
  `enable_redis_auth_cache`.
- **Enterprise features** (`public_routes`, `enforced_params`,
  `enable_oauth2_auth`, `admin_only_routes`, `enable_oauth2_proxy_auth`) require
  `litellm_license`.

## Examples

Minimal `model_list` entry (verbatim from the configs page):

```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: azure/gpt-4o-eu
      api_base: https://my-endpoint-eu.azure.com/
      api_key: os.environ/AZURE_API_KEY_EU
      rpm: 6
```

## Pitfalls

- The `model` field in the **request** is the alias (`model_name`), **not** the
  provider model (`litellm_params.model`). Confusing the two is the most common
  mistake.
- Both `enable_pre_call_check` (singular) and `enable_pre_call_checks` (plural)
  are real, distinct keys — do not assume one is a typo for the other.
- `redis_url` has a known performance issue (noted in config_settings Caveats).
- `request_timeout` default is **6000 seconds** (litellm_settings) — much longer
  than the OpenAI SDK's 600s default. Set it explicitly if you need tighter
  bounds.

## Links to local

- [config/README.md](config/README.md) — `config.yaml` reference index
- [config/config-yaml-overview.md](config/config-yaml-overview.md) — top-level YAML structure
- [schemas/config-yaml.normalized.schema.md](schemas/config-yaml.normalized.schema.md) — human-readable schema
- [schemas/config-yaml.option-index.json](schemas/config-yaml.option-index.json) — every config key with sources

## Sources

- https://docs.litellm.ai/docs/simple_proxy
- https://docs.litellm.ai/docs/proxy/configs
- https://docs.litellm.ai/docs/proxy/config_settings
- Local: `extracted/p0-simple_proxy.md`, `extracted/p0-configs.md`, `extracted/p0-config_settings.md`

## Workestrate notes

[PROJECT CONTEXT — NOT upstream docs]

- In-memory deployment: no DB, no Redis, `master_key`-only auth. See
  [00-index.md](00-index.md) → Workestrate notes.
- Client-facing aliases are **coding-tier role names**: `coding`,
  `coding-fallback`, `coding.fast`, `coding.fast-fallback`, `coding.pro`,
  `coding.pro-fallback`, `coding.free`, `coding.free-fallback`, `neural`.
- Provider prefix usage in the real config:
  - `anthropic/` for Kimi (`kimi-for-coding`) and MiniMax (`MiniMax-M3`) — both
    endpoints speak the Anthropic Messages API.
  - `openrouter/` for GLM (`z-ai/glm-5.1`), Qwen (`qwen/qwen3.7-plus`,
    `qwen/qwen3-coder:free`), Nex (`nex-agi/nex-n2-pro:free`).
  - `openai/` for Neuralwatt (`neuralwatt`, `api_base https://api.neuralwatt.com/v1`).
- Real config: `infra/litellm/config.yaml` (mounted read-only at `/app/config.yaml`).
