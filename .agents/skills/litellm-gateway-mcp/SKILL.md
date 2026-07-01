---
name: litellm-gateway-mcp
description: |
  Operational reference for the LiteLLM MCP (Model Context Protocol) Gateway:
  static mcp_servers config (NO DB), auth_type enum, mcp_aliases, runtime +
  direct REST endpoints, and the store_model_in_db requirement for
  runtime-added servers. Load when configuring or reviewing MCP servers in
  config.yaml. Distilled from docs/litellm/; does NOT cover Skills or A2A
  gateways (see litellm-gateway-skills, litellm-gateway-a2a).
---

# LiteLLM MCP Gateway

Distilled operational guidance for the LiteLLM MCP Gateway (expose configured
MCP servers, route tool calls). Full detail lives in:

- `docs/litellm/gateway/README.md` — MCP section, cross-gateway summary.
- `docs/litellm/extracted/p2-gateway-mcp.md` — verbatim config, auth, endpoints.
- `docs/litellm/extracted/p2-gateway-mcp_rest_api.md` — runtime REST surface.
- `docs/litellm/schemas/gateway-agent-mcp-skills.index.json` — machine-readable index.

Static `mcp_servers` config works **WITHOUT a database**. Runtime-added
servers (management API) need `general_settings.store_model_in_db: true`.

## Triggers

Load this skill when:

- Adding or reviewing an `mcp_servers:` block in `config.yaml`.
- Choosing `auth_type` for an MCP server.
- Configuring `mcp_aliases` or the client-side auth header.
- Invoking MCP tools via `/v1/chat/completions`, `/v1/responses`, or `/mcp-rest/*`.
- Deciding whether runtime-added servers / management CRUD need DB.

## Config section

Top-level `mcp_servers:` block (sibling to `model_list`). Aliases under
`litellm_settings.mcp_aliases`. DB toggles under `general_settings`. **Not**
under `router_settings`.

Server entry fields: `url`, `transport` (`http` / `sse` / `stdio`), `command`
+ `args` + `env` (stdio), `description`, `auth_type`, `auth_value`,
`extra_headers` (list), `static_headers` (map).

## auth_type enum (verbatim)

`none`, `api_key`, `bearer_token`, `basic`, `authorization`, `token`,
`oauth2`, `oauth2_token_exchange`, `aws_sigv4`.

- Credential via `auth_value`.
- OAuth2 (v1.77.5+): `authorization_url` / `token_url` / `registration_url` /
  `client_id` / `client_secret` / `scopes` overrides.
- AWS SigV4: `aws_role_name` / `aws_access_key_id` / `aws_secret_access_key`
  / `aws_region_name` / `aws_service_name` (defaults `bedrock-agentcore`).

## DEPRECATED

The legacy broadcast header `x-mcp-auth` is deprecated in favor of
server-specific auth headers. Override the client-side auth header name via
`general_settings.mcp_client_side_auth_header_name` (default `x-mcp-auth`).

## Endpoints

Runtime (LLM-consuming):
- `POST /v1/chat/completions` — LLM endpoint consuming MCP tools.
- `POST /v1/responses` — LLM endpoint consuming MCP tools via
  `server_url: "litellm_proxy"` (literal value required; full proxy URL is
  incorrect).

Direct REST (no LLM):
- `GET /v1/mcp/server` — list MCP servers (read-only discovery; get
  `server_id` / `server_name`).
- `GET /mcp-rest/tools/list` — list MCP tools.
- `POST /mcp-rest/tools/call` — call MCP tools.

OAuth discovery:
- `GET /.well-known/oauth-protected-resource`
- `GET /.well-known/oauth-authorization-server`

> Verbatim negative finding: CRUD management endpoints (POST/PUT/PATCH/DELETE
> `/mcp` or `/mcp/{id}`) are NOT documented on `/docs/mcp_rest_api` — that
> page is a runtime tool-calling reference, not a management CRUD surface.

## DB / Enterprise requirements

| Feature | Requirement | In-memory? |
|---------|-------------|------------|
| Static `mcp_servers` config | none | **YES** |
| `mcp_aliases` | none | **YES** |
| `mcp_client_side_auth_header_name` | none | **YES** |
| All `auth_type` values | none | **YES** |
| `/mcp-rest/tools/list` + `/call` | none (inferred) | **YES** |
| `GET /v1/mcp/server` | none | **YES** |
| Runtime-added servers (management API) | `store_model_in_db: true` | **NO** |
| Fine-grained control (`supported_db_objects: ["mcp"]`) | DB | **NO** |
| Per-key/team MCP access (`/docs/mcp_control`) | DB (virtual keys) | **NO** |
| Cost tracking (`/docs/mcp_cost`) | DB (spend logs) | **NO** |

Enterprise: not documented on the `/docs/mcp` Overview page (footer card only).

## Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| `server_url` rejected on `/v1/responses` | full proxy URL used | Use literal `server_url: "litellm_proxy"`. |
| Runtime-added server not persisted | `store_model_in_db` false/unset | Add Postgres + set `store_model_in_db: true` (M4), or define statically. |
| `x-mcp-auth` ignored / deprecated warning | legacy broadcast header | Use server-specific `auth_type`/`auth_value`; override via `mcp_client_side_auth_header_name` if needed. |
| stdio server fails to start | `command`/`args`/`env` misconfigured | Verify `transport: stdio` + executable in proxy PATH. |
| OAuth2 flow fails | pre-v1.77.5 proxy | Upgrade to v1.77.5+ for `oauth2` / `oauth2_token_exchange`. |
| Per-key MCP access rejected | needs DB (virtual keys) | Use static config in M1; defer per-key to M4. |

## Related Docs

- `docs/litellm/gateway/README.md`
- `docs/litellm/extracted/p2-gateway-mcp.md`
- `docs/litellm/extracted/p2-gateway-mcp_rest_api.md`
- `docs/litellm/schemas/gateway-agent-mcp-skills.index.json`

> Do not hallucinate `auth_type` values, endpoint paths, or DB requirements.
> Every `auth_type` and endpoint above is verbatim from
> `p2-gateway-mcp.md` / `p2-gateway-mcp_rest_api.md`. The `/mcp-rest/*`
> no-DB inference is grounded in the verbatim negative finding that the
> runtime REST page does not mention DB. Runtime-added-server DB requirement
> is documented on `/docs/mcp` (Overview), not `/docs/mcp_rest_api`.
