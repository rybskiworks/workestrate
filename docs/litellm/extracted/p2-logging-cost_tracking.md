---
source_url: https://docs.litellm.ai/docs/proxy/cost_tracking
canonical_url: https://docs.litellm.ai/docs/proxy/cost_tracking
title: "Spend Tracking"
sidebar_section_path: proxy > spend_tracking (Spend Tracking)
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: logging
---
# Spend Tracking

## Headings
- How to Track Spend with LiteLLM
- Allowing Non-Proxy Admins to access /spend endpoints
- Reset Team, API Key Spend - MASTER KEY ONLY
- Total spend per user
- Spend list endpoints (/spend/keys and /spend/users)
- Daily Spend Breakdown API
- Custom Tags
  - Client-side spend tag
  - Add custom headers to spend tracking
  - Disable user-agent tracking
- (Enterprise) Generate Spend Reports
- Spend Logs API - Individual Transaction Logs
- Custom Spend Log metadata

## Exact config keys found (full path, verbatim spelling)
- `general_settings.legacy_unscoped_spend_list_endpoints` (general_settings) — boolean
- `general_settings.scope_spend_list_endpoints_to_caller` (general_settings) — boolean, inverse of legacy mode
- `litellm_settings.extra_spend_tag_headers` (litellm_settings) — list of header names
- `litellm_settings.disable_add_user_agent_to_request_tags` (litellm_settings) — boolean
- `metadata.tags` — request metadata (per-request)
- `metadata.spend_logs_metadata` — request metadata (Enterprise)
- `metadata.alerting_metadata` — request metadata
- `permissions.get_spend_routes` — key permission (boolean)

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Legacy unscoped behavior (upgrade path)
general_settings:
  legacy_unscoped_spend_list_endpoints: true
```
(source: https://docs.litellm.ai/docs/proxy/cost_tracking)

```yaml
# To disable scoping without the legacy flag name
general_settings:
  scope_spend_list_endpoints_to_caller: false
```
(source: https://docs.litellm.ai/docs/proxy/cost_tracking)

```yaml
# Add custom headers to spend tracking
litellm_settings:
  extra_spend_tag_headers:
    - "x-custom-header"
```
(source: https://docs.litellm.ai/docs/proxy/cost_tracking)

```yaml
# Disable user-agent tracking
litellm_settings:
  disable_add_user_agent_to_request_tags: true
```
(source: https://docs.litellm.ai/docs/proxy/cost_tracking)

## Exact environment variables
- `LITELLM_LEGACY_UNSCOPED_SPEND_LIST_ENDPOINTS` — When set, enables legacy unscoped behavior for `/spend/keys` and `/spend/users`

## Exact endpoint paths / API routes (runtime + management)
- `POST /key/generate` — Generate virtual key (with `permissions: {"get_spend_routes": true}` or with `metadata.tags` / `metadata.spend_logs_metadata`)
- `POST /team/new` — Create team (with `metadata.tags` / `metadata.spend_logs_metadata`)
- `GET /global/spend/report` — Generate spend reports. Query params: `start_date`, `end_date`, `group_by` (`team`|`customer`), `api_key`, `internal_user_id`
- `POST /global/spend/reset` — Reset spend for all API Keys and Teams (MASTER KEY only)
- `GET /user/info?user_id=<id>` — Get user spend info
- `GET /spend/logs?start_date=...&end_date=...&summarize=true|false&request_id=<call-id>` — Spend logs (individual transactions with `summarize=false`, aggregated with `summarize=true` default)
- `GET /spend/keys` — List keys ordered by spend (admin sees all, internal user sees own only)
- `GET /spend/users` — List users ordered by spend
- `GET /user/daily/activity?start_date=...&end_date=...` — Daily spend breakdown by model/provider/api_key
- `GET /customer/info?end_user_id=<id>` — Customer (end-user) spend
- `POST /chat/completions` — OpenAI-compatible (with `user`, `metadata.tags`, `metadata.spend_logs_metadata`)

## Exact CLI commands
- none documented on this page

## Requirements
- database: required — verbatim: "Step 1 👉 [Setup LiteLLM with a Database](https://docs.litellm.ai/docs/proxy/virtual_keys#setup)". Also: "Requirements: - Virtual Keys & a database should be set up, see [virtual keys](https://docs.litellm.ai/docs/proxy/virtual_keys)" (appears twice). Database is required for spend tracking.
- redis: not documented on this page
- enterprise: required for spend reports and custom spend log metadata — verbatim: "Logging specific key,value pairs in spend logs metadata is an enterprise feature." Also: "ENTERPRISE: pass tags to track spend by tags"
- admin_ui: not documented on this page

## Deprecations
- Legacy unscoped behavior upgrade path: "Before this scoping change, any authenticated key could list the **full** key/user tables. If you rely on that behavior (for example automation using an `internal_user` key), opt out explicitly..."

## Caveats / pitfalls
- "End users can provide the `user` parameter in their request bodies, doing this will increment the cost reported via `/customer/info?end_user_id=self-declared-user`, and not for the user that owns the key... This means users could 'avoid' having their spend tracked"
- "An internal user who passes `?user_id=` for **another** user receives **HTTP 403** (not a silently filtered list)."
- NOTE: `general_settings.disable_spend_logs` is NOT documented on this page — it is documented on the `/docs/proxy/db_info` page (see p2-deploy-db_info.md)

## Related links
- https://docs.litellm.ai/docs/proxy/virtual_keys#setup
- /docs/proxy/team_logging
- /docs/observability/mlflow
- /docs/proxy/request_tags
- /docs/proxy/custom_pricing
- /docs/proxy/pricing_calculator
- /docs/proxy/billing
- /docs/proxy/config_settings#general_settings---reference
- /docs/troubleshoot/cost_discrepancy

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- Spend tracking REQUIRES a database (virtual keys + DB). The workestrate runs in-memory (no Postgres), so `/spend/*` endpoints, `/global/spend/*`, `/user/daily/activity`, and per-key/user/team spend tracking are UNAVAILABLE. The relevant setting for the in-memory deployment is `general_settings.disable_spend_logs: True` (documented on db_info page, NOT this page) — this prevents the proxy from attempting DB spend log writes. `litellm_settings.extra_spend_tag_headers` and `disable_add_user_agent_to_request_tags` are moot without spend tracking. Cost metrics can still be emitted to Prometheus/external loggers via callbacks (which work without DB).

## Confidence / uncertainty notes
- high confidence on DB requirement (verbatim quotes). The `disable_spend_logs` key is explicitly NOT on this page — it's on db_info (cross-referenced). Enterprise requirements are verbatim.
