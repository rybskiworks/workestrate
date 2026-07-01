---
name: litellm-logging-observability
description: |
  Operational reference for LiteLLM logging callbacks, alerting, and
  Prometheus metrics: `success_callback`/`failure_callback`/`callbacks`
  (langfuse/otel/datadog/prometheus etc., no DB), `turn_off_message_logging`,
  `redact_user_api_key_info`, Slack/webhook alerting (no DB; spend alerts
  need DB), and `/metrics`. Load when wiring a callback, enabling Slack
  alerts, exposing Prometheus, or redacting logs. Does NOT cover caching
  (see litellm-caching), guardrails, routing (see litellm-routing-fallbacks),
  or deployment/runtime flags (see litellm-deployment). Full detail lives in
  docs/litellm/observability-cache-guardrails/README.md.
---

# LiteLLM Logging, Observability & Alerting

Distilled operational reference for shipping LiteLLM traces/logs to external
systems, alerting via Slack/webhook, and exposing Prometheus metrics. Most
of this surface works **without a database**; the exceptions are spend/budget
alerts and budget/rate-limit metrics. Full detail lives in:

- `docs/litellm/observability-cache-guardrails/README.md` — logging/alerting/
  metrics/guardrails synthesis (DB/Redis gating).
- `docs/litellm/config/litellm-settings.md` — callback keys.
- `docs/litellm/schemas/config-yaml.option-index.json` — `litellm_settings.callbacks`,
  `general_settings.alerting` with `requires_db`/`requires_redis` flags.
- `docs/litellm/schemas/env-vars.index.json` — `LANGFUSE_*`, `OTEL_*`,
  `SLACK_WEBHOOK_URL`, `PROMETHEUS_MULTIPROC_DIR`.

> **Do not hallucinate.** Every callback name/key below traces to
> `docs/litellm/observability-cache-guardrails/README.md` or
> `docs/litellm/config/litellm-settings.md`. Inferred items are marked
> **(inferred)**.

## Triggers

Load this skill when:

- Wiring a logging callback (langfuse, otel, datadog, prometheus, sentry).
- Enabling Slack/webhook alerting.
- Exposing `/metrics` (Prometheus).
- Redacting messages or user API key info from logs.
- Diagnosing why budget/spend alerts didn't fire (DB requirement).

## Logging / callbacks

`litellm_settings.success_callback` / `failure_callback` / `callbacks`
(`callbacks` runs on both success and failure — preferred).
`service_callbacks` (`["datadog","prometheus"]`) logs redis/postgres failures.

### Callback names (verbatim)

`langfuse`, `otel`, `gcs_bucket`, `gcs_pubsub`, `deepeval`, `s3_v2`,
`aws_sqs`, `azure_storage`, `lunary`, `langsmith`, `arize`, `langtrace`,
`generic_api`, `datadog`, `prometheus`, `sentry`.

All logging callbacks work **WITHOUT a database** (external integrations).
DB requirement "not documented" on the logging page.

### Logging settings keys

| Key | Type | Notes |
|-----|------|-------|
| `turn_off_message_logging` | bool | Prevents messages/responses from being logged to callbacks; request metadata still logged. |
| `redact_user_api_key_info` | bool | Redacts hashed token, user_id, team id from logs. Supported for Langfuse, OpenTelemetry, Logfire, ArizeAI. |
| `global_disable_no_log_param` | bool | — |
| `langfuse_default_tags` | list | Default tags for Langfuse logging. |
| `forward_traceparent_to_llm_provider` | bool | Verbatim: "Only use this for self hosted LLMs, this can cause Bedrock, VertexAI calls to fail". |
| `json_logs` | bool | JSON log format (also `JSON_LOGS=True` env). |

`callback_settings.otel.message_logging` (bool) — OTEL message redaction.
Custom callback class: `callbacks: custom_callbacks.proxy_handler_instance`.

### Enterprise (verbatim "✨ enterprise feature")

