---
source_url: https://docs.litellm.ai/docs/proxy/reliability
canonical_url: https://docs.litellm.ai/docs/proxy/reliability
raw_source_url: https://raw.githubusercontent.com/BerriAI/litellm-docs/main/docs/proxy/reliability.md
title: Fallbacks
sidebar_section_path: proxy > routing
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
feature_area: routing
---
# Fallbacks

## Headings
- Fallbacks (overview)
- Quick Start
  - Setup fallbacks
  - Start Proxy
  - Test Fallbacks
  - Explanation
- Client Side Fallbacks
  - Control Fallback Prompts
- Content Policy Violation Fallback
- Context Window Exceeded Fallback
- Advanced
  - Fallbacks + Retries + Timeouts + Cooldowns
  - Fallback to Specific Model ID
  - Test Fallbacks!
  - Context Window Fallbacks (Pre-Call Checks + Fallbacks)
    - Custom max_input_tokens per deployment
  - Content Policy Fallbacks
  - Default Fallbacks
  - EU-Region Filtering (Pre-Call Checks)
  - Setting Fallbacks for Wildcard Models
  - Disable Fallbacks (Per Request/Key)

## Exact config keys found (full path, verbatim spelling)
- `router_settings.fallbacks` (router) — verbatim: "If a call fails after num_retries, fallback to another model group." Covers "all remaining errors - e.g. litellm.RateLimitError".
- `router_settings.context_window_fallbacks` (router) — "For litellm.ContextWindowExceededErrors - LiteLLM maps context window error messages across providers".
- `router_settings.content_policy_fallbacks` (router) — "For litellm.ContentPolicyViolationError - LiteLLM maps content policy violation errors across providers".
- `router_settings.default_fallbacks` (router) — verbatim: "in case a specific model group is misconfigured / bad"; "A model-specific fallbacks (e.g. `{"gpt-3.5-turbo-small": ["claude-opus"]}`) overrides default fallback."
- `router_settings.enable_pre_call_checks` (router) — verbatim: "`enable_pre_call_checks` is required for context-window enforcement. Without it, requests are sent to the provider regardless of input token count. Set `enable_pre_call_checks: true` in `router_settings` in your config."
- `litellm_settings.num_retries` (litellm) — verbatim example: `num_retries: 3 # retry call 3 times on each model_name (e.g. zephyr-beta)`.
- `litellm_settings.request_timeout` (litellm) — verbatim example: `request_timeout: 10 # raise Timeout error if call takes longer than 10s. Sets litellm.request_timeout`.
- `litellm_settings.allowed_fails` (litellm) — verbatim example: `allowed_fails: 3 # cooldown model if it fails > 1 call in a minute.`
- `litellm_settings.cooldown_time` (litellm) — verbatim example: `cooldown_time: 30 # how long to cooldown model if fails/min > allowed_fails`.
- `model_info.id` (model_list) — verbatim: "If all models in a group are in cooldown (e.g. rate limited), LiteLLM will fallback to the model with the specific model ID. This skips any cooldown check for the fallback model."
- `model_info.max_input_tokens` (model_list) — verbatim: "override the default context limit for a deployment"; requires BOTH `router_settings.enable_pre_call_checks: true` AND `model_info.max_input_tokens` on the deployment.
- `model_info.base_model` (model_list, azure-only) — "SET BASE MODEL" for azure deployments so pre-call checks know the context window.
- `litellm_params.region_name` (model_list) — for EU-region filtering pre-call checks; "LiteLLM can automatically infer region_name for Vertex AI, Bedrock, and IBM WatsonxAI based on your litellm params. For Azure, set `litellm.enable_preview = True`."
- request-body `mock_testing_fallbacks` (per-request, testing) — "Pass `mock_testing_fallbacks=true` in request body, to trigger fallbacks."
- request-body `mock_testing_content_policy_fallbacks` (per-request, testing) — triggers content-policy fallback path.
- request-body `mock_testing_context_window_fallbacks` (per-request, testing) — triggers context-window fallback path.
- request-body `disable_fallbacks` (per-request/per-key) — verbatim: "You can disable fallbacks per key by setting `disable_fallbacks: true` in your request body." Also settable in key metadata.
- request-body `fallbacks` (client-side, per-request) — verbatim: "Set fallbacks in the `.completion()` call for SDK and client-side for proxy." Supports per-model messages/temperature.

## Keys NOT documented on this page (documented elsewhere — config_settings)
The following router_settings keys are present in `schemas/config-yaml.option-index.json` but are NOT described on the /docs/proxy/reliability page:
- `router_settings.retry_policy` (and sub-keys `AuthenticationErrorRetries`, `TimeoutErrorRetries`, `RateLimitErrorRetries`, `ContentPolicyViolationErrorRetries`, `InternalServerErrorRetries`) — documented in config_settings Reference table; not on this page.
- `router_settings.allowed_fails_policy` (and sub-keys) — documented in config_settings; not on this page.
- `router_settings.retry_after` — documented in config_settings; not on this page.
- `router_settings.max_fallbacks` — documented in config_settings (default 5) and mentioned in p1-routing-fallback_management.md Notes; not described in prose on this page.
- `router_settings.disable_cooldowns` — documented in config_settings; not on this page.
- `router_settings.model_group_retry_policy` — documented in config_settings; not on this page.
- Redis requirement for cross-instance cooldown/fail state — NOT documented on this page. (The page does not mention Redis at all.)

