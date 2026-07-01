# LiteLLM Endpoint Index

> Synthesized from on-disk sources: `docs/litellm/crawl/openapi_route_inventory.json`
> (669 routes, 92 tags, 125 inference / 544 management) and the extracted doc
> cards under `docs/litellm/extracted/`. No web fetches were performed.

This document is the human-facing guide to LiteLLM's endpoint surface. Machine-
readable indexes live in `docs/litellm/schemas/`:

- [`endpoints.index.json`](../schemas/endpoints.index.json) — inference endpoints with purpose, compat, auth, streaming, pass-through behavior.
- [`openapi-management-endpoints.index.json`](../schemas/openapi-management-endpoints.index.json) — management endpoints organized by family with `requires_db` flags.
- [`openapi_route_inventory.json`](../crawl/openapi_route_inventory.json) — the full verbatim 669-route OpenAPI inventory (source of truth).

## Inference Endpoints

These are the runtime/data-plane endpoints clients call to perform LLM work.
Auth is via `Authorization: Bearer <master_key>` (or a virtual key when a DB is
configured). The workestrator uses only the master key (in-memory, no DB).

| Method | Path | Purpose | Compat | Auth |
|--------|------|---------|--------|------|
| `POST` | `/v1/chat/completions` | Primary chat completions endpoint. | OpenAI-compatible | yes |
| `POST` | `/v1/completions` | Legacy text completions endpoint (OpenAI /v1/completions). | OpenAI-compatible | yes |
| `POST` | `/v1/embeddings` | Text embeddings endpoint (OpenAI /v1/embeddings). | OpenAI-compatible | yes |
| `POST` | `/v1/moderations` | Content moderation endpoint (OpenAI /v1/moderations). | OpenAI-compatible | yes |
| `POST` | `/v1/images/generations` | Image generation endpoint (OpenAI /v1/images/generations). | OpenAI-compatible | yes |
| `POST` | `/v1/audio/transcriptions` | Audio transcription endpoint (OpenAI Whisper /v1/audio/transcriptions). | OpenAI-compatible | yes |
| `POST` | `/v1/audio/speech` | Text-to-speech endpoint (OpenAI /v1/audio/speech). | OpenAI-compatible | yes |
| `POST` | `/v1/responses` | OpenAI Responses API endpoint. | OpenAI-compatible | yes |
| `POST` | `/v1/messages` | Anthropic Messages API passthrough (beta). | Anthropic-compatible | yes |
| `POST` | `/v1/batches` | Batch API — create batch job (OpenAI /v1/batches). | OpenAI-compatible | yes |
| `POST` | `/v1/files` | File upload/management endpoint (OpenAI /v1/files). | OpenAI-compatible | yes |
| `GET` | `/v1/models` | List available models (OpenAI /v1/models). | OpenAI-compatible | yes |
| `GET` | `/mcp-rest/tools/list` | Direct REST API to list MCP tools without an LLM call. | LiteLLM-specific | yes |
| `POST` | `/mcp-rest/tools/call` | Direct REST API to invoke an MCP tool without an LLM call. | LiteLLM-specific | yes |
| `GET, POST, DELETE, GET /{skill_id}` | `/v1/skills` | Anthropic Skills API. | Anthropic-compatible | yes |
| `POST` | `/a2a/{agent_id}` | A2A (Agent-to-Agent) protocol gateway. | LiteLLM-specific | yes |
| `GET` | `/health` | Model connectivity health check. | LiteLLM-specific | yes |
| `GET` | `/health/liveliness` | Liveness probe. | LiteLLM-specific | yes |
| `GET` | `/health/readiness` | Readiness probe. | LiteLLM-specific | yes |
| `GET` | `/metrics` | Prometheus metrics scrape endpoint. | LiteLLM-specific | no |

### Notes on inference endpoints

- **`POST /v1/chat/completions`** is the primary endpoint. The `model` field
  maps to `model_name` in `config.yaml` (e.g. `coding`, `coding.pro`), **not**
  the provider-specific `litellm_params.model`. LiteLLM routes to the primary
  model and falls back per `router_settings.fallbacks`.
- **`POST /v1/messages`** is the Anthropic Messages API passthrough (beta).
  Providers configured with the `anthropic/` prefix (Kimi, MiniMax) speak this
  protocol natively.
- **`GET /v1/models`** lists configured `model_name` entries. Supports
  `include_metadata` and `fallback_type` query params. Works without a DB.
- **`GET /metrics`** is a Prometheus scrape endpoint — it is **not** in the
  OpenAPI inventory (it is a Prometheus exposition route, not a FastAPI route).
  Requires `litellm_settings.callbacks: ["prometheus"]`. Works without DB/Redis
  for basic metrics. Budget/rate-limit metrics require DB-backed virtual keys.
- **`/health/liveliness`** — note the OpenAPI contains both `/health/liveness`
  and `/health/liveliness` (misspelled). The workestrator infra uses the
  misspelled `/health/liveliness` and `/health/readiness`.