GCS Buckets, GCS PubSub, Azure Blob Storage, Custom Callback APIs,
Dynamically Disable specific callbacks.

### Env vars

`LANGFUSE_PUBLIC_KEY`/`SECRET_KEY`/`HOST`,
`OTEL_TRACER_NAME`/`OTEL_SERVICE_NAME`/`OTEL_EXPORTER`/`OTEL_ENDPOINT`/`OTEL_HEADERS`,
`GCS_BUCKET_NAME`/`GCS_PATH_SERVICE_ACCOUNT`/`GCS_PUBSUB_TOPIC_ID`/`GCS_PUBSUB_PROJECT_ID`,
`LUNARY_PUBLIC_KEY`, `GENERIC_LOGGER_ENDPOINT`/`GENERIC_LOGGER_HEADERS`,
`LANGSMITH_API_KEY`/`LANGSMITH_PROJECT`/`LANGSMITH_BASE_URL`, `ARIZE_*`,
`LANGTRACE_API_KEY`, `GALILEO_*`.

> "OTLP gRPC requires grpcio. Install via `uv add litellm[grpc]`."

## Alerting

`general_settings.alerting` (`["slack"]`, `["webhook"]`),
`alerting_threshold` (seconds — alert when requests hang/responses take
longer), `spend_report_frequency` (`"1d"`), `alerting_args` (sub-keys:
`daily_report_frequency=43200`, `report_check_interval=3600`,
`budget_alert_ttl=86400`, `outage_alert_ttl=60`,
`region_outage_alert_ttl=60`, `minor_outage_alert_threshold=5`,
`major_outage_alert_threshold=10`, `max_outage_alert_list_size=1000`,
`log_to_console=false`), `alert_types` (list), `alert_to_webhook_url` (dict),
`alert_type_config` (dict: `digest` bool, `digest_interval` int).

### Alert types

**Default On:** `llm_exceptions`, `llm_too_slow`, `llm_requests_hanging`,
`cooldown_deployment`, `new_model_added`, `outage_alerts`,
`region_outage_alerts`, `budget_alerts`, `spend_reports`,
`failed_tracking_spend`, `daily_reports`, `fallback_reports`,
`db_exceptions`.

**Default Off:** `new_virtual_key_created`, `virtual_key_updated`,
`virtual_key_deleted`, `new_team_created`, `team_updated`, `team_deleted`,
`new_internal_user_created`, `internal_user_updated`, `internal_user_deleted`.

### Env / endpoints

`SLACK_WEBHOOK_URL`, `SLACK_WEBHOOK_URL_2..20`, `WEBHOOK_URL`.
`GET /health/services?service=slack`, `GET /health/services?service=webhook`.

### In-memory availability

Budget alerts (`budget_alerts`, `spend_reports`, `failed_tracking_spend`)
require DB-backed spend tracking — **UNAVAILABLE**. `db_exceptions` moot
without DB. Region-outage alerting is Enterprise — **UNAVAILABLE**.

Alerting works **WITHOUT DB** for: `llm_exceptions`, `llm_too_slow`,
`llm_requests_hanging`, `cooldown_deployment`, `new_model_added`,
`outage_alerts`, `daily_reports`, `fallback_reports`. Digest mode is
per-instance in-memory.

### Caveats (verbatim)

> "400 status code errors are not counted (i.e. BadRequestErrors)" — for
> region-outage.

> "Per-instance... If you run multiple instances... each instance maintains
> its own digest."

> "Not durable: If an instance is terminated before the digest interval
> expires, the aggregated alerts... are lost."

## Metrics / Prometheus

`litellm_settings.callbacks: ["prometheus"]` (or `- prometheus`).
`GET /metrics` endpoint. Works **WITHOUT DB and WITHOUT Redis** for basic
metrics.

### Keys

