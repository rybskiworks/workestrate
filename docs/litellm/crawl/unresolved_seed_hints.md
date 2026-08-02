# Unresolved Seed Hints

**Generated**: 2026-06-27

Seed hints from the task prompt that could NOT be resolved to an actual page, or that
required reinterpretation to match the actual docs structure.

## Resolved Hints (confirmed to exist)

The following seed hints from the prompt were confirmed to exist as actual pages
in the sitemap or sidebar:

| Seed Hint | Resolved URL | Status |
|---|---|---|
| model_list | /docs/proxy/configs (model_list section) + /docs/proxy/config_settings | resolved |
| general_settings | /docs/proxy/config_settings#general_settings | resolved |
| router_settings | /docs/proxy/config_settings#router_settings | resolved |
| litellm_settings | /docs/proxy/config_settings#litellm_settings | resolved |
| master_key | /docs/proxy/config_settings#general_settings (master_key field) | resolved |
| completion_model | /docs/proxy/config_settings#general_settings (completion_model field) | resolved |
| disable_spend_logs | /docs/proxy/config_settings#general_settings | resolved |
| drop_params | /docs/completion/drop_params + /docs/proxy/config_settings#litellm_settings | resolved |
| request_timeout | /docs/proxy/config_settings#litellm_settings | resolved |
| force_ipv4 | /docs/proxy/config_settings#litellm_settings | resolved |
| fallbacks | /docs/proxy/fallback_management + /docs/routing | resolved |
| num_retries | /docs/proxy/config_settings#router_settings | resolved |
| timeout | /docs/proxy/timeout | resolved |
| stream_timeout | /docs/proxy/config_settings#router_settings | resolved |
| allowed_fails | /docs/proxy/config_settings#router_settings | resolved |
| cooldown_time | /docs/proxy/config_settings#router_settings | resolved |
| retry_policy | /docs/proxy/config_settings#router_settings | resolved |
| anthropic/ prefix | /docs/providers/anthropic | resolved |
| openrouter/ prefix | /docs/providers/openrouter | resolved |
| openai/ prefix (with api_base) | /docs/providers/openai + /docs/providers/openai_compatible | resolved |
| OpenAI provider | /docs/providers/openai | resolved |
| OpenAI-compatible endpoints | /docs/providers/openai_compatible (also /docs/openai_compatible) | resolved |
| OpenRouter | /docs/providers/openrouter | resolved |
| MiniMax | /docs/providers/minimax | resolved |
| Moonshot/Kimi | /docs/providers/moonshot | resolved |
| Inception | /docs/providers/inception | resolved |
| Z.AI/Zhipu/GLM | /docs/providers/zai | resolved |
| Neuralwatt | NOT found as a named provider page (see below) | partially_resolved |
| vLLM | /docs/providers/vllm | resolved |
| Ollama | /docs/providers/ollama | resolved |
| LiteLLM Proxy provider | /docs/providers/litellm_proxy | resolved |
| config.yaml structure | /docs/proxy/configs + /docs/proxy/config_settings | resolved |
| supported endpoints | /docs/supported_endpoints | resolved |
| MCP/Skills/Agent gateway | /docs/mcp + /docs/skills + /docs/a2a | resolved |
| auth/virtual keys/teams | /docs/proxy/virtual_keys + /docs/proxy/users | resolved |
| budgets/rate limits | /docs/proxy/users (Budgets + Rate Limits category) | resolved |
| routing/fallbacks | /docs/routing-load-balancing + /docs/proxy/fallback_management | resolved |
| caching | /docs/proxy/caching | resolved |
| logging/callbacks | /docs/proxy/logging + /docs/proxy/dynamic_logging | resolved |
| guardrails | /docs/proxy/guardrails/quick_start | resolved |
| deployment | /docs/proxy/deploy + /docs/proxy/docker_quick_start | resolved |
| Admin UI/CLI | /docs/proxy/ui + /docs/proxy/management_cli | resolved |
| DB/Redis/Enterprise | /docs/proxy/db_info + /docs/enterprise | resolved |

## Unresolved or Reinterpreted Hints

| Seed Hint | Issue | Resolution |
|---|---|---|
| Neuralwatt provider | No dedicated provider page exists in the LiteLLM docs for "Neuralwatt". The config uses `openai/neuralwatt` with `api_base: https://api.neuralwatt.com/v1`, which means it's an OpenAI-compatible endpoint, not a built-in provider. | Use /docs/providers/openai_compatible as the reference page. Neuralwatt is a custom OpenAI-compatible endpoint, not a named LiteLLM provider. |
| docs/llm_provider/*.md (github) | The `docs/llm_provider/` directory does NOT exist in the litellm-docs repo (HTTP 404). Provider docs are flat at `docs/providers/*.md` and top-level `docs/*.md`. | Use /docs/providers/ as the provider index. |
| Kimi (as provider name) | "Kimi" is the product name; the LiteLLM provider is "Moonshot AI" at /docs/providers/moonshot. The config uses `anthropic/kimi-for-coding` with a custom api_base. | Use /docs/providers/moonshot + /docs/providers/anthropic for the anthropic/ prefix pattern. |
| config validation | No dedicated "config validation" page found. Config validation is covered within /docs/proxy/config_settings. | Use /docs/proxy/config_settings. |
| env vars | No dedicated "environment variables" page found as a standalone URL. Env vars are documented as a section within /docs/proxy/config_settings. | Use /docs/proxy/config_settings#environment-variables. |

## Notes

- All provider names in the prompt were checked against the sitemap's /docs/providers/* URLs.
- "Neuralwatt" is the only provider name from the config that does not have a dedicated docs page —
  it is an OpenAI-compatible custom endpoint, not a built-in LiteLLM provider.
- The `anthropic/` prefix used for Kimi and MiniMax in the config is documented under
  /docs/providers/anthropic (the Anthropic Messages API protocol).
