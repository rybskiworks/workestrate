---
source_url: https://docs.litellm.ai/docs/proxy/model_access
canonical_url: https://docs.litellm.ai/docs/proxy/model_access
title: "Restrict Model Access"
sidebar_section_path: proxy > authentication
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
feature_area: auth_access
---
# Restrict Model Access

## Headings
- Restrict models by Virtual Key
  - API Reference
- Restrict models by `team_id`
  - API Reference
- View Available Fallback Models
  - Basic Usage
  - Get Fallback Models with Metadata
  - Get Specific Fallback Types
  - Example Response
  - Use Cases
  - API Parameters
- Advanced: Model Access Groups
- Role Based Access Control (RBAC)

## Exact config keys found (full path, verbatim spelling)
- `models` (request param on `/key/generate` body) — list of allowed models for the key
- `team_alias`, `models` (request params on `/team/new` body) — model restriction at team level
- `fallback_type` (query param on `/v1/models`) — filter fallbacks by type: `general`, `context_window`, `content_policy`
- `include_metadata` (query param on `/v1/models`) — boolean; include additional model metadata including fallbacks

## Exact YAML examples (verbatim — preserve indentation)
None on this page — page contains only cURL/JSON examples, no YAML config blocks.
(source: https://docs.litellm.ai/docs/proxy/model_access)

## Exact environment variables
- none documented on this page

## Exact endpoint paths / API routes (runtime + management)
- `POST /key/generate` — Create a virtual key restricted to specific `models`
- `POST /team/new` — Create a team with `models` restriction. Returns `team_id`
- `POST /key/generate` (with `team_id`) — Create a key bound to a team (inherits that team's model restrictions)
- `POST /chat/completions` — Standard chat completions; used in failure test
- `GET /v1/models` — List available models
- `GET /v1/models?include_metadata=true` — List available models with fallback metadata
- `GET /v1/models?include_metadata=true&fallback_type=general` — Filter fallbacks to `general` type
- `GET /v1/models?include_metadata=true&fallback_type=context_window` — Filter fallbacks to `context_window` type
- `GET /v1/models?include_metadata=true&fallback_type=content_policy` — Filter fallbacks to `content_policy` type

## Exact CLI commands
- none documented on this page

## Requirements
- database: not explicitly stated on this page — page uses `/key/generate`, `/team/new`, and team/key model restrictions without stating that a database is required. However, the related Virtual Keys page requires Postgres for those endpoints to function. This page assumes that prerequisite.
- redis: not documented on this page
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- "This key can only make requests to `models` that are `gpt-3.5-turbo` or `gpt-4`"
- "Expect this to fail since gpt-4o is not in the `models` for the key generated"
- Example error response: `"Invalid model for team litellm-dev: BEDROCK_GROUP.  Valid models for team are: ['azure-gpt-3.5']"`
- "The `include_metadata` parameter serves as an extension point for exposing additional model metadata in the future."

## Related links
- /docs/proxy/model_access_guide (How Model Access Works — Previous)
- /docs/proxy/model_access_groups (Next — Model Access Groups)
- /docs/proxy/access_groups (Access Groups)
- /docs/proxy/team_model_add (Allow Teams to Add Models)
- /docs/proxy/credential_routing (Per-Team/Project Credential Routing)
- /docs/proxy/jwt_auth_arch (RBAC)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- Model access restriction by virtual key (`models` param on `/key/generate`) and by team (`models` on `/team/new`) both REQUIRE a database (via virtual keys/teams). The workestrator runs in-memory (no Postgres), so per-key and per-team model restrictions are UNAVAILABLE. Model access in the in-memory deployment is controlled entirely by which `model_name` entries exist in `model_list` in config.yaml — any caller with the `master_key` can access all configured models. `GET /v1/models` (listing) works without a DB. The `include_metadata` and `fallback_type` query params on `/v1/models` work with static fallback config.

## Confidence / uncertainty notes
- high confidence on endpoints and request params. DB requirement is inferred from the virtual_keys prerequisite (not restated on this page) — marked as inferred. `GET /v1/models` likely works without DB (inferred).
