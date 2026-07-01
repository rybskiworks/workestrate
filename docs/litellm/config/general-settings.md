# general_settings

> Source: https://docs.litellm.ai/docs/proxy/config_settings (+ configs, deploy, docker_quick_start, virtual_keys, users, db_info, alerting, ui)

## Purpose

Proxy-wide auth, DB connection, spend logging, alerting, health checks, Admin UI, MCP, RBAC, pass-through endpoints, and enterprise feature gates.

## Key table

| key | type | default | requires_db | requires_redis | enterprise | deprecated | source |
| --- | --- | --- | --- | --- | --- | --- | --- |
| completion_model | string | — | false | false | false | false | config_settings |
| enable_drain_endpoint | bool | false (off by default) | false | false | false | false | config_settings |
| drain_endpoint_token | string | — | false | false | false | false | config_settings |
| disable_spend_logs | bool | — | true | false | false | false | config_settings, db_info |
| disable_spend_updates | bool | — | true | false | false | false | config_settings |
| disable_error_logs | bool | — | true | false | false | false | config_settings, db_info |
| disable_retry_on_max_parallel_request_limit_error | bool | — | false | false | false | false | config_settings |
| disable_reset_budget | bool | — | true | false | false | false | config_settings |
| disable_adding_master_key_hash_to_db | bool | — | true | false | false | false | config_settings |
| disable_responses_id_security | bool | — | false | false | false | false | config_settings |
| enable_jwt_auth | bool | — | false | false | false | false | config_settings |
| enforce_user_param | bool | — | false | false | false | false | config_settings |
| reject_clientside_metadata_tags | bool | — | false | false | false | false | config_settings |
| disable_batch_input_file_rate_limiting | bool | — | false | false | false | false | config_settings |
| skip_batch_input_file_rate_limiting_for_providers | list | — | false | false | false | false | config_settings |
| skip_batch_input_file_rate_limiting_for_models | list | — | false | false | false | false | config_settings |
| allowed_routes | list | — | false | false | false | false | config_settings |
| key_management_system | enum: `google_kms`,`azure_kms` | — | false | false | false | false | config_settings |
| master_key | string | — | false | false | false | false | config_settings, configs, deploy, docker_quick_start, virtual_keys, users |
| database_url | string | — | true | false | false | false | config_settings, configs, docker_quick_start, virtual_keys |
| database_connection_pool_limit | int | 10 | true | false | false | false | config_settings, configs |
| database_connection_timeout | int | 60 (seconds) | true | false | false | false | config_settings, configs |
| database_connect_timeout | int | — (Prisma default) | true | false | false | false | config_settings |
| database_socket_timeout | int | — | true | false | false | false | config_settings, configs |
| database_extra_connection_params | dict | — | true | false | false | false | config_settings |
| database_disable_prepared_statements | bool | — | true | false | false | false | config_settings |
| database_connection_pool_timeout | int | — | true | false | false | false | config_settings |
| allow_requests_on_db_unavailable | bool | — | true | false | false | false | config_settings |
| fail_closed_budget_enforcement | bool | false | true | true | false | false | config_settings, users |
| custom_auth | string | — | false | false | false | false | config_settings |
| custom_auth_run_common_checks | bool | — | false | false | false | false | config_settings |
| max_parallel_requests | int | 0 | false | false | false | false | config_settings |
| global_max_parallel_requests | int | 0 | false | false | false | false | config_settings |
| cancel_on_disconnect | bool | false | false | false | false | false | config_settings |
| infer_model_from_keys | bool | — | false | false | false | false | config_settings |
| background_health_checks | bool | — | false | false | false | false | config_settings |
| health_check_interval | int | 300 | false | false | false | false | config_settings |
| health_check_details | bool | — | false | false | false | false | config_settings |
| health_check_concurrency | int | — | false | false | false | false | config_settings |
| health_check_ignore_transient_errors | bool | — | false | false | false | false | config_settings |
| health_check_skip_disabled_background_models | bool | — | false | false | false | false | config_settings |
| health_check_staleness_threshold | int | — | false | false | false | false | config_settings |
| enable_health_check_routing | bool | — | false | false | false | false | config_settings |
| use_shared_health_check | bool | — | false | false | false | false | config_settings |
| alerting | list | — | false | false | false | false | config_settings, configs, alerting |
| alerting_threshold | int | 0 | false | false | false | false | config_settings, alerting |
| alerting_args | dict | — | false | false | false | false | config_settings, alerting |
| alert_types | list | — | false | false | false | false | config_settings, alerting |
| alert_type_config | dict | — | false | false | false | false | config_settings, alerting |
| alert_to_webhook_url | dict | — | false | false | false | false | config_settings, alerting |
| spend_report_frequency | string | — | true | false | false | false | config_settings, alerting |
| use_client_credentials_pass_through_routes | bool | — | false | false | false | false | config_settings |
| public_routes | list | — | false | false | **true** | false | config_settings |
| enforced_params | list | — | false | false | **true** | false | config_settings |
| enable_oauth2_auth | bool | — | false | false | **true** | false | config_settings |
| admin_only_routes | list | — | false | false | **true** | false | config_settings |
| enable_oauth2_proxy_auth | bool | — | false | false | **true** | false | config_settings |
| use_x_forwarded_for | bool | — | false | false | false | false | config_settings |
| service_account_settings | dict | — | false | false | false | false | config_settings |
| image_generation_model | string | — | false | false | false | false | config_settings |
| store_model_in_db | bool | — | true | false | false | false | config_settings |
| supported_db_objects | list | — | true | false | false | false | config_settings |
| user_mcp_management_mode | enum: `restricted`,`view_all` | — | false | false | false | false | config_settings |
| store_prompts_in_spend_logs | bool | — | true | false | false | false | config_settings |
| scope_spend_list_endpoints_to_caller | bool | — | true | false | false | false | config_settings |
| legacy_unscoped_spend_list_endpoints | bool | — | true | false | false | false | config_settings |
| max_request_size_mb | int | — | false | false | false | false | config_settings |
| max_response_size_mb | int | — | false | false | false | false | config_settings |
| proxy_budget_rescheduler_min_time | int | — | true | false | false | false | config_settings, users |
| proxy_budget_rescheduler_max_time | int | — | true | false | false | false | config_settings, users |
| proxy_batch_write_at | int | — | true | false | false | false | config_settings |
| proxy_batch_polling_interval | int | — | true | false | false | false | config_settings |
| custom_key_generate | string | — | true | false | false | false | config_settings, virtual_keys |
| allowed_ips | list | — | false | false | false | false | config_settings |
| embedding_model | string | — | false | false | false | false | config_settings |
| default_team_disabled | bool | — | true | false | false | false | config_settings |
| key_management_settings | dict | — | false | false | false | false | config_settings |
| allow_user_auth | bool | — | false | false | false | **true** (Deprecated) | config_settings |
| user_api_key_cache_ttl | int | 60 (seconds) | false | false | false | false | config_settings, caching |
| disable_prisma_schema_update | bool | — | true | false | false | false | config_settings |
| litellm_key_header_name | string | — | false | false | false | false | config_settings, virtual_keys |
| moderation_model | string | — | false | false | false | false | config_settings |
| custom_sso | string | — | false | false | false | false | config_settings |
| allow_client_side_side_credentials | bool | — | false | false | false | false | config_settings |
| use_azure_key_vault | bool | — | false | false | false | false | config_settings |
| use_google_kms | bool | — | false | false | false | false | config_settings |
| ui_access_mode | enum | — | true | false | false | false | config_settings |
| litellm_jwtauth | dict | — | false | false | false | false | config_settings |
| litellm_license | string | — | false | false | false | false | config_settings |
| oauth2_config_mappings | dict | — | false | false | false | false | config_settings |
| pass_through_endpoints | list | — | false | false | false | false | config_settings |
| pass_through_request_timeout | int | — | false | false | false | false | config_settings |
| forward_openai_org_id | bool | — | false | false | false | false | config_settings |
| forward_client_headers_to_llm_api | bool | — | false | false | false | false | config_settings |
| forward_llm_provider_auth_headers | bool | — | false | false | false | false | config_settings |
| maximum_spend_logs_retention_period | string | — | true | false | false | false | config_settings |
| maximum_spend_logs_retention_interval | string | — | true | false | false | false | config_settings |
| maximum_spend_logs_cleanup_cron | string | — | true | false | false | false | config_settings |
| always_include_stream_usage | bool | — | false | false | false | false | config_settings |
| auto_redirect_ui_login_to_sso | bool | — | true | false | false | false | config_settings |
| control_plane_url | string | — | false | false | false | false | config_settings |
| custom_ui_sso_sign_in_handler | string | — | true | false | false | false | config_settings |
| enable_mcp_registry | bool | — | false | false | false | false | config_settings |
| enforce_rbac | bool | — | false | false | false | false | config_settings |
| mcp_client_side_auth_header_name | string | — | false | false | false | false | config_settings |
| mcp_internal_ip_ranges | list | — | false | false | false | false | config_settings |
| mcp_required_fields | list | — | false | false | false | false | config_settings |
| mcp_trusted_proxy_ranges | list | — | false | false | false | false | config_settings |
| require_end_user_mcp_access_defined | bool | — | false | false | false | false | config_settings |
| role_permissions | dict | — | false | false | false | false | config_settings |
| search_tools | dict | — | false | false | false | false | config_settings |
| token_rate_limit_type | enum: `input`,`output`,`total` | `total` (inferred) | true | false | false | false | config_settings, users |
| use_redis_transaction_buffer | bool | — | true | **true** | false | false | config_settings, deploy |
| user_header_mappings | dict | — | false | false | false | false | config_settings |
| user_header_name | string | — | false | false | false | false | config_settings |
| block_robots | bool | — | false | false | **true** | false | deploy |

