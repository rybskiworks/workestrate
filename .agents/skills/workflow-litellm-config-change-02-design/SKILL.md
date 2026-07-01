---
name: workflow-litellm-config-change-02-design
description: |
  Use only for the design phase of the LiteLLM config-change workflow. Decide
  the exact keys/section, provider prefix + model string, `api_base` rules, env
  vars, and fallback wiring before implementation. Do not use for scoping,
  implementation, validation, or verification.
allowed-tools: Read Grep
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-change
  org.phase: design
  org.phase_order: "02"
---

# Workflow: LiteLLM Config Change — 02 Design

Decide the exact config shape so `03-implement` can edit without guessing. This
phase is read-only: it reads schemas and decides; it does not edit.

## Required source files to read first

- `docs/litellm/schemas/config-yaml.option-index.json` — AUTHORITATIVE for config key existence and `section` placement; also flags `deprecated`, `requires_db`, `requires_redis`.
- `docs/litellm/schemas/provider-fields.index.json` — AUTHORITATIVE for provider `litellm_prefix`, `required_env_vars`, `api_base_behavior`, `caveats`.
- `docs/litellm/schemas/env-vars.index.json` — AUTHORITATIVE for env var names, `requires_db`, `deprecated`, `workestrator_used`.
- `docs/litellm/schemas/config-yaml.normalized.schema.md` — precedence rule, env-var syntax, in-memory mapping table, enum values.
- Load regular skill `litellm-config-anatomy` for the `litellm_params` key set.

## Exact procedure

1. **Decide the exact keys and section.** For each intended key, confirm it exists in `config-yaml.option-index.json` with a matching `section`. A key valid under `router_settings` may be invalid under `litellm_settings`. Record the exact key + section + value.
2. **Decide the provider prefix + model string.** For `model_list` entries, the `litellm_params.model` value uses a prefix from `provider-fields.index.json` (`litellm_prefix`). Known valid prefixes: `openai/`, `anthropic/`, `openrouter/`, `moonshot/`, `minimax/`, `zai/`, `hosted_vllm/`, `ollama/`, `ollama_chat/`, `litellm_proxy/`, `inception/`. Record the prefix and the full model string.
3. **Decide `api_base` rules.**
   - `openai/`-prefixed (OpenAI-compatible): `api_base` MUST include the `/v1` postfix (e.g. `https://api.neuralwatt.com/v1`); MUST NOT append an endpoint path like `/v1/embedding` (the openai-client adds them); `api_key` MUST be present.
   - `anthropic/`-prefixed: LiteLLM auto-appends `/v1/messages` to `api_base` unless `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true`. Do NOT pre-include `/v1/messages` in `api_base` (would double-append). Example: `https://api.kimi.com/coding` resolves to `…/coding/v1/messages`.
   - `openrouter/`-prefixed: nested format `openrouter/<provider>/<model>` (e.g. `openrouter/z-ai/glm-5.1`); no `api_base` (uses OpenRouter default).
4. **Decide env vars.** All secrets use `os.environ/<VAR>` (runs `os.getenv("<VAR>")`). LiteLLM built-ins (per `env-vars.index.json`): `LITELLM_MASTER_KEY`, `OPENROUTER_API_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `MOONSHOT_API_KEY`, `MINIMAX_API_KEY`, `ZAI_API_KEY`, `HOSTED_VLLM_API_KEY`, `INCEPTION_API_KEY`. Project-defined vars in the real config (NOT LiteLLM built-ins): `KIMI_CODE_API_KEY`, `MINIMAX_CODING_API_KEY`, `NEURALWATT_API_KEY`. Record the exact env var name per secret.
5. **Decide fallback wiring.** Fallbacks live in `router_settings.fallbacks` (NOT `model_list`). Each entry is a dict `{<source_model_name>: [<fallback_model_name>, ...]}`. Both the source and every target MUST exist as a `model_name` in `model_list`. Example: `coding: [coding-fallback]` requires both `coding` and `coding-fallback` defined.
6. **Apply the precedence rule.** Overlap keys (`num_retries`, `timeout`, `fallbacks`, `context_window_fallbacks`, `content_policy_fallbacks`, `default_fallbacks`, `set_verbose` (deprecated), `cache`/`cache_responses`) appear under BOTH `litellm_settings` and `router_settings`; `router_settings` wins. Do not duplicate an overlap key across both sections unintentionally.

## Constraints to apply

- `constraint-litellm-config-schema` — every key traces to `config-yaml.option-index.json` (`key` + `section`).
- `constraint-litellm-provider-prefix` — every `litellm_params.model` prefix matches a `litellm_prefix` in `provider-fields.index.json`.
- `constraint-litellm-openai-compatible-api-base` — `openai/` entries: `api_base` ends in `/v1`, no appended endpoint path, `api_key` present.
- `constraint-litellm-anthropic-suffix` — `anthropic/` entries: `api_base` does not pre-include `/v1/messages` (or `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` is set).
- `constraint-litellm-fallback-resolution` — every `router_settings.fallbacks` source and target exists as a `model_name` in `model_list`.

## Validations to run

None (read-only phase).

## Handoff

Return the handoff YAML. Set `next_phase: 03-implement`.

```yaml
outcome: pass|fail|partial
files_touched: []
constraints_applied:
  - constraint-litellm-config-schema
  - constraint-litellm-provider-prefix
  - constraint-litellm-openai-compatible-api-base
  - constraint-litellm-anthropic-suffix
  - constraint-litellm-fallback-resolution
assumptions: []
risks: []
tests_run: []
tests_needed: []
next_phase: 03-implement
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```

## Anti-hallucination

Every config key must trace to `config-yaml.option-index.json` (`key` + `section`). Every provider prefix must trace to `provider-fields.index.json` (`litellm_prefix`). Every env var must trace to `env-vars.index.json` or be explicitly marked project-defined. Mark any inferred fact.
