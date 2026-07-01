---
source_url: https://docs.litellm.ai/docs/proxy/timeout
canonical_url: https://docs.litellm.ai/docs/proxy/timeout
raw_source_url: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/timeout.md
title: Timeouts
sidebar_section_path: proxy > routing
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
feature_area: routing
---
# Timeouts

## Headings
- Timeouts (overview)
- Global Timeouts
- Custom Timeouts & Stream Timeouts (Per Model)
- Setting Dynamic Timeouts - Per Request
- Testing timeout handling

## Exact config keys found (full path, verbatim spelling)
- `router_settings.timeout` (router) — global timeout for the entire call; verbatim: "The timeout set in router is for the entire length of the call, and is passed down to the completion() call level as well."
- `litellm_params.timeout` (per-model, model_list) — verbatim: "maximum time for the *complete response*. Use this to cap long-running completions."
- `litellm_params.stream_timeout` (per-model, model_list) — verbatim: "maximum time to wait for the *first chunk* (i.e., first token) in a streaming response. Use this to abort 'hanging' providers (e.g., Bedrock slow start) and retry another model."
- `litellm_params.max_retries` (per-model, model_list) — appears in per-model YAML example (e.g. `max_retries: 5`); not described in prose on this page.
- request-body `timeout` (per-request) — verbatim: "LiteLLM supports setting a `timeout` per request"; passed via `extra_body={"timeout": 1}` (OpenAI client) or top-level `"timeout": 1` (curl).
- request-body `mock_timeout` (per-request, testing) — verbatim: "set `mock_timeout=True` for testing"; "currently only supported on `/chat/completions` and `/completions` endpoints."

## Keys NOT documented on this page (documented elsewhere — config_settings)
The following router_settings keys are present in `schemas/config-yaml.option-index.json` but are NOT described on the /docs/proxy/timeout page:
- `router_settings.ttft_timeout` — documented in config_settings Reference table; not on this page.
- `router_settings.stream_idle_timeout` — documented in config_settings Reference table; not on this page.
- `litellm_settings.request_timeout` — documented on /docs/proxy/reliability (Advanced section) and config_settings; not on this page.

## Exact YAML examples (verbatim — preserve indentation)
Global timeout (verbatim):
```yaml
router_settings:
    timeout: 30 # sets a 30s timeout for the entire call
```

Per-model timeout + stream_timeout (verbatim):
```yaml
model_list:
  - model_name: gpt-3.5-turbo
    litellm_params:
      model: azure/gpt-turbo-small-eu
      api_base: https://my-endpoint-europe-berri-992.openai.azure.com/
      api_key: <your-key>
      timeout: 0.1                      # timeout in (seconds)
      stream_timeout: 0.01              # timeout for stream requests (seconds)
      max_retries: 5
```

## Exact environment variables
- none documented on this page

## Exact endpoint paths / API routes (runtime + management)
- `POST /chat/completions` — supports per-request `timeout` and `mock_timeout` body params
- `POST /completions` — supports `mock_timeout` (per page: "currently only supported on `/chat/completions` and `/completions` endpoints")

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- none documented on this page

## Caveats / pitfalls
- `stream_timeout` semantics (verbatim): "maximum time to wait for the *first chunk* (i.e., first token) in a streaming response" — this is a first-token/TTFT-style timeout, NOT an idle-between-chunks timeout. (The separate `stream_idle_timeout` key, documented in config_settings but not on this page, governs idle-between-chunks behavior.)
- `mock_timeout` is restricted to `/chat/completions` and `/completions` only (verbatim: "currently only supported on `/chat/completions` and `/completions` endpoints. Please let us know if you need this for other endpoints.")
- Per-request `timeout` overrides the router/per-model timeout for that single call.

## Related links
- /docs/proxy/reliability (Fallbacks + Retries + Timeouts + Cooldowns)
- /docs/proxy/load_balancing (Quick Start)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- The real config sets `router_settings.timeout: 300` and `router_settings.stream_timeout: 300` (both 5 min), overriding the documented default of "10 minutes" (from config_settings). Per-model `litellm_params.timeout`/`stream_timeout` are NOT set — the router-level values apply globally. `ttft_timeout` and `stream_idle_timeout` are unset. The per-request `timeout` body param and `mock_timeout` test param are available at runtime without DB/Redis.

## Confidence / uncertainty notes
- high confidence on `timeout`, `stream_timeout`, per-request `timeout`, `mock_timeout` (verbatim prose + YAML). The `stream_timeout` = "first chunk" semantics is a verbatim quote and resolves a prior ambiguity: `stream_timeout` is a TTFT-style timeout, distinct from `stream_idle_timeout` (idle-between-chunks, documented in config_settings only). `ttft_timeout` and `stream_idle_timeout` are NOT on this page — any behavioral detail for them must cite config_settings, not this page.