## alerting_args sub-keys (verbatim defaults)

| sub-key | default |
| --- | --- |
| daily_report_frequency | 43200 |
| report_check_interval | 3600 |
| budget_alert_ttl | 86400 |
| outage_alert_ttl | 60 |
| region_outage_alert_ttl | 60 |
| minor_outage_alert_threshold | 5 |
| major_outage_alert_threshold | 10 |
| max_outage_alert_list_size | 1000 |
| log_to_console | false |

## DB-required keys callout

These keys require a Postgres backend (`requires_db=true`). Without `database_url`, they are no-ops or startup failures:

- `database_url` and all `database_*` params (`database_connection_pool_limit`, `database_connection_timeout`, `database_connect_timeout`, `database_socket_timeout`, `database_extra_connection_params`, `database_disable_prepared_statements`, `database_connection_pool_timeout`, `allow_requests_on_db_unavailable`)
- `store_model_in_db`, `supported_db_objects`
- `disable_spend_logs`, `disable_spend_updates`, `disable_error_logs`, `disable_reset_budget`, `disable_adding_master_key_hash_to_db`
- `store_prompts_in_spend_logs`, `scope_spend_list_endpoints_to_caller`, `legacy_unscoped_spend_list_endpoints`
- `spend_report_frequency`
- `proxy_budget_rescheduler_min_time`, `proxy_budget_rescheduler_max_time`, `proxy_batch_write_at`, `proxy_batch_polling_interval`
- `custom_key_generate`, `default_team_disabled`
- `ui_access_mode`, `auto_redirect_ui_login_to_sso`, `custom_ui_sso_sign_in_handler`
- `maximum_spend_logs_retention_period`, `maximum_spend_logs_retention_interval`, `maximum_spend_logs_cleanup_cron`
- `disable_prisma_schema_update`
- `token_rate_limit_type`
- `use_redis_transaction_buffer` (requires_redis=true too)
- `fail_closed_budget_enforcement` (requires_redis=true too)

