---
source_url: https://docs.litellm.ai/docs/mcp
canonical_url: https://docs.litellm.ai/docs/mcp
title: "MCP Overview"
sidebar_section_path: gateway_mcp_skills_agent (Agent & MCP Gateway)
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: gateway
---
# MCP Overview

## Headings
- Overview
- Adding your MCP
  - Prerequisites
    - Fine-grained Database Storage Control
  - Add HTTP MCP Server
  - Add SSE MCP Server
  - Add STDIO MCP Server
  - OAuth Configuration & Overrides
  - AWS SigV4 Authentication
  - Static Headers
  - Server Variables
- MCP Walkthroughs
- MCP Aliases
- Converting OpenAPI Specs to MCP Servers
- MCP OAuth
  - How It Works
- Forwarding Custom Headers to MCP Servers
  - Passing Request Headers to STDIO env Vars
- Control MCP Access for End Users
- Calling the Proxy's /v1/responses Endpoint
  - Sending Custom Headers to MCP Servers
- Using your MCP with client side credentials
  - New Server-Specific Auth Headers (Recommended)
  - Legacy Auth Header (Deprecated)
- Use MCP tools with /chat/completions
- LiteLLM Proxy - Walk through MCP Gateway
- LiteLLM Python SDK MCP Bridge

## Exact config keys found (full path, verbatim spelling)
- `general_settings.store_model_in_db` (general_settings) — bool; enables DB storage of MCP servers
- `general_settings.supported_db_objects` (general_settings) — list e.g. `["mcp"]`; restricts DB storage to specific object types
- `general_settings.mcp_client_side_auth_header_name` (general_settings) — overrides default `x-mcp-auth` header
- `litellm_settings.mcp_aliases` (litellm_settings) — dict mapping alias → server name
- `mcp_servers.<name>.url` (mcp_servers)
- `mcp_servers.<name>.transport` (mcp_servers) — values: `sse`, `http`, `stdio`; defaults to `sse`
- `mcp_servers.<name>.command` (mcp_servers) — required for stdio
- `mcp_servers.<name>.args` (mcp_servers) — optional for stdio
- `mcp_servers.<name>.env` (mcp_servers) — optional for stdio
- `mcp_servers.<name>.description` (mcp_servers) — optional
- `mcp_servers.<name>.auth_type` (mcp_servers) — values: `none`, `api_key`, `bearer_token`, `basic`, `authorization`, `token`, `oauth2`, `oauth2_token_exchange`, `aws_sigv4`
- `mcp_servers.<name>.auth_value` (mcp_servers)
- `mcp_servers.<name>.extra_headers` (mcp_servers) — list of header names to forward
- `mcp_servers.<name>.static_headers` (mcp_servers) — map of header key/value pairs
- `mcp_servers.<name>.spec_version` (mcp_servers) — optional; defaults to `2025-06-18`
- `mcp_servers.<name>.allow_all_keys` (mcp_servers)
- `mcp_servers.<name>.authorization_url` (mcp_servers) — OAuth override
- `mcp_servers.<name>.token_url` (mcp_servers) — OAuth override
- `mcp_servers.<name>.registration_url` (mcp_servers) — OAuth override
- `mcp_servers.<name>.client_id` (mcp_servers) — OAuth
- `mcp_servers.<name>.client_secret` (mcp_servers) — OAuth
- `mcp_servers.<name>.scopes` (mcp_servers) — OAuth
- `mcp_servers.<name>.aws_role_name` (mcp_servers) — SigV4
- `mcp_servers.<name>.aws_access_key_id` (mcp_servers) — SigV4
- `mcp_servers.<name>.aws_secret_access_key` (mcp_servers) — SigV4
- `mcp_servers.<name>.aws_region_name` (mcp_servers) — SigV4
- `mcp_servers.<name>.aws_service_name` (mcp_servers) — SigV4; defaults to `bedrock-agentcore`

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Prerequisites - config.yaml enabling DB storage
general_settings:
  store_model_in_db: true
```
(source: https://docs.litellm.ai/docs/mcp)

```yaml
# Fine-grained Database Storage Control - store only MCPs in DB
general_settings:
  store_model_in_db: true
  supported_db_objects: ["mcp"]  # Only store MCP servers in DB

model_list:
  - model_name: gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_key: sk-xxxxxxx
