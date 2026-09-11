# router_settings

> Source: https://docs.litellm.ai/docs/proxy/config_settings (+ configs, deploy)

## Purpose

Routing, load-balancing, fallbacks, retries, cooldowns, timeouts, and Redis-based cross-instance router state.

## Precedence rule (verbatim)

> "Most values can also be set via `litellm_settings`. If you see overlapping values, settings on `router_settings` will override those on `litellm_settings`." — config_settings Caveats.

Overlap keys (appear under BOTH `litellm_settings` and `router_settings`): `num_retries`, `timeout`, `fallbacks`, `context_window_fallbacks`, `content_policy_fallbacks`, `default_fallbacks`, `set_verbose` (deprecated), `cache`/`cache_responses`. When both are set, `router_settings` wins.

## Key table

| key | type | default | requires_db | requires_redis | enterprise | deprecated | source |
| --- | --- | --- | --- | --- | --- | --- | --- |
| routing_strategy | enum: `simple-shuffle`,`least-busy`,`usage-based-routing`,`latency-based-routing` | `simple-shuffle` | false | false | false | false | config_settings, configs |
| redis_host | string | — | false | **true** | false | false | config_settings, configs, deploy |
| redis_password | string | — | false | **true** | false | false | config_settings, configs, deploy |
| redis_port | int | — | false | **true** | false | false | config_settings, configs, deploy |
| redis_db | int | — | false | **true** | false | false | config_settings |
| redis_url | string | — | false | **true** | false | false | config_settings |
| enable_pre_call_check | bool | — | false | false | false | false | config_settings |
| enable_pre_call_checks | bool | false | false | false | false | false | config_settings |
| optional_pre_call_checks | list | — | false | false | false | false | config_settings |
| content_policy_fallbacks | list | — | false | false | false | false | config_settings |
| fallbacks | list | — | false | false | false | false | config_settings, configs |
| context_window_fallbacks | list | — | false | false | false | false | config_settings |
| default_fallbacks | list | — | false | false | false | false | config_settings |
| enable_tag_filtering | bool | — | false | false | false | false | config_settings |
| tag_filtering_match_any | bool | — | false | false | false | false | config_settings |
| enable_weighted_failover | bool | false | false | false | false | false | config_settings |
| cooldown_time | int | 30 (seconds, inferred from example) | false | false | false | false | config_settings |
| disable_cooldowns | bool | — | false | false | false | false | config_settings |
| retry_policy | dict | — | false | false | false | false | config_settings |
| allowed_fails | int | — | false | false | false | false | config_settings, configs |
| allowed_fails_policy | dict | — | false | false | false | false | config_settings |
| default_max_parallel_requests | int | — | false | false | false | false | config_settings |
| default_priority | int | — | false | false | false | false | config_settings |
| polling_interval | int | — | false | false | false | false | config_settings |
| max_fallbacks | int | 5 | false | false | false | false | config_settings |
| default_litellm_params | dict | — | false | false | false | false | config_settings |
| timeout | int | 10 minutes | false | false | false | false | config_settings, configs |
| stream_timeout | int | uses `timeout` value if unset | false | false | false | false | config_settings |
| ttft_timeout | int | — | false | false | false | false | config_settings |
| stream_idle_timeout | int | — | false | false | false | false | config_settings |
| debug_level | string | — | false | false | false | false | config_settings |
| client_ttl | int | 3600 | false | false | false | false | config_settings |
| cache_kwargs | dict | — | false | false | false | false | config_settings |
| routing_strategy_args | dict | — | false | false | false | false | config_settings |
| model_group_alias | dict | — | false | false | false | false | config_settings, configs |
| num_retries | int | 3 | false | false | false | false | config_settings, configs |
| caching_groups | list | — | false | false | false | false | config_settings |
| alerting_config | dict | — | false | false | false | false | config_settings |
| assistants_config | dict | — | false | false | false | false | config_settings |
| set_verbose | bool | — | false | false | false | **true** | config_settings |
| retry_after | int | — | false | false | false | false | config_settings |
| provider_budget_config | dict | — | false | false | false | false | config_settings |
| model_group_retry_policy | dict | — | false | false | false | false | config_settings |
| cache_responses | bool | false | false | false | false | false | config_settings |
| router_general_settings | dict | — | false | false | false | false | config_settings |

