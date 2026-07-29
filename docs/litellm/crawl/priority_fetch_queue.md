# Priority Fetch Queue — Phase 3 Extraction

**Generated**: 2026-06-27
**Based on**: Discovered sitemap URLs + config.yaml patterns + sidebar hierarchy

## P0 — Must Extract (10 pages)

These pages cover the core config.yaml structure and proxy fundamentals directly used by
the workestrate config.

| # | URL | Why It Matters | Classification |
|---|---|---|---|
| 1 | https://docs.litellm.ai/docs/proxy/configs | config.yaml structure — model_list, the primary config entry point | proxy_config |
| 2 | https://docs.litellm.ai/docs/proxy/config_settings | litellm_settings, router_settings, general_settings, env vars — the config reference | proxy_config |
| 3 | https://docs.litellm.ai/docs/proxy/config_management | config file management — hot reload, file rotation | proxy_config |
| 4 | https://docs.litellm.ai/docs/simple_proxy | proxy overview — the gateway landing page | proxy_config |
| 5 | https://docs.litellm.ai/docs/supported_endpoints | supported endpoints — the endpoint index | endpoints |
| 6 | https://docs.litellm.ai/docs/proxy/quick_start | proxy quick start — deployment basics | deployment_ops |
| 7 | https://docs.litellm.ai/docs/proxy/docker_quick_start | docker quick start — primary deployment method | deployment_ops |
| 8 | https://docs.litellm.ai/docs/proxy/user_keys | making LLM requests — request format, auth headers | auth_access |
| 9 | https://docs.litellm.ai/docs/completion | chat/completions endpoint — the primary inference endpoint | endpoints |
| 10 | https://docs.litellm.ai/docs/proxy/deploy | production deployment guide | deployment_ops |

## P1 — High Priority (20 pages)

Priority providers (those used in config.yaml) and routing/auth/budget fundamentals.

| # | URL | Why It Matters | Classification |
|---|---|---|---|
| 1 | https://docs.litellm.ai/docs/providers/openai | OpenAI provider — openai/ prefix used in config | providers |
| 2 | https://docs.litellm.ai/docs/providers/openai_compatible | OpenAI-compatible endpoints — pattern for Neuralwatt | providers |
| 3 | https://docs.litellm.ai/docs/openai_compatible | OpenAI-compatible endpoints (alt page) | providers |
| 4 | https://docs.litellm.ai/docs/providers/openrouter | OpenRouter provider — openrouter/ prefix | providers |
| 5 | https://docs.litellm.ai/docs/providers/minimax | MiniMax provider — anthropic/MiniMax-M3 in config | providers |
| 6 | https://docs.litellm.ai/docs/providers/moonshot | Moonshot/Kimi provider — anthropic/kimi-for-coding in config | providers |
| 7 | https://docs.litellm.ai/docs/providers/inception | Inception provider | providers |
| 8 | https://docs.litellm.ai/docs/providers/zai | Z.AI/Zhipu/GLM provider — openrouter/z-ai/glm in config | providers |
| 9 | https://docs.litellm.ai/docs/providers/vllm | vLLM provider — local model pattern | providers |
| 10 | https://docs.litellm.ai/docs/providers/ollama | Ollama provider — local model pattern | providers |
| 11 | https://docs.litellm.ai/docs/providers/litellm_proxy | LiteLLM Proxy provider — chained proxy pattern | providers |
| 12 | https://docs.litellm.ai/docs/providers/anthropic | Anthropic provider — anthropic/ prefix for Kimi/MiniMax | providers |
| 13 | https://docs.litellm.ai/docs/routing-load-balancing | Routing & load balancing — fallbacks, retries | routing_reliability |
| 14 | https://docs.litellm.ai/docs/proxy/fallback_management | Fallback management — config fallbacks pattern | routing_reliability |
| 15 | https://docs.litellm.ai/docs/proxy/reliability | Reliability — allowed_fails, cooldown_time, retry_policy | routing_reliability |
| 16 | https://docs.litellm.ai/docs/proxy/timeout | Timeout — timeout, stream_timeout, request_timeout | routing_reliability |
| 17 | https://docs.litellm.ai/docs/proxy/virtual_keys | Virtual keys — key management | auth_access |
| 18 | https://docs.litellm.ai/docs/proxy/model_access | Model access — model access control | auth_access |
| 19 | https://docs.litellm.ai/docs/proxy/users | Budgets + rate limits — user/team budgets | budgets_rate_limits |
| 20 | https://docs.litellm.ai/docs/proxy/rate_limit_tiers | Rate limit tiers | budgets_rate_limits |

## P2 — Medium Priority (25 pages)

Caching, logging, guardrails, MCP/skills/agent gateway, deployment, admin UI/CLI, DB/enterprise.

