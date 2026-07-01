# Observability, Cache & Guardrails

> Synthesized from on-disk corpus under `docs/litellm/extracted/` and
> `docs/litellm/schemas/`. No web fetches were performed. Every key, default,
> and requirement traces to a cited corpus file or is marked **(inferred)** /
> **"not documented in fetched source"**.

## Sources

- https://docs.litellm.ai/docs/proxy/caching — `extracted/p2-feature-caching.md`
- https://docs.litellm.ai/docs/proxy/all_caches — `extracted/p2-feature-all_caches.md`
- https://docs.litellm.ai/docs/proxy/logging — `extracted/p2-logging-logging.md`
- https://docs.litellm.ai/docs/proxy/alerting — `extracted/p2-logging-alerting.md`
- https://docs.litellm.ai/docs/proxy/metrics — `extracted/p2-logging-metrics.md`
- https://docs.litellm.ai/docs/proxy/prometheus — `extracted/p2-logging-prometheus.md`
- https://docs.litellm.ai/docs/proxy/guardrails/quick_start — `extracted/p2-guardrails-quick_start.md`
- https://docs.litellm.ai/docs/proxy/guardrail_policies — `extracted/p2-guardrails-guardrail_policies.md`
- `schemas/config-yaml.option-index.json`
- Real config: `infra/litellm/config.yaml`

## What this area controls

Response caching (avoid redundant upstream calls), logging callbacks
(ship traces/logs to external systems), alerting (Slack/webhook
notifications), Prometheus metrics, and guardrails (PII / content filtering
pre/post/during call). Most of this surface works **without a database**;
the exceptions are spend/budget alerts and budget/rate-limit metrics.

## Verified behavior

### Caching

#### `cache_params.type` values (verbatim)

`"local"`, `"redis"`, `"redis-semantic"`, `"valkey-semantic"`,
`"qdrant-semantic"`, `"s3"`, `"gcs"`, `"disk"`.

(`all_caches` adds `"azure-blob"` on the SDK side — may not be a proxy
`cache_params.type` **(inferred)**.)

| Type | Redis needed? | DB needed? | Works in-memory? |
| --- | --- | --- | --- |
| `local` | no | no | **YES** (in-process) |
| `disk` | no | no | **YES** |
| `s3` | no | no | **YES** |
| `gcs` | no | no | **YES** |
| `qdrant-semantic` | no | no | **YES** |
| `valkey-semantic` | no | no | **YES** |
| `redis` | **YES** | no | NO (no Redis) |
| `redis-semantic` | **YES** | no | NO (no Redis) |
| `enable_redis_auth_cache` | **YES** | no | NO (moot without virtual keys) |

#### Verbatim YAML — local

```yaml
litellm_settings:
  cache: True
  cache_params:
    type: local
```

#### Verbatim YAML — disk

```yaml
cache_params:
  type: disk
  disk_cache_dir: /tmp/litellm-cache   # default ./.litellm_cache
```

#### `cache_params` keys (verbatim)

`ttl`, `default_in_memory_ttl`, `default_in_redis_ttl`, `max_connections`,
`type`, `supported_call_types` (`["acompletion","atext_completion","aembedding","atranscription"]`),
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

#### `litellm_settings` cache keys

`cache` (bool), `enable_redis_auth_cache` (bool, optional, off by default),
`enable_caching_on_provider_specific_optional_params` (bool).

`general_settings.user_api_key_cache_ttl` (int seconds, default `60`) —
controls `master_key` auth cache TTL; relevant in-memory.

#### Cache endpoints

- `POST /cache/ping` — health-check cache backend.
- `POST /cache/delete` — body `{"keys":[...]}`.
- Per-request cache controls on `/v1/chat/completions` and `/embeddings`:
  `cache: {ttl|s-maxage|no-cache|no-store|namespace|use-cache}`.

#### Caveats (verbatim)

> "For non-string Redis parameters... avoid using REDIS_* environment
> variables... use cache_kwargs."

> "REDIS_URL not recommended in prod."

> "If you see errors like No connection available, try increasing
> max_connections."

### Logging / callbacks

`litellm_settings.success_callback` / `failure_callback` / `callbacks`
(preferred). `service_callbacks` (`["datadog","prometheus"]` — logs
redis/postgres failures).

