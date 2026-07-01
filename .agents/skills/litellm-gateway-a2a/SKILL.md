---
name: litellm-gateway-a2a
description: |
  Operational reference for the LiteLLM A2A (Agent-to-Agent) Agent Gateway:
  static agents config (NO DB), protocolVersion 0.3/1.0, JSON-RPC endpoints,
  management POST /v1/agents, trace-id enforcement, and per-key/team
  permissions + cost tracking (likely need DB, inferred). Load when
  configuring or reviewing A2A agents in config.yaml. Distilled from
  docs/litellm/.
---

# LiteLLM A2A Agent Gateway

Distilled operational guidance for the LiteLLM A2A (Agent-to-Agent) Agent
Gateway (JSON-RPC 2.0, A2A 0.3 or 1.0 wire format). Full detail lives in:

- `docs/litellm/gateway/README.md` — A2A section, cross-gateway summary.
- `docs/litellm/extracted/p2-gateway-a2a.md` — verbatim config, endpoints,
  headers, protocol versioning.
- `docs/litellm/schemas/gateway-agent-mcp-skills.index.json` — index.

Static `agents:` config works **WITHOUT a database**. `POST /v1/agents` admin
registration, per-key/team permissions, cost tracking, and iteration budgets
likely require DB (inferred).

## Triggers

Load this skill when:

- Adding or reviewing an `agents:` block in `config.yaml`.
- Choosing `protocolVersion` (`"0.3"` or `"1.0"`).
- Invoking agents via `POST /a2a/{agent_id}` (JSON-RPC).
- Enforcing trace IDs (`require_trace_id_on_calls_to_agent` /
  `require_trace_id_on_calls_by_agent`).
- Deciding whether management/permissions/cost need DB.

## Config section

Top-level `agents:` block (list of agent entries). Each entry: `agent_name`,
`agent_card_params` (with `name`, `url`, `protocolVersion`), optional
`litellm_params`.

```yaml
agents:
  - agent_name: my-agent
    agent_card_params:
      name: "My Agent"
      url: "http://localhost:10001"
      protocolVersion: "1.0"  # or "0.3"
```

Supported agent providers: A2A, Vertex AI Agent Engine, LangGraph, Azure AI
Foundry, Bedrock AgentCore, Pydantic AI. Requires `a2a-sdk>=1.1.0` (included
in `proxy` / `proxy-dev` dependency groups).

## Auth model

LiteLLM virtual key auth: `Authorization: Bearer sk-your-litellm-key` or
`x-litellm-api-key` header. Per-agent permission check after virtual key auth
(HTTP 403 if not allowed). Per-key/team permissions require DB (virtual
keys). Caller's virtual key and end-user ID are NOT automatically forwarded.

## Headers

- `Authorization: Bearer sk-your-litellm-key`
- `x-litellm-api-key`
- `a2a-version: 1.0`
- `X-LiteLLM-Trace-Id`
- `X-LiteLLM-Agent-Id`
- `x-litellm-trace-id` / `x-litellm-session-id`
- `x-a2a-{agent_name_or_id}-{header}` (forwarding custom headers upstream)

## Protocol versioning (verbatim)

Accepted `protocolVersion` values: `"0.3"` or `"1.0"` — other values return
HTTP 400 at registration. Proxied agent card defaults to `1.0` when unset;
legacy `message/send` callers without an `a2a-version` header receive
`0.3`-shaped responses.

## Endpoints

Runtime (JSON-RPC 2.0):
- `POST /a2a/{agent_id}` — primary; accepts any A2A method (`message/send`,
  `message/stream`, `tasks/get`, `tasks/list`, `tasks/cancel`,
  `tasks/resubscribe`, `tasks/pushNotificationConfig/*`,
  `agent/getAuthenticatedExtendedCard`).
- `POST /a2a/{agent_id}/message/send` — alias for `message/send` only.
- `POST /v1/a2a/{agent_id}/message/send` — alias for `message/send` only.
- `GET /a2a/{agent_id}/.well-known/agent.json` — agent card discovery
  (proxy URL in `url` field).
- `GET /a2a/{agent_id}/.well-known/agent-card.json` — standard card path.

Management:
- `POST /v1/agents` — admin endpoint to register a new agent. Inferred to
  require DB for persistence.

## Tracing / Permissions / Cost

- **Tracing:** `X-LiteLLM-Trace-Id` / `x-litellm-trace-id` /
  `x-litellm-session-id`. When `require_trace_id_on_calls_to_agent` or
  `require_trace_id_on_calls_by_agent` is set, requests missing the trace id
  are rejected with HTTP 400. Sub-agent identity propagation supported.
  `message/send` and `message/stream` go through LiteLLM's A2A client
  (logging, guardrails, spend); other methods forwarded to upstream unchanged.
- **Permissions:** per-agent permission check after virtual key auth (HTTP
  403). Per-key/team permissions require DB.
- **Cost tracking:** feature card Logging ✅, Load Balancing ✅, Streaming ✅,
  Iteration Budgets ✅. Likely requires DB (inferred).
- **Iteration budgets:** likely require DB for persistence (inferred).

## DB / Enterprise requirements

| Feature | Requirement | In-memory? |
|---------|-------------|------------|
| Static `agents:` config | none | **YES** |
| `POST /a2a/{agent_id}` JSON-RPC | none | **YES** |
| Agent card discovery (`.well-known/agent.json`) | none | **YES** |
| Trace ID enforcement | none | **YES** |
| Load balancing across static deployments | none | **YES** |
| `POST /v1/agents` admin registration | DB (inferred) | **NO** |
| Per-key/team agent permissions | DB (virtual keys) | **NO** |
| Cost tracking | DB (inferred, spend logs) | **NO** |
| Iteration budgets | DB (inferred) | **NO** |

Enterprise: not documented on the `/docs/a2a` Overview page (footer card only).

## Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| `protocolVersion` rejected (HTTP 400) | value other than `"0.3"`/`"1.0"` | Use exactly `"0.3"` or `"1.0"`. |
| Trace-id requests rejected (HTTP 400) | `require_trace_id_*` set, header missing | Send `X-LiteLLM-Trace-Id` / `x-litellm-trace-id`. |
| Per-agent permission 403 | virtual key not allowed for agent | Add Postgres (M4) for per-key/team permissions, or allow via static config. |
| `POST /v1/agents` not persisted | inferred needs DB | Define agents statically in `agents:` block (M1). |
| Legacy caller gets wrong response shape | no `a2a-version` header → `0.3` shape | Send `a2a-version: 1.0` for 1.0-shaped responses. |
| `a2a-sdk` missing | <1.1.0 installed | Install via `proxy` / `proxy-dev` dependency groups. |
| Cost tracking not populating | needs DB (inferred) | Add Postgres; remove `disable_spend_logs` if spend needed. |

## Related Docs

- `docs/litellm/gateway/README.md`
- `docs/litellm/extracted/p2-gateway-a2a.md`
- `docs/litellm/schemas/gateway-agent-mcp-skills.index.json`

> Do not hallucinate endpoints, headers, or protocol versions. Every endpoint
> and header above is verbatim from `p2-gateway-a2a.md`. DB requirements for
> `POST /v1/agents`, permissions, cost tracking, and iteration budgets are
> INFERENCE (marked), grounded in upstream silence on persistence — clearly
> flagged.
