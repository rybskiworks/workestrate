---
name: litellm-caching
description: |
  Operational reference for LiteLLM response caching: cache backend selection
  (local/disk/s3/gcs/qdrant-semantic/valkey-semantic without Redis vs
  redis/redis-semantic with Redis), `cache_params` keys, `supported_call_types`,
  `mode`, and `enable_redis_auth_cache`. Load when enabling caching, choosing a
  backend, diagnosing cache misses/connection errors, or deciding what works
  in-memory. Does NOT cover logging/callbacks (see litellm-logging-
  observability), routing (see litellm-routing-fallbacks), or config anatomy
  (see litellm-config-anatomy). Full detail lives in
  docs/litellm/observability-cache-guardrails/README.md + config/litellm-settings.md.
---

# LiteLLM Caching

Distilled operational reference for LiteLLM response caching — avoiding
redundant upstream calls by caching chat/completion/embedding responses.
Full detail lives in:

- `docs/litellm/observability-cache-guardrails/README.md` — caching +
  observability + guardrails synthesis (DB/Redis gating).
- `docs/litellm/config/litellm-settings.md` — `cache`, `cache_params` sub-keys.
- `docs/litellm/schemas/config-yaml.option-index.json` — `cache_params.*`
  with `requires_redis` flags.

> **Do not hallucinate.** Every type/key below traces to
> `docs/litellm/observability-cache-guardrails/README.md` or
> `docs/litellm/config/litellm-settings.md`. Inferred items are marked
> **(inferred)**.

## Triggers

Load this skill when:

- Enabling response caching (`cache: true` + `cache_params`).
- Choosing a cache backend for an in-memory (no Redis) deployment.
- Diagnosing "No connection available" or cache-miss issues.
- Deciding whether `enable_redis_auth_cache` applies.
- Reviewing `supported_call_types` / `mode` settings.

## Cache backend types (verbatim)

`cache_params.type` values: `"local"`, `"redis"`, `"redis-semantic"`,
`"valkey-semantic"`, `"qdrant-semantic"`, `"s3"`, `"gcs"`, `"disk"`.

(`all_caches` adds `"azure-blob"` on the SDK side — may not be a valid proxy
`cache_params.type` **(inferred)**. Prefer `s3`/`gcs`/`disk`/`local`.)

| Type | Redis needed? | DB needed? | Works in-memory? |
|------|---------------|------------|-------------------|
| `local` | no | no | **YES** (in-process) |
| `disk` | no | no | **YES** |
| `s3` | no | no | **YES** |
| `gcs` | no | no | **YES** |
| `qdrant-semantic` | no | no | **YES** |
| `valkey-semantic` | no | no | **YES** |
| `redis` | **YES** | no | NO (no Redis) |
| `redis-semantic` | **YES** | no | NO (no Redis) |
| `enable_redis_auth_cache` | **YES** | no | NO (moot without virtual keys) |

## Minimal YAML — local (verbatim)

```yaml
litellm_settings:
  cache: True
  cache_params:
    type: local
```

## Minimal YAML — disk (verbatim)

```yaml
cache_params:
  type: disk
  disk_cache_dir: /tmp/litellm-cache   # default ./.litellm_cache
```

## `cache_params` keys (verbatim)

`ttl`, `default_in_memory_ttl`, `default_in_redis_ttl`, `max_connections`,
`type`, `supported_call_types` (default
`["acompletion","atext_completion","aembedding","atranscription"]`),
`mode` (default `default_off`), `host`/`port`/`password` (redis),
`namespace`, `redis_startup_nodes`, `service_name`, `sentinel_nodes`,
`sentinel_password`, `gcp_service_account`, `gcp_ssl_ca_certs`,
`ssl`/`ssl_cert_reqs`/`ssl_check_hostname`,
`s3_bucket_name`/`s3_region_name`/`s3_api_version`/`s3_use_ssl`/`s3_verify`/`s3_endpoint_url`/`s3_aws_access_key_id`/`s3_aws_secret_access_key`/`s3_aws_session_token`,
`gcs_bucket_name`/`gcs_path_service_account`/`gcs_path`,
`similarity_threshold`, `redis_semantic_cache_embedding_model`,
`valkey_semantic_cache_embedding_model`/`valkey_semantic_cache_index_name`,
`qdrant_semantic_cache_embedding_model`/`qdrant_collection_name`/`qdrant_quantization_config`/`qdrant_semantic_cache_vector_size`,
`disk_cache_dir`.

## `litellm_settings` cache keys

| Key | Type | Redis? | Notes |
|-----|------|--------|-------|
| `cache` | bool | no (if type=local/disk/s3/gcs) | Master toggle. |
| `cache_params` | dict | see type | Backend config. |
| `enable_redis_auth_cache` | bool (default false) | **YES** | Requires `cache: true` + `cache_params.type: redis`. Moot without virtual keys. |
| `enable_caching_on_provider_specific_optional_params` | bool | no | — |

`general_settings.user_api_key_cache_ttl` (int seconds, default 60) —
controls `master_key` auth cache TTL; relevant even in-memory.

## `mode` (verbatim)

`mode: default_off` — if `default_off`, you must opt in to caching on a
per-call basis.

## Cache endpoints

- `POST /cache/ping` — health-check cache backend.
- `POST /cache/delete` — body `{"keys":[...]}`.
- Per-request cache controls on `/v1/chat/completions` and `/embeddings`:
  `cache: {ttl|s-maxage|no-cache|no-store|namespace|use-cache}`.

## Caveats (verbatim)

> "For non-string Redis parameters... avoid using REDIS_* environment
> variables... use cache_kwargs."

> "REDIS_URL not recommended in prod."

> "If you see errors like No connection available, try increasing
> max_connections."

## Common Mistakes

| Mistake | Cause | Fix |
|---------|-------|-----|
| `redis` cache used without Redis | No Redis backend | Use `local`/`disk`/`s3`/`gcs` in-memory. |
| `enable_redis_auth_cache: true` without Redis | Requires Redis + virtual keys | Leave false (default) in-memory. |
| "No connection available" | `max_connections` too low | Increase `max_connections`. |
| Cache not firing | `mode: default_off` | Opt in per-call, or change `mode`. |
| `REDIS_URL` used in prod | Known performance issue | Prefer `redis_host`/`redis_port` (moot in-memory). |
| `azure-blob` cache type used | SDK-side only **(inferred)** | Use `s3`/`gcs`/`disk`/`local`. |
| `supported_call_types` wrong | Default is async variants | Ensure the call type you cache is listed. |
| `disk_cache_dir` unwritable | Default `./.litellm_cache` | Set `disk_cache_dir` to a writable path (e.g. `/tmp/litellm-cache`). |

## Project context [PROJECT — not upstream docs]

The workestrator config (`infra/litellm/config.yaml`) sets **none** of the
cache keys — caching is entirely off (deliberate minimal in-memory baseline).
Recommended DB-free additions:

- `cache_params: {type: local}` or
  `{type: disk, disk_cache_dir: /tmp/litellm-cache}` for idempotent
  coding-tier responses.
- `user_api_key_cache_ttl` (default 60s) governs the `master_key` auth cache
  TTL — relevant even in-memory (single key, low churn).

## Related Docs

- `docs/litellm/observability-cache-guardrails/README.md`
- `docs/litellm/config/litellm-settings.md`
- `docs/litellm/schemas/config-yaml.option-index.json`
- `docs/litellm/schemas/env-vars.index.json`
