---
source_url: https://docs.litellm.ai/docs/proxy/caching
canonical_url: https://docs.litellm.ai/docs/proxy/caching
title: "Caching"
sidebar_section_path: proxy > caching
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: caching
---
# Caching

## Headings
- Supported Caches
- Virtual Key Authentication Cache (Redis)
- Quick Start
- Namespace
- Redis Cluster
- Redis Sentinel
- TTL
- SSL
- GCP IAM Authentication
- Qdrant Semantic cache
- Valkey Semantic cache
- S3 Bucket cache
- GCS Bucket cache
- Redis Semantic cache
- In-Memory (Local) cache
- Disk cache
- Usage
- Basic
- Dynamic Cache Controls
- Set cache for proxy, but not on the actual llm api call
- Debugging Caching - /cache/ping
- Advanced
- Control Call Types Caching is on for
- Set Cache Params on config.yaml
- Deleting Cache Keys - /cache/delete
- Viewing Cache Keys from responses
- Set Caching Default Off - Opt in only
- Redis max_connections
- Supported cache_params on proxy config.yaml
- Provider-Specific Optional Parameters Caching
- Advanced - user api key cache ttl

## Exact config keys found (full path, verbatim spelling)
- `litellm_settings.cache` (litellm_settings) — bool; default True on redis when enabled
- `litellm_settings.cache_params` (litellm_settings) — dict; cache backend config
- `litellm_settings.cache_params.type` (litellm_settings.cache_params) — "local" | "redis" | "redis-semantic" | "valkey-semantic" | "qdrant-semantic" | "s3" | "gcs" | "disk"
- `litellm_settings.cache_params.namespace` — string prefix
- `litellm_settings.cache_params.ttl` — float seconds
- `litellm_settings.cache_params.default_in_memory_ttl` — float
- `litellm_settings.cache_params.default_in_redis_ttl` — float
- `litellm_settings.cache_params.max_connections` — int (Redis)
- `litellm_settings.cache_params.supported_call_types` — list
- `litellm_settings.cache_params.mode` — "default_off"
- `litellm_settings.cache_params.host` / `port` / `password` (redis)
- `litellm_settings.cache_params.redis_startup_nodes` — list of {host, port}
- `litellm_settings.cache_params.service_name` (redis sentinel)
- `litellm_settings.cache_params.sentinel_nodes` (redis sentinel)
- `litellm_settings.cache_params.sentinel_password` (redis sentinel)
- `litellm_settings.cache_params.gcp_service_account` (GCP IAM)
- `litellm_settings.cache_params.gcp_ssl_ca_certs` (GCP IAM)
- `litellm_settings.cache_params.ssl` / `ssl_cert_reqs` / `ssl_check_hostname` (SSL)
- `litellm_settings.cache_params.s3_bucket_name` / `s3_region_name` / `s3_api_version` / `s3_use_ssl` / `s3_verify` / `s3_endpoint_url` / `s3_aws_access_key_id` / `s3_aws_secret_access_key` / `s3_aws_session_token` (S3)
- `litellm_settings.cache_params.gcs_bucket_name` / `gcs_path_service_account` / `gcs_path` (GCS)
- `litellm_settings.cache_params.similarity_threshold` (semantic caches)
- `litellm_settings.cache_params.redis_semantic_cache_embedding_model` (redis-semantic)
- `litellm_settings.cache_params.valkey_semantic_cache_embedding_model` (valkey-semantic)
- `litellm_settings.cache_params.valkey_semantic_cache_index_name` (valkey-semantic)
- `litellm_settings.cache_params.qdrant_semantic_cache_embedding_model` (qdrant-semantic)
- `litellm_settings.cache_params.qdrant_collection_name` (qdrant-semantic)
- `litellm_settings.cache_params.qdrant_quantization_config` (qdrant-semantic)
- `litellm_settings.cache_params.qdrant_semantic_cache_vector_size` (qdrant-semantic)
- `litellm_settings.cache_params.disk_cache_dir` (disk)
- `litellm_settings.enable_redis_auth_cache` (litellm_settings) — bool
- `litellm_settings.enable_caching_on_provider_specific_optional_params` (litellm_settings) — bool
- `litellm_settings.set_verbose` (litellm_settings) — bool
- `general_settings.user_api_key_cache_ttl` (general_settings) — int seconds (default 60)

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Quick Start - default Redis cache
model_list:
  - model_name: gpt-3.5-turbo
    litellm_params:
      model: gpt-3.5-turbo
  - model_name: text-embedding-ada-002
    litellm_params:
      model: text-embedding-ada-002