## Exact YAML examples (verbatim — preserve indentation)
Quick-start fallbacks (verbatim):
```yaml
router_settings:
  fallbacks: [{"gpt-3.5-turbo": ["gpt-4"]}]
```

Advanced: Fallbacks + Retries + Timeouts + Cooldowns (verbatim):
```yaml
litellm_settings:
  num_retries: 3 # retry call 3 times on each model_name (e.g. zephyr-beta)
  request_timeout: 10 # raise Timeout error if call takes longer than 10s. Sets litellm.request_timeout 
  fallbacks: [{"zephyr-beta": ["gpt-3.5-turbo"]}] # fallback to gpt-3.5-turbo if call fails num_retries 
  allowed_fails: 3 # cooldown model if it fails > 1 call in a minute. 
  cooldown_time: 30 # how long to cooldown model if fails/min > allowed_fails
```

Pre-call checks + max_input_tokens (verbatim):
```yaml
router_settings:
  enable_pre_call_checks: true  # Required for enforcement

model_list:
  - model_name: gpt-4o
    litellm_params:
      model: openai/gpt-4o
      api_key: os.environ/OPENAI_API_KEY
    model_info:
      max_input_tokens: 10  # Override: reject prompts > 10 tokens
```

Default fallbacks (verbatim):
```yaml
litellm_settings:
  default_fallbacks: ["claude-opus"]
```

## Exact environment variables
- none documented on this page (note: `litellm.enable_preview = True` is referenced as a Python SDK flag for Azure region inference, not an env var)

## Exact endpoint paths / API routes (runtime + management)
- `POST /chat/completions` — supports `mock_testing_fallbacks`, `mock_testing_content_policy_fallbacks`, `mock_testing_context_window_fallbacks`, `disable_fallbacks`, client-side `fallbacks` body params
- `POST /key/generate` — supports `metadata.disable_fallbacks: true` (per-key fallback disable)

## Requirements
- database: not documented on this page
- redis: not documented on this page (page does not mention Redis; cross-instance cooldown/state sharing is NOT described here)
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- none documented on this page

## Caveats / pitfalls
- Fallback ordering (verbatim): "Fallbacks are done in-order - ["gpt-3.5-turbo, "gpt-4", "gpt-4-32k"], will do 'gpt-3.5-turbo' first, then 'gpt-4', etc."
- Fallback trigger condition (verbatim): "If a call fails after num_retries, fallback to another model group." — i.e. fallbacks fire AFTER retries are exhausted, not before.
- `enable_pre_call_checks` is REQUIRED for context-window enforcement (verbatim admonition): "Without it, requests are sent to the provider regardless of input token count."
- `max_input_tokens` enforcement requires BOTH `enable_pre_call_checks: true` AND `model_info.max_input_tokens` (verbatim: "Both of the following are required").
- `default_fallbacks` is overridden by model-specific fallbacks (verbatim: "A model-specific fallbacks ... overrides default fallback.").
- Specific-model-ID fallback skips cooldown checks (verbatim: "This skips any cooldown check for the fallback model.").
- `request_timeout` (litellm_settings) is distinct from `router_settings.timeout`: the page notes it "Sets litellm.request_timeout" — a litellm-level timeout, separate from the router-level `timeout` (documented on /docs/proxy/timeout).

## Related links
- /docs/proxy/load_balancing (Quick Start — load balancing)
- /docs/proxy/timeout (Timeouts)
- /docs/proxy/fallback_management (dynamic fallback management endpoints — DB-required)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- The real config uses static `router_settings.fallbacks` (coding-tier chains), `num_retries: 2`, `allowed_fails: 3`, `cooldown_time: 60`, and `retry_policy` (TimeoutErrorRetries/RateLimitErrorRetries/InternalServerErrorRetries). All of these work without DB/Redis. `enable_pre_call_checks` is NOT set, so context-window pre-checks do not run. `default_fallbacks`, `request_timeout`, and per-request `disable_fallbacks` are unset. The `mock_testing_*` test params are available at runtime. Cross-instance cooldown sharing would require Redis (not configured).

## Confidence / uncertainty notes
- high confidence on fallback types, ordering, `num_retries`/`request_timeout`/`allowed_fails`/`cooldown_time` semantics, `enable_pre_call_checks` requirement, `default_fallbacks` override rule, and `disable_fallbacks` (all verbatim). The `retry_policy`, `allowed_fails_policy`, `retry_after`, `max_fallbacks`, `disable_cooldowns`, and Redis cross-instance behavior are NOT on this page — cite config_settings / p0-config_settings.md for those. The `request_timeout` vs `router_settings.timeout` distinction is inferred from the page note "Sets litellm.request_timeout" plus the separate /docs/proxy/timeout page; both are verbatim-anchored.
