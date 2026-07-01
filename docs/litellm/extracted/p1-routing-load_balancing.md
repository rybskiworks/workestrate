---
source_url: https://docs.litellm.ai/docs/routing-load-balancing
canonical_url: https://docs.litellm.ai/docs/routing-load-balancing
title: "Routing & Load Balancing"
sidebar_section_path: routing (top-level)
fetched_http_status: 200
priority_tier: P1
extraction_confidence: medium
feature_area: routing
---
# Routing & Load Balancing

## Headings
- This page is an index/landing page. It contains no h2/h3 of its own — only a grid of card-style links to child pages:
  - Router - Load Balancing
  - [BETA] Adaptive Router
  - [BETA] Request Prioritization
  - Auto Routing
  - Proxy - Load Balancing
  - UI - Router Settings for Keys and Teams
  - Budget Routing
  - Fallbacks
  - [New] Fallback Management Endpoints
  - Tag Based Routing
  - Timeouts
  - Provider specific Wildcard routing
  - Health Check Driven Routing

## Exact config keys found (full path, verbatim spelling)
- none on this index page (YAML config lives on child pages). The "Fallbacks" card mentions conceptual key `num_retries` in its description: "If a call fails after num_retries, fallback to another model group" — but no YAML/dotted-path shown on this page.

## Exact YAML examples (verbatim — preserve indentation)
None on this index page.
(source: https://docs.litellm.ai/docs/routing-load-balancing)

## Exact environment variables
- none documented on this page

## Exact endpoint paths / API routes (runtime + management)
- none documented on this page

## Exact CLI commands
- none documented on this page

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- none documented on this page itself (index page only)

## Related links
- /docs/routing — Router - Load Balancing
- /docs/adaptive_router — [BETA] Adaptive Router
- /docs/scheduler — [BETA] Request Prioritization
- /docs/proxy/auto_routing — Auto Routing
- /docs/proxy/load_balancing — Proxy - Load Balancing
- /docs/proxy/keys_teams_router_settings — UI - Router Settings for Keys and Teams
- /docs/proxy/provider_budget_routing — Budget Routing
- /docs/proxy/reliability — Fallbacks
- /docs/proxy/fallback_management — [New] Fallback Management Endpoints
- /docs/proxy/tag_routing — Tag Based Routing
- /docs/proxy/timeout — Timeouts
- /docs/wildcard_routing — Provider specific Wildcard routing
- /docs/proxy/health_check_routing — Health Check Driven Routing

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- This is the routing landing page. The actual `router_settings.fallbacks`, `num_retries`, `timeout`, `stream_timeout`, `allowed_fails`, `cooldown_time`, `retry_policy` config lives on child pages (especially /docs/proxy/reliability and /docs/routing). For the in-memory workestrator deployment, static router_settings in config.yaml is the relevant path. The dynamic fallback management endpoints (child page) require a DB and are unavailable.

## Confidence / uncertainty notes
- medium confidence — page is an index only; no verbatim config extracted. Child pages (reliability, timeout) were DEFERRED due to budget but their config keys are expected to be covered by the P0 config_settings page (handled by another lead).
