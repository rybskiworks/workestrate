# litellm_settings

> Source: https://docs.litellm.ai/docs/proxy/config_settings (+ configs, caching, logging, prometheus, virtual_keys, users, docker_quick_start, alerting)

## Purpose

Module-level LiteLLM proxy settings — logging/callbacks, networking, caching, cost tracking, fallbacks (overlap with `router_settings`), MCP aliases, debugging toggles, virtual-key generation, and Prometheus integration.

## Key table

| key | type | default | requires_db | requires_redis | enterprise | deprecated | replacement | source |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| success_callback | list | — | false | false | false | false | — | config_settings, logging |
| failure_callback | list | — | false | false | false | false | — | config_settings |
| callbacks | list | — | false | false | false | false | — | config_settings, logging, prometheus |
| service_callbacks | list | — | false | false | false | false | — | config_settings |
| service_callback | list | — | false | false | false | false | — | prometheus |
| turn_off_message_logging | bool | — | false | false | false | false | — | config_settings, logging |
| modify_params | bool | — | false | false | false | false | — | config_settings |
| enable_preview_features | bool | — | false | false | false | false | — | config_settings |
| redact_user_api_key_info | bool | — | false | false | false | false | — | config_settings, logging |
| redact_messages_in_exceptions | bool | — | false | false | false | false | — | alerting |
| mcp_aliases | dict | — | false | false | false | false | — | config_settings |
| langfuse_default_tags | list | — | false | false | false | false | — | config_settings |
| set_verbose | bool | — | false | false | false | **true** | `LITELLM_LOG` / `--debug` | config_settings, caching, virtual_keys |
| json_logs | bool | — | false | false | false | false | — | config_settings |
| global_disable_no_log_param | bool | — | false | false | false | false | — | logging |
| forward_traceparent_to_llm_provider | bool | — | false | false | false | false | — | logging |
| default_fallbacks | list | — | false | false | false | false | — | config_settings `[also router_settings]` |
| request_timeout | int | 6000 (seconds) | false | false | false | false | — | config_settings, configs |
| force_ipv4 | bool | — | false | false | false | false | — | config_settings |
| content_policy_fallbacks | list | — | false | false | false | false | — | config_settings `[also router_settings]` |
| context_window_fallbacks | list | — | false | false | false | false | — | config_settings, configs `[also router_settings]` |
| fallbacks | list | — | false | false | false | false | — | configs `[also router_settings]` |
| num_retries | int | 3 | false | false | false | false | — | configs `[also router_settings]` |
| allowed_fails | int | — | false | false | false | false | — | configs `[also router_settings]` |
| cache | bool | — | false | false (if type=local/disk/s3/gcs) | false | false | — | config_settings, caching |
| cache_params | dict | — | false | see type | false | false | — | config_settings, caching |
| enable_redis_auth_cache | bool | false | false | **true** | false | false | — | config_settings, caching |
| enable_caching_on_provider_specific_optional_params | bool | — | false | false | false | false | — | caching |
| disable_end_user_cost_tracking | bool | — | false | false | false | false | — | config_settings |
| enable_end_user_cost_tracking_prometheus_only | bool | false (disabled by default) | false | false | false | false | — | config_settings, prometheus |
| cost_discount_config | dict | — | false | false | false | false | — | config_settings |
| cost_margin_config | dict | — | false | false | false | false | — | config_settings |
| key_generation_settings | dict | — | true | false | false | false | — | config_settings, virtual_keys |
| upperbound_key_generate_params | dict | — | true | false | false | false | — | virtual_keys |
| default_key_generate_params | dict | — | true | false | false | false | — | virtual_keys |
| disable_add_transform_inline_image_block | bool | — | false | false | false | false | — | config_settings |
| use_chat_completions_url_for_anthropic_messages | bool | — | false | false | false | false | — | config_settings |
| route_all_chat_openai_to_responses | bool | — | false | false | false | false | — | config_settings |
| skip_system_message_in_guardrail | bool | — | false | false | false | false | — | config_settings |
| disable_hf_tokenizer_download | bool | — | false | false | false | false | — | config_settings |
| enable_json_schema_validation | bool | — | false | false | false | false | — | config_settings |
| enable_key_alias_format_validation | bool | — | false | false | false | false | — | config_settings |
| require_managed_files | bool | — | false | false | false | false | — | config_settings |
| user_url_validation | bool | — | false | false | false | false | — | config_settings |
| user_url_allowed_hosts | list | — | false | false | false | false | — | config_settings |
| disable_copilot_system_to_assistant | bool | — | false | false | false | **true** | — | config_settings (under cache_params) |
| default_team_params | dict | — | true | false | false | false | — | config_settings |
| drop_params | bool | — | false | false | false | false | — | configs, virtual_keys |
| ssl_verify | bool | — | false | false | false | false | — | docker_quick_start |
| max_budget | float | null | true | false | false | false | — | users |
| budget_duration | string | — | true | false | false | false | — | users |
| max_end_user_budget | float | — | true | false | false | false | false | — | users |
| max_internal_user_budget | float | — | true | false | false | false | — | users |
| internal_user_budget_duration | string | — | true | false | false | false | — | users |
| s3_callback_params | dict | — | false | false | false | false | — | logging |
| aws_sqs_callback_params | dict | — | false | false | false | false | — | logging |
| prometheus_initialize_budget_metrics | bool | — | true | false | false | false | — | prometheus |
| prometheus_emit_stream_label | bool | — | false | false | false | false | — | prometheus |
| custom_prometheus_metadata_labels | list | — | false | false | false | false | — | prometheus |
| custom_prometheus_tags | list | — | false | false | false | false | — | prometheus |
| prometheus_metrics_config | list | — | false | false | false | false | — | prometheus |
| require_auth_for_metrics_endpoint | bool | — | false | false | false | false | — | prometheus |