## Enterprise keys callout

These keys are Enterprise-gated (`enterprise=true`):

- `public_routes`, `enforced_params`, `enable_oauth2_auth`, `admin_only_routes`, `enable_oauth2_proxy_auth`
- `allow_user_auth` (deprecated)
- `block_robots`

## Verified behavior notes (verbatim caveats)

- **fail_closed_budget_enforcement** — "Default false."
- **cancel_on_disconnect** — "Default false."
- **enable_drain_endpoint** — "Off by default; only enable it when the health port is reachable solely from inside the cluster."
- **database_disable_prepared_statements** — "An explicit `pgbouncer` key in `database_extra_connection_params` takes precedence."
- **database_extra_connection_params** — "Keys here override any default LiteLLM sets."
- **legacy_unscoped_spend_list_endpoints** — "Overrides `scope_spend_list_endpoints_to_caller`."
- **disable_prisma_schema_update** — `true` in general_settings skips automatic Prisma schema migration.
- **database_connection_pool_limit** — default `10`.
- **database_connection_timeout** — default `60s`.
- **health_check_interval** — `300`.
- **alerting_threshold** — `0`.
- **user_api_key_cache_ttl** — `60s`.

## YAML example (verbatim)

```yaml
general_settings:
  completion_model: string
  store_prompts_in_spend_logs: boolean
  forward_client_headers_to_llm_api: boolean
  disable_spend_logs: boolean  # turn off writing each transaction to the db
  disable_master_key_return: boolean  # turn off returning master key on UI (checked on '/user/info' endpoint)
  disable_retry_on_max_parallel_request_limit_error: boolean
  disable_reset_budget: boolean  # turn off reset budget scheduled task
  disable_adding_master_key_hash_to_db: boolean  # turn off storing master key hash in db, for spend tracking
  disable_responses_id_security: boolean
  enable_jwt_auth: boolean  # allow proxy admin to auth in via jwt tokens with 'litellm_proxy_admin' in claims
  enforce_user_param: boolean  # requires all openai endpoint requests to have a 'user' param
  reject_clientside_metadata_tags: boolean
  disable_batch_input_file_rate_limiting: boolean
  skip_batch_input_file_rate_limiting_for_providers: ["hosted_vllm"]
  skip_batch_input_file_rate_limiting_for_models: ["my-batch-model-prefix"]
  allowed_routes: ["route1", "route2"]
  key_management_system: google_kms  # either google_kms or azure_kms
  master_key: string
  maximum_spend_logs_retention_period: 30d
  maximum_spend_logs_retention_interval: 1d
  user_mcp_management_mode: restricted  # or "view_all"
  database_url: string
  database_connection_pool_limit: 0  # default 10
  database_connection_timeout: 0  # default 60s
  database_connect_timeout: 0  # Prisma `connect_timeout` URL param (seconds). Unset => Prisma default.
  database_socket_timeout: 0  # Prisma `socket_timeout` URL param (seconds).
  database_extra_connection_params: {}  # Extra key/value pairs appended to the Prisma DATABASE_URL / DIRECT_URL query string (e.g. sslmode, pgbouncer, statement_cache_size). Overrides LiteLLM defaults.
  database_disable_prepared_statements: boolean  # if true, appends pgbouncer=true to the Prisma connection URL, disabling server-side prepared statements. For PgBouncer transaction pooling and avoiding "cached plan must not change result type" errors during rolling migrations.
  allow_requests_on_db_unavailable: boolean
  fail_closed_budget_enforcement: boolean  # if true, validates spend against the DB for every budgeted request and rejects with 503 when spend cannot be verified against Redis or the DB
  custom_auth: string
  max_parallel_requests: 0
  global_max_parallel_requests: 0
  infer_model_from_keys: true
  background_health_checks: true
  health_check_interval: 300
  alerting: ["slack", "email"]
  alerting_threshold: 0
  use_client_credentials_pass_through_routes: boolean
```

