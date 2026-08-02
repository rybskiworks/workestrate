# Environment variables

> Source: https://docs.litellm.ai/docs/proxy/config_settings (+ configs, deploy, docker_quick_start, caching, logging, alerting, prometheus, virtual_keys, ui, db_info, providers/moonshot, providers/openrouter)
> Raw markdown (verbatim): https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/config_settings.md

## Purpose

How `os.environ/X` resolution works at config load time; the proxy env-var surface; provider/observability env vars; project-defined keys.

## os.environ/ resolution syntax

(Verbatim) `os.environ/<YOUR-ENV-VAR>` runs `os.getenv("YOUR-ENV-VAR")`. Used for `api_key`, `master_key`, `api_base`, and any string config value. Confirmed on configs, deploy, docker_quick_start, config_settings.

Env vars can ALSO be injected via the top-level `environment_variables:` dict in `config.yaml` (the real workestrate config does **not** use this — env vars are set in the proxy process environment instead).

> The env var must be set in the proxy process environment **before** startup for `os.environ/` resolution to succeed. An unset var resolves to empty/`None`, which may cause downstream auth failure rather than a config validation failure.

## Key proxy env vars

| env var | purpose | scope | requires_db | deprecated | replacement | workestrate_used |
| --- | --- | --- | --- | --- | --- | --- |
| LITELLM_MASTER_KEY | Proxy admin master key. Must start with 'sk-'. Also used as the Admin UI credential. | proxy | false | false | — | **USED** (resolved via `general_settings.master_key: os.environ/LITELLM_MASTER_KEY`; works in-memory) |
| LITELLM_SALT_KEY | Encryption salt for LLM API key credentials. Cannot be changed after adding a model. | proxy | true | false | — | NOT used (no DB) |
| LITELLM_LICENSE | Enterprise license key (enables Enterprise features). | proxy | false | false | — | NOT used (OSS) |
| LITELLM_LOG | Log level: 'INFO' / 'DEBUG' / 'ERROR' (also 'None' on quick_start page). Replaces deprecated SET_VERBOSE. | proxy | false | false | — | not currently set; available for debugging |
| LITELLM_SET_VERBOSE | (deprecated) | proxy | false | **true** | LITELLM_LOG | NOT used |
| SET_VERBOSE | (deprecated) | proxy | false | **true** | LITELLM_LOG | NOT used |
| DATABASE_URL | Primary PostgreSQL connection string (postgresql://user:password@host:port/dbname). Required for virtual keys, spend tracking, teams, users, Admin UI. | proxy | true | false | — | NOT set (no Postgres); `disable_spend_logs:true` compensates |
| CONFIG_FILE_PATH | Path to config.yaml (easier Azure container deployment). | proxy | false | false | — | not used; config passed via `--config` CLI flag |
| JSON_LOGS | Equivalent to `litellm_settings.json_logs=true`. Env-driven JSON log format. | proxy | false | false | — | not used |
| LITELLM_DROP_PARAMS | Drop unsupported params (env equivalent of `litellm_settings.drop_params`). | proxy | false | false | — | not used as env; real config sets `litellm_settings.drop_params: true` |
| LITELLM_LOCAL_MODEL_COST_MAP | Set to 'True' to disable pulling live model prices (air-gapped/offline). | proxy | false | false | — | not used |
| LITELLM_USE_CHAT_COMPLETIONS_URL_FOR_ANTHROPIC_MESSAGES | env equivalent of `litellm_settings.use_chat_completions_url_for_anthropic_messages`. | proxy | false | false | — | not used |
| LITELLM_ROUTE_ALL_CHAT_OPENAI_TO_RESPONSES | env equivalent of `litellm_settings.route_all_chat_openai_to_responses`. | proxy | false | false | — | not used |
| LITELLM_LEGACY_UNSCOPED_SPEND_LIST_ENDPOINTS | env equivalent of `general_settings.legacy_unscoped_spend_list_endpoints`. | proxy | true | false | — | not used |
| LITELLM_WORKER_STARTUP_HOOKS | Worker startup hook, e.g. 'my_module:init_async_connections'. | proxy | false | false | — | not used |
| DRAIN_ENDPOINT_TOKEN | Token for the /health/drain endpoint. Also settable via `general_settings.drain_endpoint_token`. | proxy | false | false | — | not used |
| STORE_MODEL_IN_DB | Equivalent to `general_settings.store_model_in_db=true`. Stores LLMs in DB (LiteLLM_ProxyModelTable). | proxy | true | false | — | NOT set (leave False); models defined statically in config.yaml `model_list` |
| REDIS_URL | Full Redis URL (redis://username:password@hostname:port/database). Not recommended for prod (performance). | proxy | false | false | — | NOT set (no Redis) |
| REDIS_HOST | Redis connection param. | proxy | false | false | — | NOT set |
| REDIS_PORT | Redis connection param. | proxy | false | false | — | NOT set |
| REDIS_PASSWORD | Redis connection param. | proxy | false | false | — | NOT set |
| REDIS_USERNAME | Redis connection param. | proxy | false | false | — | NOT set |
| REDIS_SSL | Redis connection param. | proxy | false | false | — | NOT set |
| REDIS_CONNECTION_POOL_KWARGS | JSON string for Redis pool kwargs (for non-string Redis params). | proxy | false | false | — | not used |
| REDIS_CLUSTER_NODES | JSON list of {host,port} for Redis Cluster. | proxy | false | false | — | not used |
| REDIS_SENTINEL_NODES | JSON list of [host, port] for Redis Sentinel. | proxy | false | false | — | not used |
| REDIS_SERVICE_NAME | Redis Sentinel master service name. | proxy | false | false | — | not used |
| USE_LITELLM_PROXY | 'True' enables proxy usage (SDK-side). | sdk | false | false | — | not used |
| USE_LITELLM_DOTENV | Use litellm .env file. | sdk | false | false | — | not used |
| NUM_WORKERS | Number of workers. | proxy | false | false | — | not used as env; passed via `--num_workers` CLI flag in docker-compose |
| NO_PROXY | List of addresses to bypass proxy. | sdk | false | false | — | not used |
| DISABLE_ADMIN_UI | Set to 'True' to disable the Admin UI. | admin_ui | false | false | — | **RECOMMENDED** to set 'True' for in-memory deployment (Admin UI requires DB) |
| DISABLE_ADMIN_ENDPOINTS | Disable admin management endpoints. | admin_ui | false | false | — | not used |
| DISABLE_LLM_API_ENDPOINTS | Disable LLM API endpoints. | admin_ui | false | false | — | NOT used (would disable chat completions endpoints workestrate needs) |
| PROXY_BASE_URL | Public URL of proxy. | proxy | false | false | — | not used |
| PROXY_ADMIN_ID | Proxy admin ID. | proxy | false | false | — | not used |
| PROXY_BATCH_WRITE_AT | env equivalent of `general_settings.proxy_batch_write_at`. | proxy | true | false | — | not used |
| PROXY_BATCH_POLLING_INTERVAL | env equivalent of `general_settings.proxy_batch_polling_interval`. | proxy | true | false | — | not used |
| PROXY_DATABASE_URL_ENCRYPTED | Encrypted DB URL (for Google KMS). | proxy | true | false | — | not used |
| UI_USERNAME | Admin UI login. | admin_ui | true | false | — | NOT used (Admin UI requires DB) |
| UI_PASSWORD | Admin UI login. | admin_ui | true | false | — | NOT used |
| UI_BASE_PATH | Admin UI base path. | admin_ui | true | false | — | NOT used |
| DOCS_URL | Set docs (Swagger) to a different path. Default '/'. | proxy | false | false | — | not used |
| ROOT_REDIRECT_URL | Redirect root path '/' to this URL when DOCS_URL is changed. | proxy | false | false | — | not used |
| SERVER_ROOT_PATH | Custom server root path (e.g. '/api/v1'). | proxy | false | false | — | not used |
| KEEPALIVE_TIMEOUT | Keepalive timeout in seconds (default 5). | proxy | false | false | — | not used |
| MAX_REQUESTS_BEFORE_RESTART | Restart workers after N requests (default disabled). Not supported with Granian. | proxy | false | false | — | not used |
| GRACEFUL_SHUTDOWN_TIMEOUT | Referenced in drain endpoint context. | proxy | false | false | — | not used |
| LITELLM_CONFIG_BUCKET_TYPE | GCS bucket config loading. | proxy | false | false | — | not used; config loaded from local file |
| LITELLM_CONFIG_BUCKET_NAME | GCS bucket config loading. | proxy | false | false | — | not used |
| LITELLM_CONFIG_BUCKET_OBJECT_KEY | GCS bucket config loading. | proxy | false | false | — | not used |
| LITELLM_ENVIRONMENT | Set environment for supported_environments (production/staging/development). | proxy | false | false | — | not used |
| LITELLM_KEY | Proxy key used in Authorization: Bearer header. | proxy | false | false | — | not used; `master_key` used instead |
| NO_DOCS | disable Swagger. | proxy | false | false | — | not used |
| NO_REDOC | disable Redoc. | proxy | false | false | — | not used |
| AWS_WEB_IDENTITY_TOKEN | IAM tokens for RDS auth. | proxy | true | false | — | not used |
| AWS_ROLE_NAME | IAM tokens for RDS auth. | proxy | true | false | — | not used |
| AWS_SESSION_NAME | IAM tokens for RDS auth. | proxy | true | false | — | not used |
| DATABASE_USER | RDS connection param. | proxy | true | false | — | not used |
| DATABASE_PORT | RDS connection param. | proxy | true | false | — | not used |
| DATABASE_HOST | RDS connection param. | proxy | true | false | — | not used |
| DATABASE_NAME | RDS connection param. | proxy | true | false | — | not used |
| DATABASE_SCHEMA | RDS connection param. | proxy | true | false | — | not used |
| LITELLM_KEY_ROTATION_ENABLED | key rotation worker. Enterprise + DB. | proxy | true | false | — | not used |
| LITELLM_KEY_ROTATION_CHECK_INTERVAL_SECONDS | key rotation worker. Enterprise + DB. | proxy | true | false | — | not used |
| LITELLM_KEY_ROTATION_GRACE_PERIOD | key rotation worker. Enterprise + DB. | proxy | true | false | — | not used |
| PROMETHEUS_MULTIPROC_DIR | Required env var for multi-worker aggregated Prometheus metric collection. Directory must exist and be writable. | observability | false | false | — | not used (single-process in-memory) |

## Deprecated

- **`SET_VERBOSE`** and **`LITELLM_SET_VERBOSE`** — both deprecated; replacement is `LITELLM_LOG` (verbatim deprecation note: "Replaces deprecated SET_VERBOSE."). Use `LITELLM_LOG="INFO"|"DEBUG"|"ERROR"` instead.

## Provider env vars

(Compact list, grouped. Verbatim from `env-vars.index.json`.)

- **OpenAI**: `OPENAI_API_KEY`
- **Anthropic**: `ANTHROPIC_API_KEY`
- **Azure**: `AZURE_API_KEY`, `AZURE_API_BASE`, `AZURE_API_VERSION`, `AZURE_API_KEY_EU` / `AZURE_API_KEY_CA` / `AZURE_NORTH_AMERICA_API_KEY` (region examples)
- **AWS**: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION_NAME`
- **Cohere**: `COHERE_API_KEY`
- **HuggingFace**: `HUGGINGFACE_API_KEY` (also `HF_TOKEN`)
- **Together**: `TOGETHERAI_API_KEY`
- **PaLM**: `PALM_API_KEY`
- **xAI**: `XAI_API_KEY`
- **Vertex**: `VERTEX_PROJECT`, `VERTEX_LOCATION`
- **Replicate**: `REPLICATE_API_KEY`
- **AI21**: `AI21_API_KEY`
- **Moonshot**: `MOONSHOT_API_KEY`, `MOONSHOT_API_BASE`
- **OpenRouter**: `OPENROUTER_API_KEY`, `OPENROUTER_API_BASE`, `OR_SITE_URL`, `OR_APP_NAME` — **workestrate-used** (`OPENROUTER_API_KEY` resolved via `os.environ/` for `openrouter/`-prefixed models: GLM/Qwen/Nex).
- **GCS**: `GCS_BUCKET_NAME`, `GCS_PATH_SERVICE_ACCOUNT`, `GCS_PUBSUB_TOPIC_ID` (Enterprise), `GCS_PUBSUB_PROJECT_ID` (Enterprise)
- **Qdrant**: `QDRANT_API_KEY`, `QDRANT_API_BASE`
- **Valkey**: `VALKEY_HOST`, `VALKEY_PORT`, `VALKEY_PASSWORD`

## Observability env vars

(Compact list. Verbatim from `env-vars.index.json`.)

- **Langfuse**: `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`, `LANGFUSE_HOST` — available-for-in-memory (works without DB).
- **OpenTelemetry**: `OTEL_TRACER_NAME`, `OTEL_SERVICE_NAME`, `OTEL_EXPORTER`, `OTEL_ENDPOINT`, `OTEL_HEADERS`, `OTEL_EXPORTER_OTLP_ENDPOINT` — `OTEL_SERVICE_NAME` available-for-in-memory.
- **Datadog**: `DD_API_KEY`, `DD_APP_KEY`, `DD_SITE`
- **Sentry**: `SENTRY_DSN`
- **Slack**: `SLACK_WEBHOOK_URL`, `SLACK_WEBHOOK_URL_2` (up to `_20`) — `SLACK_WEBHOOK_URL` available-for-in-memory (works without DB).
- **Lunary**: `LUNARY_PUBLIC_KEY`
- **Langsmith**: `LANGSMITH_API_KEY`, `LANGSMITH_PROJECT`, `LANGSMITH_BASE_URL`
- **Arize**: `ARIZE_SPACE_KEY`, `ARIZE_API_KEY`, `ARIZE_ENDPOINT`, `ARIZE_HTTP_ENDPOINT`
- **Langtrace**: `LANGTRACE_API_KEY`
- **Galileo**: `GALILEO_API_KEY`, `GALILEO_PROJECT_ID`, `GALILEO_LOG_STREAM_ID`, `GALILEO_BASE_URL`, `GALILEO_USERNAME`, `GALILEO_PASSWORD`
- **Generic logger**: `GENERIC_LOGGER_ENDPOINT`, `GENERIC_LOGGER_HEADERS`
- **Azure Storage** (Enterprise): `AZURE_STORAGE_ACCOUNT_NAME`, `AZURE_STORAGE_FILE_SYSTEM`, `AZURE_STORAGE_ACCOUNT_KEY`, `AZURE_STORAGE_TENANT_ID`, `AZURE_STORAGE_CLIENT_ID`, `AZURE_STORAGE_CLIENT_SECRET`
- **Confident**: `CONFIDENT_API_KEY`
- **Webhook (budget alerts BETA)**: `WEBHOOK_URL` (requires_db=true)

## PROJECT-DEFINED env vars

> **NOT LiteLLM built-ins.** These have empty `source_urls` and `project_defined=true` in `env-vars.index.json`. They are NOT documented on any LiteLLM docs page. They are workestrate-specific keys referenced via `os.environ/` in `model_list.litellm_params.api_key`.

| env var | purpose (verbatim) | workestrate usage |
| --- | --- | --- |
| KIMI_CODE_API_KEY | API key for Kimi coding endpoint (api.kimi.com/coding) which speaks the Anthropic Messages API. Referenced via `os.environ/KIMI_CODE_API_KEY`. | **USED** for `anthropic/kimi-for-coding` (coding tier primary + `coding.pro`-fallback). NOT documented on any LiteLLM docs page. |
| MINIMAX_CODING_API_KEY | API key for MiniMax coding endpoint (api.minimax.io/anthropic) which speaks the Anthropic Messages API. | **USED** for `anthropic/MiniMax-M3` (coding-fallback + `coding.fast` primary). NOT documented on any LiteLLM docs page. |
| NEURALWATT_API_KEY | API key for Neuralwatt endpoint (api.neuralwatt.com/v1) which speaks the OpenAI-compatible API. | **USED** for `openai/neuralwatt` (neural tier). NOT documented on any LiteLLM docs page. |

## Pitfalls

- **`LITELLM_SALT_KEY`** — "Cannot be changed after adding a model." Set once and never rotate without re-adding all model credentials. Requires DB.
- **`REDIS_URL`** — "Known performance issue with Redis URL." Prefer `REDIS_HOST`/`REDIS_PORT`/`REDIS_PASSWORD` for production.
- **`DISABLE_ADMIN_UI`** — recommended `True` for in-memory deployments; the Admin UI requires a DB and will not function without one.
- **`STORE_MODEL_IN_DB`** — must stay `False` (unset) for static config; the real config defines models in `config.yaml` `model_list`.
- **`os.environ/<VAR>` resolution timing** — the env var must be set in the proxy process environment **before** startup. An unset var resolves to empty/`None`, which typically causes a downstream auth failure (not a config validation failure).
- **`LITELLM_SET_VERBOSE` / `SET_VERBOSE`** — deprecated; use `LITELLM_LOG`.
- **`NUM_WORKERS`** — not used as env in workestrate; passed via `--num_workers` CLI flag in docker-compose.

## Reference

- Env-var index: [`env-vars.index.json`](../schemas/env-vars.index.json)

## Workestrate notes

> **PROJECT CONTEXT** — not upstream LiteLLM docs. Describes the workestrate deployment specifically.

The real config resolves the following via `os.environ/`:

- `LITELLM_MASTER_KEY` (via `general_settings.master_key`)
- `OPENROUTER_API_KEY` (provider env, for `openrouter/`-prefixed models)
- `KIMI_CODE_API_KEY` (project-defined, for `anthropic/kimi-for-coding`)
- `MINIMAX_CODING_API_KEY` (project-defined, for `anthropic/MiniMax-M3`)
- `NEURALWATT_API_KEY` (project-defined, for `openai/neuralwatt`)

NOT set (in-memory, no DB / no Redis / no Admin UI):

- `DATABASE_URL`, all `REDIS_*`, `UI_USERNAME` / `UI_PASSWORD` / `UI_BASE_PATH`, `LITELLM_SALT_KEY`, `STORE_MODEL_IN_DB`, `PROXY_DATABASE_URL_ENCRYPTED`, all `DATABASE_USER`/`PORT`/`HOST`/`NAME`/`SCHEMA`, `LITELLM_KEY_ROTATION_*`.

- `LITELLM_LOG` — not currently set; available for debugging (`export LITELLM_LOG="DEBUG"`).
- `DISABLE_ADMIN_UI=True` — recommended for the in-memory deployment.
- `NUM_WORKERS` — passed via CLI (`--num_workers`), not env.
- `LITELLM_DROP_PARAMS` — not used as env; the equivalent is set via `litellm_settings.drop_params: true` in config.