## cache_params sub-keys

| key | type | default | requires_redis | source |
| --- | --- | --- | --- | --- |
| cache_params.type | enum: `local`,`redis`,`redis-semantic`,`valkey-semantic`,`qdrant-semantic`,`s3`,`gcs`,`disk` | — | only redis/redis-semantic | config_settings, caching |
| cache_params.host | string | — | true (redis) | config_settings, caching |
| cache_params.port | int | — | true (redis) | config_settings, caching |
| cache_params.password | string | — | true (redis) | config_settings, caching |
| cache_params.namespace | string | — | false | config_settings, caching |
| cache_params.ttl | float | — | false | config_settings, caching |
| cache_params.default_in_memory_ttl | float | None | false | caching |
| cache_params.default_in_redis_ttl | float | None | true | caching |
| cache_params.max_connections | int | — | true | config_settings, caching |
| cache_params.supported_call_types | list | — | false | config_settings, caching |
| cache_params.mode | enum: `default_off` | — | false | config_settings, caching |
| cache_params.redis_startup_nodes | list | — | true | config_settings |
| cache_params.service_name | string | — | true (sentinel) | config_settings |
| cache_params.sentinel_nodes | list | — | true (sentinel) | config_settings |
| cache_params.sentinel_password | string | — | true (sentinel) | caching |
| cache_params.gcp_service_account | string | — | false | config_settings |
| cache_params.gcp_ssl_ca_certs | string | — | false | config_settings |
| cache_params.ssl | bool | — | false | config_settings |
| cache_params.ssl_cert_reqs | — | null | false | config_settings |
| cache_params.ssl_check_hostname | bool | — | false | config_settings |
| cache_params.similarity_threshold | float | — | false | config_settings, caching |
| cache_params.redis_semantic_cache_embedding_model | string | — | true | caching |
| cache_params.valkey_semantic_cache_embedding_model | string | — | false | caching |
| cache_params.valkey_semantic_cache_index_name | string | — | false | caching |
| cache_params.qdrant_semantic_cache_embedding_model | string | — | false | config_settings, caching |
| cache_params.qdrant_collection_name | string | — | false | config_settings, caching |
| cache_params.qdrant_quantization_config | string | — | false | config_settings |
| cache_params.qdrant_semantic_cache_vector_size | int | — | false | config_settings |
| cache_params.s3_bucket_name | string | — | false | config_settings, caching |
| cache_params.s3_region_name | string | — | false | config_settings, caching |
| cache_params.s3_api_version | string | — | false | caching |
| cache_params.s3_use_ssl | bool | — | false | caching |
| cache_params.s3_verify | bool | — | false | caching |
| cache_params.s3_endpoint_url | string | — | false | config_settings, caching |
| cache_params.s3_aws_access_key_id | string | — | false | config_settings, caching |
| cache_params.s3_aws_secret_access_key | string | — | false | config_settings, caching |
| cache_params.s3_aws_session_token | string | — | false | caching |
| cache_params.gcs_bucket_name | string | — | false | config_settings, caching |
| cache_params.gcs_path_service_account | string | — | false | config_settings, caching |
| cache_params.gcs_path | string | — | false | config_settings, caching |
| cache_params.disk_cache_dir | string | `./.litellm_cache` (inferred) | false | caching |

