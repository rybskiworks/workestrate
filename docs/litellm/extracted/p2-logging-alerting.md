---
source_url: https://docs.litellm.ai/docs/proxy/alerting
canonical_url: https://docs.litellm.ai/docs/proxy/alerting
title: "Alerting / Webhooks"
sidebar_section_path: proxy > logging (Logging, Alerting, Metrics)
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: logging
---
# Alerting / Webhooks

## Headings
- Quick Start
  - Step 1: Add a Slack Webhook URL to env
  - Step 2: Setup Proxy
  - Step 3: Test it!
- Advanced
  - Redacting Messages from Alerts
  - Soft Budget Alerts for Virtual Keys
  - Add Metadata to alerts
  - Select specific alert types
  - Map slack channels to alert type
  - MS Teams Webhooks
  - Discord Webhooks
- [BETA] Webhooks for Budget Alerts
  - API Spec for Webhook Event
  - Digest Mode (Reducing Alert Noise)
- Region-outage alerting (Enterprise feature)
- All Possible Alert Types
- alerting_args Specification

## Exact config keys found (full path, verbatim spelling)
- `general_settings.alerting` (general_settings) — list of alerting channels, e.g. `["slack"]`, `["webhook"]`
- `general_settings.alerting_threshold` (general_settings) — seconds; alert when requests hang or responses take longer than this
- `general_settings.spend_report_frequency` (general_settings) — e.g. `"1d"`
- `general_settings.alerting_args` (general_settings) — sub-keys: `daily_report_frequency`, `report_check_interval`, `budget_alert_ttl`, `outage_alert_ttl`, `region_outage_alert_ttl`, `minor_outage_alert_threshold`, `major_outage_alert_threshold`, `max_outage_alert_list_size`, `log_to_console`
- `general_settings.alert_types` (general_settings) — list, opt-in alert types
- `general_settings.alert_to_webhook_url` (general_settings) — dict mapping alert_type → webhook url or list of urls
- `general_settings.alert_type_config` (general_settings) — dict mapping alert_type → `{digest: bool, digest_interval: int}`
- `general_settings.master_key` (general_settings)
- `litellm_settings.redact_messages_in_exceptions` (litellm_settings)
- `litellm_settings.success_callback` (litellm_settings)

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Quick Start - Step 2: Setup Proxy
general_settings: 
    alerting: ["slack"]
    alerting_threshold: 300 # sends alerts if requests hang for 5min+ and responses take 5min+ 
    spend_report_frequency: "1d" # [Optional] set as 1d, 2d, 30d .... Specify how often you want a Spend Report to be sent

    # [OPTIONAL ALERTING ARGS]
    alerting_args:
        daily_report_frequency: 43200  # 12 hours in seconds
        report_check_interval: 3600    # 1 hour in seconds
        budget_alert_ttl: 86400        # 24 hours in seconds
        outage_alert_ttl: 60           # 1 minute in seconds
        region_outage_alert_ttl: 60    # 1 minute in seconds
        minor_outage_alert_threshold: 5 
        major_outage_alert_threshold: 10
        max_outage_alert_list_size: 1000
        log_to_console: false    
```
(source: https://docs.litellm.ai/docs/proxy/alerting)

```yaml
# Select specific alert types
general_settings:
  alerting: ["slack"]
  alert_types: [
    "llm_exceptions",
    "llm_too_slow",
    "llm_requests_hanging",
    "budget_alerts",
    "spend_reports",
    "db_exceptions",
    "daily_reports",
    "cooldown_deployment",
    "new_model_added",
  ] 
```
(source: https://docs.litellm.ai/docs/proxy/alerting)

```yaml
# [BETA] Webhooks for Budget Alerts
general_settings: 
  alerting: ["webhook"] # 👈 KEY CHANGE
```
(source: https://docs.litellm.ai/docs/proxy/alerting)

```yaml
# Digest Mode (Reducing Alert Noise)
general_settings:
  alerting: ["slack"]
  alert_type_config:
    llm_requests_hanging:
      digest: true
      digest_interval: 86400  # 24 hours (default)
    llm_too_slow:
      digest: true
      digest_interval: 3600   # 1 hour
    llm_exceptions:
      digest: true
      # uses default interval (86400 seconds / 24 hours)
```
(source: https://docs.litellm.ai/docs/proxy/alerting)

```yaml
# Region-outage alerting (Enterprise)
general_settings: 
    alerting: ["slack"]
    alert_types: ["region_outage_alerts"] 
    alerting_args:
        region_outage_alert_ttl: 60 # time-window in seconds
        minor_outage_alert_threshold: 5 # number of errors to trigger a minor alert
        major_outage_alert_threshold: 10 # number of errors to trigger a major alert
