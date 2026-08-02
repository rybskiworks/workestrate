---
source_url: https://docs.litellm.ai/docs/proxy/db_info
canonical_url: https://docs.litellm.ai/docs/proxy/db_info
title: "What is stored in the DB"
sidebar_section_path: proxy > other
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: deploy
---
# What is stored in the DB

## Headings
- Link to DB Schema
- DB Tables
  - Organizations, Teams, Users, End Users
  - Authentication
  - Model (LLM) Management
  - Budget Management
  - Tracking & Logging
- Disable LiteLLM_SpendLogs
  - What is the impact of disabling these logs?
- Migrating Databases

## Exact config keys found (full path, verbatim spelling)
- `general_settings.disable_spend_logs` (general_settings) — set to `True` to disable writing spend logs to DB
- `general_settings.disable_error_logs` (general_settings) — set to `True` to disable writing error logs to DB (does NOT stop regular spend logs unless `disable_spend_logs: True` is also set)
- `STORE_MODEL_IN_DB` (env var) — referenced inside the `LiteLLM_ProxyModelTable` row: "Only migrate if you store your LLMs in the DB (i.e you set `STORE_MODEL_IN_DB=True`)"

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Disable LiteLLM_SpendLogs
general_settings:
  disable_spend_logs: True   # Disable writing spend logs to DB
  disable_error_logs: True   # Only disable writing error logs to DB, regular spend logs will still be written unless `disable_spend_logs: True`
```
(source: https://docs.litellm.ai/docs/proxy/db_info)

## Exact environment variables
- `STORE_MODEL_IN_DB` — referenced inside the `LiteLLM_ProxyModelTable` row in the migration table: "Only migrate if you store your LLMs in the DB (i.e you set `STORE_MODEL_IN_DB=True`)". No further documentation on this page.

## Exact endpoint paths / API routes (runtime + management)
- none documented on this page

## Exact CLI commands
- none documented on this page

## Requirements
- database: required (PostgreSQL) — verbatim: "The LiteLLM Proxy uses a PostgreSQL database to store various information. Here's are the main features the DB is used for:" Features listed: "Virtual Keys, Organizations, Teams, Users, Budgets, and more." and "Per request Usage Tracking". Migration table Required rows (verbatim):
  - "`LiteLLM_VerificationToken` **Required** to ensure existing virtual keys continue working"
  - "`LiteLLM_UserTable` **Required** to ensure existing virtual keys continue working"
  - "`LiteLLM_TeamTable` **Required** to ensure Teams are migrated"
  - "`LiteLLM_TeamMembership` **Required** to ensure Teams member budgets are migrated"
  - "`LiteLLM_BudgetTable` **Required** to migrate existing budgeting settings"
- redis: not documented on this page
- enterprise: not documented on this page. The `LiteLLM_AuditLog` table is described as "**Off by default**" — not as enterprise-only.
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- "You can disable spend_logs and error_logs by setting `disable_spend_logs` and `disable_error_logs` to `True` on the `general_settings` section of your proxy_config.yaml file."
- "When disabling spend logs (`disable_spend_logs: True`): You **will not** be able to view Usage on the LiteLLM UI; You **will** continue seeing cost metrics on s3, Prometheus, Langfuse (any other Logging integration you are using)"
- "When disabling error logs (`disable_error_logs: True`): You **will not** be able to view Errors on the LiteLLM UI; You **will** continue seeing error logs in your application logs and any other logging integrations you are using"
- "If you need to migrate Databases the following Tables should be copied to ensure continuation of services and no downtime"
- `disable_error_logs` comment: "Only disable writing error logs to DB, regular spend logs will still be written unless `disable_spend_logs: True`" — these flags are independent
- DB Schema link: https://github.com/BerriAI/litellm/blob/main/schema.prisma (Prisma schema)

## Related links
- https://github.com/BerriAI/litellm/blob/main/schema.prisma — full DB schema (Prisma)
- /docs/proxy/db_deadlocks (prev page)
- /docs/proxy/db_read_replica (next page)
- /docs/proxy/architecture
- /docs/proxy/multi_tenant_architecture
- /docs/proxy/control_plane_and_data_plane
- /docs/proxy/high_availability_control_plane
- /docs/proxy/spend_logs_deletion
- /docs/proxy/user_management_heirarchy
- /docs/router_architecture

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- CRITICAL PAGE for the in-memory deployment. The workestrate runs LiteLLM WITHOUT a Postgres database. This page documents that the DB stores: Virtual Keys, Organizations, Teams, Users, Budgets, Per-request Usage Tracking, Model Management (when STORE_MODEL_IN_DB=True), and SpendLogs. ALL of these are UNAVAILABLE without a DB. The key actionable setting is `general_settings.disable_spend_logs: True` — this prevents the proxy from attempting to write spend logs to a non-existent DB. `general_settings.disable_error_logs: True` similarly prevents error log DB writes. These should be set in the workestrate config.yaml to avoid DB connection errors. Cost metrics can still be emitted to Prometheus/external loggers via callbacks (which work without DB). The `LiteLLM_AuditLog` table is "Off by default" so no action needed. `STORE_MODEL_IN_DB` should NOT be set (leave False) — models are defined in config.yaml `model_list` statically.

## Confidence / uncertainty notes
- high confidence on DB requirement and disable_spend_logs/disable_error_logs config (verbatim YAML + quotes). This is the authoritative page for what requires a database. The `disable_spend_logs` setting is the most important finding for the in-memory deployment — it should be set to `True` to avoid DB write errors (high confidence, verbatim).