## Verified behavior notes

(All quotes verbatim from `docs/litellm/extracted/p0-config_settings.md`.)

- **request_timeout** — "If not set, the default value is `6000 seconds`. [For reference OpenAI Python SDK defaults to `600 seconds`.]"
- **drop_params** — "True in litellm_settings — drops params not supported by the chosen LLM rather than failing."
- **set_verbose** — deprecated; replacement `LITELLM_LOG` / `--debug`.
- **disable_copilot_system_to_assistant** — deprecated (under `cache_params`).
- **enable_redis_auth_cache** — requires `cache: true` + `cache_params.type: redis` (requires_redis=true).
- **enable_end_user_cost_tracking_prometheus_only** — "Disabled by default to keep Prometheus cardinality bounded."
- **Overlap rule (verbatim):** "Most values can also be set via `litellm_settings`. If you see overlapping values, settings on `router_settings` will override those on `litellm_settings`." Overlap keys: `num_retries`, `timeout`, `fallbacks`, `context_window_fallbacks`, `content_policy_fallbacks`, `default_fallbacks`, `set_verbose` (deprecated), `cache`/`cache_responses`. When both set, `router_settings` wins.

## YAML example (verbatim)

```yaml
litellm_settings:
  # Logging/Callback settings
  success_callback: ["langfuse"]  # list of success callbacks
  failure_callback: ["sentry"]  # list of failure callbacks
  callbacks: ["otel"]  # list of callbacks - runs on success and failure
  service_callbacks: ["datadog", "prometheus"]  # logs redis, postgres failures on datadog, prometheus
  turn_off_message_logging: boolean  # prevent the messages and responses from being logged to on your callbacks, but request metadata will still be logged. Useful for privacy/compliance when handling sensitive data.
  redact_user_api_key_info: boolean  # Redact information about the user api key (hashed token, user_id, team id, etc.), from logs. Currently supported for Langfuse, OpenTelemetry, Logfire, ArizeAI logging.
  langfuse_default_tags: ["cache_hit", "cache_key", "proxy_base_url", "user_api_key_alias", "user_api_key_user_id", "user_api_key_user_email", "user_api_key_team_alias", "semantic-similarity", "proxy_base_url"] # default tags for Langfuse Logging
  # Networking settings
  request_timeout: 10 # (int) llm requesttimeout in seconds. Raise Timeout error if call takes longer than 10s. Sets litellm.request_timeout
  force_ipv4: boolean # If true, litellm will force ipv4 for all LLM requests. Some users have seen httpx ConnectionError when using ipv6 + Anthropic API
  # Cost tracking settings
  cost_discount_config:
    vertex_ai: 0.05 # Apply a 5% discount to Vertex AI costs
    gemini: 0.05 # Apply a 5% discount to Gemini costs
  cost_margin_config:
    global: 0.05 # Apply a 5% margin to all providers
    openai: 0.10 # Apply a 10% margin to OpenAI costs
    # Debugging - see debugging docs for more options
  # Use `--debug` or `--detailed_debug` CLI flags, or set LITELLM_LOG env var to "INFO", "DEBUG", or "ERROR"
  json_logs: boolean # if true, logs will be in json format
  # Fallbacks, reliability
  default_fallbacks: ["claude-opus"] # set default_fallbacks, in case a specific model group is misconfigured / bad.
  content_policy_fallbacks: [{ "gpt-3.5-turbo-small": ["claude-opus"] }] # fallbacks for ContentPolicyErrors
  context_window_fallbacks: [{ "gpt-3.5-turbo-small": ["gpt-3.5-turbo-large", "claude-opus"] }] # fallbacks for ContextWindowExceededErrors
  # MCP Aliases - Map aliases to MCP server names for easier tool access
  mcp_aliases: {
      "github": "github_mcp_server",
      "zapier": "zapier_mcp_server",
      "deepwiki": "deepwiki_mcp_server",
    } # Maps friendly aliases to MCP server names. Only the first alias for each server is used
  # Caching settings
  cache: true
  cache_params: # set cache params for redis
    type: redis # type of cache to initialize (options: "local", "redis", "s3", "gcs")
    host: "localhost"
    port: 6379
    password: "your_password"
    namespace: "litellm.caching.caching"
    max_connections: 100
    redis_startup_nodes: [{ "host": "127.0.0.1", "port": "7001" }]
    service_name: "mymaster"
    sentinel_nodes: [["localhost", 26379]]
    gcp_service_account: "projects/-/serviceAccounts/your-sa@project.iam.gserviceaccount.com"
    gcp_ssl_ca_certs: "./server-ca.pem"
    ssl: true
    ssl_cert_reqs: null
    ssl_check_hostname: false
    qdrant_semantic_cache_embedding_model: openai-embedding
    qdrant_collection_name: test_collection
    qdrant_quantization_config: binary
    qdrant_semantic_cache_vector_size: 1536
    similarity_threshold: 0.8
    s3_bucket_name: cache-bucket-litellm
    s3_region_name: us-west-2
    s3_aws_access_key_id: os.environ/AWS_ACCESS_KEY_ID
    s3_aws_secret_access_key: os.environ/AWS_SECRET_ACCESS_KEY
    s3_endpoint_url: https://s3.amazonaws.com
    gcs_bucket_name: cache-bucket-litellm
    gcs_path_service_account: os.environ/GCS_PATH_SERVICE_ACCOUNT
    gcs_path: cache/
    supported_call_types:
      ["acompletion", "atext_completion", "aembedding", "atranscription"]
    mode: default_off # if default_off, you need to opt in to caching on a per call basis
    ttl: 600
    disable_copilot_system_to_assistant: False # DEPRECATED
  enable_redis_auth_cache: false
```

