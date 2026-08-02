# Routing, Load Balancing, Fallbacks & Reliability

> Synthesized from on-disk corpus under `docs/litellm/extracted/` and
> `docs/litellm/schemas/`. No web fetches were performed. Every key, default,
> and requirement traces to a cited corpus file or is marked **(inferred)** /
> **"not documented in fetched source"**.

## Sources

- https://docs.litellm.ai/docs/routing (load balancing) — `extracted/p1-routing-load_balancing.md`
- https://docs.litellm.ai/docs/routing_fallbacks (fallback management) — `extracted/p1-routing-fallback_management.md`
- https://docs.litellm.ai/docs/proxy/config_settings (router_settings + retry_policy + Caveats) — `extracted/p0-config_settings.md`
- `schemas/config-yaml.option-index.json` — per-key `requires_db` / `requires_redis` / `default` flags
- Real config: `infra/litellm/config.yaml`

## What this area controls

Routing decides which upstream deployment serves a request, how the proxy
retries failed calls, when it falls back to alternate `model_name` aliases,
and how it cools down unhealthy deployments. In the workestrate (in-memory,
no DB, no Redis) the **only** available path is **static `router_settings`
in `config.yaml`**. Dynamic fallback management endpoints REQUIRE a database
and are unavailable.

## Verified behavior

### Static fallbacks work without DB/Redis

`schemas/config-yaml.option-index.json` marks `router_settings.fallbacks`
with `requires_db=false` and `requires_redis=false`. The real config uses
this path. Verbatim rule (from `p1-routing-fallback_management.md`):

> "Fallbacks are triggered after the configured number of retries fails."

> "Fallbacks are attempted in the order specified in fallback_models."

### Fallback types (verbatim)

- `general` — generic fallback chain (`router_settings.fallbacks`).
- `context_window` — fallback when context window exceeded
  (`router_settings.context_window_fallbacks`).
- `content_policy` — fallback on content-policy violations
  (`router_settings.content_policy_fallbacks`).

### Dynamic fallback management endpoints REQUIRE DB

Verbatim (from `p1-routing-fallback_management.md`):

> "Database storage must be enabled: Set STORE_MODEL_IN_DB=True"

Endpoints `POST /fallback`, `GET /fallback/{model}`, `DELETE /fallback/{model}`
are **UNAVAILABLE** in the in-memory workestrate.

### Validation rules (verbatim)

> "Model Existence", "Fallback Model Existence", "No Self-Fallback",
> "No Duplicates", "Database Enabled"

The "Database Enabled" rule is the gate that blocks dynamic fallback
management in-memory.

### Precedence rule (verbatim)

> "Most values can also be set via litellm_settings. If you see overlapping
> values, settings on router_settings will override those on litellm_settings."

The real config sets `timeout: 300` and `request_timeout: 300` in both
`router_settings` and `litellm_settings` — `router_settings` wins on overlap.

## Config keys / requirements

### `router_settings` keys (all `requires_db=false`, `requires_redis=false` unless noted)

| Key | Type / values | Default | Source |
| --- | --- | --- | --- |
| `routing_strategy` | enum: `simple-shuffle` \| `least-busy` \| `usage-based-routing` \| `latency-based-routing` | `simple-shuffle` | option-index |
| `fallbacks` | list of `{model: [fallback_models]}` | — | option-index (DB-free) |
| `context_window_fallbacks` | list | — | option-index (DB-free) |
| `content_policy_fallbacks` | list | — | option-index (DB-free) |
| `default_fallbacks` | list | — | option-index (DB-free) |
| `num_retries` | int | `3` | option-index |
| `timeout` | int (seconds) | `"10 minutes"` | option-index |
| `stream_timeout` | int (seconds) | uses `timeout` if unset | option-index |
| `ttft_timeout` | int | — | option-index |
| `stream_idle_timeout` | int | — | option-index |
| `allowed_fails` | int | — | option-index |
| `allowed_fails_policy` | object | — | option-index |
| `cooldown_time` | int (seconds) | `30` | option-index |
| `disable_cooldowns` | bool | — | option-index |
| `retry_policy` | object | — | option-index |
| `model_group_retry_policy` | object | — | option-index |
| `max_fallbacks` | int | `5` | option-index |
| `retry_after` | int | — | option-index |
| `enable_pre_call_checks` | bool | `false` | option-index |
| `enable_pre_call_check` (singular) | bool | — | option-index (also exists) |
| `optional_pre_call_checks` | list | — | option-index |
| `enable_tag_filtering` | bool | — | option-index |
| `tag_filtering_match_any` | bool | — | option-index |
| `enable_weighted_failover` | bool | `false` | option-index |
| `model_group_alias` | object | — | option-index |
| `default_litellm_params` | object | — | option-index |
| `default_priority` | int | — | option-index |
| `default_max_parallel_requests` | int | — | option-index |
| `polling_interval` | int | — | option-index |
| `client_ttl` | int (seconds) | `3600` | option-index |
| `cache_kwargs` | object | — | option-index |
| `cache_responses` | bool | `false` | option-index |
| `routing_strategy_args` | object | — | option-index |
| `provider_budget_config` | object | — | option-index |
| `caching_groups` | list | — | option-index |
| `alerting_config` | object | — | option-index |
| `assistants_config` | object | — | option-index |
| `router_general_settings` | object | — | option-index |
| `debug_level` | str | — | option-index |
| `redis_host` / `redis_password` / `redis_port` / `redis_db` / `redis_url` | str/int | — | option-index (**`requires_redis=true`**) |
| `set_verbose` | bool | — | option-index (**DEPRECATED**, replacement `LITELLM_LOG`) |