## Pitfalls

- **`enable_drain_endpoint`** — off by default; only enable when the health port is reachable solely from inside the cluster.
- **`fail_closed_budget_enforcement`** — requires both DB and Redis; default `false`. Enabling without both backends will reject budgeted requests with 503.
- **`legacy_unscoped_spend_list_endpoints`** — overrides `scope_spend_list_endpoints_to_caller` silently.
- **`database_disable_prepared_statements`** — an explicit `pgbouncer` key in `database_extra_connection_params` takes precedence.
- **`database_extra_connection_params`** — keys here override LiteLLM defaults; misconfiguration can break the Prisma connection URL.
- **`disable_prisma_schema_update: true`** — skips automatic Prisma schema migration; useful for PgBouncer/rolling deploys but means schema drift is not auto-fixed.
- **`store_model_in_db`** — requires DB; if set `true` without `database_url`, startup fails.
- **`ui_access_mode`** — requires DB; Admin UI cannot function without a DB backend.
- **`user_api_key_cache_ttl`** — in-memory cache TTL (60s default); does not require Redis but is per-worker.
- **Enterprise keys** — `public_routes`, `enforced_params`, `enable_oauth2_auth`, `admin_only_routes`, `enable_oauth2_proxy_auth`, `allow_user_auth` (deprecated), `block_robots` require an Enterprise license.

## Reference

- Authoritative key index: [`config-yaml.option-index.json`](../schemas/config-yaml.option-index.json)
- Normalized schema: [`config-yaml.normalized.schema.md`](../schemas/config-yaml.normalized.schema.md)

## Workestrator notes

> **PROJECT CONTEXT** — not upstream LiteLLM docs. Describes the workestrator deployment specifically.

Real config (`infra/litellm/config.yaml`):

```yaml
general_settings:
  master_key: os.environ/LITELLM_MASTER_KEY
  completion_model: coding
  disable_spend_logs: true
```

