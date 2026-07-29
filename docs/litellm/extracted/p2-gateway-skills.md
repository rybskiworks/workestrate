---
source_url: https://docs.litellm.ai/docs/skills
canonical_url: https://docs.litellm.ai/docs/skills
title: "/skills - Anthropic Skills API"
sidebar_section_path: gateway_mcp_skills_agent
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: gateway
---
# /skills - Anthropic Skills API

## Headings
- LiteLLM Python SDK Usage
  - Quick Start - Create a Skill
  - List Skills
  - Get Skill Details
  - Delete a Skill
  - Async Usage
- LiteLLM Proxy Usage
  - Authentication
  - Basic Usage
    - Create Skill
    - List Skills
    - Get Skill
    - Delete Skill
  - Model-Based Routing (Multi-Account)
- SKILL.md Format
  - YAML Frontmatter Requirements
  - File Structure
- Response Format
  - Skill Object
  - List Skills Response
- Supported Providers

## Exact config keys found (full path, verbatim spelling)
- `model_list[].model_name` (model_list)
- `model_list[].litellm_params.model` (model_list)
- `model_list[].litellm_params.api_key` (model_list)
- `SKILL.md frontmatter.name` (required, must match directory name; lowercase, numbers, hyphens only)
- `SKILL.md frontmatter.description` (required)
- `SKILL.md frontmatter.license` (optional, e.g. `MIT`, `Apache-2.0`)
- `SKILL.md frontmatter.allowed-tools` (optional list of Claude tool identifiers)
- `SKILL.md frontmatter.metadata` (optional additional custom metadata)
- NOTE: No `litellm_settings.skills` or `skills:` config block documented on this page. Skills are managed via `/v1/skills` REST endpoints.

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# SKILL.md frontmatter example
---
name: test-skill
description: A brief description of what this skill does
license: MIT
allowed-tools:
  - computer_20250124
  - text_editor_20250124
---
```
(source: https://docs.litellm.ai/docs/skills)

```yaml
# Authentication - Option 2 (model-based)
model_list:
  - model_name: claude-sonnet
    litellm_params:
      model: anthropic/claude-3-5-sonnet-20241022
      api_key: os.environ/ANTHROPIC_API_KEY
```
(source: https://docs.litellm.ai/docs/skills)

```yaml
# Model-Based Routing (Multi-Account)
model_list:
  - model_name: claude-team-a
    litellm_params:
      model: anthropic/claude-3-5-sonnet-20241022
      api_key: os.environ/ANTHROPIC_API_KEY_TEAM_A
  - model_name: claude-team-b
    litellm_params:
      model: anthropic/claude-3-5-sonnet-20241022
      api_key: os.environ/ANTHROPIC_API_KEY_TEAM_B
```
(source: https://docs.litellm.ai/docs/skills)

## Exact environment variables
- `ANTHROPIC_API_KEY` — Default Anthropic API key used when no `model` parameter is sent in the request
- `ANTHROPIC_API_KEY_TEAM_A` / `ANTHROPIC_API_KEY_TEAM_B` — Per-account keys for model-based routing

## Exact endpoint paths / API routes (runtime + management)
- `POST /v1/skills?beta=true` — Create a skill (upload ZIP or SKILL.md multipart form)
- `GET /v1/skills?beta=true` — List skills (supports `limit` query param; SDK uses `limit=20`)
- `GET /v1/skills/{skill_id}?beta=true` — Get a single skill's details
- `DELETE /v1/skills/{skill_id}?beta=true` — Delete a skill (only when no versions exist)
- Request headers: `X-Api-Key: sk-1234`, `anthropic-version: 2023-06-01`, `anthropic-beta: skills-2025-10-02`
- Query parameter: `model=<model_name>` — selects which configured model / credentials to use
- NOTE: This page does NOT document `POST /claude-code/plugins` or `GET /public/skill_hub`. Those endpoints likely live in `/docs/skills_gateway` (sibling page).

## Exact CLI commands
- `litellm --config /path/to/config.yaml` — Start the LiteLLM proxy with the provided config

## Requirements
- database: not documented on this page (no mention of Postgres or any DB requirement for the `/v1/skills` endpoint)
- redis: not documented on this page
- enterprise: not documented on this page (Enterprise features advertised in footer card but no EE requirement stated for skills itself)
- admin_ui: not documented on this page

## Deprecations
- none documented on this page

## Caveats / pitfalls
- "LiteLLM follows the [Anthropic Skills API](https://docs.anthropic.com/en/docs/build-with-claude/skills) for creating, managing, and using reusable AI capabilities."
- "The folder name (in ZIP or filename path) **must exactly match** the `name` field in SKILL.md frontmatter"
- "`SKILL.md` must be in the root of the skill directory (not in a subdirectory)"
- "All additional files must be in the same skill directory"
- SKILL.md `name` field rule: "lowercase, numbers, hyphens only. Must match the directory name."
- Delete behavior: "Delete skill (if no versions exist)" — deletion fails when versions exist
- Feature support: Cost Tracking ✅, Logging ✅, Load Balancing ✅, Supported Providers: `anthropic`

## Related links
- https://docs.anthropic.com/en/docs/build-with-claude/skills — Anthropic Skills API spec
- /docs/skills_gateway — "Skills Gateway" sibling page (SEPARATE from /docs/skills)
- /docs/enterprise

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- The `/v1/skills` endpoints are Anthropic-compatible passthrough endpoints. DB requirement is NOT documented on this page — skills may be stored in DB or filesystem (inferred). The workestrate could potentially use `/v1/skills` if skills storage doesn't require DB (unconfirmed). Model-based routing (`model_list` with anthropic models) works without DB. The `anthropic-beta: skills-2025-10-02` header and `?beta=true` query param are required. Management endpoints (`POST /claude-code/plugins`, `GET /public/skill_hub`) are on the sibling `/docs/skills_gateway` page (deferred — not fetched). SKILL.md frontmatter format is relevant for skill authoring.

## Confidence / uncertainty notes
- high confidence on endpoints and SKILL.md format (verbatim). DB requirement is "not documented" — skills storage backend is unconfirmed on this page (may require DB for persistence — inferred medium confidence). The sibling `/docs/skills_gateway` page (deferred) likely documents management endpoints and DB requirements.