```
(source: https://docs.litellm.ai/docs/proxy/alerting)

## Exact environment variables
- `SLACK_WEBHOOK_URL` — Slack incoming webhook URL (also used for MS Teams and Discord with `/slack` suffix appended)
- `SLACK_WEBHOOK_URL_2` … `SLACK_WEBHOOK_URL_20` — Multiple slack webhook env vars
- `WEBHOOK_URL` — Generic webhook URL for budget alerts (BETA)

## Exact endpoint paths / API routes (runtime + management)
- `GET /health/services?service=slack` — Health check for slack alerting connection
- `GET /health/services?service=webhook` — Health check for webhook alerting connection
- `POST /key/generate` — Create virtual key (with `soft_budget` for soft budget alerts)
- `POST /chat/completions` / `POST /v1/chat/completions` — Standard chat completions (with `metadata.alerting_metadata`)

## Exact CLI commands
- `litellm --config /path/to/config.yaml` — Start proxy with alerting config

## Requirements
- database: not explicitly documented on this page. Database-related alert type `db_exceptions` is listed, implying DB is required for it, but no verbatim DB requirement text.
- redis: not documented on this page (no Redis mention). Alerting uses TTL/cache values (`budget_alert_ttl`, `outage_alert_ttl`) which imply Redis/cache backend, but Redis is not named verbatim.
- enterprise: required for region-outage alerting — verbatim: "Region-outage alerting (✨ Enterprise feature)" / "Get a free 2-week license"
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- "Note: This is a beta feature, so the spec might change." ([BETA] Webhooks for Budget Alerts)
- "'400' status code errors are not counted (i.e. BadRequestErrors)." (Region-outage alerting)
- Digest Mode Limitations: "Per-instance: Digest state is held in memory per proxy instance. If you run multiple instances (e.g., Cloud Run with autoscaling), each instance maintains its own digest and emits its own summary." Also: "Not durable: If an instance is terminated before the digest interval expires, the aggregated alerts for that instance are lost."
- All Possible Alert Types (Default On ✅ / Default Off ❌):
  - `llm_exceptions` ✅, `llm_too_slow` ✅, `llm_requests_hanging` ✅, `cooldown_deployment` ✅, `new_model_added` ✅, `outage_alerts` ✅, `region_outage_alerts` ✅, `budget_alerts` ✅, `spend_reports` ✅, `failed_tracking_spend` ✅, `daily_reports` ✅, `fallback_reports` ✅, `db_exceptions` ✅
  - `new_virtual_key_created` ❌, `virtual_key_updated` ❌, `virtual_key_deleted` ❌, `new_team_created` ❌, `team_updated` ❌, `team_deleted` ❌, `new_internal_user_created` ❌, `internal_user_updated` ❌, `internal_user_deleted` ❌
- Default alerting_args: `daily_report_frequency`=43200, `report_check_interval`=3600, `budget_alert_ttl`=86400, `outage_alert_ttl`=60, `region_outage_alert_ttl`=60, `minor_outage_alert_threshold`=5, `major_outage_alert_threshold`=10, `max_outage_alert_list_size`=1000, `log_to_console`=false

## Related links
- /docs/proxy/alerting (self)
- /docs/proxy/pagerduty (Next)
- https://api.slack.com/messaging/webhooks
- https://enterprise.litellm.ai/demo

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- Alerting (`general_settings.alerting`, `alerting_threshold`, `alert_types`) works WITHOUT a database — it sends webhooks to Slack/MS Teams/Discord. The workestrator can use `alerting: ["slack"]` in-memory for `llm_exceptions`, `llm_too_slow`, `llm_requests_hanging`, `cooldown_deployment`, `new_model_added`, `outage_alerts`, `daily_reports`, `fallback_reports`. Budget alerts (`budget_alerts`, `spend_reports`, `failed_tracking_spend`) require DB-backed spend tracking (unavailable). `db_exceptions` is moot without DB. Region-outage alerting is Enterprise (unavailable). Digest mode works in-memory (per-instance state). `alerting_args` TTL values may use in-memory cache (no Redis confirmed on page).

## Confidence / uncertainty notes
- high confidence on alerting config keys and alert type names (verbatim). DB requirement for budget alerts is inferred (not verbatim on this page). Redis requirement is not documented — alerting TTL/cache may use in-memory (inferred). Enterprise requirement for region-outage is verbatim.