### retry_policy sub-keys

Enum (all `int`): `AuthenticationErrorRetries`, `TimeoutErrorRetries`, `RateLimitErrorRetries`, `ContentPolicyViolationErrorRetries`, `InternalServerErrorRetries`.

### allowed_fails_policy sub-keys

(All `int`): `BadRequestErrorAllowedFails`, `AuthenticationErrorAllowedFails`, `TimeoutErrorAllowedFails`, `RateLimitErrorAllowedFails`, `ContentPolicyViolationErrorAllowedFails`, `InternalServerErrorAllowedFails`.

## Verified defaults (verbatim)

- `num_retries` — "Defaults to 3." (router_settings; config_settings Caveats)
- `timeout` — "Default 10 minutes." (router_settings; config_settings Caveats)
- `max_fallbacks` — "Defaults to 5."
- `allowed_fails` — 3 (config_settings YAML example)
- `cooldown_time` — 30 (config_settings YAML example, inferred)
- `client_ttl` — "Defaults to 3600."
- `routing_strategy` — `simple-shuffle` (config_settings YAML)
- `enable_pre_call_checks` — "Default: false."
- `enable_weighted_failover` — "Default: false."
- `cache_responses` — "Defaults to False."
- `stream_timeout` — "If not set, the 'timeout' value is used."
- `redis_url` — "Known performance issue with Redis URL."

## YAML example (verbatim)

```yaml
router_settings:
  routing_strategy: simple-shuffle # Literal["simple-shuffle", "least-busy", "usage-based-routing","latency-based-routing"], default="simple-shuffle" - RECOMMENDED for best performance
  redis_host: <your-redis-host>
  redis_password: <your-redis-password>
  redis_port: <your-redis-port>
  enable_pre_call_checks: true            # bool - Before call is made check if a call is within model context window 
  allowed_fails: 3 # cooldown model if it fails > 1 call in a minute. 
  cooldown_time: 30 # (in seconds) how long to cooldown model if fails/min > allowed_fails
  disable_cooldowns: True
  enable_tag_filtering: True
  tag_filtering_match_any: True
  retry_policy: {
    "AuthenticationErrorRetries": 3,
    "TimeoutErrorRetries": 3,
    "RateLimitErrorRetries": 3,
    "ContentPolicyViolationErrorRetries": 4,
    "InternalServerErrorRetries": 4
  }
  allowed_fails_policy: {
    "BadRequestErrorAllowedFails": 1000,
    "AuthenticationErrorAllowedFails": 10,
    "TimeoutErrorAllowedFails": 12,
    "RateLimitErrorAllowedFails": 10000,
    "ContentPolicyViolationErrorAllowedFails": 15,
    "InternalServerErrorAllowedFails": 20,
  }
  content_policy_fallbacks=[{"claude-2": ["my-fallback-model"]}]
  fallbacks=[{"claude-2": ["my-fallback-model"]}]
```

## Pitfalls

- **`redis_url` performance** — "Known performance issue with Redis URL." Prefer `redis_host`/`redis_port`/`redis_password` for production.
- **`enable_pre_call_check` (singular) vs `enable_pre_call_checks` (plural)** — both appear verbatim in the router_settings Reference table. `enable_pre_call_checks` is the documented default (`false`); `enable_pre_call_check` (singular) is also present. Treat both as valid spellings.
- **`stream_timeout` fallback** — if unset, the `timeout` value is used. Setting only `timeout` implicitly sets the stream timeout too.
- **Overlap silent override** — overlap keys set in both `litellm_settings` and `router_settings` are silently won by `router_settings`; not a validation error.
- **`redis_*` keys require Redis** — without Redis, cross-instance rate-limit tracking, cooldown sharing, and router-state sharing are unavailable. Each worker tracks cooldowns locally only.
- **`enable_pre_call_checks`** — "Required for `model_info.max_input_tokens` enforcement. Default: false." Without it, context-window pre-checks do not run.

## Reference

- Authoritative key index: [`config-yaml.option-index.json`](../schemas/config-yaml.option-index.json)
- Normalized schema: [`config-yaml.normalized.schema.md`](../schemas/config-yaml.normalized.schema.md)

## Workestrate notes

