---
source_url: https://docs.litellm.ai/docs/skills_gateway
canonical_url: https://docs.litellm.ai/docs/skills_gateway
raw_source_url: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/skills_gateway.md
title: "Skills Gateway"
sidebar_section_path: gateway_mcp_skills_agent
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: gateway
---
# Skills Gateway

## Headings
- Skills Gateway (intro)
- How it works (mermaid diagram)
- Quick start
  - 1. Register a skill
  - 2. Publish to hub
  - 3. Browse the hub
  - 4. Install in Claude Code
- Skill fields
- API reference

## Verbatim intro
"LiteLLM acts as a **Skills Registry** — a central place to register, manage, and discover Claude Code skills across your organization. Teams can publish skills once and have agents and developers find them through a single hub."

## Exact config keys found (full path, verbatim spelling)
- NOTE: No `skills:`, `litellm_settings.skills`, or any config.yaml block is documented on this page. The word `config.yaml` does NOT appear on this page. Skills are registered at runtime via `POST /claude-code/plugins` (REST), not via config.
- Skill registration request body fields (JSON, verbatim from "Skill fields" table):
  - `name` — Unique skill identifier (used in `/plugin marketplace add`)
  - `source` — Git source object: `source` (`github` | `url` | `git-subdir`), `url`, `path`
  - `description` — Short description shown in the hub
  - `domain` — Category for grouping (e.g. `Engineering`, `Productivity`)
  - `namespace` — Subcategory within a domain (e.g. `quality`, `meetings`)
  - `keywords` — Tags for search and filtering
  - `version` — Semver string

## Exact YAML/JSON examples (verbatim — preserve indentation)
```json
// ~/.claude/settings.json — point Claude Code at proxy marketplace
{
  "extraKnownMarketplaces": {
    "my-org": {
      "source": "url",
      "url": "https://your-proxy/claude-code/marketplace.json"
    }
  }
}
```
(source: https://docs.litellm.ai/docs/skills_gateway)

```bash
# Register a skill (verbatim curl)
curl -X POST https://your-proxy/claude-code/plugins \
  -H "Authorization: Bearer $LITELLM_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "grill-me",
    "source": {
      "source": "git-subdir",
      "url": "https://github.com/mattpocock/skills",
      "path": "grill-me"
    },
    "description": "Interview skill for relentless questioning",
    "domain": "Productivity",
    "namespace": "interviews"
  }'
```
(source: https://docs.litellm.ai/docs/skills_gateway)

```bash
# Publish to hub (verbatim curl)
curl -X POST https://your-proxy/claude-code/plugins/grill-me/enable \
  -H "Authorization: Bearer $LITELLM_KEY"
```
(source: https://docs.litellm.ai/docs/skills_gateway)

## Exact environment variables
- `LITELLM_KEY` — LiteLLM API key used in `Authorization: Bearer $LITELLM_KEY` header for authenticated endpoints

## Exact endpoint paths / API routes (runtime + management)
Verbatim from the "API reference" table:
- `POST /claude-code/plugins` — Auth: Required — Register a skill
- `GET /claude-code/plugins` — Auth: Required — List all skills (admin)
- `POST /claude-code/plugins/{name}/enable` — Auth: Required — Publish a skill
- `POST /claude-code/plugins/{name}/disable` — Auth: Required — Unpublish a skill
- `GET /public/skill_hub` — Auth: None — List public skills
- `GET /claude-code/marketplace.json` — Auth: None — Claude Code marketplace manifest

Additional surfaces mentioned in prose:
- Admin UI: AI Hub → Skill Hub tab (select skills to make public)
- Public page: `/ui/model_hub` → Skill Hub tab (no login required)
- Claude Code install command: `/plugin marketplace add <name>`

## Headers
- `Authorization: Bearer $LITELLM_KEY` (required for `/claude-code/plugins*` endpoints; NOT required for `/public/skill_hub` or `/claude-code/marketplace.json`)
- `Content-Type: application/json`

## Requirements
- database: NOT MENTIONED on page. The strings "database", "DB", "Postgres", "PostgreSQL", "SQLite", "MySQL" do NOT appear anywhere in docs/skills_gateway.md. No persistence model, storage backend, or restart-survival behavior is documented on this page.
- redis: NOT MENTIONED on page
- enterprise: NOT MENTIONED on page
- admin_ui: referenced as the publish surface ("AI Hub → Skill Hub") but no Admin UI setup requirement documented

## Deprecations
- none documented on this page

## Caveats / pitfalls
- "Skills nested in subdirectories (e.g. `github.com/org/repo/tree/main/skill-name`) are supported — LiteLLM parses the URL automatically in the UI."
- The `source` field describes the upstream git source (GitHub URL, git-subdir path); no storage backend for the registry itself is documented.
- This page is SEPARATE from `/docs/skills` (the Anthropic `/v1/skills` API). The `/v1/skills` path does NOT appear on this page. Model-based routing, cost tracking, budgets, permissions, and tracing are NOT mentioned on this page.

## Related links
- Claude Code marketplace: `~/.claude/settings.json` `extraKnownMarketplaces`
- Sibling page `/docs/skills` — Anthropic Skills API (`/v1/skills` endpoints)

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- The Skills Gateway central registry endpoints are now CONFIRMED verbatim upstream (POST /claude-code/plugins, GET /claude-code/plugins, POST /claude-code/plugins/{name}/enable|disable, GET /public/skill_hub, GET /claude-code/marketplace.json). However, the DB/persistence requirement is NOT documented on this page — upstream is silent on whether the registry requires a database. The prior project-context claim that these "require DB for persistence" remains an INFERENCE (runtime-registered skills are plausibly lost on restart without a DB, but this is not stated upstream). Model-based routing and `/v1/skills` are on the separate `/docs/skills` page, not here.

## Confidence / uncertainty notes
- high confidence on the six endpoints, auth model, and skill fields (verbatim from the API reference table and curl examples). DB/persistence requirement is a NEGATIVE FINDING: "NOT MENTIONED on page" — upstream does not document it, so any DB claim must remain `(inferred)` and cite this negative finding.
