---
source_url: https://docs.litellm.ai/docs/proxy/prometheus
canonical_url: https://docs.litellm.ai/docs/proxy/prometheus
title: "📈 Prometheus metrics"
sidebar_section_path: proxy > logging (Logging, Alerting, Metrics)
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: logging
---
# 📈 Prometheus metrics

## Headings
- Quick Start
  - Multiple Workers
- Virtual Keys, Teams, Internal Users
  - Team - Budget
  - Virtual Key - Budget
  - Virtual Key - Rate Limit
  - Initialize Budget Metrics on Startup
- Pod Health Metrics
  - When to use this
- Proxy Level Tracking Metrics
  - Callback Logging Metrics
- LLM Provider Metrics
  - Labels Tracked
  - Success and Failure
  - Remaining Requests and Tokens
  - Deployment State
    - Fallback (Failover) Metrics
- Request Counting Metrics
- Request Latency Metrics
- Tracking end_user on Prometheus
  - Emit Stream Label
- [BETA] Custom Metrics
  - Custom Metadata Labels
  - Custom Tags
- Configuring Metrics and Labels
  - Enable Specific Metrics and Labels
  - Filter Labels Per Metric
  - Advanced Configuration
- Monitor System Health
  - DB Transaction Queue Health Metrics
- LiteLLM Maintained Grafana Dashboards
- Deprecated Metrics
- Add authentication on /metrics endpoint
- FAQ

## Exact config keys found (full path, verbatim spelling)
- `litellm_settings.callbacks` (litellm_settings) — set to `["prometheus"]` or `- prometheus`
- `litellm_settings.prometheus_initialize_budget_metrics` (litellm_settings) — bool
- `litellm_settings.enable_end_user_cost_tracking_prometheus_only` (litellm_settings) — bool
- `litellm_settings.prometheus_emit_stream_label` (litellm_settings) — bool
- `litellm_settings.custom_prometheus_metadata_labels` (litellm_settings) — list e.g. `["metadata.foo", "metadata.bar"]`
- `litellm_settings.custom_prometheus_tags` (litellm_settings) — list e.g. `["prod", "staging", "User-Agent: RooCode/*"]`
- `litellm_settings.prometheus_metrics_config` (litellm_settings) — list of `{group, metrics, include_labels}`
- `litellm_settings.prometheus_metrics_config[].group` — string
- `litellm_settings.prometheus_metrics_config[].metrics` — list of metric names
- `litellm_settings.prometheus_metrics_config[].include_labels` — list of label names
- `litellm_settings.service_callback` (litellm_settings) — set to `["prometheus_system"]` for system health
- `litellm_settings.require_auth_for_metrics_endpoint` (litellm_settings) — bool; auth on /metrics

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Quick Start
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: gpt-4o

litellm_settings:
  callbacks:
    - prometheus
```
(source: https://docs.litellm.ai/docs/proxy/prometheus)

```yaml
# Initialize Budget Metrics on Startup
litellm_settings:
  callbacks: ["prometheus"]
  prometheus_initialize_budget_metrics: true
```
(source: https://docs.litellm.ai/docs/proxy/prometheus)

```yaml
# Tracking end_user on Prometheus
litellm_settings:
  callbacks: ["prometheus"]
  enable_end_user_cost_tracking_prometheus_only: true
```
(source: https://docs.litellm.ai/docs/proxy/prometheus)

```yaml
# Custom Metadata Labels
model_list:
  - model_name: openai/gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_key: os.environ/OPENAI_API_KEY

litellm_settings:
  callbacks: ["prometheus"]
  custom_prometheus_metadata_labels: ["metadata.foo", "metadata.bar"]
```
(source: https://docs.litellm.ai/docs/proxy/prometheus)

```yaml
# Custom Tags
litellm_settings:
  callbacks: ["prometheus"]
  custom_prometheus_metadata_labels: ["metadata.foo", "metadata.bar"]
  custom_prometheus_tags: 
    - "prod"
    - "staging"
    - "batch-job"
    - "User-Agent: RooCode/*"
    - "User-Agent: claude-cli/*"
```
(source: https://docs.litellm.ai/docs/proxy/prometheus)

```yaml
# Enable Specific Metrics and Labels
model_list:
 - model_name: gpt-4o
    litellm_params:
      model: gpt-4o

litellm_settings:
  callbacks: ["prometheus"]
  prometheus_metrics_config:
    - group: "proxy_metrics"
      metrics:
        - "litellm_proxy_total_requests_metric"
        - "litellm_proxy_failed_requests_metric"
      include_labels:
        - "hashed_api_key"
        - "requested_model"
        - "model_group"
