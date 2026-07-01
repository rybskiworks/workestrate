---
source_url: https://docs.litellm.ai/docs/proxy/metrics
canonical_url: https://docs.litellm.ai/docs/proxy/metrics
title: "💸 GET Daily Spend, Usage Metrics"
sidebar_section_path: proxy > logging (Logging, Alerting, Metrics)
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: logging
---
# 💸 GET Daily Spend, Usage Metrics

## Headings
- Request Format
- Response format

## Exact config keys found (full path, verbatim spelling)
- none documented on this page

## Exact YAML examples (verbatim — preserve indentation)
None on this page — only JSON and bash code blocks.
(source: https://docs.litellm.ai/docs/proxy/metrics)

## Exact environment variables
- none documented on this page

## Exact endpoint paths / API routes (runtime + management)
- `GET /daily_metrics` — returns daily spend, spend per model, spend per api key

## Exact CLI commands
- none documented (only curl examples)

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: not documented on this page (no EE callout)
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- none documented

## Related links
- /docs/enterprise

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- This page is thin — documents only `GET /daily_metrics` (daily spend/usage). Since spend tracking requires a database (see p2-logging-cost_tracking.md), `/daily_metrics` likely requires DB (inferred). The richer Prometheus metrics content lives at `/docs/proxy/prometheus` (see p2-logging-prometheus.md) which works WITHOUT a DB using `callbacks: ["prometheus"]`. For the in-memory workestrator, Prometheus metrics (not spend metrics) are the viable observability path.

## Confidence / uncertainty notes
- high confidence on the single endpoint. DB requirement is inferred from spend tracking domain (not stated on this page). This page is very thin — the Prometheus page is the primary metrics reference.
