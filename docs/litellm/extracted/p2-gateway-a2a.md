---
source_url: https://docs.litellm.ai/docs/a2a
canonical_url: https://docs.litellm.ai/docs/a2a
title: "Agent Gateway (A2A Protocol) - Overview"
sidebar_section_path: gateway_mcp_skills_agent (Agent & MCP Gateway)
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: gateway
---
# Agent Gateway (A2A Protocol) - Overview

## Headings
- Adding your Agent
  - Add A2A Agents
  - Add Azure AI Foundry Agents
  - Add Vertex AI Agent Engine
  - Add Bedrock AgentCore Agents
  - Add LangGraph Agents
  - Add Pydantic AI Agents
- Protocol versioning
  - Pinning a version
  - When protocolVersion is not pinned
  - Dependency
- Invoking your Agents
- Tracking Agent Logs
- Forwarding LiteLLM Context Headers
  - Implementation Steps
  - Result
- API Reference
  - Endpoints
  - Supported JSON-RPC methods
  - Authentication
    - Per-agent permission check
    - Trace ID enforcement (optional, per-agent)
    - Sub-agent identity propagation
  - Request Format
  - Response Format
  - Example: tasks/get
- Agent Registry

## Exact config keys found (full path, verbatim spelling)
- `agents[].agent_name` (agents)
- `agents[].agent_card_params.name` (agents)
- `agents[].agent_card_params.url` (agents)
- `agents[].agent_card_params.protocolVersion` (agents) — accepted values: `"0.3"` or `"1.0"`; HTTP 400 for other values
- `agents[].litellm_params.require_trace_id_on_calls_to_agent` (agents / litellm_params)
- `agents[].litellm_params.require_trace_id_on_calls_by_agent` (agents / litellm_params)

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Add A2A Agents - config.yaml agents block
agents:
  - agent_name: my-agent
    agent_card_params:
      name: "My Agent"
      url: "http://localhost:10001"
      protocolVersion: "1.0"  # or "0.3"
```
(source: https://docs.litellm.ai/docs/a2a)

NOTE: Only ONE config.yaml `agents:` block is shown on this Overview page. Full agent definitions with all optional fields are on sibling pages (`/docs/a2a_agent_card`, `/docs/a2a_invoking_agents`, etc.).

## Exact environment variables
- none documented on this page (no env vars specific to A2A are shown)

## Exact endpoint paths / API routes (runtime + management)
- `POST /a2a/{agent_id}` (JSON-RPC 2.0) — Primary A2A endpoint, accepts any A2A method (message/send, message/stream, tasks/get, tasks/list, tasks/cancel, tasks/resubscribe, tasks/pushNotificationConfig/*, agent/getAuthenticatedExtendedCard)
- `POST /a2a/{agent_id}/message/send` (JSON-RPC) — Alias for `message/send` only
- `POST /v1/a2a/{agent_id}/message/send` (JSON-RPC) — Alias for `message/send` only
- `GET /a2a/{agent_id}/.well-known/agent.json` — Agent card discovery (proxy URL in `url` field)
- `GET /a2a/{agent_id}/.well-known/agent-card.json` — Agent card discovery (standard path)
- `POST /v1/agents` — Admin endpoint to register a new agent (used in trace-id-enforcement example)
- Headers: `Authorization: Bearer sk-your-litellm-key`, `x-litellm-api-key`, `a2a-version: 1.0`, `X-LiteLLM-Trace-Id`, `X-LiteLLM-Agent-Id`, `x-litellm-trace-id` / `x-litellm-session-id`, `x-a2a-{agent_name_or_id}-{header}`

## Exact CLI commands
- `pip install "a2a-sdk>=1.1.0,<2.0"` — Install a2a SDK 1.x (required if calling agents from own code)

## Requirements
- database: not documented on this page (no mention of Postgres or DB requirement for A2A on the Overview page; the `agents:` config block is file-based registration)
- redis: not documented on this page
- enterprise: not documented on this page (Enterprise only advertised in footer card)
- admin_ui: not documented on this page

## Deprecations
- "Only `\"0.3\"` and `\"1.0\"` are accepted; other values return HTTP 400 at registration." (protocolVersion restriction, not a deprecation per se)

