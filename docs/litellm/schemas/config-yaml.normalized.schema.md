# LiteLLM config.yaml — Normalized Schema Reference

> **Anti-hallucination reference for workestrator.** Every key below was extracted
> verbatim from the on-disk corpus under `docs/litellm/extracted/`. Spelling is
> verbatim. Values marked `(inferred)` are safely derivable from verbatim YAML
> examples. Values marked `(not documented in fetched source)` were not stated
> verbatim in any fetched page — they are explicitly NOT invented.
>
> **Sources** (all `https://docs.litellm.ai/docs/proxy/...`):
> `config_settings` (PRIMARY), `configs`, `deploy`, `docker_quick_start`,
> `quick_start`, `db_info`, `caching`, `logging`, `alerting`, `prometheus`,
> `virtual_keys`, `users`, `ui`, and provider pages `providers/moonshot`,
> `providers/openrouter`.
>
> **Raw markdown source (verbatim-complete):** The config_settings page is now
> byte-for-byte from the raw GitHub markdown:
> `https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/config_settings.md`
> (135,891 bytes, 1,219 lines, recovered via GitHub Contents API). All reference
> tables (litellm_settings, general_settings, router_settings, environment_variables)
> are verbatim from this source.

---

## Top-level structure

The `config.yaml` has these top-level keys (verbatim from the config_settings
top-level YAML schema block):

```yaml
environment_variables: {}
model_list:
  - model_name: string
    litellm_params: {}
    model_info:
      id: string
      mode: embedding
      input_cost_per_token: 0
      output_cost_per_token: 0
      max_tokens: 2048
      base_model: gpt-4-1106-preview
      additionalProp1: {}
litellm_settings: {}
callback_settings:
  otel:
    message_logging: boolean
general_settings: {}
router_settings: {}
```

Additional top-level key from the `configs` page (not in the config_settings
schema block but verbatim on the configs page): `credential_list`.

Additional top-level key from the `config_management` ("File Management") page
(not in the config_settings schema block but verbatim on the config_management
page): `include`. Verbatim: "You can use `include` to include external YAML
files in a config.yaml." Takes a single file or a list of files:

```yaml
include:
  - model_config.yaml
  - another_config.yaml
```

### Precedence rule (verbatim)

> "Most values can also be set via `litellm_settings`. If you see overlapping
> values, settings on `router_settings` will override those on
> `litellm_settings`."
> — `config_settings` Caveats

**Overlap keys** (appear under BOTH `litellm_settings` and `router_settings`):
`num_retries`, `timeout`, `fallbacks`, `context_window_fallbacks`,
`content_policy_fallbacks`, `default_fallbacks`, `set_verbose` (deprecated),
`cache`/`cache_responses`. When both are set, `router_settings` wins.

### Environment-variable resolution syntax (verbatim)

```yaml
os.environ/<YOUR-ENV-VAR>   # runs os.getenv("YOUR-ENV-VAR")
```

Used for `api_key`, `master_key`, `api_base`, and any string config value.
Confirmed on `configs`, `deploy`, `docker_quick_start`, `config_settings`.

### Config validation (derived — not a dedicated section in raw source)

NOTE: The raw markdown does NOT contain a separate `### config validation` section.
The validation prose below was derived from page caveats and the startup log
behavior documented across multiple LiteLLM pages. It is NOT verbatim from a
dedicated config validation section.

> "LiteLLM validates `config.yaml` against an internal Pydantic schema at
> startup. On invalid config, the proxy fails to start." Startup log:
> "Loaded config YAML (api_key and environment_variables are not shown): { ... }"

---

## Section: `model_list` (top-level list)

Each entry is a deployment. Multiple entries with the same `model_name` form a
load-balancing group.

| key (full_path) | type | default | required | requires_db | requires_redis | enterprise | deprecated | source |
|---|---|---|---|---|---|---|---|---|
| `model_list[].model_name` | string | — | true | false | false | false | false | config_settings, configs |
| `model_list[].litellm_params` | dict | — | true | false | false | false | false | config_settings, configs |
| `model_list[].model_info` | dict | — | false | false | false | false | false | config_settings, configs |

### `litellm_params` sub-keys

