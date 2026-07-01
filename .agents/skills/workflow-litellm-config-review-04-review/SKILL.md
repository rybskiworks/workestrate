---
name: workflow-litellm-config-review-04-review
description: |
  Use only for the review phase of the LiteLLM config-review workflow. Perform the
  human-judgment review against the 8 constraints and flag violations with
  severity. Do not use for scoping, analysis, running checks, or issuing a verdict.
allowed-tools: Read Bash(git:*) Bash(python:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-review
  org.phase: review
  org.phase_order: "04"
---

# Phase 04: review (LiteLLM config review)

## Phase purpose

Perform the human-judgment review against the 8 LiteLLM config constraints. Walk
each constraint's checklist explicitly and cite the schema index field or doc
rule violated rather than paraphrasing.

## Steps to perform

1. **`constraint-litellm-config-schema`** — every touched key exists in
   `docs/litellm/schemas/config-yaml.option-index.json` under the correct
   `section`; verify `model_list[].model_name` + `litellm_params.model` shape and
   that overlap keys (`num_retries`, `timeout`, `fallbacks`, `set_verbose`,
   `cache`) respect the `router_settings`-overrides-`litellm_settings` precedence.
2. **`constraint-litellm-in-memory-no-db`** — no `requires_db: true` key is set
   in in-memory mode (`database_url`, `store_model_in_db`, `disable_spend_*`,
   `custom_key_generate`, `ui_access_mode`, the `database_*` family) except the
   intentional compensating control; no `requires_redis: true` key is set
   (`enable_redis_auth_cache`, redis `cache_params`, `router_settings.redis_*`).
3. **`constraint-litellm-secret-hygiene`** — `api_key`, `master_key`,
   `credential_values`, `custom_tokenizer.auth_token` use `os.environ/<VAR>`;
   `master_key` resolves to `os.environ/LITELLM_MASTER_KEY`; no hardcoded secret
   literals.
4. **`constraint-litellm-provider-prefix`** — every `litellm_params.model` prefix
   matches a `litellm_prefix` in `provider-fields.index.json` (`openai/`,
   `anthropic/`, `openrouter/`, `moonshot/`, `minimax/`, `zai/`, `hosted_vllm/`,
   `ollama/`, `ollama_chat/`, `litellm_proxy/`, `inception/`); the deprecated
   `vllm/` prefix is flagged.
5. **`constraint-litellm-openai-compatible-api-base`** — for every `openai/`-
   prefix entry: `api_base` ends in `/v1`, does NOT append an endpoint path
   (`/v1/embedding`, `/v1/chat/completions`), and `api_key` is present (else
   recommend `hosted_vllm/`).
6. **`constraint-litellm-anthropic-suffix`** — for every `anthropic/`-prefix
   entry: `api_base` does not pre-include `/v1/messages` (LiteLLM auto-appends it)
   unless `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` is set.
7. **`constraint-litellm-fallback-resolution`** — every
   `router_settings.fallbacks` (and `context_window_fallbacks` /
   `content_policy_fallbacks` / `default_fallbacks`) source and target
   `model_name` exists in `model_list`; no dangling fallback targets.
8. **`constraint-litellm-deprecation-free`** — no deprecated key is used
   (`set_verbose` → `LITELLM_LOG` / `--debug`; deprecated env vars
   `LITELLM_SET_VERBOSE`, `SET_VERBOSE`); each is flagged with its replacement.
9. Walk each constraint's checklist explicitly; record pass/fail/N-A per item.
   **Cite the schema index field or doc rule violated; do not paraphrase.**

## Docs to consult

- `docs/litellm/schemas/config-yaml.option-index.json`
- `docs/litellm/schemas/provider-fields.index.json`
- `docs/litellm/schemas/env-vars.index.json`
- `docs/litellm/schemas/config-yaml.normalized.schema.md`
- `docs/litellm/config/config-yaml-overview.md`
- `docs/litellm/config/general-settings.md`
- `docs/litellm/config/router-settings.md`
- `docs/litellm/config/environment-variables.md`

## Operational skills to load

- `litellm-config-anatomy`
- `litellm-providers` (if provider-prefix changes present)
- `litellm-openai-compatible` (if `openai/`-prefix `api_base` changes present)
- `litellm-routing-fallbacks` (if routing/fallback changes present)

## Constraints to apply

- `constraint-litellm-config-schema`
- `constraint-litellm-in-memory-no-db`
- `constraint-litellm-secret-hygiene`
- `constraint-litellm-provider-prefix`
- `constraint-litellm-openai-compatible-api-base`
- `constraint-litellm-anthropic-suffix`
- `constraint-litellm-fallback-resolution`
- `constraint-litellm-deprecation-free`

## Validations to run

None — this is a manual review phase that uses the 8-constraint checklists.
Automated validations ran in phase 03 (workflow-litellm-config-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-litellm-config-review-00-orchestration`. Set:

- `outcome`: `pass` if no findings; `partial` if findings exist but none are
  blockers; `fail` if a blocker-level finding was identified.
- `constraints_applied`: all 8 constraints listed above that applied.
- `risks`: each finding with file:line/key-path, the constraint and schema index
  field or doc rule violated (cited), and a concrete suggested fix. Severity
  classification happens in phase 05-verdict; here record the raw findings.
- `assumptions`: any N-A constraint and why.
- `next_phase`: `05-verdict`.
- `next_workflow`: `null`.
