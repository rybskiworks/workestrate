---
source_url: https://docs.litellm.ai/docs/mcp_rest_api
canonical_url: https://docs.litellm.ai/docs/mcp_rest_api
raw_source_url: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/mcp_rest_api.md
title: "MCP REST API"
sidebar_section_path: gateway_mcp_skills_agent
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: gateway
---
# MCP REST API

## Headings
- MCP REST API (intro)
- Endpoints
- Tool naming
- 1. List MCP servers
- 2. List tools
  - All servers
  - One server (recommended for discovery)
- 3. Call a tool
  - Request body
  - Works: prefixed name + server UUID
  - Works: unprefixed name + server name
  - Works: x-litellm-api-key header
- What does not work
  - Missing server_id
  - arguments: null
  - Wrong separator in tool name
  - Tool belongs to a different server than server_id
  - Invalid or unknown server_id
  - Placeholder server ids from customer examples
- Quick reference
- Related docs

## Verbatim intro
"Guide to call MCP tools **directly** over HTTP. Use this when you already know which tool to run. For LLM-driven tool use, see [Using your MCP](./mcp_usage.md)."
"**Base URL:** `http://localhost:4000` (replace with your LiteLLM proxy URL)"
"**Auth:** LiteLLM API key on every request"

## Exact config keys found (full path, verbatim spelling)
- NOTE: No config.yaml keys are documented on this page. The strings `mcp_servers`, `store_model_in_db`, `config.yaml`, `general_settings` do NOT appear on this page. This page is purely a runtime REST API reference for calling tools; server provisioning (config.yaml vs DB) is out of scope.

## Exact environment variables
- `MCP_TOOL_PREFIX_SEPARATOR` — overrides the default tool-name separator (default `-` hyphen)
- `LITELLM_USE_SHORT_MCP_TOOL_PREFIX` — set to `true` to use a 3-character id prefix instead of the server name

## Exact endpoint paths / API routes (runtime + management)
Verbatim from the "Endpoints" table:
- `GET /v1/mcp/server` — List MCP servers (get `server_id` / `server_name`)
- `GET /mcp-rest/tools/list` — List tools (all servers, or one server)
- `POST /mcp-rest/tools/call` — Execute a tool

Verbatim note on the JSON-RPC transport (separate surface):
"These routes are separate from the JSON-RPC MCP transport at `/mcp` or `/{server_name}/mcp` used by Claude Desktop and Cursor."

Query/body parameters (verbatim):
- `GET /mcp-rest/tools/list?server_id=<UUID|server_name|alias>` — `server_id` accepts UUID, `server_name`, or alias
- `POST /mcp-rest/tools/call` request body:
  - `server_id` (required, string) — UUID, `server_name`, or alias
  - `name` (required, string) — Prefixed or unprefixed tool name
  - `arguments` (recommended, object) — Tool parameters; use `{}` when none. If omitted, the proxy treats it as `{}`. Do not pass `null`.

## Tool naming (verbatim)
"**Prefix format:** `{server_prefix}{separator}{upstream_tool_name}`"
- Default separator is `-` (hyphen).
- Override with env var `MCP_TOOL_PREFIX_SEPARATOR`.
- With `LITELLM_USE_SHORT_MCP_TOOL_PREFIX=true`, the prefix is a 3-character id instead of the server name.
- The proxy calls the upstream MCP server with the unprefixed tool name (e.g. `getPlaces`), not the full prefixed string.

Two patterns:
- Prefixed: `places_api-getPlaces` (global tool list / self-contained tool id)
- Unprefixed + `server_id`: `server_id: places_api`, `name: getPlaces` (per-server tool list)

## Headers
- `Authorization: Bearer sk-1234` (or `x-litellm-api-key: sk-1234`)
- `Content-Type: application/json` (required for `tools/call` POST body)

## Error responses (verbatim)
- `400 missing_parameter` — missing `server_id` in `tools/call`
- `500` — `arguments: null` (arguments must be a JSON object, not null)
- `403 tool_server_mismatch` — tool belongs to a different server than `server_id`
- `404 server_not_found` — unknown `server_id` (name/uuid)
- `403 access_denied` — server exists but key cannot access it

## Requirements
- database: NOT MENTIONED on page. The strings "database", "DB", "Postgres", "store_model_in_db", "SQL" do NOT appear anywhere in docs/mcp_rest_api.md. No persistence model is documented on this page.
- redis: NOT MENTIONED on page
- enterprise: NOT MENTIONED on page
- admin_ui: NOT MENTIONED on page

## Deprecations
- none documented on this page

## Caveats / pitfalls
- This page documents ONLY a read-only server-discovery endpoint (`GET /v1/mcp/server`) and the `/mcp-rest/tools/*` runtime surface. There are NO CRUD management endpoints (POST/PUT/PATCH/DELETE `/mcp` or `/mcp/{id}`) on this page.
- `store_model_in_db` is NOT mentioned on this page. The DB requirement for runtime-added MCP servers is documented on the sibling `/docs/mcp` Overview page (see p2-gateway-mcp.md), NOT here.
- Do not pass `arguments: null`; use `{}` or omit the field.
- Default tool-name separator is `-` (hyphen), not `_`.
- `server_id` accepts UUID, `server_name`, OR alias (all three work in `/mcp-rest/*`).

## Related links
- /docs/mcp_usage — Using your MCP (Responses API, Cursor, OpenAI SDK — LLM-driven MCP)
- /docs/mcp — MCP Overview (Gateway setup and JSON-RPC `/mcp` route)
- /docs/mcp_oauth — OAuth-protected MCP servers
- /docs/mcp_zero_trust — JWT signing for upstream MCP servers
- /docs/mcp_troubleshoot — Connectivity and auth issues

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- The `/mcp-rest/tools/list` and `/mcp-rest/tools/call` runtime endpoints are CONFIRMED verbatim upstream, plus the read-only `GET /v1/mcp/server` discovery endpoint. These operate on already-configured (config.yaml) servers; the page does NOT state a DB requirement for them, so the prior `(inferred no-DB)` assessment stands but is now grounded in a verbatim negative finding (DB NOT mentioned on the runtime REST API page). The CRUD management endpoints (POST/PUT/DELETE `/mcp`) are NOT documented on this page — the prior "deferred to /docs/mcp_rest_api" claim is RESOLVED as a NEGATIVE: they are not there. `store_model_in_db` and the DB requirement for runtime-added servers remain documented only on `/docs/mcp` (see p2-gateway-mcp.md).

## Confidence / uncertainty notes
- high confidence on the three endpoints, auth, tool-naming, request body, and error responses (verbatim). The absence of CRUD management endpoints and `store_model_in_db` on this page is a NEGATIVE FINDING (high confidence): upstream `/docs/mcp_rest_api` is a runtime tool-calling reference, not a management CRUD surface.