## Caveats / pitfalls
- Supported agent providers: "A2A, Vertex AI Agent Engine, LangGraph, Azure AI Foundry, Bedrock AgentCore, Pydantic AI"
- Feature support: Logging ✅, Load Balancing ✅, Streaming ✅, Iteration Budgets ✅
- "LiteLLM proxy routes A2A agents using **a2a-sdk 1.x** and can serve either **A2A 0.3** or **1.0** wire format to clients per agent."
- "The proxied agent card defaults to `1.0` when unset, but legacy `message/send` callers without an `a2a-version` header receive **0.3**-shaped responses."
- "Task methods (`tasks/get`, `tasks/list`, …) are forwarded to the upstream agent unchanged."
- "LiteLLM proxy A2A routes require **a2a-sdk >= 1.1.0** (included in the `proxy` / `proxy-dev` dependency groups)."
- "After the virtual key is authenticated, LiteLLM checks whether the calling key (and its team) is allowed to invoke the requested agent. If not, the response is HTTP 403."
- "When set, requests missing `x-litellm-trace-id` (or `x-litellm-session-id`) are rejected with HTTP 400." (trace ID enforcement)
- "The caller's **virtual key** and **end-user ID** are not automatically forwarded."
- "`message/send` and `message/stream` go through LiteLLM's A2A client (logging, guardrails, spend). All other methods are forwarded to the upstream URL."
- "Streaming events use `statusUpdate` / `artifactUpdate` keys instead of `kind: \"status-update\"`."

## Related links
- /docs/a2a_agent_card — A2A Agent Card
- /docs/a2a_invoking_agents — Invoking A2A Agents
- /docs/a2a_agent_headers — A2A Agent Authentication Headers
- /docs/a2a_cost_tracking — A2A Agent Cost Tracking
- /docs/a2a_agent_permissions — Agent Permission Management
- /docs/a2a_iteration_budgets — Agent Iteration Budgets
- /docs/providers/azure_ai_agents#litellm-a2a-gateway — Azure AI Foundry
- /docs/providers/vertex_ai_agent_engine — Vertex AI Agent Engine
- /docs/providers/bedrock_agentcore#litellm-a2a-gateway — Bedrock AgentCore
- /docs/providers/langgraph#register-a-langgraph-platform-agent — LangGraph
- /docs/providers/pydantic_ai_agent#litellm-a2a-gateway — Pydantic AI
- /docs/proxy/ai_hub — AI Hub for central agent registry
- https://github.com/google/A2A — A2A Protocol spec

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- A2A agents defined in config.yaml (`agents:` block) work WITHOUT a database — they're static config with file-based registration. The workestrate can define agents in config.yaml and invoke them via `POST /a2a/{agent_id}` (JSON-RPC). `agent_card_params.url` points to the upstream agent. `protocolVersion: "1.0"` or `"0.3"` is required. Per-agent permission checks (`require_trace_id_on_calls_to_agent`) work without DB (inferred). `POST /v1/agents` (admin registration) may require DB for persistence (unconfirmed — inferred). Agent cost tracking (`/docs/a2a_cost_tracking`) likely requires DB (inferred). The `a2a-sdk>=1.1.0` dependency is included in proxy groups. Virtual key/team-based agent permissions require DB (unavailable). Trace ID enforcement works without DB. Load balancing across multiple agent deployments works without DB (static config).

## Confidence / uncertainty notes
- high confidence on agents config format and endpoints (verbatim). DB requirement is "not documented" — config-file agents work without DB (inferred high confidence). `POST /v1/agents` admin endpoint DB requirement is unconfirmed (inferred it may need DB for persistence). Cost tracking and per-key permissions require DB (inferred from virtual_keys domain).