- **`master_key: os.environ/LITELLM_MASTER_KEY`** — the SOLE auth mechanism. Single admin key, no virtual keys / teams / users / budgets. Resolved via `os.environ/` at config load time.
- **`completion_model: coding`** — the default model alias for `/chat/completions` when no model is specified. `coding` is an alias defined in `model_list`.
- **`disable_spend_logs: true`** — compensates for the absence of a DB; prevents spend-log write attempts that would otherwise fail.
- **No `database_url`** — in-memory deployment. All `database_*` params, `store_model_in_db`, `custom_key_generate`, `ui_access_mode`, `maximum_spend_logs_*`, `proxy_budget_rescheduler_*`, `proxy_batch_*`, `token_rate_limit_type`, `use_redis_transaction_buffer`, `fail_closed_budget_enforcement` are unset.
- **No alerting** — `alerting`, `alerting_threshold`, `alerting_args`, `alert_types`, `alert_type_config`, `alert_to_webhook_url`, `spend_report_frequency` all unset.
- **No UI keys** — `ui_access_mode`, `auto_redirect_ui_login_to_sso`, `custom_ui_sso_sign_in_handler` unset. Admin UI requires a DB; recommend setting `DISABLE_ADMIN_UI=True` env var for the in-memory deployment.
- **No `pass_through_endpoints`** — unset.
- **No enterprise features** — `litellm_license` unset; `public_routes`, `enforced_params`, `enable_oauth2_auth`, `admin_only_routes`, `enable_oauth2_proxy_auth`, `block_robots` all unset.
- **No `custom_auth`** — auth is purely `master_key` based.

## Behavioral detail — secret managers (KMS / Vault)

> Source: https://docs.litellm.ai/docs/secret_managers/overview (raw: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/secret_managers/overview.md) — `extracted/p2-feature-secret_managers.md`

### Enterprise requirement (verbatim)

> "✨ This is an Enterprise Feature"

All secret-manager functionality (AWS KMS, AWS Secret Manager, Azure Key Vault, CyberArk Conjur, Google Secret Manager, Google KMS, Hashicorp Vault) is Enterprise-gated. Without a `litellm_license`, these keys are no-ops.

### `key_management_system` (REQUIRED, unified path)

Verbatim: `key_management_system` is **REQUIRED**. Enum value shown on the overview page: `"aws_secret_manager"`. The config_settings Reference table documents the enum as `google_kms | azure_kms`; the overview page's unified example uses `aws_secret_manager`. The full supported-provider list (verbatim): AWS KMS, AWS Secret Manager, Azure Key Vault, CyberArk Conjur, Google Secret Manager, Google KMS, Hashicorp Vault.

### `key_management_settings` sub-keys (verbatim)

```yaml
general_settings:
  key_management_system: "aws_secret_manager" # REQUIRED
  key_management_settings:  

    # Storing Virtual Keys Settings
    store_virtual_keys: true # OPTIONAL. Defaults to False, when True will store virtual keys in secret manager
    prefix_for_stored_virtual_keys: "litellm/" # OPTIONAL. If set, this prefix will be used for stored virtual keys in the secret manager
    
    # Access Mode Settings
    access_mode: "write_only" # OPTIONAL. Literal["read_only", "write_only", "read_and_write"]. Defaults to "read_only"
    
    # Hosted Keys Settings
    hosted_keys: ["litellm_master_key"] # OPTIONAL. Specify which env keys you stored on AWS

    # K/V pairs in 1 AWS Secret Settings
    primary_secret_name: "litellm_secrets" # OPTIONAL. Read multiple keys from one JSON secret on AWS Secret Manager
```

- `store_virtual_keys` (bool) — verbatim: "Defaults to False, when True will store virtual keys in secret manager".
- `prefix_for_stored_virtual_keys` (string) — verbatim: "If set, this prefix will be used for stored virtual keys in the secret manager".
- `access_mode` (enum: `read_only` | `write_only` | `read_and_write`) — verbatim: "Defaults to "read_only"".
- `hosted_keys` (list) — verbatim: "Specify which env keys you stored on AWS".
- `primary_secret_name` (string) — verbatim: "Read multiple keys from one JSON secret on AWS Secret Manager".

### `use_google_kms` / `use_azure_key_vault` (older/provider-specific booleans)

These two boolean keys are documented in the config_settings Reference table (option-index) but are **NOT** described on the /docs/secret_managers/overview page. The overview page standardizes on the unified `key_management_system` + `key_management_settings` path. Both spellings exist in the corpus — treat `key_management_system` as the current unified approach. *(The relationship between the two boolean keys and the unified `key_management_system` key is inferred from the corpus: the overview page does not mention the booleans, while config_settings lists them.)*

### Team-level secret manager settings

Team-level secret manager settings (per-team bring-your-own-key-management) are configured via the Admin UI Teams page (Create Team → Additional Settings → Secret Manager Settings panel), using provider-specific JSON. Verbatim: "JSON is required today, but we plan to add a more UI-friendly editor." This requires the Admin UI (and therefore a DB backend).

### Workestrator applicability

The workestrator does NOT use any secret manager. `key_management_system`, `key_management_settings`, `use_google_kms`, and `use_azure_key_vault` are all unset. API keys are resolved via `os.environ/` at config load time. Secret-manager integration is Enterprise-gated and therefore unavailable without a `litellm_license`.