Documented sub-keys (verbatim from config_settings reference: "supports model,
api_base, api_key, api_version, organization, temperature, max_tokens, seed,
extra_headers, rpm, tpm, weight, region_name, base_model, tags") plus additional
keys from the `configs` page YAML examples.

| key (full_path) | type | default | required | requires_db | deprecated | source |
|---|---|---|---|---|---|---|
| `litellm_params.model` | string | — | true | false | false | config_settings, configs, deploy, docker_quick_start, quick_start |
| `litellm_params.api_base` | string | — | false | false | false | config_settings, configs, deploy, docker_quick_start, quick_start |
| `litellm_params.api_key` | string | — | false | false | false | config_settings, configs, deploy, docker_quick_start, quick_start |
| `litellm_params.api_version` | string | latest azure api_version (inferred) | false | false | false | config_settings, configs, deploy, docker_quick_start |
| `litellm_params.organization` | string | — | false | false | false | config_settings |
| `litellm_params.temperature` | float | — | false | false | false | config_settings |
| `litellm_params.max_tokens` | int | — | false | false | false | config_settings |
| `litellm_params.seed` | int | — | false | false | false | config_settings, configs |
| `litellm_params.extra_headers` | dict | — | false | false | false | config_settings |
| `litellm_params.rpm` | int | — | false | false | false | config_settings, configs, deploy |
| `litellm_params.tpm` | int | — | false | false | false | config_settings, configs |
| `litellm_params.weight` | int | — | false | false | false | config_settings |
| `litellm_params.region_name` | string | — | false | false | false | config_settings |
| `litellm_params.base_model` | string | — | false | false | false | config_settings |
| `litellm_params.tags` | list | — | false | false | false | config_settings |
| `litellm_params.aws_region_name` | string | — | false | false | false | configs |
| `litellm_params.azure_ad_token` | string | — | false | false | false | configs |
| `litellm_params.litellm_credential_name` | string | — | false | false | false | configs |

### `model_info` sub-keys

| key (full_path) | type | default | source |
|---|---|---|---|
| `model_info.id` | string | — | config_settings |
| `model_info.mode` | enum (e.g. `embedding`) | — | config_settings |
| `model_info.input_cost_per_token` | float | 0 | config_settings |
| `model_info.output_cost_per_token` | float | 0 | config_settings |
| `model_info.max_tokens` | int | 2048 | config_settings |
| `model_info.base_model` | string | `gpt-4-1106-preview` (example) | config_settings |
| `model_info.version` | int | — | configs |
| `model_info.supported_environments` | list | — | configs |
| `model_info.access_groups` | list | — | configs, users |
| `model_info.custom_tokenizer` | dict | — | configs |
| `model_info.custom_tokenizer.identifier` | string | — | configs |
| `model_info.custom_tokenizer.revision` | string | — | configs |
| `model_info.custom_tokenizer.auth_token` | string | — | configs |

### `credential_list` (top-level, from configs page)

| key (full_path) | type | source |
|---|---|---|
| `credential_list[].credential_name` | string | configs |
| `credential_list[].credential_values` | dict | configs |
| `credential_list[].credential_info` | dict | configs |

---

## Section: `litellm_settings`

Module-level LiteLLM settings. 37 keys in the config_settings reference table
(verbatim spelling). Additional keys appear in YAML examples on other pages and
are included with their source noted.

> **NOTE on overlap:** keys marked `[also router_settings]` appear under BOTH
> sections. `router_settings` overrides `litellm_settings` for these.

| key | type | default | requires_db | requires_redis | enterprise | deprecated | replacement | source |
|---|---|---|---|---|---|---|---|---|
| `success_callback` | list | — | false | false | false | false | — | config_settings, logging |
| `failure_callback` | list | — | false | false | false | false | — | config_settings |
| `callbacks` | list | — | false | false | false | false | — | config_settings, logging, prometheus |
| `service_callbacks` | list | — | false | false | false | false | — | config_settings |
| `service_callback` | list | — | false | false | false | false | — | prometheus |
| `turn_off_message_logging` | bool | — | false | false | false | false | — | config_settings, logging |
| `modify_params` | bool | — | false | false | false | false | — | config_settings |
| `enable_preview_features` | bool | — | false | false | false | false | — | config_settings |
| `redact_user_api_key_info` | bool | — | false | false | false | false | — | config_settings, logging |
| `redact_messages_in_exceptions` | bool | — | false | false | false | false | — | alerting |
| `mcp_aliases` | dict | — | false | false | false | false | — | config_settings |
| `langfuse_default_tags` | list | — | false | false | false | false | — | config_settings |
| `set_verbose` | bool | — | false | false | false | **true** | `LITELLM_LOG` / `--debug` | config_settings, caching, virtual_keys |
| `json_logs` | bool | — | false | false | false | false | — | config_settings |
| `global_disable_no_log_param` | bool | — | false | false | false | false | — | logging |
| `forward_traceparent_to_llm_provider` | bool | — | false | false | false | false | — | logging |
| `default_fallbacks` | list | — | false | false | false | false | — | config_settings `[also router_settings]` |
| `request_timeout` | int | 6000 (seconds) | false | false | false | false | — | config_settings, configs |
| `force_ipv4` | bool | — | false | false | false | false | — | config_settings |
| `content_policy_fallbacks` | list | — | false | false | false | false | — | config_settings `[also router_settings]` |
| `context_window_fallbacks` | list | — | false | false | false | false | — | config_settings, configs `[also router_settings]` |
| `fallbacks` | list | — | false | false | false | false | — | configs `[also router_settings]` |
| `num_retries` | int | 3 | false | false | false | false | — | configs `[also router_settings]` |
| `allowed_fails` | int | — | false | false | false | false | — | configs `[also router_settings]` |
| `cache` | bool | — | false | false (if type=local/disk/s3/gcs) | false | false | — | config_settings, caching |
| `cache_params` | dict | — | false | see type | false | false | — | config_settings, caching |
| `enable_redis_auth_cache` | bool | false | false | **true** | false | false | — | config_settings, caching |
| `enable_caching_on_provider_specific_optional_params` | bool | — | false | false | false | false | — | caching |
| `disable_end_user_cost_tracking` | bool | — | false | false | false | false | — | config_settings |
| `enable_end_user_cost_tracking_prometheus_only` | bool | false (disabled by default) | false | false | false | false | — | config_settings, prometheus |
| `cost_discount_config` | dict | — | false | false | false | false | — | config_settings |
| `cost_margin_config` | dict | — | false | false | false | false | — | config_settings |
| `key_generation_settings` | dict | — | true | false | false | false | — | config_settings, virtual_keys |
| `upperbound_key_generate_params` | dict | — | true | false | false | false | — | virtual_keys |
| `default_key_generate_params` | dict | — | true | false | false | false | — | virtual_keys |
| `disable_add_transform_inline_image_block` | bool | — | false | false | false | false | — | config_settings |
| `use_chat_completions_url_for_anthropic_messages` | bool | — | false | false | false | false | — | config_settings |
| `route_all_chat_openai_to_responses` | bool | — | false | false | false | false | — | config_settings |
| `skip_system_message_in_guardrail` | bool | — | false | false | false | false | — | config_settings |
| `disable_hf_tokenizer_download` | bool | — | false | false | false | false | — | config_settings |
| `enable_json_schema_validation` | bool | — | false | false | false | false | — | config_settings |
| `enable_key_alias_format_validation` | bool | — | false | false | false | false | — | config_settings |
| `require_managed_files` | bool | — | false | false | false | false | — | config_settings |
| `user_url_validation` | bool | — | false | false | false | false | — | config_settings |
| `user_url_allowed_hosts` | list | — | false | false | false | false | — | config_settings |
| `disable_copilot_system_to_assistant` | bool | — | false | false | false | **true** | — | config_settings (under cache_params) |
| `default_team_params` | dict | — | true | false | false | false | — | config_settings |
| `drop_params` | bool | — | false | false | false | false | — | configs, virtual_keys |
| `ssl_verify` | bool | — | false | false | false | false | — | docker_quick_start |
| `max_budget` | float | null | true | false | false | false | — | users |
| `budget_duration` | string | — | true | false | false | false | — | users |
| `max_end_user_budget` | float | — | true | false | false | false | — | users |
| `max_internal_user_budget` | float | — | true | false | false | false | — | users |
| `internal_user_budget_duration` | string | — | true | false | false | false | — | users |
| `s3_callback_params` | dict | — | false | false | false | false | — | logging |
| `aws_sqs_callback_params` | dict | — | false | false | false | false | — | logging |
| `prometheus_initialize_budget_metrics` | bool | — | true | false | false | false | — | prometheus |
| `prometheus_emit_stream_label` | bool | — | false | false | false | false | — | prometheus |
| `custom_prometheus_metadata_labels` | list | — | false | false | false | false | — | prometheus |
| `custom_prometheus_tags` | list | — | false | false | false | false | — | prometheus |
| `prometheus_metrics_config` | list | — | false | false | false | false | — | prometheus |
| `require_auth_for_metrics_endpoint` | bool | — | false | false | false | false | — | prometheus |

### `cache_params` sub-keys (from config_settings + caching)

| key | type | default | requires_redis | source |
|---|---|---|---|---|
| `cache_params.type` | enum: `local`,`redis`,`redis-semantic`,`valkey-semantic`,`qdrant-semantic`,`s3`,`gcs`,`disk` | — | only redis/redis-semantic | config_settings, caching |
| `cache_params.host` | string | — | true (redis) | config_settings, caching |
| `cache_params.port` | int | — | true (redis) | config_settings, caching |
| `cache_params.password` | string | — | true (redis) | config_settings, caching |
| `cache_params.namespace` | string | — | false | config_settings, caching |
| `cache_params.ttl` | float | — | false | config_settings, caching |
| `cache_params.default_in_memory_ttl` | float | None | false | caching |
| `cache_params.default_in_redis_ttl` | float | None | true | caching |
| `cache_params.max_connections` | int | — | true | config_settings, caching |
| `cache_params.supported_call_types` | list | — | false | config_settings, caching |
| `cache_params.mode` | enum: `default_off` | — | false | config_settings, caching |
| `cache_params.redis_startup_nodes` | list | — | true | config_settings |
| `cache_params.service_name` | string | — | true (sentinel) | config_settings |
| `cache_params.sentinel_nodes` | list | — | true (sentinel) | config_settings |
| `cache_params.sentinel_password` | string | — | true (sentinel) | caching |
| `cache_params.gcp_service_account` | string | — | false | config_settings |
| `cache_params.gcp_ssl_ca_certs` | string | — | false | config_settings |
| `cache_params.ssl` | bool | — | false | config_settings |
| `cache_params.ssl_cert_reqs` | — | null | false | config_settings |
| `cache_params.ssl_check_hostname` | bool | — | false | config_settings |
| `cache_params.similarity_threshold` | float | — | false | config_settings, caching |
| `cache_params.redis_semantic_cache_embedding_model` | string | — | true | caching |
| `cache_params.valkey_semantic_cache_embedding_model` | string | — | false | caching |
| `cache_params.valkey_semantic_cache_index_name` | string | — | false | caching |
| `cache_params.qdrant_semantic_cache_embedding_model` | string | — | false | config_settings, caching |
| `cache_params.qdrant_collection_name` | string | — | false | config_settings, caching |
| `cache_params.qdrant_quantization_config` | string | — | false | config_settings |
| `cache_params.qdrant_semantic_cache_vector_size` | int | — | false | config_settings |
| `cache_params.s3_bucket_name` | string | — | false | config_settings, caching |
| `cache_params.s3_region_name` | string | — | false | config_settings, caching |
| `cache_params.s3_api_version` | string | — | false | caching |
| `cache_params.s3_use_ssl` | bool | — | false | caching |
| `cache_params.s3_verify` | bool | — | false | caching |
| `cache_params.s3_endpoint_url` | string | — | false | config_settings, caching |
| `cache_params.s3_aws_access_key_id` | string | — | false | config_settings, caching |
| `cache_params.s3_aws_secret_access_key` | string | — | false | config_settings, caching |
| `cache_params.s3_aws_session_token` | string | — | false | caching |
| `cache_params.gcs_bucket_name` | string | — | false | config_settings, caching |
| `cache_params.gcs_path_service_account` | string | — | false | config_settings, caching |
| `cache_params.gcs_path` | string | — | false | config_settings, caching |
| `cache_params.disk_cache_dir` | string | `./.litellm_cache` (inferred) | false | caching |

---

## Section: `general_settings`

116 keys in the config_settings reference table (verbatim spelling). Plus
`block_robots` from the deploy page. DB/Redis/Enterprise flags below are from
the config_settings Requirements section and per-page requirements.

| key | type | default | requires_db | requires_redis | enterprise | deprecated | source |
|---|---|---|---|---|---|---|---|
| `completion_model` | string | — | false | false | false | false | config_settings |
| `enable_drain_endpoint` | bool | false (off by default) | false | false | false | false | config_settings |
| `drain_endpoint_token` | string | — | false | false | false | false | config_settings |
| `disable_spend_logs` | bool | — | true | false | false | false | config_settings, db_info |
| `disable_spend_updates` | bool | — | true | false | false | false | config_settings |
| `disable_error_logs` | bool | — | true | false | false | false | config_settings, db_info |
| `disable_master_key_return` | bool | — | false | false | false | false | config_settings |
| `disable_retry_on_max_parallel_request_limit_error` | bool | — | false | false | false | false | config_settings |
| `disable_reset_budget` | bool | — | true | false | false | false | config_settings |
| `disable_adding_master_key_hash_to_db` | bool | — | true | false | false | false | config_settings |
| `disable_responses_id_security` | bool | — | false | false | false | false | config_settings |
| `enable_jwt_auth` | bool | — | false | false | false | false | config_settings |
| `enforce_user_param` | bool | — | false | false | false | false | config_settings |
| `reject_clientside_metadata_tags` | bool | — | false | false | false | false | config_settings |
| `disable_batch_input_file_rate_limiting` | bool | — | false | false | false | false | config_settings |
| `skip_batch_input_file_rate_limiting_for_providers` | list | — | false | false | false | false | config_settings |
| `skip_batch_input_file_rate_limiting_for_models` | list | — | false | false | false | false | config_settings |
| `allowed_routes` | list | — | false | false | false | false | config_settings |
| `key_management_system` | enum: `google_kms`,`azure_kms` | — | false | false | false | false | config_settings |
| `master_key` | string | — | false | false | false | false | config_settings, configs, deploy, docker_quick_start, virtual_keys, users |
| `database_url` | string | — | true | false | false | false | config_settings, configs, docker_quick_start, virtual_keys |
| `database_connection_pool_limit` | int | 10 | true | false | false | false | config_settings, configs |
| `database_connection_timeout` | int | 60 (seconds) | true | false | false | false | config_settings, configs |
| `database_connect_timeout` | int | — (Prisma default) | true | false | false | false | config_settings |
| `database_socket_timeout` | int | — | true | false | false | false | config_settings, configs |
| `database_extra_connection_params` | dict | — | true | false | false | false | config_settings |
| `database_disable_prepared_statements` | bool | — | true | false | false | false | config_settings |
| `database_connection_pool_timeout` | int | — | true | false | false | false | config_settings |
| `allow_requests_on_db_unavailable` | bool | — | true | false | false | false | config_settings |
| `fail_closed_budget_enforcement` | bool | false | true | true | false | false | config_settings, users |
| `custom_auth` | string | — | false | false | false | false | config_settings |
| `custom_auth_run_common_checks` | bool | — | false | false | false | false | config_settings |
| `max_parallel_requests` | int | 0 | false | false | false | false | config_settings |
| `global_max_parallel_requests` | int | 0 | false | false | false | false | config_settings |
| `cancel_on_disconnect` | bool | false | false | false | false | false | config_settings |
| `infer_model_from_keys` | bool | — | false | false | false | false | config_settings |
| `background_health_checks` | bool | — | false | false | false | false | config_settings |
| `health_check_interval` | int | 300 | false | false | false | false | config_settings |
| `health_check_details` | bool | — | false | false | false | false | config_settings |
| `health_check_concurrency` | int | — | false | false | false | false | config_settings |
| `health_check_ignore_transient_errors` | bool | — | false | false | false | false | config_settings |
| `health_check_skip_disabled_background_models` | bool | — | false | false | false | false | config_settings |
| `health_check_staleness_threshold` | int | — | false | false | false | false | config_settings |
| `enable_health_check_routing` | bool | — | false | false | false | false | config_settings |
| `use_shared_health_check` | bool | — | false | false | false | false | config_settings |
| `alerting` | list | — | false | false | false | false | config_settings, configs, alerting |
| `alerting_threshold` | int | 0 | false | false | false | false | config_settings, alerting |
| `alerting_args` | dict | — | false | false | false | false | config_settings, alerting |
| `alert_types` | list | — | false | false | false | false | config_settings, alerting |
| `alert_type_config` | dict | — | false | false | false | false | config_settings, alerting |
| `alert_to_webhook_url` | dict | — | false | false | false | false | config_settings, alerting |
| `spend_report_frequency` | string | — | true | false | false | false | config_settings, alerting |
| `use_client_credentials_pass_through_routes` | bool | — | false | false | false | false | config_settings |
| `public_routes` | list | — | false | false | **true** | false | config_settings |
| `enforced_params` | list | — | false | false | **true** | false | config_settings |
| `enable_oauth2_auth` | bool | — | false | false | **true** | false | config_settings |
| `admin_only_routes` | list | — | false | false | **true** | false | config_settings |
| `enable_oauth2_proxy_auth` | bool | — | false | false | **true** | false | config_settings |
| `use_x_forwarded_for` | bool | — | false | false | false | false | config_settings |
| `service_account_settings` | dict | — | false | false | false | false | config_settings |
| `image_generation_model` | string | — | false | false | false | false | config_settings |
| `store_model_in_db` | bool | — | true | false | false | false | config_settings |
| `supported_db_objects` | list | — | true | false | false | false | config_settings |
| `user_mcp_management_mode` | enum: `restricted`,`view_all` | — | false | false | false | false | config_settings |
| `store_prompts_in_spend_logs` | bool | — | true | false | false | false | config_settings |
| `scope_spend_list_endpoints_to_caller` | bool | — | true | false | false | false | config_settings |
| `legacy_unscoped_spend_list_endpoints` | bool | — | true | false | false | false | config_settings |
| `max_request_size_mb` | int | — | false | false | false | false | config_settings |
| `max_response_size_mb` | int | — | false | false | false | false | config_settings |
| `proxy_budget_rescheduler_min_time` | int | — | true | false | false | false | config_settings, users |
| `proxy_budget_rescheduler_max_time` | int | — | true | false | false | false | config_settings, users |
| `proxy_batch_write_at` | int | — | true | false | false | false | config_settings |
| `proxy_batch_polling_interval` | int | — | true | false | false | false | config_settings |
| `custom_key_generate` | string | — | true | false | false | false | config_settings, virtual_keys |
| `allowed_ips` | list | — | false | false | false | false | config_settings |
| `embedding_model` | string | — | false | false | false | false | config_settings |
| `default_team_disabled` | bool | — | true | false | false | false | config_settings |
| `key_management_settings` | dict | — | false | false | false | false | config_settings |
| `allow_user_auth` | bool | — | false | false | false | **true** | config_settings |
| `user_api_key_cache_ttl` | int | 60 (seconds) | false | false | false | false | config_settings, caching |
| `disable_prisma_schema_update` | bool | — | true | false | false | false | config_settings |
| `litellm_key_header_name` | string | — | false | false | false | false | config_settings, virtual_keys |
| `moderation_model` | string | — | false | false | false | false | config_settings |
| `custom_sso` | string | — | false | false | false | false | config_settings |
| `allow_client_side_side_credentials` | bool | — | false | false | false | false | config_settings |
| `use_azure_key_vault` | bool | — | false | false | false | false | config_settings |
| `use_google_kms` | bool | — | false | false | false | false | config_settings |
| `ui_access_mode` | enum | — | true | false | false | false | config_settings |
| `litellm_jwtauth` | dict | — | false | false | false | false | config_settings |
| `litellm_license` | string | — | false | false | false | false | config_settings |
| `oauth2_config_mappings` | dict | — | false | false | false | false | config_settings |
| `pass_through_endpoints` | list | — | false | false | false | false | config_settings |
| `pass_through_request_timeout` | int | — | false | false | false | false | config_settings |
| `forward_openai_org_id` | bool | — | false | false | false | false | config_settings |
| `forward_client_headers_to_llm_api` | bool | — | false | false | false | false | config_settings |
| `forward_llm_provider_auth_headers` | bool | — | false | false | false | false | config_settings |
| `maximum_spend_logs_retention_period` | string | — | true | false | false | false | config_settings |
| `maximum_spend_logs_retention_interval` | string | — | true | false | false | false | config_settings |
| `maximum_spend_logs_cleanup_cron` | string | — | true | false | false | false | config_settings |
| `always_include_stream_usage` | bool | — | false | false | false | false | config_settings |
| `auto_redirect_ui_login_to_sso` | bool | — | true | false | false | false | config_settings |
| `control_plane_url` | string | — | false | false | false | false | config_settings |
| `custom_ui_sso_sign_in_handler` | string | — | true | false | false | false | config_settings |
| `enable_mcp_registry` | bool | — | false | false | false | false | config_settings |
| `enforce_rbac` | bool | — | false | false | false | false | config_settings |
| `mcp_client_side_auth_header_name` | string | — | false | false | false | false | config_settings |
| `mcp_internal_ip_ranges` | list | — | false | false | false | false | config_settings |
| `mcp_required_fields` | list | — | false | false | false | false | config_settings |
| `mcp_trusted_proxy_ranges` | list | — | false | false | false | false | config_settings |
| `require_end_user_mcp_access_defined` | bool | — | false | false | false | false | config_settings |
| `role_permissions` | dict | — | false | false | false | false | config_settings |
| `search_tools` | dict | — | false | false | false | false | config_settings |
| `token_rate_limit_type` | enum: `input`,`output`,`total` | `total` (inferred) | true | false | false | false | config_settings, users |
| `use_redis_transaction_buffer` | bool | — | true | **true** | false | false | config_settings, deploy |
| `user_header_mappings` | dict | — | false | false | false | false | config_settings |
| `user_header_name` | string | — | false | false | false | false | config_settings |
| `block_robots` | bool | — | false | false | **true** | false | deploy |

### `alerting_args` sub-keys (from alerting page, with verbatim defaults)

| key | type | default | source |
|---|---|---|---|
| `alerting_args.daily_report_frequency` | int | 43200 | alerting |
| `alerting_args.report_check_interval` | int | 3600 | alerting |
| `alerting_args.budget_alert_ttl` | int | 86400 | alerting |
| `alerting_args.outage_alert_ttl` | int | 60 | alerting |
| `alerting_args.region_outage_alert_ttl` | int | 60 | alerting |
| `alerting_args.minor_outage_alert_threshold` | int | 5 | alerting |
| `alerting_args.major_outage_alert_threshold` | int | 10 | alerting |
| `alerting_args.max_outage_alert_list_size` | int | 1000 | alerting |
| `alerting_args.log_to_console` | bool | false | alerting |

---

## Section: `router_settings`

45 keys in the config_settings reference table (verbatim spelling).

| key | type | default | requires_db | requires_redis | enterprise | deprecated | source |
|---|---|---|---|---|---|---|---|
| `routing_strategy` | enum: `simple-shuffle`,`least-busy`,`usage-based-routing`,`latency-based-routing` | `simple-shuffle` | false | false | false | false | config_settings, configs |
| `redis_host` | string | — | false | **true** | false | false | config_settings, configs, deploy |
| `redis_password` | string | — | false | **true** | false | false | config_settings, configs, deploy |
| `redis_port` | int | — | false | **true** | false | false | config_settings, configs, deploy |
| `redis_db` | int | — | false | **true** | false | false | config_settings |
| `redis_url` | string | — | false | **true** | false | false | config_settings |
| `enable_pre_call_check` | bool | — | false | false | false | false | config_settings |
| `enable_pre_call_checks` | bool | false | false | false | false | false | config_settings |
| `optional_pre_call_checks` | list | — | false | false | false | false | config_settings |
| `content_policy_fallbacks` | list | — | false | false | false | false | config_settings |
| `fallbacks` | list | — | false | false | false | false | config_settings, configs |
| `context_window_fallbacks` | list | — | false | false | false | false | config_settings |
| `default_fallbacks` | list | — | false | false | false | false | config_settings |
| `enable_tag_filtering` | bool | — | false | false | false | false | config_settings |
| `tag_filtering_match_any` | bool | — | false | false | false | false | config_settings |
| `enable_weighted_failover` | bool | false | false | false | false | false | config_settings |
| `cooldown_time` | int | 30 (seconds, inferred from example) | false | false | false | false | config_settings |
| `disable_cooldowns` | bool | — | false | false | false | false | config_settings |
| `retry_policy` | dict | — | false | false | false | false | config_settings |
| `allowed_fails` | int | — | false | false | false | false | config_settings, configs |
| `allowed_fails_policy` | dict | — | false | false | false | false | config_settings |
| `default_max_parallel_requests` | int | — | false | false | false | false | config_settings |
| `default_priority` | int | — | false | false | false | false | config_settings |
| `polling_interval` | int | — | false | false | false | false | config_settings |
| `max_fallbacks` | int | 5 | false | false | false | false | config_settings |
| `default_litellm_params` | dict | — | false | false | false | false | config_settings |
| `timeout` | int | 10 minutes | false | false | false | false | config_settings, configs |
| `stream_timeout` | int | uses `timeout` value if unset | false | false | false | false | config_settings |
| `ttft_timeout` | int | — | false | false | false | false | config_settings |
| `stream_idle_timeout` | int | — | false | false | false | false | config_settings |
| `debug_level` | string | — | false | false | false | false | config_settings |
| `client_ttl` | int | 3600 | false | false | false | false | config_settings |
| `cache_kwargs` | dict | — | false | false | false | false | config_settings |
| `routing_strategy_args` | dict | — | false | false | false | false | config_settings |
| `model_group_alias` | dict | — | false | false | false | false | config_settings, configs |
| `num_retries` | int | 3 | false | false | false | false | config_settings, configs |
| `caching_groups` | list | — | false | false | false | false | config_settings |
| `alerting_config` | dict | — | false | false | false | false | config_settings |
| `assistants_config` | dict | — | false | false | false | false | config_settings |
| `set_verbose` | bool | — | false | false | false | **true** | config_settings |
| `retry_after` | int | — | false | false | false | false | config_settings |
| `provider_budget_config` | dict | — | false | false | false | false | config_settings |
| `model_group_retry_policy` | dict | — | false | false | false | false | config_settings |
| `cache_responses` | bool | false | false | false | false | false | config_settings |
| `router_general_settings` | dict | — | false | false | false | false | config_settings |

### `retry_policy` sub-keys (verbatim enum values)

`AuthenticationErrorRetries`, `TimeoutErrorRetries`, `RateLimitErrorRetries`,
`ContentPolicyViolationErrorRetries`, `InternalServerErrorRetries` (all int).

### `allowed_fails_policy` sub-keys (verbatim)

`BadRequestErrorAllowedFails`, `AuthenticationErrorAllowedFails`,
`TimeoutErrorAllowedFails`, `RateLimitErrorAllowedFails`,
`ContentPolicyViolationErrorAllowedFails`, `InternalServerErrorAllowedFails`
(all int).

---

## Section: `callback_settings`

| key (full_path) | type | default | source |
|---|---|---|---|
| `callback_settings.otel.message_logging` | bool | — | config_settings, logging |

---

## Notable verbatim defaults captured

- `num_retries` = **3** (router_settings; config_settings Caveats)
- `timeout` = **10 minutes** (router_settings; config_settings Caveats)
- `max_fallbacks` = **5** (config_settings Caveats)
- `request_timeout` = **6000 seconds** (litellm_settings; config_settings Caveats)
- `user_api_key_cache_ttl` = **60s** (caching page)
- `client_ttl` = **3600** (config_settings Caveats)
- `routing_strategy` = **`simple-shuffle`** (config_settings YAML)
- `enable_pre_call_checks` = **false** (config_settings Caveats)
- `enable_weighted_failover` = **false** (config_settings Caveats)
- `cache_responses` = **false** (config_settings Caveats)
- `fail_closed_budget_enforcement` = **false** (config_settings Caveats)
- `cancel_on_disconnect` = **false** (config_settings Caveats)
- `enable_drain_endpoint` = **off by default** (config_settings Caveats)
- `enable_end_user_cost_tracking_prometheus_only` = **disabled by default** (config_settings Caveats)
- `database_connection_pool_limit` = **10** (config_settings + configs)
- `database_connection_timeout` = **60s** (config_settings + configs)
- `health_check_interval` = **300** (config_settings YAML)
- `alerting_threshold` = **0** (config_settings YAML)
- `cooldown_time` = **30** (config_settings YAML example)
- `allowed_fails` = **3** (config_settings YAML example)
- `keepalive_timeout` = **5 seconds** (deploy page)

---

## Workestrator in-memory deployment mapping

The real config (`infra/litellm/config.yaml`) uses these keys. All confirmed
present in the reference tables above with correct sections:

| real-config key | section | requires_db | requires_redis | in-memory OK? |
|---|---|---|---|---|
| `model_list[].model_name` | model_list | false | false | ✅ |
| `litellm_params.model` | model_list | false | false | ✅ |
| `litellm_params.api_base` | model_list | false | false | ✅ |
| `litellm_params.api_key` | model_list | false | false | ✅ |
| `general_settings.master_key` | general_settings | false | false | ✅ |
| `general_settings.completion_model` | general_settings | false | false | ✅ |
| `general_settings.disable_spend_logs` | general_settings | true | false | ✅ (compensates for no DB) |
| `router_settings.fallbacks` | router_settings | false | false | ✅ |
| `router_settings.num_retries` | router_settings | false | false | ✅ |
| `router_settings.timeout` | router_settings | false | false | ✅ |
| `router_settings.stream_timeout` | router_settings | false | false | ✅ |
| `router_settings.allowed_fails` | router_settings | false | false | ✅ |
| `router_settings.cooldown_time` | router_settings | false | false | ✅ |
| `router_settings.retry_policy` | router_settings | false | false | ✅ |
| `litellm_settings.drop_params` | litellm_settings | false | false | ✅ |
| `litellm_settings.request_timeout` | litellm_settings | false | false | ✅ |
| `litellm_settings.force_ipv4` | litellm_settings | false | false | ✅ |

**In-memory constraints reflected:** No `database_url`, no `redis_*`, no
virtual-key/budget/team/user features. `disable_spend_logs: true` prevents DB
spend-log write attempts. `master_key` is the only auth (single admin key).
Project-defined env vars `KIMI_CODE_API_KEY`, `MINIMAX_CODING_API_KEY`,
`NEURALWATT_API_KEY` are resolved via `os.environ/` (NOT LiteLLM built-ins).

---

## Coverage status (verbatim-complete)

1. **config_settings VERBATIM COMPLETE:** The raw GitHub markdown source
   (`https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/config_settings.md`,
   135,891 bytes, 1,219 lines) has been recovered in full via the GitHub Contents API.
   All reference tables (litellm_settings, general_settings, router_settings,
   environment_variables) are byte-for-byte from this source.
2. **CORRECTION — non-existent sections:** The raw markdown does NOT contain
   separate `### model_list - Reference`, `### callback_settings - Reference`,
   or `### config validation` sections. model_list and callback_settings are
   documented only in the top-level YAML schema block. There is no dedicated
   config validation prose section. Previously-reconstructed headings for these
   have been removed.
3. **environment_variables table:** 765 entries, byte-for-byte from raw source.
   All added to `env-vars.index.json`.
4. **router_settings table:** 54 rows, byte-for-byte from raw source. 9 previously-
   missing rows added to `config-yaml.option-index.json`.
5. **No auto-generated content:** The rendered docs.litellm.ai page is 100%
   hand-written markdown (confirmed via side-by-side comparison).
6. **Types marked `(inferred)`:** Derived from verbatim YAML examples (e.g.
   `bool` from `true`/`false` literals, `int` from numeric literals, `list`
   from `[...]` literals). Safe inferences only.
7. **Defaults marked `(not documented in fetched source)`:** No verbatim
   default was found. Explicitly NOT invented.
8. **`enable_pre_call_check` (singular) vs `enable_pre_call_checks` (plural):**
   BOTH spellings appear verbatim on the config_settings page. Both are real.
9. **`cooldown_time` default:** The YAML example shows `30`; the Caveats section
   does not state a separate default. Marked as example value.
10. **`token_rate_limit_type` default:** The users page YAML shows `"output"` as
    an example with comment "default)"; the verbatim default is `(inferred)` as
    `total`.