## Pitfalls

- `set_verbose` is deprecated — use `LITELLM_LOG` env var or `--debug` / `--detailed_debug` CLI flags instead.
- `disable_copilot_system_to_assistant` is deprecated (under `cache_params`).
- Overlap keys set in both `litellm_settings` and `router_settings` are silently overridden by `router_settings` — not an error, but easy to misread.
- `enable_redis_auth_cache` requires Redis (`requires_redis=true`); do not enable without a Redis backend.
- `enable_end_user_cost_tracking_prometheus_only` is disabled by default to keep Prometheus cardinality bounded — enabling it can blow up metric cardinality.
- `key_generation_settings`, `upperbound_key_generate_params`, `default_key_generate_params`, `max_budget`, `budget_duration`, `default_team_params`, `prometheus_initialize_budget_metrics` all require a DB (`requires_db=true`) — they are no-ops (or startup failures) without `database_url`.
- `request_timeout` default (6000s) is much higher than the OpenAI SDK default (600s) — easy to assume the SDK default applies.

## Reference

- Authoritative key index: [`config-yaml.option-index.json`](../schemas/config-yaml.option-index.json)
- Normalized schema: [`config-yaml.normalized.schema.md`](../schemas/config-yaml.normalized.schema.md)

## Workestrate notes

> **PROJECT CONTEXT** — not upstream LiteLLM docs. Describes the workestrate deployment specifically.

Real config (`infra/litellm/config.yaml`) uses only:

```yaml
litellm_settings:
  drop_params: true
  request_timeout: 300
  force_ipv4: true
```

- `drop_params: true` — drops unsupported params rather than failing (matches `LITELLM_DROP_PARAMS` env equivalent, but set via config).
- `request_timeout: 300` — deliberate override of the 6000s default.
- `force_ipv4: true` — avoids `httpx ConnectionError` seen with IPv6 + Anthropic API (relevant since `anthropic/` prefix is used for Kimi + MiniMax).
- **No `cache` / `cache_params`** — in-memory deployment, no Redis. Caching is entirely off.
- **No callbacks** (`success_callback`, `failure_callback`, `callbacks`, `service_callbacks`) — no observability backend wired.
- **No `key_generation_settings` / `default_key_generate_params` / `upperbound_key_generate_params`** — no DB, so virtual-key generation features are unavailable.
- **`set_verbose` NOT used** — deprecated; `LITELLM_LOG` is the recommended replacement (not currently set).
- **Overlap rule consistency:** the real config places `fallbacks`, `num_retries`, `timeout`, `stream_timeout`, `allowed_fails`, `cooldown_time`, `retry_policy` under `router_settings` (not `litellm_settings`), which is consistent with the precedence rule (`router_settings` wins on overlap).