```
(source: https://docs.litellm.ai/docs/proxy/prometheus)

```yaml
# Add authentication on /metrics endpoint
litellm_settings:
  require_auth_for_metrics_endpoint: true
```
(source: https://docs.litellm.ai/docs/proxy/prometheus)

```yaml
# Monitor System Health
model_list:
 - model_name: gpt-4o
    litellm_params:
      model: gpt-4o

litellm_settings:
  service_callback: ["prometheus_system"]
```
(source: https://docs.litellm.ai/docs/proxy/prometheus)

## Exact environment variables
- `PROMETHEUS_MULTIPROC_DIR` — "When using LiteLLM with multiple workers, you need to set the `PROMETHEUS_MULTIPROC_DIR` environment variable to enable aggregated metric collection across worker processes."

## Exact endpoint paths / API routes (runtime + management)
- `GET /metrics` — Prometheus scrape endpoint; "LiteLLM Exposes a `/metrics` endpoint for Prometheus to Poll"
- `GET /health/backlog` — Returns `{"in_flight_requests": 47}`; current value of `litellm_in_flight_requests`

## Exact CLI commands
- `uv add prometheus_client==0.20.0` — Install prometheus client dependency (already pre-installed on litellm Docker image)
- `export PROMETHEUS_MULTIPROC_DIR="/prometheus_multiproc"` — Required env var for multi-worker aggregated metric collection
- `litellm --config config.yaml --debug` — Start the proxy

## Requirements
- database: not documented on this page (mentioned only in metric descriptions, e.g., "Every 5 minutes litellm runs a cron job to read all keys, teams from the database")
- redis: partially — DB Transaction Queue Health Metrics section lists `litellm_pod_lock_manager_size`, `litellm_redis_daily_spend_update_queue_size`, `litellm_redis_spend_update_queue_size` with Storage Type = "Redis". Also `litellm_redis_latency`, `litellm_redis_fails` in Monitor System Health. However, Prometheus metrics themselves do NOT require Redis — they only require `prometheus_client` (already in Docker image). Redis only enters for queue-size metrics and multi-worker aggregation.
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- "`litellm_llm_api_failed_requests_metric` — **deprecated** use `litellm_proxy_failed_requests_metric`"

## Caveats / pitfalls
- "By default LiteLLM does not track `end_user` on Prometheus. This is done to reduce the cardinality of the metrics from LiteLLM Proxy."
- "By default /metrics endpoint is unauthenticated."
- "This label is opt-in because adding a new label to an existing metric changes its cardinality and breaks existing Prometheus queries / Grafana dashboards"
- "Default Behavior: If no `prometheus_metrics_config` is specified, all metrics are enabled with their default labels (backward compatible)."
- "`litellm_input_cached_tokens_metric` tracks **provider-side** prompt-cache reads... This is different from `litellm_cached_tokens_metric`, which tracks LiteLLM's own response-cache hits"
- "`_created` metrics are metrics that are created when the proxy starts; `_total` metrics are metrics that are incremented for each request. You should consume the `_total` metrics"
- "With multiple workers, values are summed across all live workers (`livesum`)."
- Re `PROMETHEUS_MULTIPROC_DIR`: "Make sure the directory exists and is writable by your LiteLLM process."

## Related links
- /docs/proxy/alerting
- /docs/proxy/pagerduty
- /docs/proxy/pyroscope_profiling
- /docs/proxy/virtual_keys
- https://github.com/BerriAI/litellm/tree/main/cookbook/litellm_proxy_server/grafana_dashboard
- /docs/enterprise
- https://litellm-api.up.railway.app/ (Swagger)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- CRITICAL: Prometheus metrics work WITHOUT a database and WITHOUT Redis (for basic metrics). `litellm_settings.callbacks: ["prometheus"]` + `GET /metrics` endpoint work in-memory. The workestrator can use Prometheus for request counting, latency, LLM provider metrics, deployment state, fallback metrics, and proxy-level tracking. Budget/rate-limit metrics (`litellm_remaining_team_budget_metric`, `litellm_remaining_team_budget_metric`, etc.) require DB-backed virtual keys/teams (unavailable). `prometheus_initialize_budget_metrics` is moot without DB. `require_auth_for_metrics_endpoint: true` works without DB (uses master_key auth). `custom_prometheus_tags` with `User-Agent: claude-cli/*` is directly relevant for workestrator agent tracking. Multi-worker requires `PROMETHEUS_MULTIPROC_DIR` (single-process in-memory doesn't need it).

## Confidence / uncertainty notes
- high confidence on config keys and that Prometheus works without DB/Redis (verbatim YAML + `prometheus_client` is the only dependency). Redis is only needed for queue-size metrics (verbatim Storage Type column). Budget metrics require DB (inferred from "read all keys, teams from the database" cron job).
