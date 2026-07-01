# Agent / MCP / Skills Gateway

LiteLLM proxy exposes three gateway types under the "Agent & MCP Gateway" sidebar section. This guide synthesizes the on-disk extracted docs (`docs/litellm/extracted/p2-gateway-*.md`, `p0-config_settings.md`) plus local project context (`docs/integration-plan.md`). Every config key, endpoint path, auth type, and header name below traces to a fetched source file; inferred items are marked.

Source pages:
- MCP: https://docs.litellm.ai/docs/mcp
- Skills: https://docs.litellm.ai/docs/skills
- A2A: https://docs.litellm.ai/docs/a2a
- Config reference: https://docs.litellm.ai/docs/proxy/config_settings
- Skills Gateway: https://docs.litellm.ai/docs/skills_gateway
- MCP REST API: https://docs.litellm.ai/docs/mcp_rest_api

Machine-readable index: [`../schemas/gateway-agent-mcp-skills.index.json`](../schemas/gateway-agent-mcp-skills.index.json)

---

## 1. MCP Gateway

### Concept
The MCP (Model Context Protocol) Gateway lets the LiteLLM proxy expose configured MCP servers and route tool calls to them. Clients invoke MCP tools via `/v1/chat/completions`, `/v1/responses` (with `server_url: "litellm_proxy"`), or the direct `/mcp-rest/tools/*` REST API. (source: https://docs.litellm.ai/docs/mcp)

### Config section
Top-level `mcp_servers:` block in `config.yaml` (sibling to `model_list`). Aliases under `litellm_settings.mcp_aliases`. DB-storage toggles under `general_settings`. **Not** under `router_settings`.

```yaml
# Adding MCP in config.yaml (main mcp_servers block) — verbatim from /docs/mcp
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_key: sk-xxxxxxx

litellm_settings:
  # MCP Aliases - Map aliases to server names for easier tool access
  mcp_aliases:
    "github": "github_mcp_server"
    "zapier": "zapier_mcp_server"
    "deepwiki": "deepwiki_mcp_server"

mcp_servers:
  # HTTP Streamable Server
  deepwiki_mcp:
    url: "https://mcp.deepwiki.com/mcp"

  # SSE Server
  zapier_mcp:
    url: "https://actions.zapier.com/mcp/sk-akxxxxx/sse"

  # Standard Input/Output (stdio) Server - CircleCI Example
  circleci_mcp:
    transport: "stdio"
    command: "npx"
    args: ["-y", "@circleci/mcp-server-circleci"]
    env:
      CIRCLECI_TOKEN: "your-circleci-token"
      CIRCLECI_BASE_URL: "https://circleci.com"

  # Full configuration with all optional fields
  my_http_server:
    url: "https://my-mcp-server.com/mcp"
    transport: "http"
    description: "My custom MCP server"
    auth_type: "api_key"
    auth_value: "abc123"
```
(source: https://docs.litellm.ai/docs/mcp)

### Auth model
Per-server `auth_type` enum (verbatim): `none`, `api_key`, `bearer_token`, `basic`, `authorization`, `token`, `oauth2`, `oauth2_token_exchange`, `aws_sigv4`. Credential via `auth_value`. OAuth2 (v1.77.5+) supports `authorization_url`/`token_url`/`registration_url`/`client_id`/`client_secret`/`scopes` overrides. AWS SigV4 uses `aws_role_name`/`aws_access_key_id`/`aws_secret_access_key`/`aws_region_name`/`aws_service_name` (defaults to `bedrock-agentcore`).

**DEPRECATED:** the legacy broadcast header `x-mcp-auth` is deprecated in favor of server-specific auth headers. Override the client-side auth header name via `general_settings.mcp_client_side_auth_header_name` (default `x-mcp-auth`).

```yaml
# MCP auth examples — verbatim from /docs/mcp
mcp_servers:
  api_key_example:
    url: "https://my-mcp-server.com/mcp"
    auth_type: "api_key"
    auth_value: "abc123"        # headers={"X-API-Key": "abc123"}

  oauth2_example:
    url: "https://my-mcp-server.com/mcp"
    auth_type: "oauth2"
    authorization_url: "https://my-mcp-server.com/oauth/authorize"
    token_url: "https://my-mcp-server.com/oauth/token"
    registration_url: "https://my-mcp-server.com/oauth/register"
    client_id: os.environ/OAUTH_CLIENT_ID
    client_secret: os.environ/OAUTH_CLIENT_SECRET
    scopes: ["tool.read", "tool.write"]

  bearer_example:
    url: "https://my-mcp-server.com/mcp"
    auth_type: "bearer_token"
    auth_value: "abc123"        # headers={"Authorization": "Bearer abc123"}

  basic_example:
    url: "https://my-mcp-server.com/mcp"
    auth_type: "basic"
    auth_value: "dXNlcjpwYXNz"  # headers={"Authorization": "Basic dXNlcjpwYXNz"}

  custom_auth_example:
    url: "https://my-mcp-server.com/mcp"
    auth_type: "authorization"
    auth_value: "Token example123"  # headers={"Authorization": "Token example123"}

  agentcore_mcp:
    url: "https://bedrock-agentcore.us-east-1.amazonaws.com/runtimes/<url-encoded-ARN>/invocations"
    transport: "http"
    auth_type: "aws_sigv4"
    aws_role_name: os.environ/AWS_ROLE_ARN
    aws_access_key_id: os.environ/AWS_ACCESS_KEY_ID
    aws_secret_access_key: os.environ/AWS_SECRET_ACCESS_KEY
    aws_region_name: us-east-1
    aws_service_name: bedrock-agentcore

  github_mcp:
    url: "https://api.githubcopilot.com/mcp"
    auth_type: "bearer_token"
    auth_value: "ghp_example_token"
    extra_headers: ["custom_key", "x-custom-header"]

  my_mcp_server:
    url: "https://my-mcp-server.com/mcp"
    static_headers:
      X-API-Key: "abc123"
      X-Custom-Header: "some-value"
```
(source: https://docs.litellm.ai/docs/mcp)

### Headers
- `extra_headers` (list): client header names forwarded to the MCP server.
- `static_headers` (map): static key/value pairs sent to the MCP server.
- Client-side auth header: default `x-mcp-auth` (DEPRECATED); override via `general_settings.mcp_client_side_auth_header_name`.

### Endpoints
Runtime:
- `POST /v1/chat/completions` — LLM endpoint consuming MCP tools (provider-agnostic).
- `POST /v1/responses` — LLM endpoint consuming MCP tools via `server_url: "litellm_proxy"` (literal value required — using the full proxy URL is incorrect).

Direct REST (no LLM):
- `GET /v1/mcp/server` — list MCP servers (read-only discovery; get `server_id`/`server_name`).
- `GET /mcp-rest/tools/list` — list MCP tools via curl.
- `POST /mcp-rest/tools/call` — call MCP tools via curl.

OAuth discovery:
- `GET /.well-known/oauth-protected-resource`
- `GET /.well-known/oauth-authorization-server`

**Note (verbatim from /docs/mcp_rest_api):** the runtime REST surface is `GET /v1/mcp/server` (read-only server discovery), `GET /mcp-rest/tools/list`, and `POST /mcp-rest/tools/call` — these are separate from the JSON-RPC MCP transport at `/mcp` or `/{server_name}/mcp`. CRUD management endpoints (POST/PUT/PATCH/DELETE `/mcp` or `/mcp/{id}`) are NOT documented on `/docs/mcp_rest_api` (verbatim negative finding — that page is a runtime tool-calling reference, not a management CRUD surface). `store_model_in_db` and the DB requirement for runtime-added servers are documented on `/docs/mcp` (see MCP Overview extraction), NOT on `/docs/mcp_rest_api`. `/mcp-rest/*` working without DB is inferred (operates on configured servers; DB not mentioned on the runtime REST page).

### Tracing / Permissions / Cost
- **Tracing:** tool calls via `/v1/responses` and `/v1/chat/completions` go through LiteLLM logging/callbacks. See `/docs/mcp_troubleshoot`.
- **Permissions:** per-key/team MCP access on `/docs/mcp_control` (deferred). `general_settings.require_end_user_mcp_access_defined` and `user_mcp_management_mode` govern user-level access. Per-key permissions require DB (virtual keys).
- **Cost tracking:** `/docs/mcp_cost` (deferred). Generally requires DB (spend logs); `general_settings.disable_spend_logs: true` disables spend log writes.

### DB / Enterprise requirements
- **DB:** Static `mcp_servers` config works **without** a DB. Runtime-added servers (management API) need `general_settings.store_model_in_db: true`. Fine-grained control via `supported_db_objects: ["mcp"]`. `mcp_aliases` and `mcp_client_side_auth_header_name` work without DB.
- **Enterprise:** not documented on the /docs/mcp Overview page (footer card only).

### Workestrator notes
In the current in-memory (no-DB) deployment: **MCP Gateway is fully usable.** Define `mcp_servers` (HTTP/SSE/stdio) in `config.yaml`, set `mcp_aliases`, invoke via `/v1/chat/completions` or `/v1/responses` with `server_url: "litellm_proxy"`, and use `/mcp-rest/tools/list|call` (inferred no-DB). All auth types work without DB. Avoid the deprecated `x-mcp-auth` header. Runtime-added MCP servers (management API) are unavailable without DB.

---

## 2. Skills Gateway

### Concept
The Skills Gateway exposes Anthropic-compatible `/v1/skills` endpoints for creating, managing, and using reusable Claude skills in `SKILL.md` format. LiteLLM follows the [Anthropic Skills API](https://docs.anthropic.com/en/docs/build-with-claude/skills). A sibling `/docs/skills_gateway` page documents the central registry (Skills Registry) — verbatim upstream: `POST /claude-code/plugins`, `GET /claude-code/plugins`, `POST /claude-code/plugins/{name}/enable`, `POST /claude-code/plugins/{name}/disable`, `GET /public/skill_hub`, `GET /claude-code/marketplace.json`. (source: https://docs.litellm.ai/docs/skills_gateway)

### Config section
No `skills:` or `litellm_settings.skills` config block is documented on `/docs/skills`. Skills are managed via `/v1/skills` REST endpoints. Model-based routing uses standard `model_list` entries with anthropic models.

```yaml
# Model-Based Routing (Multi-Account) — verbatim from /docs/skills
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

### SKILL.md format
```yaml
# SKILL.md frontmatter example — verbatim from /docs/skills
---
name: test-skill
description: A brief description of what this skill does
license: MIT
allowed-tools:
  - computer_20250124
  - text_editor_20250124
---
```
Rules: `name` must be lowercase, numbers, hyphens only, and **must exactly match** the directory name. `SKILL.md` must be in the root of the skill directory (not a subdirectory). All additional files must be in the same skill directory. (source: https://docs.litellm.ai/docs/skills)

### Auth model
Anthropic-style auth: `X-Api-Key` header OR model-based routing via `model_list` with anthropic models (per-account `ANTHROPIC_API_KEY`). No mcp-style `auth_type` enum.

### Headers
All `/v1/skills` requests require:
- `X-Api-Key: sk-1234`
- `anthropic-version: 2023-06-01`
- `anthropic-beta: skills-2025-10-02`
- Query param `?beta=true` on every request
- Optional query param `model=<model_name>` for model-based routing

### Endpoints
- `POST /v1/skills?beta=true` — create a skill (upload ZIP or SKILL.md multipart form).
- `GET /v1/skills?beta=true` — list skills (supports `limit`; SDK uses `limit=20`).
- `GET /v1/skills/{skill_id}?beta=true` — get a single skill's details.
- `DELETE /v1/skills/{skill_id}?beta=true` — delete a skill (only when no versions exist).

**Central registry (verbatim from /docs/skills_gateway):**
- `POST /claude-code/plugins` — register a skill (Auth: Required).
- `GET /claude-code/plugins` — list all skills, admin (Auth: Required).
- `POST /claude-code/plugins/{name}/enable` — publish a skill (Auth: Required).
- `POST /claude-code/plugins/{name}/disable` — unpublish a skill (Auth: Required).
- `GET /public/skill_hub` — list public skills (Auth: None).
- `GET /claude-code/marketplace.json` — Claude Code marketplace manifest (Auth: None).

> ⚠️ **Source caveat (resolved):** the six central-registry endpoints above are now CONFIRMED verbatim on `/docs/skills_gateway` (source: https://docs.litellm.ai/docs/skills_gateway). However, the DB/persistence requirement is NOT documented on that page (verbatim negative finding — strings 'database'/'DB'/'Postgres' absent). The prior project-context claim that these 'require DB for persistence' (skills lost on restart) remains an INFERENCE, now grounded in the upstream silence on persistence.

### Tracing / Permissions / Cost
- **Tracing:** feature card on `/docs/skills` lists Logging ✅.
- **Permissions:** not documented on `/docs/skills`. Model-based routing provides per-account credential isolation.
- **Cost tracking:** feature card lists Cost Tracking ✅. Requires DB for spend persistence (inferred).

### DB / Enterprise requirements
- **DB:** DB requirement is **not documented** on `/docs/skills` (Anthropic `/v1/skills` page) NOR on `/docs/skills_gateway` (central registry page) — verbatim negative finding on both pages (strings 'database'/'DB'/'Postgres' absent). Skills storage backend is unconfirmed upstream. The prior project-context inference that runtime skill registration (`/v1/skills` and `/claude-code/plugins` registry) needs DB for persistence (skills lost on restart) remains an INFERENCE, now grounded in the verbatim negative finding that upstream is silent on persistence. Model-based routing works without DB.
- **Enterprise:** not documented on `/docs/skills` (footer card only).
- **Supported providers:** `anthropic`.

### Workestrator notes
In the current in-memory (no-DB) deployment: **Skills Gateway persistence is NOT usable.** `/v1/skills` endpoints have an unconfirmed DB requirement (inferred needs DB for persistence); `POST /claude-code/plugins` and `GET /public/skill_hub` require DB per project context — runtime-registered skills are lost on restart. Model-based routing (`model_list` with anthropic models) works without DB. Skip Skills gateway persistence for M1; revisit when Postgres is added.

---

## 3. A2A Agent Gateway

### Concept
The A2A (Agent-to-Agent) Agent Gateway exposes configured agents via the A2A protocol (JSON-RPC 2.0). The proxy routes A2A agents using `a2a-sdk` 1.x and serves either A2A 0.3 or 1.0 wire format per agent. Supported agent providers: A2A, Vertex AI Agent Engine, LangGraph, Azure AI Foundry, Bedrock AgentCore, Pydantic AI. (source: https://docs.litellm.ai/docs/a2a)

### Config section
Top-level `agents:` block in `config.yaml` (list of agent entries). Each entry has `agent_name`, `agent_card_params`, and optional `litellm_params`.

```yaml
# Add A2A Agents - config.yaml agents block — verbatim from /docs/a2a
agents:
  - agent_name: my-agent
    agent_card_params:
      name: "My Agent"
      url: "http://localhost:10001"
      protocolVersion: "1.0"  # or "0.3"
```
(source: https://docs.litellm.ai/docs/a2a)

> Note: only one `agents:` block example is shown on the Overview page. Full agent definitions with all optional fields are on sibling pages (`/docs/a2a_agent_card`, `/docs/a2a_invoking_agents`, etc. — deferred).

### Auth model
LiteLLM virtual key auth: `Authorization: Bearer sk-your-litellm-key` or `x-litellm-api-key` header. Per-agent permission check: after virtual key auth, LiteLLM checks whether the calling key (and its team) is allowed to invoke the requested agent; HTTP 403 if not. Per-key/team permissions require DB (virtual keys). The caller's virtual key and end-user ID are not automatically forwarded.

### Headers
- `Authorization: Bearer sk-your-litellm-key`
- `x-litellm-api-key`
- `a2a-version: 1.0`
- `X-LiteLLM-Trace-Id`
- `X-LiteLLM-Agent-Id`
- `x-litellm-trace-id` / `x-litellm-session-id`
- `x-a2a-{agent_name_or_id}-{header}` (forwarding custom headers to upstream agent)

### Protocol versioning
Accepted `protocolVersion` values: `"0.3"` or `"1.0"` — other values return HTTP 400 at registration. The proxied agent card defaults to `1.0` when unset; legacy `message/send` callers without an `a2a-version` header receive `0.3`-shaped responses. `a2a-sdk>=1.1.0` required (included in `proxy` / `proxy-dev` dependency groups).

### Endpoints
Runtime (JSON-RPC 2.0):
- `POST /a2a/{agent_id}` — primary A2A endpoint; accepts any A2A method (`message/send`, `message/stream`, `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/resubscribe`, `tasks/pushNotificationConfig/*`, `agent/getAuthenticatedExtendedCard`).
- `POST /a2a/{agent_id}/message/send` — alias for `message/send` only.
- `POST /v1/a2a/{agent_id}/message/send` — alias for `message/send` only.
- `GET /a2a/{agent_id}/.well-known/agent.json` — agent card discovery (proxy URL in `url` field).
- `GET /a2a/{agent_id}/.well-known/agent-card.json` — agent card discovery (standard path).

Management:
- `POST /v1/agents` — admin endpoint to register a new agent (used in trace-id-enforcement example). Inferred to require DB for persistence.

### Tracing / Permissions / Cost
- **Tracing:** `X-LiteLLM-Trace-Id` / `x-litellm-trace-id` / `x-litellm-session-id` headers. When `require_trace_id_on_calls_to_agent` or `require_trace_id_on_calls_by_agent` is set, requests missing the trace id are rejected with HTTP 400. Sub-agent identity propagation supported. `message/send` and `message/stream` go through LiteLLM's A2A client (logging, guardrails, spend); other methods are forwarded to the upstream URL unchanged.
- **Permissions:** per-agent permission check after virtual key auth (HTTP 403 if not allowed). `/docs/a2a_agent_permissions` (deferred). Per-key/team permissions require DB.
- **Cost tracking:** `/docs/a2a_cost_tracking` (deferred). Feature card: Logging ✅, Load Balancing ✅, Streaming ✅, Iteration Budgets ✅. Likely requires DB (inferred).
- **Iteration budgets:** `/docs/a2a_iteration_budgets` (deferred). Likely requires DB for persistence (inferred).

### DB / Enterprise requirements
- **DB:** static `agents:` config works **without** a DB (file-based registration). `POST /v1/agents` admin registration may require DB for persistence (unconfirmed — inferred). Per-key/team agent permissions require DB (virtual keys). Cost tracking and iteration budgets likely require DB (inferred). Trace ID enforcement works without DB. Load balancing across multiple agent deployments works without DB (static config).
- **Enterprise:** not documented on the /docs/a2a Overview page (footer card only).

### Workestrator notes
In the current in-memory (no-DB) deployment: **A2A Agent Gateway is usable for static agents.** Define agents in the `config.yaml` `agents:` block, invoke via `POST /a2a/{agent_id}` (JSON-RPC), set `protocolVersion: "1.0"` or `"0.3"`, enable trace ID enforcement (`require_trace_id_on_calls_to_agent` / `require_trace_id_on_calls_by_agent`) without DB, and load-balance across static deployments without DB. `POST /v1/agents` admin registration, per-key/team permissions, cost tracking, and iteration budgets likely require DB (inferred) — unavailable.

---

## Cross-gateway summary

| Gateway | Static config works without DB? | Runtime management without DB? | Workestrator M1 usable? |
|---|---|---|---|
| MCP | Yes (`mcp_servers:` block) | No (needs `store_model_in_db`) | Yes |
| Skills | Model-based routing only | No (`/v1/skills` persistence + `/claude-code/plugins` + `/public/skill_hub` need DB) | No (persistence) |
| A2A agents | Yes (`agents:` block) | No (`POST /v1/agents` inferred needs DB) | Yes (static) |

## Anti-hallucination provenance
- MCP config keys, auth types, YAML snippets, endpoints: verbatim from `docs/litellm/extracted/p2-gateway-mcp.md` (source https://docs.litellm.ai/docs/mcp) and `p0-config_settings.md` (source https://docs.litellm.ai/docs/proxy/config_settings).
- Skills config keys, SKILL.md format, endpoints, headers: verbatim from `docs/litellm/extracted/p2-gateway-skills.md` (source https://docs.litellm.ai/docs/skills).
- A2A config keys, endpoints, headers, protocol versions: verbatim from `docs/litellm/extracted/p2-gateway-a2a.md` (source https://docs.litellm.ai/docs/a2a).
- Six Skills Gateway central-registry endpoints (`POST/GET /claude-code/plugins`, `POST /claude-code/plugins/{name}/enable|disable`, `GET /public/skill_hub`, `GET /claude-code/marketplace.json`): verbatim from `docs/litellm/extracted/p2-gateway-skills_gateway.md` (source https://docs.litellm.ai/docs/skills_gateway). DB/persistence requirement NOT documented on that page (verbatim negative finding).
- MCP runtime REST surface (`GET /v1/mcp/server`, `GET /mcp-rest/tools/list`, `POST /mcp-rest/tools/call`): verbatim from `docs/litellm/extracted/p2-gateway-mcp_rest_api.md` (source https://docs.litellm.ai/docs/mcp_rest_api). CRUD management endpoints and `store_model_in_db` NOT documented on that page (verbatim negative finding).
- Items marked "inferred" are project-context reasoning, not verbatim upstream text.
- Any gateway detail not documented in the fetched files is marked "not documented in fetched source".