- **MCP REST** (`/mcp-rest/tools/list`, `/mcp-rest/tools/call`) operate on
  config-defined `mcp_servers` and work without a DB.
- **A2A** (`/a2a/{agent_id}`) routes to config-defined `agents[]` and works
  without a DB for config-file-registered agents.

## Management Endpoints (by family)

Management endpoints administer virtual keys, teams, users, budgets, spend,
models, guardrails, cache, and config. **The workestrator runs LiteLLM
in-memory with no Postgres**, so any endpoint flagged `requires_db` is
**unavailable** in this deployment.

| Family | Routes | requires_db |
|--------|--------|-------------|
| A2A Agents management | 9 | yes |
| Audit Logging | 2 | yes |
| Budget & Spend Tracking | 6 | yes |
| Customer Management | 8 | yes |
| Internal User management | 7 | yes |
| Logging Callbacks | 2 | no |
| Policies | 27 | yes |
| Router Settings | 5 | partial |
| SSO Settings | 7 | yes |
| Settings | 2 | no |
| UI Settings | 6 | yes |
| access_groups | 5 | yes |
| adaptive_router | 1 | no |
| budget management | 6 | yes |
| caching | 7 | partial |
| claude_code_marketplace | 7 | yes |
| compliance | 2 | no |
| config management | 10 | no |
| email management | 3 | yes |
| guardrails | 22 | partial |
| health | 15 | no |
| jwt_mappings | 5 | yes |
| key management | 14 | yes |
| langfuse_passthrough | 5 | no |
| llm utils | 3 | no |
| model management | 10 | partial |
| organization management | 10 | yes |
| project management | 5 | yes |
| prompts | 9 | yes |
| public | 9 | no |
| scim | 18 | yes |
| search_tools | 7 | yes |
| tag management | 12 | yes |
| team management | 21 | yes |
| tools | 7 | yes |
| tools management | 5 | yes |
| usage_ai | 1 | no |
| vector_store_management | 6 | partial |
| workflow management | 8 | yes |

> `requires_db` is **inferred from path semantics** (marked "(inferred from
> path)" in the JSON), not verbatim from upstream docs. `yes` = all routes in
> the family need a DB; `partial` = some do; `no` = none do (static config /
> read-only / passthrough).

### DB-required families (unavailable in workestrator)

These mutate or query DB-backed entities and will fail without a Postgres
connection:

- **key management** — `/key/generate`, `/key/update`, `/key/delete`,
  `/key/info`, `/key/list`, `/key/block`, `/key/unblock`, `/key/regenerate`,
  `/key/{key}/regenerate`, `/key/{key}/reset_spend`, etc. Virtual keys
  require `general_settings.database_url` / `DATABASE_URL`.
- **team management** — `/team/new`, `/team/update`, `/team/member_add`,
  `/team/info`, `/team/list`, `/team/delete`, etc.
- **user management** — `/user/new`, `/user/info`, `/user/update`, `/user/list`,
  `/user/delete`, etc.
- **budget management** — `/budget/new`, `/budget/update`, `/budget/info`,
  `/budget/list`, `/budget/delete`. Budget tiers are Enterprise + DB.
- **spend tracking** — `/spend/logs`, `/spend/logs/v2`, `/spend/tags`,
  `/spend/calculate`. Spend tracking requires virtual keys + DB.
- **customer / organization / tag / scim / prompts / policies / guardrails
  (CRUD) / agents registry / access_groups / audit / jwt_mappings / workflows
  / projects / vector_store / claude-code plugins** — all DB-backed.

### DB-free families (available in workestrator)

- **health** — `/health`, `/health/liveliness`, `/health/readiness`,
  `/health/services`, `/health/backlog`, `/health/test_connection`, etc. Work
  in-memory.
- **caching (read)** — `/cache/ping`, `/cache/redis/info` (needs Redis, not
  DB), `/cache/delete`, `/cache/flushall`.
- **callbacks** — `/callbacks/list`, `/callbacks/configs` (read-only).
- **config (pass-through / cost)** — `/config/pass_through_endpoint*`,
  `/config/cost_discount_config`, `/config/cost_margin_config`.
- **public** — `/public/model_hub`, `/public/providers`, `/public/endpoints`,
  `/public/litellm_model_cost_map`, etc. (read-only).
- **llm utils** — `/utils/token_counter`, `/utils/supported_openai_params`,
  `/utils/transform_request`.
- **router settings** — `/router/settings`, `/router/fields` (read-only).
- **compliance** — `/compliance/eu-ai-act`, `/compliance/gdpr` (stateless
  checks).
- **langfuse passthrough** — `/langfuse/{endpoint}` (proxies to Langfuse).

## Gateway Endpoints (MCP / Skills / A2A)

These are the agent-gateway endpoints. All three can be configured statically in
`config.yaml` and work **without a database**.

### MCP (Model Context Protocol)

- `GET /mcp-rest/tools/list` — list MCP tools (no LLM call).
- `POST /mcp-rest/tools/call` — invoke an MCP tool (no LLM call).
- `POST /v1/chat/completions` and `POST /v1/responses` — consume MCP tools via
  `server_url: "litellm_proxy"`.