### `retry_policy` sub-keys (verbatim from `p0-config_settings.md`)

- `AuthenticationErrorRetries`
- `TimeoutErrorRetries`
- `RateLimitErrorRetries`
- `ContentPolicyViolationErrorRetries`
- `InternalServerErrorRetries`

### `allowed_fails_policy` sub-keys (verbatim)

- `BadRequestErrorAllowedFails`
- `AuthenticationErrorAllowedFails`
- `TimeoutErrorAllowedFails`
- `RateLimitErrorAllowedFails`
- `ContentPolicyViolationErrorAllowedFails`
- `InternalServerErrorAllowedFails`

### Caveats (verbatim)

- `enable_pre_call_checks` — "Required for `model_info.max_input_tokens`
  enforcement. Default: false."
- `redis_url` (router_settings) — "Known performance issue with Redis URL."

## Features that REQUIRE DB/Redis

| Feature | Requirement | Available in-memory? |
| --- | --- | --- |
| Dynamic fallback management (`POST /fallback`, `GET /fallback/{model}`, `DELETE /fallback/{model}`) | `STORE_MODEL_IN_DB=True` + DB | **NO** |
| Redis-backed routing state (`redis_host`, `redis_url`, etc.) | Redis | **NO** (not configured) |
| Multi-instance rate limiting / cross-pod counters | Redis | **NO** |
| Tag-based routing (`enable_tag_filtering`) | virtual keys / teams (DB) | moot without DB |

Static `router_settings.fallbacks`, `retry_policy`, `num_retries`,
`allowed_fails`, `cooldown_time`, `timeout`, `stream_timeout`,
`max_fallbacks`, `routing_strategy` all work **without** DB/Redis.

## Pitfalls

- **Dynamic endpoints silently unavailable.** `POST /fallback` returns an
  error in-memory because `STORE_MODEL_IN_DB` is not set. Use static
  `router_settings.fallbacks` instead.
- **400/BadRequestErrors are not counted** toward region-outage / fail
  counters (verbatim from alerting corpus). `BadRequestErrorAllowedFails`
  default is high in `allowed_fails_policy` — a steady stream of 400s will
  not trip fallbacks.
- **`max_fallbacks` default is 5.** Long fallback chains beyond 5 entries
  are silently truncated unless raised.
- **`timeout` default is "10 minutes".** The real config overrides to `300`
  (5 min) in both `router_settings` and `litellm_settings`.
- **`num_retries` default is 3.** The real config sets `2`. Combined with
  `allowed_fails: 3` and `cooldown_time: 60`, a deployment is cooled down
  for 60s after 3 fails.
- **`set_verbose` is DEPRECATED.** Use `LITELLM_LOG` env var instead.
- **`redis_url` has a known performance issue** — avoid in prod; prefer
  discrete `redis_host`/`redis_port` (moot in workestrate: no Redis).
- **Precedence:** `router_settings` overrides `litellm_settings` on
  overlapping keys. Do not assume `litellm_settings.timeout` wins.

## Verbatim real-config `router_settings.fallbacks` block

From `infra/litellm/config.yaml`:

```yaml
router_settings:
  fallbacks:
    - coding: [coding-fallback]
    - coding.fast: [coding.fast-fallback]
    - coding.pro: [coding.pro-fallback]
    - coding.free: [coding.free-fallback]
  num_retries: 2
  timeout: 300
  stream_timeout: 300
  allowed_fails: 3
  cooldown_time: 60
  retry_policy:
    TimeoutErrorRetries: 2
    RateLimitErrorRetries: 3
    InternalServerErrorRetries: 2
```

Each tier (`coding`, `coding.fast`, `coding.pro`, `coding.free`) maps to a
single `-fallback` sibling `model_name`. The `neural` tier has **no**
fallback configured.

## Related schema / workflow

- [`schemas/config-yaml.option-index.json`](../schemas/config-yaml.option-index.json) — per-key `requires_db` / `requires_redis` / `default` flags (authoritative for DB/Redis gating).
- [`schemas/config-yaml.normalized.schema.md`](../schemas/config-yaml.normalized.schema.md) — `router_settings` section, precedence rule, workestrate in-memory mapping.
- `litellm-routing-fallbacks` (`.agents/skills/litellm-routing-fallbacks/`) — operational skill for adding static fallbacks (DB-free).
- [`config/README.md`](../config/README.md) — top-level config reference.

## Workestrate notes

[PROJECT CONTEXT — NOT upstream docs]

- The workestrate uses **static `router_settings.fallbacks` only**. Dynamic
  fallback management endpoints are unavailable (no DB, `STORE_MODEL_IN_DB`
  not set).
- Tier lineup (verbatim from `infra/litellm/config.yaml` header comment):
  - `coding` → Kimi K2.7 (specialist) → MiniMax M3
  - `coding.fast` → MiniMax M3 (cheap) → Qwen 3.7 Plus
  - `coding.pro` → GLM-5.1 (strong reasoning) → Kimi K2.7
  - `coding.free` → Qwen 3 Coder (free) → Nex N2 Pro (free)
- `neural` (`openai/neuralwatt`) has **no fallback** — a single OpenAI-
  compatible deployment with no redundancy.
- Reliability knobs in use: `num_retries: 2`, `timeout: 300`,
  `stream_timeout: 300`, `allowed_fails: 3`, `cooldown_time: 60`.
- `retry_policy` covers `TimeoutErrorRetries: 2`, `RateLimitErrorRetries: 3`,
  `InternalServerErrorRetries: 2`. `AuthenticationErrorRetries` and
  `ContentPolicyViolationErrorRetries` are **not set** (use defaults).
- No Redis → no cross-pod counters, no `redis_url`, no queue-size metrics.
  Single-instance only.

## Behavioral detail — timeout interactions (upstream-verified)

> Source: https://docs.litellm.ai/docs/proxy/timeout (raw: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/timeout.md) — `extracted/p1-routing-timeout.md`

The four router-level timeout keys interact as follows (verbatim-anchored):

- **`timeout`** — global, entire-call timeout. Verbatim: "The timeout set in router is for the entire length of the call, and is passed down to the completion() call level as well." Default `10 minutes` (config_settings Caveats).
- **`stream_timeout`** — first-token / TTFT-style timeout for streaming responses. Verbatim: "maximum time to wait for the *first chunk* (i.e., first token) in a streaming response. Use this to abort 'hanging' providers (e.g., Bedrock slow start) and retry another model." If unset, the `timeout` value is used. **Note:** `stream_timeout` is a *first-chunk* timeout, NOT an idle-between-chunks timeout.
- **`ttft_timeout`** — in the config_settings Reference table (option-index); NOT described in prose on the /docs/proxy/timeout page. *(behavior beyond its existence is not documented in the fetched timeout source — inferred.)*
- **`stream_idle_timeout`** — in the config_settings Reference table (option-index); NOT described in prose on the /docs/proxy/timeout page. Governs idle-between-chunks behavior (distinct from `stream_timeout`'s first-chunk semantics). *(behavior beyond its existence is not documented in the fetched timeout source — inferred.)*

### `request_timeout` (litellm_settings) vs `router_settings.timeout`

Distinct keys in distinct sections:

- `litellm_settings.request_timeout` — verbatim (from /docs/proxy/reliability): `request_timeout: 10 # raise Timeout error if call takes longer than 10s. Sets litellm.request_timeout`.
- `router_settings.timeout` — the router-level entire-call timeout.

On overlap, `router_settings` wins (precedence rule). The real config sets both to `300`.

### Per-model and per-request overrides

- Per-model: `litellm_params.timeout` / `litellm_params.stream_timeout` override the router-level values for that deployment.
- Per-request: request body `timeout` (or `extra_body={"timeout": N}`) — verbatim: "LiteLLM supports setting a `timeout` per request".
- `mock_timeout: true` (testing) — verbatim: "currently only supported on `/chat/completions` and `/completions` endpoints".

## Behavioral detail — reliability model (upstream-verified)

> Source: https://docs.litellm.ai/docs/proxy/reliability (raw: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/reliability.md) — `extracted/p1-routing-reliability.md`

### Retry → fallback ordering (verbatim)

> "If a call fails after num_retries, fallback to another model group."

Fallbacks fire AFTER retries are exhausted. `num_retries` default is `3`.

### Fallback types (verbatim)

> "There are 3 types of fallbacks:
> - `content_policy_fallbacks`: For litellm.ContentPolicyViolationError
> - `context_window_fallbacks`: For litellm.ContextWindowExceededErrors
> - `fallbacks`: For all remaining errors - e.g. litellm.RateLimitError"

In-order (verbatim): "Fallbacks are done in-order - ["gpt-3.5-turbo, "gpt-4", "gpt-4-32k"], will do 'gpt-3.5-turbo' first, then 'gpt-4', etc."