```
(source: https://docs.litellm.ai/docs/mcp)

```yaml
# Adding MCP in config.yaml (main mcp_servers block)
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

```yaml
# MCP auth examples (config.yaml)
mcp_servers:
  api_key_example:
    url: "https://my-mcp-server.com/mcp"
    auth_type: "api_key"
    auth_value: "abc123"        # headers={"X-API-Key": "abc123"}

  # NEW – OAuth 2.0 Client Credentials (v1.77.5)
  oauth2_example:
    url: "https://my-mcp-server.com/mcp"
    auth_type: "oauth2"         # 👈 KEY CHANGE
    authorization_url: "https://my-mcp-server.com/oauth/authorize" # optional override
    token_url: "https://my-mcp-server.com/oauth/token"             # optional override
    registration_url: "https://my-mcp-server.com/oauth/register"   # optional override
    client_id: os.environ/OAUTH_CLIENT_ID
    client_secret: os.environ/OAUTH_CLIENT_SECRET
    scopes: ["tool.read", "tool.write"] # optional override

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

  # AWS SigV4 for Bedrock AgentCore MCP servers
  agentcore_mcp:
    url: "https://bedrock-agentcore.us-east-1.amazonaws.com/runtimes/<url-encoded-ARN>/invocations"
    transport: "http"
    auth_type: "aws_sigv4"
    aws_role_name: os.environ/AWS_ROLE_ARN          # optional — IAM role to assume
    aws_access_key_id: os.environ/AWS_ACCESS_KEY_ID  # optional — falls back to IAM role
    aws_secret_access_key: os.environ/AWS_SECRET_ACCESS_KEY
    aws_region_name: us-east-1
    aws_service_name: bedrock-agentcore

  # Example with extra headers forwarding
  github_mcp:
    url: "https://api.githubcopilot.com/mcp"
    auth_type: "bearer_token"
    auth_value: "ghp_example_token"
    extra_headers: ["custom_key", "x-custom-header"]  # These headers will be forwarded from client

  # Example with static headers
  my_mcp_server:
    url: "https://my-mcp-server.com/mcp"
    static_headers: # These headers will be requested to the MCP server
      X-API-Key: "abc123"
      X-Custom-Header: "some-value"
```
(source: https://docs.litellm.ai/docs/mcp)

```yaml
# MCP Aliases
litellm_settings:
  mcp_aliases:
    "github": "github_mcp_server"      # Maps "github" alias to "github_mcp_server"
    "zapier": "zapier_mcp_server"      # Maps "zapier" alias to "zapier_mcp_server"
    "docs": "deepwiki_mcp_server"      # Maps "docs" alias to "deepwiki_mcp_server"
    "github_alt": "github_mcp_server"  # This will be ignored since "github" already maps to this server
```
(source: https://docs.litellm.ai/docs/mcp)

```yaml
# Customize the MCP Auth Header Name
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_key: sk-xxxxxxx

general_settings:
  mcp_client_side_auth_header_name: "authorization"
```
(source: https://docs.litellm.ai/docs/mcp)

## Exact environment variables
- `STORE_MODEL_IN_DB` — required to enable DB storage of MCP servers (set to `True`)
- `LITELLM_MCP_CLIENT_SIDE_AUTH_HEADER_NAME` — overrides the default `x-mcp-auth` header
- `OAUTH_CLIENT_ID` / `OAUTH_CLIENT_SECRET` — OAuth2 client credentials
- `GITHUB_OAUTH_CLIENT_ID` / `GITHUB_OAUTH_CLIENT_SECRET` — GitHub MCP OAuth
- `AWS_ROLE_ARN` / `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` — SigV4
- `FASTMCP_LOG_LEVEL` — stdio MCP process env var
- `CIRCLECI_TOKEN` / `CIRCLECI_BASE_URL` — stdio MCP process env vars

## Exact endpoint paths / API routes (runtime + management)
- `/mcp-rest/tools/list` — Direct REST API for listing MCP tools via curl without an LLM
- `/mcp-rest/tools/call` — Direct REST API for calling MCP tools without an LLM
- `/.well-known/oauth-protected-resource` — Resource metadata endpoint for OAuth discovery
- `/.well-known/oauth-authorization-server` — OAuth server metadata endpoint
- `/v1/responses` — LLM endpoint that consumes MCP tools via `server_url: "litellm_proxy"`
- `/v1/chat/completions` — LLM endpoint that consumes MCP tools (provider-agnostic)
- NOTE: This Overview page does NOT document management endpoints (e.g. POST `/mcp`, GET `/mcp`). Management endpoints likely live in `/docs/mcp_rest_api`.

## Exact CLI commands
- `npx -y @circleci/mcp-server-circleci` — example stdio MCP server launch command
- `uvx strands-agents-mcp-server` — example stdio MCP server launch command

## Requirements
- database: required only for DB-stored MCP servers — verbatim: "To store MCP servers in the database, you need to enable database storage". Also: "By default, when `store_model_in_db` is `true`, all object types (models, MCPs, guardrails, vector stores, etc.) are stored in the database." Config-file-defined MCP servers (in `mcp_servers:` block) do NOT require a DB.
- redis: not documented on this page
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- `x-mcp-auth` broadcast header is deprecated — verbatim: "You can also specify your MCP auth token using the header `x-mcp-auth`. This will be forwarded to all MCP servers and is deprecated in favor of server-specific headers." Section titled "Legacy Auth Header (Deprecated)".

## Caveats / pitfalls
- "Starting in LiteLLM v1.80.18, the LiteLLM MCP protocol version is `2025-11-25`. LiteLLM namespaces multiple MCP servers by prefixing each tool name with its MCP server name, so newly created servers now must use names that comply with SEP-986"
- For `/v1/responses`: "Do not use the full proxy URL — Using `server_url: \"https://your-proxy.com/mcp\"` is incorrect... The proxy needs the literal value `litellm_proxy` to route to its configured MCP servers."
- Spec Version default: "Optional MCP specification version (defaults to `2025-06-18`)"
- Auth type table caveat: "the header table above describes the managed SSE/HTTP transport path. The OpenAPI-tool path emits `Authorization: ApiKey <value>` instead of `X-API-Key` for `auth_type: api_key`"

## Related links
- /docs/mcp_usage — Using your MCP
- /docs/mcp_rest_api — MCP REST API
- /docs/mcp_openapi — MCP from OpenAPI Specs
- /docs/mcp_oauth — MCP OAuth
- /docs/mcp_obo_auth — MCP OBO Auth
- /docs/mcp_aws_sigv4 — MCP AWS SigV4 Auth
- /docs/mcp_zero_trust — MCP Zero Trust Auth (JWT Signer)
- /docs/mcp_public_internet — Exposing MCPs on Public Internet
- /docs/mcp_deployment — MCP Deployment Guide
- /docs/mcp_semantic_filter — MCP Semantic Tool Filter
- /docs/mcp_control — MCP Permission Management
- /docs/mcp_cost — MCP Cost Tracking
- /docs/mcp_guardrail — MCP Guardrails
- /docs/mcp_server_submissions — MCP Server Submissions
- /docs/mcp_toolsets — MCP Toolsets
- /docs/mcp_troubleshoot — MCP Troubleshooting Guide
- /docs/skills_gateway — Skills Gateway
- /docs/a2a — A2A Agent Gateway

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- CRITICAL: MCP servers defined in config.yaml (`mcp_servers:` block) work WITHOUT a database — they're static config. The workestrator can define MCP servers (HTTP, SSE, stdio) in config.yaml and use them via `/v1/chat/completions` or `/v1/responses` with `server_url: "litellm_proxy"`. `litellm_settings.mcp_aliases` works without DB. `general_settings.mcp_client_side_auth_header_name` works without DB. DB storage (`store_model_in_db: true`) is OPTIONAL — only needed for runtime-added MCP servers via management API (unavailable without DB). `/mcp-rest/tools/list` and `/mcp-rest/tools/call` likely work without DB (operate on configured servers — inferred). Auth types (api_key, bearer_token, basic, oauth2, aws_sigv4) work without DB. The deprecated `x-mcp-auth` header should be avoided.

## Confidence / uncertainty notes
- high confidence on mcp_servers config format and DB requirement (verbatim quotes). Config-file MCP servers work without DB (inferred high confidence — DB is only for "storing" servers, not for serving them). Management endpoints (POST /mcp etc.) are NOT on this page — they're on /docs/mcp_rest_api (deferred). `/mcp-rest/*` endpoints working without DB is inferred.