litellm_settings:
  set_verbose: True
  cache: True # set cache responses to True, litellm defaults to using a redis cache
```
(source: https://docs.litellm.ai/docs/proxy/caching)

```yaml
# In-Memory (Local) cache - NO REDIS REQUIRED
litellm_settings:
  cache: True
  cache_params:
    type: local
```
(source: https://docs.litellm.ai/docs/proxy/caching)

```yaml
# Disk cache - NO REDIS REQUIRED
litellm_settings:
  cache: True
  cache_params:
    type: disk
    disk_cache_dir: /tmp/litellm-cache # OPTIONAL, default to ./.litellm_cache
```
(source: https://docs.litellm.ai/docs/proxy/caching)

```yaml
# S3 Bucket cache - NO REDIS REQUIRED
model_list:
  - model_name: gpt-3.5-turbo
    litellm_params:
      model: gpt-3.5-turbo
  - model_name: text-embedding-ada-002
    litellm_params:
      model: text-embedding-ada-002
litellm_settings:
  set_verbose: True
  cache: True # set cache responses to True
  cache_params: # set cache params for s3
    type: s3
    s3_bucket_name: cache-bucket-litellm # AWS Bucket Name for S3
    s3_region_name: us-west-2 # AWS Region Name for S3
    s3_aws_access_key_id: os.environ/AWS_ACCESS_KEY_ID # us os.environ/<variable name> to pass environment variables. This is AWS Access Key ID for S3
    s3_aws_secret_access_key: os.environ/AWS_SECRET_ACCESS_KEY # AWS Secret Access Key for S3
    s3_endpoint_url: https://s3.amazonaws.com # [OPTIONAL] S3 endpoint URL, if you want to use Backblaze/cloudflare s3 buckets
```
(source: https://docs.litellm.ai/docs/proxy/caching)

```yaml
# Namespace
litellm_settings:
  cache: true
  cache_params: # set cache params for redis
    type: redis
    namespace: "litellm.caching.caching"
```
(source: https://docs.litellm.ai/docs/proxy/caching)

```yaml
# TTL
litellm_settings:
  cache: true
  cache_params: # set cache params for redis
    type: redis
    ttl: 600 # will be cached on redis for 600s
    # default_in_memory_ttl: Optional[float], default is None. time in seconds.
    # default_in_redis_ttl: Optional[float], default is None. time in seconds.
```
(source: https://docs.litellm.ai/docs/proxy/caching)

```yaml
# Set Caching Default Off - Opt in only
model_list:
  - model_name: fake-openai-endpoint
    litellm_params:
      model: openai/fake
      api_key: fake-key
      api_base: https://exampleopenaiendpoint-production.up.railway.app/
# default off model
litellm_settings:
  set_verbose: True
  cache: True
  cache_params:
    mode: default_off # 👈 Key change cache is default_off
```
(source: https://docs.litellm.ai/docs/proxy/caching)

```yaml
# Virtual Key Authentication Cache (Redis)
litellm_settings:
  cache: true
  enable_redis_auth_cache: true
  cache_params:
    type: redis
    host: os.environ/REDIS_HOST
    port: 6379
general_settings:
  user_api_key_cache_ttl: 300 # optional; seconds
```
(source: https://docs.litellm.ai/docs/proxy/caching)

```yaml
# Supported cache_params on proxy config.yaml (reference)
cache_params:
  # ttl
  ttl: Optional[float]
  default_in_memory_ttl: Optional[float]
  default_in_redis_ttl: Optional[float]
  max_connections: Optional[Int]
  # Type of cache (options: "local", "redis", "s3", "gcs")
  type: s3
  # List of litellm call types to cache for
  supported_call_types:
    ["acompletion", "atext_completion", "aembedding", "atranscription"]
    # /chat/completions, /completions, /embeddings, /audio/transcriptions
  # Redis cache parameters
  host: localhost # Redis server hostname or IP address
  port: "6379" # Redis server port (as a string)
  password: secret_password # Redis server password
  namespace: Optional[str] = None,
  # S3 cache parameters
  s3_bucket_name: your_s3_bucket_name
  s3_region_name: us-west-2
  s3_endpoint_url: https://s3.amazonaws.com
  s3_aws_access_key_id: your_access_key
  s3_aws_secret_access_key: your_secret_key
  # GCS cache parameters
  gcs_bucket_name: your_gcs_bucket_name
  gcs_path_service_account: /path/to/service-account.json
  gcs_path: cache/ # [OPTIONAL] GCS path prefix for cache objects
