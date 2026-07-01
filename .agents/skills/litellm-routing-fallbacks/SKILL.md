---
name: litellm-routing-fallbacks
description: |
  Operational reference for LiteLLM routing, fallbacks, retries, cooldowns,
  and timeouts: static `router_settings.fallbacks` (no DB) vs dynamic
  `/fallback` endpoints (DB required), fallback types, `num_retries`/
  `timeout`/`stream_timeout`/`allowed_fails`/`cooldown_time`/`retry_policy`
  keys + defaults. Load when adding a fallback chain, tuning retry/cooldown,
  diagnosing why fallbacks did/didn't fire, or choosing `routing_strategy`.
  Does NOT cover provider prefix selection (see litellm-providers), config
  top-level structure (see litellm-config-anatomy), or caching (see
  litellm-caching). Full detail lives in docs/litellm/routing/README.md +
  config/router-settings.md.
---

# LiteLLM Routing, Fallbacks & Reliability

Distilled operational reference for how LiteLLM routes requests, retries
failed calls, falls back to alternate `model_name` aliases, and cools down
unhealthy deployments. Full detail lives in:

- `docs/litellm/routing/README.md` — routing/fallback/reliability synthesis.
- `docs/litellm/config/router-settings.md` — full `router_settings` key table.
- `docs/litellm/schemas/config-yaml.option-index.json` — per-key
  `requires_db` / `requires_redis` / `default` flags (authoritative for
  DB/Redis gating).

> **Do not hallucinate.** Every key/default below traces to
> `docs/litellm/routing/README.md`, `docs/litellm/config/router-settings.md`,
> or `config-yaml.option-index.json`. Inferred items are marked **(inferred)**.

## Triggers

Load this skill when:

- Adding a static fallback chain to `router_settings.fallbacks`.
- Tuning `num_retries`, `timeout`, `stream_timeout`, `allowed_fails`,
  `cooldown_time`, or `retry_policy`.
- Diagnosing why fallbacks did or did not fire.
- Choosing a `routing_strategy`.
- Deciding whether a feature needs DB/Redis (in-memory deployment).

## Static fallbacks work WITHOUT DB/Redis

`router_settings.fallbacks` is marked `requires_db=false`,
`requires_redis=false` in the option-index. Static fallback chains in
`config.yaml` work in the in-memory workestrator.

Verbatim rules:

> "Fallbacks are triggered after the configured number of retries fails."
> "Fallbacks are attempted in the order specified in fallback_models."

### Fallback types (verbatim)

> "There are 3 types of fallbacks:
> - `content_policy_fallbacks`: For litellm.ContentPolicyViolationError
> - `context_window_fallbacks`: For litellm.ContextWindowExceededErrors
> - `fallbacks`: For all remaining errors - e.g. litellm.RateLimitError"

| Type | Key | Triggers on |
|------|-----|-------------|
| general | `fallbacks` | all remaining errors (e.g. RateLimitError) |
| context_window | `context_window_fallbacks` | `ContextWindowExceededError` |
| content_policy | `content_policy_fallbacks` | `ContentPolicyViolationError` |
| default | `default_fallbacks` | "in case a specific model group is misconfigured / bad" |

Fallbacks are attempted **in-order** (verbatim): `["gpt-3.5-turbo",
"gpt-4", "gpt-4-32k"]` does `gpt-3.5-turbo` first, then `gpt-4`, etc.

## Dynamic fallback management REQUIRES DB

Verbatim: "Database storage must be enabled: Set STORE_MODEL_IN_DB=True".

Endpoints `POST /fallback`, `GET /fallback/{model}`,
`DELETE /fallback/{model}` are **UNAVAILABLE** in-memory. Validation rules:
"Model Existence", "Fallback Model Existence", "No Self-Fallback",
"No Duplicates", "Database Enabled" — the "Database Enabled" rule gates
dynamic management.

## Retry → fallback ordering (verbatim)

> "If a call fails after num_retries, fallback to another model group."

Fallbacks fire **AFTER** retries are exhausted, not before.

## `router_settings` keys (DB/Redis-free unless noted)

| Key | Type / values | Default | DB? | Redis? |
|-----|---------------|---------|-----|--------|
| `routing_strategy` | `simple-shuffle` \| `least-busy` \| `usage-based-routing` \| `latency-based-routing` | `simple-shuffle` | no | no |
| `fallbacks` | list of `{model: [fallback_models]}` | — | no | no |
| `context_window_fallbacks` | list | — | no | no |
| `content_policy_fallbacks` | list | — | no | no |
| `default_fallbacks` | list | — | no | no |
| `num_retries` | int | 3 | no | no |
| `timeout` | int (seconds) | 10 minutes | no | no |
| `stream_timeout` | int (seconds) | uses `timeout` if unset | no | no |
| `ttft_timeout` | int | — | no | no |
| `stream_idle_timeout` | int | — | no | no |
| `allowed_fails` | int | 3 (YAML example) | no | no |
| `allowed_fails_policy` | object | — | no | no |
| `cooldown_time` | int (seconds) | 30 (YAML example) | no | no |
| `disable_cooldowns` | bool | — | no | no |
| `retry_policy` | object | — | no | no |
| `model_group_retry_policy` | object | — | no | no |
| `max_fallbacks` | int | 5 | no | no |
| `retry_after` | int | — | no | no |
| `enable_pre_call_checks` | bool | false | no | no |
| `enable_pre_call_check` (singular) | bool | — | no | no |
| `enable_weighted_failover` | bool | false | no | no |
| `model_group_alias` | object | — | no | no |
| `client_ttl` | int (seconds) | 3600 | no | no |
| `cache_responses` | bool | false | no | no |
| `redis_host`/`redis_password`/`redis_port`/`redis_db`/`redis_url` | str/int | — | no | **YES** |
| `set_verbose` | bool | — | no | no (DEPRECATED → `LITELLM_LOG`) |