> **PROJECT CONTEXT** — not upstream LiteLLM docs. Describes the workestrate deployment specifically.

Real config (`infra/litellm/config.yaml`):

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

- **`fallbacks`** — coding-tier chains (`coding` → `coding-fallback`, `coding.fast` → `coding.fast-fallback`, `coding.pro` → `coding.pro-fallback`, `coding.free` → `coding.free-fallback`). Model names are aliases defined in `model_list`.
- **`num_retries: 2`** — deliberate override of the default `3`.
- **`timeout: 300`** — deliberate override of the default `10 minutes` (600s).
- **`stream_timeout: 300`** — set explicitly (would otherwise inherit `timeout`).
- **`allowed_fails: 3`** — matches the YAML example default.
- **`cooldown_time: 60`** — deliberate override of the example value `30`.
- **`retry_policy`** — only `TimeoutErrorRetries`, `RateLimitErrorRetries`, `InternalServerErrorRetries` are set; `AuthenticationErrorRetries` and `ContentPolicyViolationErrorRetries` are left at their defaults.
- **No `redis_*` keys** — in-memory deployment. Consequence: cross-instance rate-limit tracking and router-state sharing are unavailable; each worker tracks cooldowns locally.
- **No `enable_pre_call_checks`** — context-window pre-checks are not enabled.
- **No `model_group_alias`** — model aliases are defined directly via `model_name` in `model_list`.
- **No `routing_strategy` override** — defaults to `simple-shuffle`.

## Behavioral detail — timeout interactions

> Source: https://docs.litellm.ai/docs/proxy/timeout (raw: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/timeout.md) — `extracted/p1-routing-timeout.md`

The four router-level timeout keys interact as follows (verbatim-anchored):

- **`timeout`** — global, entire-call timeout. Verbatim: "The timeout set in router is for the entire length of the call, and is passed down to the completion() call level as well." Default `10 minutes` (config_settings Caveats). Example: `router_settings.timeout: 30 # sets a 30s timeout for the entire call`.
- **`stream_timeout`** — first-token / TTFT-style timeout for streaming responses. Verbatim: "maximum time to wait for the *first chunk* (i.e., first token) in a streaming response. Use this to abort 'hanging' providers (e.g., Bedrock slow start) and retry another model." If unset, the `timeout` value is used (config_settings). **Note:** `stream_timeout` is a *first-chunk* timeout, NOT an idle-between-chunks timeout.
- **`ttft_timeout`** — documented in the config_settings Reference table (option-index); NOT described in prose on the /docs/proxy/timeout page. Treat as a dedicated time-to-first-token knob distinct from `stream_timeout`. *(behavior beyond its existence is not documented in the fetched timeout source — inferred.)*
- **`stream_idle_timeout`** — documented in the config_settings Reference table (option-index); NOT described in prose on the /docs/proxy/timeout page. Governs idle-between-chunks behavior (distinct from `stream_timeout`'s first-chunk semantics). *(behavior beyond its existence is not documented in the fetched timeout source — inferred.)*

### `request_timeout` (litellm_settings) vs `router_settings.timeout`

These are distinct keys in distinct sections:

- `litellm_settings.request_timeout` — verbatim (from /docs/proxy/reliability Advanced section): `request_timeout: 10 # raise Timeout error if call takes longer than 10s. Sets litellm.request_timeout`. This is a litellm-level timeout.
- `router_settings.timeout` — the router-level entire-call timeout (above).

Per the precedence rule, on overlap `router_settings` wins over `litellm_settings`. The real config sets both `router_settings.timeout: 300` and `litellm_settings.request_timeout: 300` — `router_settings` governs.

### Per-model and per-request timeout overrides

- **Per-model** (`litellm_params.timeout` / `litellm_params.stream_timeout` in `model_list`) — overrides the router-level values for that deployment. Verbatim: `timeout` → "maximum time for the *complete response*"; `stream_timeout` → "maximum time to wait for the *first chunk*".
- **Per-request** (request body `timeout`, or `extra_body={"timeout": N}` via the OpenAI client) — verbatim: "LiteLLM supports setting a `timeout` per request". Overrides for that single call.
- **`mock_timeout: true`** (request body, testing) — verbatim: "set `mock_timeout=True` for testing"; "currently only supported on `/chat/completions` and `/completions` endpoints".

## Behavioral detail — reliability model (retries, fallbacks, cooldowns)

> Source: https://docs.litellm.ai/docs/proxy/reliability (raw: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/reliability.md) — `extracted/p1-routing-reliability.md`

### Retry → fallback ordering (verbatim)

> "If a call fails after num_retries, fallback to another model group."

Fallbacks fire AFTER retries are exhausted, not before. `num_retries` default is `3` (config_settings Caveats).

### Fallback types (verbatim)

> "There are 3 types of fallbacks:
> - `content_policy_fallbacks`: For litellm.ContentPolicyViolationError
> - `context_window_fallbacks`: For litellm.ContextWindowExceededErrors
> - `fallbacks`: For all remaining errors - e.g. litellm.RateLimitError"

Fallbacks are attempted in-order (verbatim): "Fallbacks are done in-order - ["gpt-3.5-turbo, "gpt-4", "gpt-4-32k"], will do 'gpt-3.5-turbo' first, then 'gpt-4', etc."

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
- `cooldown_time` — verbatim: "how long to cooldown model if fails/min > allowed_fails" (seconds; example value `30`).
- `disable_cooldowns` — documented in config_settings (option-index); NOT described in prose on the /docs/proxy/reliability page. Disables the cooldown mechanism. *(behavior beyond its existence is not documented in the fetched reliability source — inferred.)*
- `retry_after` — documented in config_settings (option-index); NOT described on the /docs/proxy/reliability page. *(behavior not documented in the fetched reliability source — inferred.)*