- MCP servers are defined in the `mcp_servers:` config block (HTTP, SSE, stdio
  transports). `litellm_settings.mcp_aliases` maps alias → server name.
- DB storage (`store_model_in_db: true`) is **optional** — only needed for
  runtime-added MCP servers via the management API (unavailable without DB).
- Source: https://docs.litellm.ai/docs/mcp

### Skills (Anthropic Skills API)

- `GET /v1/skills` — list skills.
- `POST /v1/skills` — create a skill (ZIP or SKILL.md multipart).
- `GET /v1/skills/{skill_id}` — get skill details.
- `DELETE /v1/skills/{skill_id}` — delete a skill.
- Requires `?beta=true` and headers `anthropic-beta: skills-2025-10-02`,
  `anthropic-version: 2023-06-01`. Optional `model` param selects configured
  Anthropic credentials (multi-account routing).
- DB requirement is **not documented** on the source page (storage backend
  unconfirmed).
- Source: https://docs.litellm.ai/docs/skills

### A2A (Agent-to-Agent)

- `POST /a2a/{agent_id}` — JSON-RPC 2.0 gateway (message/send,
  message/stream, tasks/*, agent/getAuthenticatedExtendedCard).
- `POST /a2a/{agent_id}/message/send` — alias for message/send.
- `POST /v1/a2a/{agent_id}/message/send` — alias for message/send.
- `GET /a2a/{agent_id}/.well-known/agent.json` — agent card discovery.
- `GET /a2a/{agent_id}/.well-known/agent-card.json` — agent card discovery.
- Agents are defined in the `agents:` config block (`agent_name`,
  `agent_card_params.url`, `protocolVersion: "0.3"|"1.0"`). Config-file agents
  work without a DB. `POST /v1/agents` (admin registration) likely needs DB for
  persistence (inferred).
- Source: https://docs.litellm.ai/docs/a2a

## Workestrator in Practice

The workestrator runs LiteLLM as an in-memory gateway (no Postgres, no Redis,
no UI). Agent sandboxes (Pi, Odysseus) call LiteLLM, not provider APIs
directly.

### Endpoints the sandboxes actually hit

| Endpoint | Usage |
|----------|-------|
| `POST /v1/chat/completions` | Primary inference. `model` = tier name (`coding`, `coding.fast`, `coding.pro`, `coding.free`, `neural`). Auth: `Authorization: Bearer $LITELLM_MASTER_KEY`. |
| `GET /v1/models` | List available tier names. |
| `GET /health/liveliness` | Liveness probe. |
| `GET /health/readiness` | Readiness probe (DB status N/A). |
| `GET /health` | Model connectivity check. |
| `GET /metrics` | Prometheus scrape (if `callbacks: ["prometheus"]` configured). |

### What is unavailable (in-memory, no DB)

- Virtual keys (`/key/*`), teams (`/team/*`), users (`/user/*`), budgets
  (`/budget/*`), spend tracking (`/spend/*`), customers, organizations.
- Per-key / per-team / per-user budgets and rate limits.
- Per-key model access restrictions (model access is controlled entirely by
  which `model_name` entries exist in `config.yaml`).
- Key rotation, scheduled rotations, spend reports.
- DB-backed guardrail/policy/agent/prompt CRUD (static config still works).

### What works without a DB

- All inference endpoints (`/v1/chat/completions`, `/v1/models`, etc.).
- Static config: `model_list`, `router_settings.fallbacks`, `mcp_servers`,
  `agents`, `guardrails`, `litellm_settings.drop_params`.
- Health endpoints, `/metrics` (Prometheus), alerting webhooks (Slack/Teams).
- `general_settings.master_key` auth (single admin key).
- `general_settings.disable_spend_logs: true` (prevents DB spend-log writes).

### Config reference

See [`infra/litellm/config.yaml`](../../../infra/litellm/config.yaml) and
[`infra/litellm/README.md`](../../../infra/litellm/README.md) for the
workestrator deployment details. The proxy listens on `:4000`. Clients set
`base_url=http://<litellm-host>:4000` and `api_key=$LITELLM_MASTER_KEY`.

## Sources

- OpenAPI inventory (verbatim, 669 routes): `docs/litellm/crawl/openapi_route_inventory.json`
- Supported endpoints index: https://docs.litellm.ai/docs/supported_endpoints
- Client usage (OpenAI SDK / curl): https://docs.litellm.ai/docs/proxy/user_keys
- OpenAPI JSON: https://www.litellm.org/openapi.json
- Virtual keys: https://docs.litellm.ai/docs/proxy/virtual_keys
- MCP: https://docs.litellm.ai/docs/mcp
- Skills: https://docs.litellm.ai/docs/skills
- A2A: https://docs.litellm.ai/docs/a2a
- Prometheus: https://docs.litellm.ai/docs/proxy/prometheus
- Guardrails: https://docs.litellm.ai/docs/proxy/guardrails/quick_start
- Spend tracking: https://docs.litellm.ai/docs/proxy/cost_tracking