### Cooldown model (verbatim from /docs/proxy/reliability Advanced YAML)

```yaml
litellm_settings:
  num_retries: 3 # retry call 3 times on each model_name (e.g. zephyr-beta)
  request_timeout: 10 # raise Timeout error if call takes longer than 10s. Sets litellm.request_timeout 
  fallbacks: [{"zephyr-beta": ["gpt-3.5-turbo"]}] # fallback to gpt-3.5-turbo if call fails num_retries 
  allowed_fails: 3 # cooldown model if it fails > 1 call in a minute. 
  cooldown_time: 30 # how long to cooldown model if fails/min > allowed_fails
```

- `allowed_fails` — verbatim: "cooldown model if it fails > 1 call in a minute."
- `cooldown_time` — verbatim: "how long to cooldown model if fails/min > allowed_fails" (seconds; example `30`).
- `disable_cooldowns` / `retry_after` — in config_settings (option-index); NOT described in prose on the /docs/proxy/reliability page. *(behavior not documented in the fetched reliability source — inferred.)*
- `retry_policy` / `allowed_fails_policy` sub-keys — in config_settings; NOT described in prose on the /docs/proxy/reliability page. (See `config/router-settings.md` for the sub-key lists.)

### `max_fallbacks` and `default_fallbacks`

- `max_fallbacks` — default `5` (config_settings). Chains beyond 5 are silently truncated unless raised.
- `default_fallbacks` — verbatim: "in case a specific model group is misconfigured / bad"; "A model-specific fallbacks ... overrides default fallback."

### `enable_pre_call_checks` (context-window enforcement)

Verbatim admonition (from /docs/proxy/reliability):

> "`enable_pre_call_checks` is required for context-window enforcement. Without it, requests are sent to the provider regardless of input token count. Set `enable_pre_call_checks: true` in `router_settings` in your config."

`model_info.max_input_tokens` enforcement requires BOTH `enable_pre_call_checks: true` AND `model_info.max_input_tokens` (verbatim: "Both of the following are required").

### Redis requirement for cross-instance state

> **Not documented on the /docs/proxy/reliability page.** The reliability page does not mention Redis. Cross-instance cooldown sharing, cross-pod fail counters, and multi-instance rate-limit tracking require Redis (`router_settings.redis_host`/`redis_url` etc.). Without Redis, each worker tracks cooldowns and fails locally only. *(Redis-requirement inferred from config_settings `requires_redis=true` flags — not from the fetched reliability source.)*

### Per-request / per-key fallback controls

- `disable_fallbacks: true` (request body or key metadata) — verbatim: "You can disable fallbacks per key by setting `disable_fallbacks: true` in your request body."
- `mock_testing_fallbacks` / `mock_testing_content_policy_fallbacks` / `mock_testing_context_window_fallbacks` (request body, testing) — trigger each fallback path.
- Client-side `fallbacks` (request body) — verbatim: "Set fallbacks in the `.completion()` call for SDK and client-side for proxy."