### `retry_policy` and `allowed_fails_policy` (sub-keys)

These are documented in config_settings (option-index) and the YAML example above; NOT described in prose on the /docs/proxy/reliability page. Sub-keys (verbatim from config_settings):

- `retry_policy`: `AuthenticationErrorRetries`, `TimeoutErrorRetries`, `RateLimitErrorRetries`, `ContentPolicyViolationErrorRetries`, `InternalServerErrorRetries` (all `int`).
- `allowed_fails_policy`: `BadRequestErrorAllowedFails`, `AuthenticationErrorAllowedFails`, `TimeoutErrorAllowedFails`, `RateLimitErrorAllowedFails`, `ContentPolicyViolationErrorAllowedFails`, `InternalServerErrorAllowedFails` (all `int`).

### `max_fallbacks` and `default_fallbacks`

- `max_fallbacks` — default `5` (config_settings). Mentioned in `p1-routing-fallback_management.md` Notes. Long fallback chains beyond 5 entries are silently truncated unless raised.
- `default_fallbacks` — verbatim (from /docs/proxy/reliability): "in case a specific model group is misconfigured / bad"; "A model-specific fallbacks ... overrides default fallback."

### `enable_pre_call_checks` (context-window enforcement)

Verbatim admonition (from /docs/proxy/reliability):

> "`enable_pre_call_checks` is required for context-window enforcement. Without it, requests are sent to the provider regardless of input token count. Set `enable_pre_call_checks: true` in `router_settings` in your config."

`model_info.max_input_tokens` enforcement requires BOTH `enable_pre_call_checks: true` AND `model_info.max_input_tokens` on the deployment (verbatim: "Both of the following are required").

### Redis requirement for cross-instance state

> **Not documented on the /docs/proxy/reliability page.** The reliability page does not mention Redis. Cross-instance cooldown sharing, cross-pod fail counters, and multi-instance rate-limit tracking require Redis (`router_settings.redis_host`/`redis_url` etc., per config_settings). Without Redis, each worker tracks cooldowns and fails locally only. *(Redis-requirement for cross-instance state is inferred from config_settings `requires_redis=true` flags on `redis_*` keys and the `fail_closed_budget_enforcement` note — not from the fetched reliability source.)*

### Per-request / per-key fallback controls

- `disable_fallbacks: true` (request body or key metadata) — verbatim: "You can disable fallbacks per key by setting `disable_fallbacks: true` in your request body."
- `mock_testing_fallbacks` / `mock_testing_content_policy_fallbacks` / `mock_testing_context_window_fallbacks` (request body, testing) — trigger each fallback path for testing.
- Client-side `fallbacks` (request body) — verbatim: "Set fallbacks in the `.completion()` call for SDK and client-side for proxy." Supports per-model messages/temperature.