#### Callback names (verbatim)

`langfuse`, `otel`, `gcs_bucket`, `gcs_pubsub`, `deepeval`, `s3_v2`,
`aws_sqs`, `azure_storage`, `lunary`, `langsmith`, `arize`, `langtrace`,
`generic_api`, `datadog`, `prometheus`, `sentry`.

#### Logging settings keys

`turn_off_message_logging` (bool), `redact_user_api_key_info` (bool),
`global_disable_no_log_param` (bool), `langfuse_default_tags` (list),
`forward_traceparent_to_llm_provider` (bool — verbatim: "Only use this for
self hosted LLMs, this can cause Bedrock, VertexAI calls to fail").

`s3_callback_params` sub-keys: `s3_bucket_name`, `s3_region_name`,
`s3_aws_access_key_id`, `s3_aws_secret_access_key`, `s3_path`,
`s3_endpoint_url`, `s3_use_virtual_hosted_style`, `s3_strip_base64_files`,
`s3_use_team_prefix`, `s3_use_key_prefix`.

`aws_sqs_callback_params` sub-keys: `sqs_queue_url`, `sqs_region_name`,
`sqs_strip_base64_files`, `s3_use_team_prefix`, `s3_use_key_prefix`.

`callback_settings.otel.message_logging` (bool) — OTEL message redaction.

Custom callback class: `callbacks: custom_callbacks.proxy_handler_instance`.

#### Enterprise (verbatim "✨ enterprise feature")

GCS Buckets, GCS PubSub, Azure Blob Storage, Custom Callback APIs,
Dynamically Disable specific callbacks.

All logging callbacks work **WITHOUT a database** (external integrations).
DB requirement "not documented" on the logging page.

#### Env vars

`LANGFUSE_PUBLIC_KEY`/`SECRET_KEY`/`HOST`,
`OTEL_TRACER_NAME`/`OTEL_SERVICE_NAME`/`OTEL_EXPORTER`/`OTEL_ENDPOINT`/`OTEL_HEADERS`,
`GCS_BUCKET_NAME`/`GCS_PATH_SERVICE_ACCOUNT`/`GCS_PUBSUB_TOPIC_ID`/`GCS_PUBSUB_PROJECT_ID`,
`CONFIDENT_API_KEY`, `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY`/`AWS_REGION_NAME`,
`AZURE_STORAGE_*`, `LUNARY_PUBLIC_KEY`,
`GENERIC_LOGGER_ENDPOINT`/`GENERIC_LOGGER_HEADERS`,
`LANGSMITH_API_KEY`/`LANGSMITH_PROJECT`/`LANGSMITH_BASE_URL`, `ARIZE_*`,
`LANGTRACE_API_KEY`, `GALILEO_*`.

> "OTLP gRPC requires grpcio. Install via `uv add litellm[grpc]`."

### Alerting

`general_settings.alerting` (`["slack"]`, `["webhook"]`),
`alerting_threshold` (seconds — alert when requests hang/responses take
longer), `spend_report_frequency` (`"1d"`), `alerting_args` (sub-keys:
`daily_report_frequency=43200`, `report_check_interval=3600`,
`budget_alert_ttl=86400`, `outage_alert_ttl=60`,
`region_outage_alert_ttl=60`, `minor_outage_alert_threshold=5`,
`major_outage_alert_threshold=10`, `max_outage_alert_list_size=1000`,
`log_to_console=false`), `alert_types` (list), `alert_to_webhook_url` (dict),
`alert_type_config` (dict: `digest` bool, `digest_interval` int).

#### Alert types

**Default On ✅:** `llm_exceptions`, `llm_too_slow`, `llm_requests_hanging`,
`cooldown_deployment`, `new_model_added`, `outage_alerts`,
`region_outage_alerts`, `budget_alerts`, `spend_reports`,
`failed_tracking_spend`, `daily_reports`, `fallback_reports`,
`db_exceptions`.

**Default Off ❌:** `new_virtual_key_created`, `virtual_key_updated`,
`virtual_key_deleted`, `new_team_created`, `team_updated`, `team_deleted`,
`new_internal_user_created`, `internal_user_updated`,
`internal_user_deleted`.

#### Env / endpoints

`SLACK_WEBHOOK_URL`, `SLACK_WEBHOOK_URL_2..20`, `WEBHOOK_URL`.
`GET /health/services?service=slack`, `GET /health/services?service=webhook`.

#### In-memory availability

Budget alerts (`budget_alerts`, `spend_reports`, `failed_tracking_spend`)
require DB-backed spend tracking — **UNAVAILABLE**. `db_exceptions` moot
without DB. Region-outage alerting is Enterprise — **UNAVAILABLE**.

Alerting works **WITHOUT DB** for: `llm_exceptions`, `llm_too_slow`,
`llm_requests_hanging`, `cooldown_deployment`, `new_model_added`,
`outage_alerts`, `daily_reports`, `fallback_reports`. Digest mode is
per-instance in-memory.

#### Caveats (verbatim)

> "400 status code errors are not counted (i.e. BadRequestErrors)" — for
> region-outage.

> "Per-instance... If you run multiple instances... each instance maintains
> its own digest."

> "Not durable: If an instance is terminated before the digest interval
> expires, the aggregated alerts... are lost."

### Metrics / Prometheus

`litellm_settings.callbacks: ["prometheus"]` or `- prometheus`. `GET /metrics`
endpoint. Works **WITHOUT DB and WITHOUT Redis** for basic metrics.

Keys: `prometheus_initialize_budget_metrics` (bool — moot without DB),
`enable_end_user_cost_tracking_prometheus_only` (bool, disabled by default
to keep cardinality bounded), `prometheus_emit_stream_label` (bool),
`custom_prometheus_metadata_labels` (list), `custom_prometheus_tags` (list,
e.g. `["prod","staging","User-Agent: RooCode/*"]`),
`prometheus_metrics_config` (list of `{group, metrics, include_labels}`),
`service_callback: ["prometheus_system"]` for system health,
`require_auth_for_metrics_endpoint` (bool — uses `master_key` auth).

Env: `PROMETHEUS_MULTIPROC_DIR` (required for multi-worker aggregated metric
collection; single-process doesn't need it).

`GET /health/backlog` returns `{"in_flight_requests":47}`.

Budget/rate-limit metrics (`litellm_remaining_team_budget_metric` etc.)
require DB-backed virtual keys/teams — **UNAVAILABLE**. Redis only needed
for queue-size metrics (`litellm_redis_daily_spend_update_queue_size` etc.)
and `litellm_redis_latency`/`fails`.

**Deprecated:** `litellm_llm_api_failed_requests_metric` → use
`litellm_proxy_failed_requests_metric`.

#### Caveats (verbatim)

> "By default LiteLLM does not track end_user on Prometheus."

> "By default /metrics endpoint is unauthenticated."

> "_created metrics... _total metrics... You should consume the _total
> metrics."

> "With multiple workers, values are summed across all live workers
> (livesum)."

`GET /daily_metrics` (daily spend/usage) — likely requires DB **(inferred,
spend tracking)**.

### Guardrails

`guardrails:` top-level config list. Each entry:

- `guardrail_name` (required)
- `litellm_params.guardrail` (required provider id)
- `litellm_params.mode` (required: `"pre_call"`|`"post_call"`|`"during_call"`|`"logging_only"` — string, list, or `Mode` object)
- `litellm_params.api_key` (required)
- `litellm_params.api_base` (optional)
- `litellm_params.default_on` (optional, default `False`)
- `litellm_params.presidio_language`, `litellm_params.pii_entities_config`,
  `litellm_params.presidio_score_thresholds`,
  `litellm_params.additional_provider_specific_params`,
  `litellm_params.guard_name`,
  `litellm_params.skip_system_message_in_guardrail`
- `guardrail_info` (optional dict, returned on `GET /guardrails/list`)

`model_list[].litellm_params.guardrails` (list of guardrail names —
Model-level Guardrails).

`litellm_settings.skip_system_message_in_guardrail` (global bool).

#### Guardrail provider names (verbatim)

`cato_networks`, `aporia`, `lakera`, `presidio`, `generic_guardrail_api`,
`guardrails_ai`, `azure/text_moderations`, `bedrock`, `hide-secrets`,
`litellm_content_filter`, `OpenAI Moderation`.

Specification block verbatim lists: `"aporia"`, `"bedrock"`, `"guardrails_ai"`,
`"lakera"`, `"presidio"`, `"hide-secrets"`.

#### Unified guardrail path (applies on `/v1/chat/completions` and `/v1/messages`)

Verbatim: "Presidio, Bedrock guardrails, `litellm_content_filter`, OpenAI
Moderation, Generic Guardrail API, and custom code guardrails that define
`apply_guardrail`."

#### Does NOT apply (direct hooks)

Verbatim: "Lakera v2, Aporia, DynamoAI, Javelin, Lasso, Pangea, Model Armor,
Azure Content Safety hooks, Guardrails AI, AIM, Cato Networks, tool
permission, MCP security".

#### Endpoints

`GET /guardrails/list`, `POST /v1/chat/completions` (`guardrails` param —
list or dict), `POST /key/generate` (`guardrails` list — **DB-backed**),
`POST /key/update`, `POST /team/update`
(`metadata {"guardrails":{"modify_guardrails":false}}`).

#### Enterprise (verbatim)

Pass Dynamic Parameters to Guardrail, Control Guardrails per API Key,
Tag-based Guardrail Modes, Model-level Guardrails, Disable team from
turning on/off guardrails.

> **Inconsistency flag:** Model-level Guardrails is listed Enterprise in
> "Proxy Admin Controls" but shown in the spec example.

#### Guardrails config works WITHOUT a database

Static `guardrails:` config, `GET /guardrails/list`, per-request
`guardrails` param, and `default_on: true` all work without DB.

#### Guardrail Policies ([Beta])

`policies:` + `policy_attachments:` top-level config.

- `policies.<name>.description` / `inherit` / `guardrails.add` /
  `guardrails.remove` / `condition.model` / `pipeline`
- `policy_attachments[].policy` / `scope` / `teams` / `keys` / `models` /
  `tags`

`scope:"*"` (global) and `condition.model` work without DB. Team/key-based
attachments are Enterprise. Tag-based reads `metadata.tags` on keys/teams —
moot without virtual keys/teams. `POST /policies/resolve` works without DB.
Page is **[Beta]**.

#### Caveat (verbatim)

> `default_on: true` "will run even if user specifies a different guardrail
> or empty guardrails array".

## Config keys / requirements

See the per-area tables above. All caching/logging/alerting/metrics/
guardrails keys are sourced from `schemas/config-yaml.option-index.json`
and the cited extracted cards. Keys with `requires_db=true`:
`prometheus_initialize_budget_metrics` (moot), budget/rate-limit metrics,
`token_rate_limit_type`. Keys with `requires_redis=true`: `redis`/
`redis-semantic` cache types, `enable_redis_auth_cache`, queue-size metrics.

## Features that REQUIRE DB/Redis

| Feature | Requirement | Available in-memory? |
| --- | --- | --- |
| `redis` / `redis-semantic` cache | Redis | **NO** |
| `enable_redis_auth_cache` | Redis (+ virtual keys) | **NO** (moot) |
| Budget alerts (`budget_alerts`, `spend_reports`, `failed_tracking_spend`) | DB-backed spend | **NO** |
| `db_exceptions` alerts | DB | moot |
| Region-outage alerting | Enterprise | **NO** |
| Budget/rate-limit Prometheus metrics | DB-backed virtual keys/teams | **NO** |
| Redis queue-size / latency / fails metrics | Redis | **NO** |
| `prometheus_initialize_budget_metrics` | DB | moot |
| `/daily_metrics` | DB (inferred — spend tracking) | **NO** |
| Per-key guardrail control (`/key/generate` `guardrails`) | DB | **NO** |
| Team/key-based policy attachments | Enterprise + DB | **NO** |
| Tag-based guardrail modes | virtual keys/teams (DB) | moot |

Works **without** DB/Redis: `local`/`disk`/`s3`/`gcs`/`qdrant-semantic`/
`valkey-semantic` cache; all logging callbacks (external integrations);
`llm_exceptions`/`llm_too_slow`/`llm_requests_hanging`/`cooldown_deployment`/
`new_model_added`/`outage_alerts`/`daily_reports`/`fallback_reports` alerts;
basic Prometheus metrics; static `guardrails:` config + `GET /guardrails/list`
+ per-request `guardrails` param + `default_on`; global `scope:"*"` and
`condition.model` policies.

## Pitfalls

- **`/metrics` is unauthenticated by default.** Set
  `require_auth_for_metrics_endpoint: true` (uses `master_key`) if the
  endpoint is reachable outside the sandbox.
- **`_created` vs `_total` metrics.** Consume the `_total` metrics
  (verbatim). The `_created` series are Prometheus boilerplate.
- **Multi-worker metrics not aggregated** without `PROMETHEUS_MULTIPROC_DIR`.
  Single-process workestrator is fine; scaling out requires it.
- **`forward_traceparent_to_llm_provider` breaks Bedrock/VertexAI.** Only
  use for self-hosted LLMs (verbatim). The workestrator's upstreams are
  Kimi/MiniMax/OpenRouter/Neuralwatt — verify before enabling.
- **`default_on: true` guardrails run even on empty `guardrails` array.**
  Verbatim caveat — a `default_on` guardrail cannot be opted out per
  request.
- **Budget/spend alerts silently unavailable.** `budget_alerts`,
  `spend_reports`, `failed_tracking_spend` need DB-backed spend; they will
  not fire in-memory even if configured.
- **Region-outage digest is per-instance and not durable.** If the
  microVM restarts before the digest interval expires, aggregated alerts
  are lost (verbatim).
- **400/BadRequestErrors not counted** for region-outage. A steady stream
  of 400s will not trip outage alerts.
- **`azure-blob` cache type** may not be a valid proxy `cache_params.type`
  (SDK-side only) — **(inferred)**. Prefer `s3`/`gcs`/`disk`/`local`.
- **Model-level Guardrails inconsistency.** Listed Enterprise in "Proxy
  Admin Controls" but shown in spec example — flag before relying on it.
- **OTLP gRPC needs `grpcio`.** Install via `uv add litellm[grpc]` (verbatim).

## Related schema / workflow

- [`schemas/config-yaml.option-index.json`](../schemas/config-yaml.option-index.json) — `cache_params`, `litellm_settings.callbacks`, `general_settings.alerting`, guardrail keys with `requires_db`/`requires_redis` flags.
- [`schemas/env-vars.index.json`](../schemas/env-vars.index.json) — `LANGFUSE_*`, `OTEL_*`, `GCS_*`, `SLACK_WEBHOOK_URL`, `PROMETHEUS_MULTIPROC_DIR`.
- [`endpoints/README.md`](../endpoints/README.md) — `/metrics`, `/cache/*`, `/guardrails/list`, `/health/backlog`, `/health/services`.
- `litellm-caching` (`.agents/skills/litellm-caching/`) — operational skill (DB-free cache types).
- `litellm-logging-observability` (`.agents/skills/litellm-logging-observability/`) — operational skill (callbacks/alerting/prometheus).
- `litellm-guardrails` (`.agents/skills/litellm-guardrails/`) — operational skill (static guardrails).

## Workestrator notes

[PROJECT CONTEXT — NOT upstream docs]

- The real config (`infra/litellm/config.yaml`) sets **none** of the
  observability/cache/guardrails keys. No `cache`, no `callbacks`, no
  `alerting`, no `guardrails`, no `prometheus` callbacks. This is a
  deliberate minimal in-memory baseline.
- `general_settings.disable_spend_logs: true` is set (the only
  observability-adjacent key) — it prevents DB spend-log write errors.
- To add lightweight observability without DB/Redis, the recommended
  (project) additions are:
  - `litellm_settings.callbacks: ["prometheus"]` + `GET /metrics`
    (single-process, no `PROMETHEUS_MULTIPROC_DIR` needed).
  - `cache_params: {type: local}` or `{type: disk, disk_cache_dir: /tmp/litellm-cache}`
    for idempotent coding-tier responses.
  - `general_settings.alerting: ["slack"]` + `SLACK_WEBHOOK_URL` for
    `llm_exceptions` / `llm_too_slow` / `cooldown_deployment` /
    `fallback_reports` (all DB-free).
- Budget/spend alerts, `/daily_metrics`, budget/rate-limit Prometheus
  metrics, and per-key guardrail control remain **unavailable** until
  Postgres is added (M4). See
  [workestrator-recommended-patterns.md](../workestrator-recommended-patterns.md).
- `user_api_key_cache_ttl` (default 60s) governs the `master_key` auth
  cache TTL — relevant even in-memory (single key, low churn).