| # | URL | Why It Matters | Classification |
|---|---|---|---|
| 1 | https://docs.litellm.ai/docs/proxy/caching | Caching | caching |
| 2 | https://docs.litellm.ai/docs/caching/all_caches | All cache backends | caching |
| 3 | https://docs.litellm.ai/docs/proxy/logging | Logging | logging_observability |
| 4 | https://docs.litellm.ai/docs/proxy/dynamic_logging | Dynamic logging | logging_observability |
| 5 | https://docs.litellm.ai/docs/proxy/metrics | Metrics | logging_observability |
| 6 | https://docs.litellm.ai/docs/proxy/prometheus | Prometheus integration | logging_observability |
| 7 | https://docs.litellm.ai/docs/proxy/alerting | Alerting | logging_observability |
| 8 | https://docs.litellm.ai/docs/proxy/cost_tracking | Cost tracking — disable_spend_logs | logging_observability |
| 9 | https://docs.litellm.ai/docs/proxy/guardrails/quick_start | Guardrails quick start | guardrails |
| 10 | https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies | Guardrail policies | guardrails |
| 11 | https://docs.litellm.ai/docs/mcp | MCP — Model Context Protocol gateway | gateway_mcp_skills_agent |
| 12 | https://docs.litellm.ai/docs/skills | Skills — Anthropic Skills API | gateway_mcp_skills_agent |
| 13 | https://docs.litellm.ai/docs/a2a | A2A — Agent gateway | gateway_mcp_skills_agent |
| 14 | https://docs.litellm.ai/docs/proxy/deploy | Deployment guide | deployment_ops |
| 15 | https://docs.litellm.ai/docs/proxy/docker_image_security | Docker image security | deployment_ops |
| 16 | https://docs.litellm.ai/docs/proxy/ui | Admin UI | admin_ui_cli |
| 17 | https://docs.litellm.ai/docs/proxy/management_cli | Proxy CLI | admin_ui_cli |
| 18 | https://docs.litellm.ai/docs/proxy/db_info | DB info — database requirements | deployment_ops |
| 19 | https://docs.litellm.ai/docs/enterprise | Enterprise features | deployment_ops |
| 20 | https://docs.litellm.ai/docs/proxy/architecture | Architecture overview | deployment_ops |
| 21 | https://docs.litellm.ai/docs/proxy/prod | Production setup | deployment_ops |
| 22 | https://docs.litellm.ai/docs/secret_managers/overview | Secret managers overview | auth_access |
| 23 | https://docs.litellm.ai/docs/proxy/health | Health checks | routing_reliability |
| 24 | https://docs.litellm.ai/docs/proxy/load_balancing | Load balancing | routing_reliability |
| 25 | https://docs.litellm.ai/docs/proxy/auto_routing | Auto routing / cost optimization | routing_reliability |

## P3 — Optional/Low Priority (611 pages)

All other discovered docs pages not in P0-P2 and not excluded. Includes remaining providers,
endpoint details, observability integrations, tutorials, troubleshooting, etc.

- `https://docs.litellm.ai/completion/input`
- `https://docs.litellm.ai/completion/output`
- `https://docs.litellm.ai/completion/supported`
- `https://docs.litellm.ai/contact`
- `https://docs.litellm.ai/contributing`
- `https://docs.litellm.ai/embedding/supported_embedding`
- `https://docs.litellm.ai/observability/callbacks`
- `https://docs.litellm.ai/observability/helicone_integration`
- `https://docs.litellm.ai/observability/supabase_integration`
- `https://docs.litellm.ai/stream`
- `https://docs.litellm.ai/token_usage`
- `https://docs.litellm.ai/docs/`
- `https://docs.litellm.ai/docs/a2a_agent_card`
- `https://docs.litellm.ai/docs/a2a_agent_headers`
- `https://docs.litellm.ai/docs/a2a_agent_permissions`
- `https://docs.litellm.ai/docs/a2a_cost_tracking`
- `https://docs.litellm.ai/docs/a2a_invoking_agents`
- `https://docs.litellm.ai/docs/a2a_iteration_budgets`
- `https://docs.litellm.ai/docs/adaptive_router`
- `https://docs.litellm.ai/docs/adding_provider/adding_guardrail_support`
- `https://docs.litellm.ai/docs/adding_provider/directory_structure`
- `https://docs.litellm.ai/docs/adding_provider/generic_guardrail_api`
- `https://docs.litellm.ai/docs/adding_provider/generic_prompt_management_api`
- `https://docs.litellm.ai/docs/adding_provider/new_rerank_provider`
- `https://docs.litellm.ai/docs/adding_provider/simple_guardrail_tutorial`
- `https://docs.litellm.ai/docs/agent_sdks`
- `https://docs.litellm.ai/docs/ai_tools`
- `https://docs.litellm.ai/docs/aiohttp_benchmarks`
- `https://docs.litellm.ai/docs/anthropic_count_tokens`
- `https://docs.litellm.ai/docs/anthropic_unified/`
- *(... +581 more — see url_inventory.json)*

## Summary

| Tier | Count |
|---|---|
| P0 (must extract) | 10 |
| P1 (high) | 20 |
| P2 (medium) | 25 |
| P3 (optional/low) | 611 |
| Excluded (low_priority) | 258 |

## Extraction Order Recommendation

1. Start with P0 pages — these define the config.yaml schema and proxy fundamentals.
2. Then P1 providers — extract the `anthropic/`, `openrouter/`, `openai/` prefix patterns.
3. Then P1 routing/auth/budgets — extract fallback, retry, and key management patterns.
4. P2 pages can be extracted in parallel with P1 if resources allow.
5. P3 pages are optional — extract only if specific gaps are identified.
