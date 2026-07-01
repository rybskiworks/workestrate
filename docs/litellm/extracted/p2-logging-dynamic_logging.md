---
source_url: https://docs.litellm.ai/docs/proxy/dynamic_logging
canonical_url: https://docs.litellm.ai/docs/proxy/dynamic_logging
title: "Dynamic Callback Management"
sidebar_section_path: proxy > logging (Logging, Alerting, Metrics)
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: logging
---
# Dynamic Callback Management

## Headings
- Getting Started: List and Disable Callbacks
- 1. List Active Callbacks
- 2. Disable Callbacks
  - Disable a Single Callback
  - Disable Multiple Callbacks
- Header Format and Case Sensitivity
  - Expected Header Format
  - Case Sensitivity
- Disabling Dynamic Callback Management (Enterprise)
  - Use Case
  - How to Disable
  - Effect
  - Example: Compliance Logging Setup

## Exact config keys found (full path, verbatim spelling)
- `litellm_settings.allow_dynamic_callback_disabling` (litellm_settings) — boolean, defaults to `true`
- `litellm_settings.callbacks` (litellm_settings)
- `model_list[].litellm_params.model` (model_list)
- `model_list[].litellm_params.api_key` (model_list)

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Disabling Dynamic Callback Management (Enterprise) - How to Disable
litellm_settings:
  allow_dynamic_callback_disabling: false
```
(source: https://docs.litellm.ai/docs/proxy/dynamic_logging)

```yaml
# Example: Compliance Logging Setup
model_list:
  - model_name: gpt-4
    litellm_params:
      model: openai/gpt-4
      api_key: os.environ/OPENAI_API_KEY
litellm_settings:
  callbacks: ["langfuse", "datadog", "s3"]
  # Disable dynamic callback disabling for compliance
  allow_dynamic_callback_disabling: false
```
(source: https://docs.litellm.ai/docs/proxy/dynamic_logging)

## Exact environment variables
- none documented on this page

## Exact endpoint paths / API routes (runtime + management)
- `GET /callbacks/list` — List all currently enabled callbacks on the proxy. Auth via `x-litellm-api-key` header. Returns `{success: [...], failure: [...], success_and_failure: [...]}`
- `POST /chat/completions` — OpenAI-compatible chat completions (used with `x-litellm-disable-callbacks` header)
- `POST /v1/chat/completions` — Same; supports `extra_headers` for `x-litellm-disable-callbacks`

## Exact CLI commands
- none documented on this page

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: required for disabling dynamic callback management — verbatim: "✨ This is an enterprise feature. Get started with LiteLLM Enterprise" (top banner). The `allow_dynamic_callback_disabling: false` config to enforce guaranteed logging is Enterprise.
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- "Callback name checks are case insensitive"
- "When specifying multiple callbacks, use comma-separated values without spaces around the commas."
- "Default Behavior: Dynamic callback disabling is enabled by default (`allow_dynamic_callback_disabling: true`). You must explicitly set it to `false` to enforce guaranteed logging."
- When `allow_dynamic_callback_disabling: false`: "The `x-litellm-disable-callbacks` header will be ignored" and "All configured callbacks will always execute for every request"

## Related links
- /docs/enterprise
- /docs/traffic_mirroring (Previous)
- /docs/proxy/logging (Next)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- Dynamic callback management (`GET /callbacks/list`, `x-litellm-disable-callbacks` header) works WITHOUT a database — it operates on in-memory callback state. The workestrator can use per-request callback disabling via the `x-litellm-disable-callbacks` header in-memory. The `allow_dynamic_callback_disabling` config key works without DB. However, setting it to `false` (compliance mode) is an Enterprise feature. The `GET /callbacks/list` endpoint works without DB (lists configured callbacks).

## Confidence / uncertainty notes
- high confidence on config keys and endpoints (verbatim). Enterprise requirement for the `false` setting is verbatim. The dynamic disabling feature itself (default `true`) works without DB (inferred high confidence — it's an in-memory operation).
