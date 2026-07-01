---
source_url: https://docs.litellm.ai/docs/proxy/fallback_management
canonical_url: https://docs.litellm.ai/docs/proxy/fallback_management
title: "[New] Fallback Management Endpoints"
sidebar_section_path: proxy > routing
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
feature_area: routing
---
# [New] Fallback Management Endpoints

## Headings
- Overview
- Prerequisites
- Endpoints
  - POST /fallback
  - GET /fallback/{model}
  - DELETE /fallback/{model}
  - Test fallback
- Validation
- Error Responses
  - 400 Bad Request
  - 404 Not Found
  - 500 Internal Server Error
- Fallback Types Explained
  - General Fallbacks
  - Context Window Fallbacks
  - Content Policy Fallbacks
- Benefits Over /config/update
- Notes

## Exact config keys found (full path, verbatim spelling)
- `STORE_MODEL_IN_DB` (env prerequisite) — Must be True; required for fallback management endpoints (DB must be enabled)
- `router.max_fallbacks` (router) — maximum number of fallbacks attempted (mentioned in Notes)

## Exact YAML examples (verbatim — preserve indentation)
None on this page — page contains only JSON request/response examples and cURL/Python snippets, no YAML config blocks.
(source: https://docs.litellm.ai/docs/proxy/fallback_management)

## Exact environment variables
- `STORE_MODEL_IN_DB` — Must be `True`; required prerequisite for the fallback management endpoints (DB must be enabled)

## Exact endpoint paths / API routes (runtime + management)
- `POST /fallback` — Create or update fallbacks for a specific model. Body: `{model, fallback_models, fallback_type}`
- `GET /fallback/{model}` — Get fallback configuration for a specific model. Query param: `fallback_type` (default `general`)
- `DELETE /fallback/{model}` — Delete fallback configuration for a specific model. Query param: `fallback_type` (default `general`)
- `POST /chat/completions` — Test endpoint used with `mock_testing_fallbacks: true` to trigger fallback behavior end-to-end
- `/config/update` — referenced as the older/less safe alternative that these new endpoints replace

## Exact CLI commands
- none documented on this page

## Requirements
- database: required — verbatim: "Database storage must be enabled: Set `STORE_MODEL_IN_DB=True` in your environment". Also: "**Database Enabled**: Requires `STORE_MODEL_IN_DB=True` to be set". Also: "Changes take effect immediately and are persisted to the database"
- redis: not documented on this page
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- none documented (page introduces new endpoints; no explicit deprecation of `/config/update` stated, only that new endpoints are "a cleaner and safer way to manage fallbacks compared to using the `/config/update` endpoint")

## Caveats / pitfalls
- "Fallbacks are triggered after the configured number of retries fails"
- Validation rules verbatim: "Model Existence", "Fallback Model Existence", "No Self-Fallback", "No Duplicates", "Database Enabled"
- "Fallbacks are attempted in the order specified in `fallback_models`"
- Fallback types: `general`, `context_window`, `content_policy`

## Related links
- /docs/proxy/reliability (Fallbacks — Previous)
- /docs/proxy/tag_routing (Next)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- These dynamic fallback management endpoints REQUIRE a database (`STORE_MODEL_IN_DB=True`). The workestrator runs LiteLLM in-memory (no Postgres), so these endpoints are UNAVAILABLE. Fallbacks must instead be configured statically via `router_settings.fallbacks` in config.yaml. The `router.max_fallbacks` setting and the static `router_settings.fallbacks` list (documented on /docs/proxy/reliability) are the in-memory-compatible path. `num_retries`, `timeout`, `stream_timeout`, `allowed_fails`, `cooldown_time`, `retry_policy` in router_settings work without a DB.

## Confidence / uncertainty notes
- high confidence on endpoints and DB requirement (verbatim quotes). The `router.max_fallbacks` key is mentioned in Notes but its exact config section (router_settings vs litellm_settings) is not shown on this page — inferred from router context.
