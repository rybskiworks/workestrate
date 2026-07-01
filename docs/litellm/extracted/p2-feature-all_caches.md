---
source_url: https://docs.litellm.ai/docs/caching/all_caches
canonical_url: https://docs.litellm.ai/docs/caching/all_caches
title: "Caching - In-Memory, Redis, s3, gcs, Redis Semantic Cache, Disk"
sidebar_section_path: caching
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: caching
---
# Caching - In-Memory, Redis, s3, gcs, Redis Semantic Cache, Disk

## Headings
- Initialize Cache - In Memory, Redis, s3 Bucket, gcs Bucket, Redis Semantic, Disk Cache, Qdrant Semantic
- Quick Start (in memory)
- Quick Start (disk)
- Switch Cache On / Off Per LiteLLM Call
- Cache Context Manager - Enable, Disable, Update Cache
- Custom Cache Keys
- Cache Initialization Parameters
- Logging

## Exact config keys found (full path, verbatim spelling)
- This page is the Python SDK `Cache(...)` constructor signature — no YAML dotted paths. Cache type literal values accepted by `Cache(type=...)`:
  - `"local"` (in-memory) — works WITHOUT Redis
  - `"redis"` — requires Redis
  - `"redis-semantic"` — requires Redis + `redisvl==0.4.1`
  - `"valkey-semantic"` — requires Valkey with `valkey-search` module
  - `"qdrant-semantic"` — requires Qdrant
  - `"s3"` — works WITHOUT Redis
  - `"gcs"` — works WITHOUT Redis
  - `"disk"` — works WITHOUT Redis
  - `"azure-blob"` — works WITHOUT Redis (uses Azure Blob Storage)
- Cache constructor parameters (from `Cache.__init__`):
  - `type` — Literal["local","redis","redis-semantic","valkey-semantic","s3","gcs","disk"]; default `"local"`
  - `supported_call_types` — list; default `["completion","acompletion","embedding","aembedding","atranscription","transcription"]`
  - `ttl` — Optional[float]
  - `default_in_memory_ttl` — Optional[float]
  - `host`, `port`, `password` — Redis
  - `namespace` — Optional[str]
  - `default_in_redis_ttl` — Optional[float]
  - `s3_bucket_name`, `s3_region_name`, `s3_endpoint_url`, `s3_aws_access_key_id`, `s3_aws_secret_access_key`, `s3_aws_session_token`, `s3_path`, `s3_use_ssl`, `s3_verify`, `s3_api_version`, `s3_config`
  - `disk_cache_dir` — None default
  - `gcs_bucket_name`, `gcs_path_service_account`
  - `qdrant_api_base`, `qdrant_api_key`, `qdrant_collection_name`, `qdrant_quantization_config`, `qdrant_semantic_cache_embedding_model`, `qdrant_semantic_cache_vector_size`
  - `azure_account_url`, `azure_blob_container` (Azure Blob variant)
  - `similarity_threshold`, `redis_semantic_cache_embedding_model`, `redis_semantic_cache_index_name`, `valkey_semantic_cache_embedding_model`, `valkey_semantic_cache_index_name`

## Exact YAML examples (verbatim — preserve indentation)
None on this page — this is the SDK-side documentation; the page uses Python code blocks instead of YAML for cache configuration. The "Cache Initialization Parameters" section is rendered as a Python `__init__` signature, not YAML.
(source: https://docs.litellm.ai/docs/caching/all_caches)

## Exact environment variables
- `REDIS_HOST`, `REDIS_PORT`, `REDIS_PASSWORD` — Redis SDK config
- `REDIS_GCP_SERVICE_ACCOUNT`, `REDIS_SSL` — Redis GCP IAM
- `VALKEY_HOST`, `VALKEY_PORT`, `VALKEY_PASSWORD` — Valkey SDK config (falls back to `REDIS_*` if unset)
- `QDRANT_API_BASE`, `QDRANT_API_KEY` — Qdrant
- `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` — S3
- `GCS_BUCKET_NAME`, `GCS_PATH_SERVICE_ACCOUNT` — GCS
- `GOOGLE_APPLICATION_CREDENTIALS` — GCP credentials

## Exact endpoint paths / API routes (runtime + management)
- none — this is the SDK-side doc; no proxy HTTP endpoints

## Exact CLI commands
- `uv add redis` — install Redis client for SDK caching
- `uv add redisvl==0.4.1` — install redisvl for redis-semantic cache
- `uv add google-cloud-iam` — install for GCP IAM Redis auth
- `uv add boto3` — install for S3 cache
- `uv add azure-storage-blob azure-identity` — install for Azure Blob cache
- `uv add "litellm[caching]"` — install disk cache extra

## Requirements
- database: not documented on this page
- redis: required only for `type: "redis"` and `type: "redis-semantic"` — verbatim: "Install redis: `uv add redis`". Cache backends that work WITHOUT Redis: `local`, `s3`, `gcs`, `disk`, `azure-blob`, `qdrant-semantic`, `valkey-semantic`
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- "If you need to pass non-string Redis parameters (integers, booleans, complex objects), avoid `REDIS_*` environment variables as they may fail during Redis client initialization. Instead, pass them directly as kwargs to the `Cache()` constructor."
- "ElastiCache **Serverless does not support vector search**, so a serverless endpoint will not work here."
- "Multi-shard (cluster-mode-enabled) endpoints are not supported by this backend"
- "RediSearch and RedisVL are not required; LiteLLM drives valkey-search directly over the Redis protocol."
- "If you run the code two times, response1 will use the cache from the first run that was stored in a cache file." (disk cache)

## Related links
- https://github.com/BerriAI/litellm/blob/main/litellm/caching/caching.py
- /docs/proxy/caching (Proxy caching)
- /docs/completion/prompt_caching

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- This is the SDK-side cache doc. For the workestrator (proxy in-memory), the relevant cache types that work WITHOUT Redis are: `local` (in-memory), `disk`, `s3`, `gcs`, `azure-blob`, `qdrant-semantic`, `valkey-semantic`. The proxy config uses `litellm_settings.cache_params.type` (see p2-feature-caching.md). `local` is the simplest for in-memory deployment. This page confirms `azure-blob` as an additional option not shown on the proxy caching page.

## Confidence / uncertainty notes
- high confidence on cache type list and Redis requirements. `azure-blob` is documented only in SDK code examples (not in proxy config YAML) — may not be available as a proxy `cache_params.type` (inferred; not confirmed on proxy page).