### `retry_policy` sub-keys (all int)

`AuthenticationErrorRetries`, `TimeoutErrorRetries`, `RateLimitErrorRetries`,
`ContentPolicyViolationErrorRetries`, `InternalServerErrorRetries`.

### `allowed_fails_policy` sub-keys (all int)

`BadRequestErrorAllowedFails`, `AuthenticationErrorAllowedFails`,
`TimeoutErrorAllowedFails`, `RateLimitErrorAllowedFails`,
`ContentPolicyViolationErrorAllowedFails`, `InternalServerErrorAllowedFails`.

## Timeout interactions

- **`timeout`** — global, entire-call timeout. Verbatim: "The timeout set in
  router is for the entire length of the call, and is passed down to the
  completion() call level as well." Default 10 minutes.
- **`stream_timeout`** — first-chunk / TTFT-style timeout for streaming.
  Verbatim: "maximum time to wait for the *first chunk* (i.e., first token)
  in a streaming response." If unset, `timeout` is used. **NOT** an
  idle-between-chunks timeout.
- **`ttft_timeout`** — in the option-index; NOT described in prose on the
  /docs/proxy/timeout page **(inferred)** — dedicated TTFT knob distinct from
  `stream_timeout`.
- **`stream_idle_timeout`** — in the option-index; governs idle-between-chunks
  behavior **(inferred)**.
- **`litellm_settings.request_timeout`** — distinct key in a distinct section
  (litellm-level). On overlap, `router_settings` wins.

Per-model overrides: `litellm_params.timeout` / `litellm_params.stream_timeout`.
Per-request: request body `timeout` (or `extra_body={"timeout": N}`).

## Cooldown model (verbatim)

> `allowed_fails` — "cooldown model if it fails > 1 call in a minute."
> `cooldown_time` — "how long to cooldown model if fails/min > allowed_fails"
> (seconds; example value 30).

A deployment is cooled down for `cooldown_time` seconds after exceeding
`allowed_fails` fails in a minute.

## `enable_pre_call_checks` (context-window enforcement)

Verbatim: "`enable_pre_call_checks` is required for context-window
enforcement. Without it, requests are sent to the provider regardless of
input token count."

`model_info.max_input_tokens` enforcement requires BOTH
`enable_pre_call_checks: true` AND `model_info.max_input_tokens`.

## Features that REQUIRE DB/Redis

| Feature | Requirement | In-memory? |
|---------|--------------|------------|
| Dynamic fallback management (`POST /fallback` etc.) | `STORE_MODEL_IN_DB=True` + DB | NO |
| Redis-backed routing state (`redis_*`) | Redis | NO |
| Cross-pod fail counters / multi-instance rate limits | Redis | NO |
| Tag-based routing (`enable_tag_filtering`) | virtual keys/teams (DB) | moot |

Static `fallbacks`, `retry_policy`, `num_retries`, `allowed_fails`,
`cooldown_time`, `timeout`, `stream_timeout`, `max_fallbacks`,
`routing_strategy` all work **without** DB/Redis.

## Common Mistakes

| Mistake | Cause | Fix |
|---------|-------|-----|
| `POST /fallback` fails in-memory | `STORE_MODEL_IN_DB` not set | Use static `router_settings.fallbacks`. |
| Fallback chain silently truncated | `max_fallbacks` default 5 | Raise `max_fallbacks` for longer chains. |
| 400/BadRequestErrors don't trip fallbacks | `BadRequestErrorAllowedFails` default high | Tune `allowed_fails_policy` if needed. |
| `stream_timeout` assumed idle-between-chunks | Misreading | It's a first-chunk (TTFT) timeout; use `stream_idle_timeout` for idle. |
| `litellm_settings.timeout` assumed active | Precedence rule | `router_settings` wins on overlap; set in one place. |
| `enable_pre_call_check` vs `enable_pre_call_checks` | Both valid spellings | Do not assume one is a typo. |
| `set_verbose` used | Deprecated | Use `LITELLM_LOG`. |
| `redis_url` used in prod | Known performance issue | Prefer `redis_host`/`redis_port` (moot in-memory). |
| Context-window checks not running | `enable_pre_call_checks` false | Set true AND `model_info.max_input_tokens`. |
| Cross-instance cooldowns expected without Redis | No Redis | Each worker tracks locally only. |

## Per-request / per-key fallback controls

- `disable_fallbacks: true` (request body or key metadata) — disable per key.
- `mock_testing_fallbacks` / `mock_testing_content_policy_fallbacks` /
  `mock_testing_context_window_fallbacks` (request body, testing) — trigger
  each fallback path.
- Client-side `fallbacks` (request body) — set in `.completion()` call.

## Related Docs

- `docs/litellm/routing/README.md`
- `docs/litellm/config/router-settings.md`
- `docs/litellm/config/litellm-settings.md`
- `docs/litellm/schemas/config-yaml.option-index.json`
- `docs/litellm/schemas/config-yaml.normalized.schema.md`