| Key | Type | DB? | Notes |
|-----|------|-----|-------|
| `prometheus_initialize_budget_metrics` | bool | YES | Moot without DB. |
| `enable_end_user_cost_tracking_prometheus_only` | bool (default false) | no | Disabled by default to keep cardinality bounded. |
| `prometheus_emit_stream_label` | bool | no | — |
| `custom_prometheus_metadata_labels` | list | no | — |
| `custom_prometheus_tags` | list | no | e.g. `["prod","staging","User-Agent: RooCode/*"]`. |
| `prometheus_metrics_config` | list | no | List of `{group, metrics, include_labels}`. |
| `service_callback` | list | no | `["prometheus_system"]` for system health. |
| `require_auth_for_metrics_endpoint` | bool | no | Uses `master_key` auth. |

Env: `PROMETHEUS_MULTIPROC_DIR` (required for multi-worker aggregated metric
collection; single-process doesn't need it).

`GET /health/backlog` returns `{"in_flight_requests":47}`.

Budget/rate-limit metrics (`litellm_remaining_team_budget_metric` etc.)
require DB-backed virtual keys/teams — **UNAVAILABLE**. Redis only needed
for queue-size metrics and `litellm_redis_latency`/`fails`.

**Deprecated:** `litellm_llm_api_failed_requests_metric` → use
`litellm_proxy_failed_requests_metric`.

### Caveats (verbatim)

> "By default LiteLLM does not track end_user on Prometheus."
> "By default /metrics endpoint is unauthenticated."
> "_created metrics... _total metrics... You should consume the _total metrics."
> "With multiple workers, values are summed across all live workers (livesum)."

`GET /daily_metrics` (daily spend/usage) — likely requires DB **(inferred,
spend tracking)**.

## Common Mistakes

| Mistake | Cause | Fix |
|---------|-------|-----|
| `/metrics` exposed unauthenticated | Default is unauthenticated | Set `require_auth_for_metrics_endpoint: true` if reachable outside sandbox. |
| Consuming `_created` metrics | Prometheus boilerplate | Consume the `_total` metrics (verbatim). |
| Multi-worker metrics not aggregated | Missing `PROMETHEUS_MULTIPROC_DIR` | Set env var (single-process is fine). |
| `forward_traceparent_to_llm_provider` breaks Bedrock/VertexAI | Verbatim caveat | Only use for self-hosted LLMs; verify upstreams. |
| Budget/spend alerts don't fire | Need DB-backed spend | Unavailable in-memory until Postgres added. |
| Region-outage digest lost on restart | Per-instance, not durable | Accept for single-instance; add Redis/DB for durability. |
| 400s don't trip outage alerts | BadRequestErrors not counted | Tune `allowed_fails_policy` if needed. |
| OTLP gRPC fails | Missing `grpcio` | `uv add litellm[grpc]`. |
| `prometheus_initialize_budget_metrics` set without DB | requires_db=true | Leave unset in-memory. |

## Project context [PROJECT — not upstream docs]

The workestrator config sets **none** of the observability keys — no
`callbacks`, no `alerting`, no `prometheus`, no `cache`. The only
observability-adjacent key is `general_settings.disable_spend_logs: true`
(prevents DB spend-log write errors). Recommended DB-free additions:

- `litellm_settings.callbacks: ["prometheus"]` + `GET /metrics`
  (single-process, no `PROMETHEUS_MULTIPROC_DIR` needed).
- `general_settings.alerting: ["slack"]` + `SLACK_WEBHOOK_URL` for
  `llm_exceptions` / `llm_too_slow` / `cooldown_deployment` /
  `fallback_reports` (all DB-free).
- `require_auth_for_metrics_endpoint: true` if `/metrics` is reachable.

Budget/spend alerts, `/daily_metrics`, budget/rate-limit Prometheus metrics,
and per-key guardrail control remain **unavailable** until Postgres is added.

## Related Docs

- `docs/litellm/observability-cache-guardrails/README.md`
- `docs/litellm/config/litellm-settings.md`
- `docs/litellm/config/general-settings.md`
- `docs/litellm/schemas/config-yaml.option-index.json`
- `docs/litellm/schemas/env-vars.index.json`
