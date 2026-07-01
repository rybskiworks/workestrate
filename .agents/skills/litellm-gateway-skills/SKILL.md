---
name: litellm-gateway-skills
description: |
  DB-GATED-for-persistence operational reference for the LiteLLM Skills
  Gateway: /v1/skills endpoints (anthropic-beta: skills-2025-10-02,
  anthropic provider only), central registry (/claude-code/plugins,
  /public/skill_hub, /claude-code/marketplace.json), and model-based routing.
  Persistence requires DB (defer to Postgres/M4 in workestrator). Load when
  planning Skills gateway enablement or reviewing why runtime-registered
  skills are lost on restart. Distilled from docs/litellm/.
---

# LiteLLM Skills Gateway (DB-GATED for persistence)

> **⚠️ DB-GATED for persistence.** Runtime skill registration (`/v1/skills`
> and the `/claude-code/plugins` registry) needs DB for persistence —
> runtime-registered skills are lost on restart. Model-based routing works
> without DB. Defer Skills gateway persistence to Postgres/M4.

Distilled operational guidance for the LiteLLM Skills Gateway (Anthropic-
compatible `/v1/skills` endpoints + central registry). Full detail lives in:

- `docs/litellm/gateway/README.md` — Skills section, cross-gateway summary.
- `docs/litellm/extracted/p2-gateway-skills.md` — verbatim config, SKILL.md
  format, endpoints, headers.
- `docs/litellm/extracted/p2-gateway-skills_gateway.md` — central registry
  endpoints (verbatim).
- `docs/litellm/schemas/gateway-agent-mcp-skills.index.json` — index.

## Triggers

Load this skill when:

- Configuring model-based routing for Claude skills (multi-account).
- Reviewing `/v1/skills` request headers (`anthropic-beta`, `?beta=true`).
- Planning the central registry (`/claude-code/plugins`, `/public/skill_hub`).
- Diagnosing why runtime-registered skills disappear on restart.
- Authoring a `SKILL.md` for upload.

## Config section

No `skills:` or `litellm_settings.skills` config block is documented on
`/docs/skills`. Skills are managed via `/v1/skills` REST endpoints.
Model-based routing uses standard `model_list` entries with anthropic models.

## SKILL.md frontmatter (verbatim)

```yaml
---
name: test-skill
description: A brief description of what this skill does
license: MIT
allowed-tools:
  - computer_20250124
  - text_editor_20250124
---
```

Rules (verbatim): `name` must be lowercase, numbers, hyphens only, and **must
exactly match** the directory name. `SKILL.md` must be in the root of the
skill directory. All additional files must be in the same skill directory.

## Auth model

Anthropic-style auth: `X-Api-Key` header OR model-based routing via
`model_list` with anthropic models (per-account `ANTHROPIC_API_KEY`). No
mcp-style `auth_type` enum.

## Headers (all `/v1/skills` requests)

- `X-Api-Key: sk-1234`
- `anthropic-version: 2023-06-01`
- `anthropic-beta: skills-2025-10-02`
- Query param `?beta=true` on every request.
- Optional query param `model=<model_name>` for model-based routing.

## Endpoints

`/v1/skills` (Anthropic-compatible):
- `POST /v1/skills?beta=true` — create (upload ZIP or SKILL.md multipart).
- `GET /v1/skills?beta=true` — list (supports `limit`; SDK uses `limit=20`).
- `GET /v1/skills/{skill_id}?beta=true` — get single skill.
- `DELETE /v1/skills/{skill_id}?beta=true` — delete (only when no versions).

Central registry (verbatim from `/docs/skills_gateway`):
- `POST /claude-code/plugins` — register a skill (Auth: Required).
- `GET /claude-code/plugins` — list all skills, admin (Auth: Required).
- `POST /claude-code/plugins/{name}/enable` — publish (Auth: Required).
- `POST /claude-code/plugins/{name}/disable` — unpublish (Auth: Required).
- `GET /public/skill_hub` — list public skills (Auth: None).
- `GET /claude-code/marketplace.json` — Claude Code marketplace manifest
  (Auth: None).

## DB / Enterprise requirements

| Feature | Requirement | In-memory? |
|---------|-------------|------------|
| Model-based routing (`model_list` anthropic) | none | **YES** |
| `/v1/skills` persistence | DB (inferred) | **NO** (lost on restart) |
| `/claude-code/plugins` registry persistence | DB (inferred) | **NO** |
| `/public/skill_hub` + `/claude-code/marketplace.json` | DB (inferred) | **NO** |
| Cost tracking (feature card ✅) | DB (inferred, spend logs) | **NO** |

> Verbatim negative finding: DB/persistence requirement is NOT documented on
> `/docs/skills` NOR on `/docs/skills_gateway` (strings 'database'/'DB'/
> 'Postgres' absent on both). The persistence-needs-DB claim is an INFERENCE,
> grounded in upstream silence on persistence. Supported provider:
> `anthropic` only.

## Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| Runtime-registered skill gone after restart | no DB persistence | Add Postgres (M4) or define skills statically outside LiteLLM. |
| `/v1/skills` 400 / auth error | missing `anthropic-beta: skills-2025-10-02` or `?beta=true` | Send both on every `/v1/skills` request. |
| Skill upload rejected | `name` ≠ directory name, or non-anthropic provider | Match name to dir; use anthropic model. |
| `/claude-code/plugins` 401 | Auth: Required | Send `X-Api-Key`. |
| Cost tracking not populating | needs DB (inferred) | Add Postgres; remove `disable_spend_logs` if spend needed. |
| Non-anthropic model used | Skills gateway is anthropic-only | Use an anthropic `model_list` entry. |

## Related Docs

- `docs/litellm/gateway/README.md`
- `docs/litellm/extracted/p2-gateway-skills.md`
- `docs/litellm/extracted/p2-gateway-skills_gateway.md`
- `docs/litellm/schemas/gateway-agent-mcp-skills.index.json`

> Do not hallucinate endpoints, headers, or DB requirements. The six
> central-registry endpoints are verbatim from `p2-gateway-skills_gateway.md`.
> The `/v1/skills` endpoints/headers are verbatim from
> `p2-gateway-skills.md`. The DB-persistence requirement is an INFERENCE
> (verbatim negative finding: upstream is silent on persistence) — clearly
> marked. This skill is DB-GATED for persistence.