```
(source: https://docs.litellm.ai/docs/proxy/caching)

## Exact environment variables
- `REDIS_URL` — full Redis URL (e.g. `redis://username:password@hostname:port/database`)
- `REDIS_HOST` — Redis host
- `REDIS_PORT` — Redis port
- `REDIS_PASSWORD` — Redis password
- `REDIS_USERNAME` — optional Redis username
- `REDIS_SSL` — "True"/"False" to enable SSL
- `REDIS_CONNECTION_POOL_KWARGS` — JSON string for pool kwargs
- `REDIS_CLUSTER_NODES` — JSON list of {host,port}
- `REDIS_SENTINEL_NODES` — JSON list of [host, port]
- `REDIS_SERVICE_NAME` — Sentinel master service name
- `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` — S3 creds
- `GCS_BUCKET_NAME` / `GCS_PATH_SERVICE_ACCOUNT` — GCS cache creds
- `QDRANT_API_KEY` / `QDRANT_API_BASE` — Qdrant
- `VALKEY_HOST` / `VALKEY_PORT` / `VALKEY_PASSWORD` — Valkey connection

## Exact endpoint paths / API routes (runtime + management)
- `POST /cache/ping` — health-check the cache backend; returns `status`, `cache_type`, `ping_response`, `set_cache_response`, `litellm_cache_params`, `redis_cache_params`
- `POST /cache/delete` — delete cache keys; body `{"keys": [...]}`
- `POST /v1/chat/completions` — supports per-request `cache: {ttl|s-maxage|no-cache|no-store|namespace|use-cache}`
- `POST /embeddings` — supports the same per-request `cache` controls

## Exact CLI commands
- `litellm --config /path/to/config.yaml` — start the proxy with the given config

## Requirements
- database: not documented on this page (caching does not mention Postgres or any DB requirement)
- redis: required only for `type: redis` / `redis-semantic` / `enable_redis_auth_cache` — verbatim: "`litellm_settings.cache` must be **`true`** (Redis for the proxy is initialized during cache setup)." Also: "`cache_params.type` must be **`redis`** (or Redis Cluster, per your cache config); the auth cache attaches to that Redis client." Also: "litellm defaults to using a redis cache". Caches that work WITHOUT Redis: `local` (in-memory), `disk`, `s3`, `gcs`, `qdrant-semantic`, `valkey-semantic`
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- "For non-string Redis parameters (like integers, booleans, or complex objects), avoid using `REDIS_*` environment variables as they may fail during Redis client initialization. Instead, use `cache_kwargs` in your router configuration for such parameters."
- "For quick testing, you can also use REDIS_URL, eg.: `REDIS_URL="rediss://.."` but we **don't** recommend using REDIS_URL in prod. We've noticed a performance difference between using it vs. redis_host, port, etc."
- "If you see errors like `No connection available`, try increasing [`max_connections`]"
- "By default this value is set to 60s." (user_api_key_cache_ttl)
- "Set `supported_call_types: []` to disable caching on the actual api call."
- `enable_redis_auth_cache` is OPTIONAL (off by default), only matters for sharing virtual-key auth lookups across workers

## Related links
- /docs/completion/prompt_caching
- /docs/proxy/config_settings
- /docs/caching/all_caches
- https://litellm-api.up.railway.app/ (Swagger)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- CRITICAL: caching works WITHOUT Redis using `type: local` (in-memory) or `type: disk`. The workestrator runs in-memory (no Redis), so `cache_params.type: local` is the viable caching option. `type: disk` also works (writes to local disk). `type: s3` / `type: gcs` work without Redis but require cloud credentials. `enable_redis_auth_cache` is unavailable (requires Redis) but is OPTIONAL and only relevant for multi-worker virtual-key auth caching (moot without virtual keys/DB anyway). `general_settings.user_api_key_cache_ttl` controls master_key auth cache TTL — relevant even in-memory. Per-request cache controls (`cache: {ttl, no-cache, no-store}`) work with any backend including local.

## Confidence / uncertainty notes
- high confidence on cache type options and which work without Redis (verbatim YAML blocks for `local` and `disk`). The `local` cache is in-process memory (not shared across workers) — fine for single-process in-memory deployment. `user_api_key_cache_ttl` default 60s is verbatim.
