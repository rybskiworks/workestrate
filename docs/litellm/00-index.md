# LiteLLM Knowledge Pack

LiteLLM Proxy is an OpenAI-compatible gateway that listens on `:4000`. Clients
send OpenAI-shaped requests (e.g. `POST /v1/chat/completions`) where the `model`
field is a **`model_name` alias** defined in `config.yaml`; LiteLLM resolves that
alias through `litellm_params` to an upstream provider (Anthropic, OpenAI,
OpenRouter, Bedrock, etc.). Workestrator runs LiteLLM **in-memory inside a
microsandbox** with **`master_key`-only auth** — no Postgres, no Redis, no UI.

## File map

| Path | What's there | Status |
| --- | --- | --- |
| `01-mental-model.md` | How the proxy routes requests (request lifecycle, alias resolution) | exists |
| `extracted/` | Verbatim facts extracted from upstream LiteLLM docs (`p0-*.md`, `p1-*.md`, `p2-*.md`) | exists |
| `schemas/` | Machine-readable indexes: `config-yaml.option-index.json`, `config-yaml.normalized.schema.md`, `endpoints.index.json`, `openapi-management-endpoints.index.json`, `env-vars.index.json`, `provider-fields.index.json`, `gateway-agent-mcp-skills.index.json` | exists |
| `crawl/` | Raw crawl artifacts: `sidebar_tree.md`, `source_map.md`, `url_inventory.json`, `url_classification.md`, `openapi_route_inventory.json` (669 routes verbatim) | exists |
| `config/` | `config.yaml` reference — top-level structure, `model_list` anatomy, per-section references | exists |
| `providers/` | Per-provider guides (`anthropic.md`, `openai.md`, `openai-compatible.md`, `openrouter.md`, `moonshot.md`, `minimax.md`, `zai-glm.md`, `vllm.md`, `ollama.md`, `litellm-proxy.md`, `inception.md`) + `README.md` | exists |
| `endpoints/` | Endpoint surface guide (`README.md`) | exists |
| `gateway/` | MCP / Skills / A2A gateway guide (`README.md`) | exists |
| `routing/` | Routing, load balancing, fallbacks guide (`README.md`) | exists |
| `auth-access-budget/` | Virtual keys, model access, budgets, rate limits guide (`README.md`) | exists |
| `deployment-ops/` | Deployment, Docker, DB/Redis, production ops guide (`README.md`) | exists |
| `observability-cache-guardrails/` | Logging, callbacks, caching, guardrails guide (`README.md`) | exists |
| `examples/` | Worked config examples (13 YAML files) | exists |
| `troubleshooting.md` | Known issues and diagnosis recipes | exists |
| `workestrator-recommended-patterns.md` | Workestrator in-memory deployment patterns and project choices | exists |

## Skills & Workflows

Operational skills and workflows now live under `.agents/skills/` (consolidated
from the former `docs/litellm/skills/` + `docs/litellm/workflows/` dirs). 55
packages total:

| Category | Count | Packages |
| --- | --- | --- |
| `workflow-litellm-*` | 5 workflows × 6 phases = 30 | config-change, config-review, debugging, validation, hardening |
| `constraint-litellm-*` | 8 | config-schema, in-memory-no-db, secret-hygiene, provider-prefix, openai-compatible-api-base, anthropic-suffix, fallback-resolution, deprecation-free |
| `validation-litellm-*` | 4 | config-check, startup, smoke, no-hardcoded-secrets |
| `litellm-*` (regular) | 13 | config-anatomy, providers, openai-compatible, routing-fallbacks, caching, logging-observability, guardrails, budgets-keys, gateway-mcp, gateway-skills, gateway-a2a, deployment, production-hardening |

The static config validator `check_config.py` (checks a–i) is co-located at
`.agents/skills/validation-litellm-config-check/scripts/check_config.py`.

## Key entry docs

- [01-mental-model.md](01-mental-model.md) — how the proxy routes requests
- [config/README.md](config/README.md) — `config.yaml` reference index
- [providers/README.md](providers/README.md) — per-provider guides
- [endpoints/README.md](endpoints/README.md) — endpoint surface
- [gateway/README.md](gateway/README.md) — MCP / Skills / A2A

## Anti-hallucination note

Every config key, provider prefix, endpoint, env-var, and default in this pack
traces to a corpus file under `extracted/` or `schemas/`. Items not documented
in the fetched source are marked **"not documented in fetched source"**. Items
inferred from context (not stated verbatim upstream) are marked **"(inferred)"**.
No keys, prefixes, endpoints, or defaults are invented.

## Sources

- https://docs.litellm.ai/docs/proxy/config_settings (PRIMARY — top-level YAML schema)
- https://docs.litellm.ai/docs/proxy/configs (Overview — model_list, providers, credentials)
- https://docs.litellm.ai/docs/simple_proxy
- Local corpus: `extracted/p0-config_settings.md`, `extracted/p0-configs.md`, `schemas/config-yaml.option-index.json`, `schemas/config-yaml.normalized.schema.md`

## Workestrator notes

[PROJECT CONTEXT — NOT upstream docs]

- LiteLLM runs **in-memory** in a microsandbox. No Postgres, no Redis, no UI.
- `infra/litellm/config.yaml` is mounted **read-only** at `/app/config.yaml`
  (`MountPlan::readonly`), started with `--config /app/config.yaml --host 0.0.0.0`,
  listening on `:4000`.
- Auth is **`master_key` only** (single admin key). Agents (Pi, Odysseus sandboxes)
  call LiteLLM with `Authorization: Bearer $LITELLM_MASTER_KEY`.
- `disable_spend_logs: true` compensates for the missing DB (prevents DB spend-log
  write attempts).
- Project env vars (`KIMI_CODE_API_KEY`, `MINIMAX_CODING_API_KEY`,
  `NEURALWATT_API_KEY`, `OPENROUTER_API_KEY`, `LITELLM_MASTER_KEY`) are resolved
  via `os.environ/` — they are **not** LiteLLM built-ins.
- DB-gated features unavailable: virtual keys, teams, users, budgets, spend
  tracking, per-key/per-team model access, DB-backed guardrail/policy/agent/prompt
  CRUD. See `schemas/config-yaml.normalized.schema.md` → "Workestrator in-memory
  deployment mapping".
- Health endpoints: `GET /health/liveliness` (liveness), `GET /health/readiness`
  (readiness), `GET /health` (model connectivity — makes real API calls).
