---
name: workflow-litellm-config-review-02-analyze
description: |
  Use only for the analyze phase of the LiteLLM config-review workflow. Categorize
  the diff by dimension and load the regular skill that matches. Do not use for
  scoping, running checks, manual review, or issuing a verdict.
allowed-tools: Read Bash(git:*) Bash(python:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-review
  org.phase: analyze
  org.phase_order: "02"
---

# Phase 02: analyze (LiteLLM config review)

## Phase purpose

Categorize the `config.yaml` diff by dimension and load the regular skill that
matches the categories, so phase 04-review has the right skills and docs ready.

## Steps to perform

1. Categorize the diff by dimension. Record the dimensions in `risks`:
   - **provider prefix** — any `litellm_params.model` value changed/added; check
     the prefix against `docs/litellm/schemas/provider-fields.index.json`
     (`litellm_prefix`).
   - **api_base rules** — `openai/`-prefix entries (must end in `/v1`, no
     appended endpoint path, must have `api_key`) and `anthropic/`-prefix entries
     (auto-append `/v1/messages` unless `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX`).
   - **routing/fallback** — `router_settings.fallbacks` /
     `context_window_fallbacks` / `content_policy_fallbacks` / `default_fallbacks`
     edits; source and target `model_name`s must exist in `model_list`.
   - **secret hygiene** — `api_key`, `master_key`, `credential_values`,
     `custom_tokenizer.auth_token` values; must use `os.environ/<VAR>`, never
     hardcoded literals.
   - **DB/Redis keys** — `database_url`, `store_model_in_db`, `disable_spend_*`,
     `redis_*`, `cache_params` with redis type; flagged in in-memory mode.
   - **deprecations** — `set_verbose` (→ `LITELLM_LOG` / `--debug`) and any key
     with `deprecated: true` in `config-yaml.option-index.json`.
2. Load `litellm-config-anatomy` — the top-level structure, precedence rule
   (`router_settings` overrides `litellm_settings`), env-var resolution syntax,
   and section/key placement reference.
3. Conditionally load by diff type:
   - If the diff touches `model_list` provider prefixes → load `litellm-providers`
     (provider prefix catalog, `required_env_vars`, `api_base_behavior`).
   - If the diff touches `openai/`-prefix `api_base` → load
     `litellm-openai-compatible` (the `/v1` postfix rule, no appended endpoint
     path, `api_key` requirement, `hosted_vllm/` alternative).
   - If the diff touches `router_settings.fallbacks` or other routing keys → load
     `litellm-routing-fallbacks` (fallback resolution, overlap-key precedence).
4. Apply `constraint-litellm-config-schema`: categorization is grounded in the
   schema index; do not expand scope to untouched config.

## Docs to consult

- `docs/litellm/schemas/config-yaml.option-index.json` — key + `section` index.
- `docs/litellm/schemas/provider-fields.index.json` — provider `litellm_prefix`,
  `required_env_vars`, `api_base_behavior`, caveats.
- `docs/litellm/schemas/config-yaml.normalized.schema.md` — precedence rule,
  env-var syntax, overlap keys.
- `docs/litellm/config/router-settings.md` — routing/fallback semantics.
- `docs/litellm/config/model-list.md` — `model_name` alias + `litellm_params`.

## Operational skills to load

- `litellm-config-anatomy`
- `litellm-providers` (if provider-prefix changes present)
- `litellm-openai-compatible` (if `openai/`-prefix `api_base` changes present)
- `litellm-routing-fallbacks` (if routing/fallback changes present)

## Constraints to apply

- `constraint-litellm-config-schema` — categorization is grounded in the schema
  index; review only the diff.

## Validations to run

None — validations run in phase 03 (workflow-litellm-config-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-litellm-config-review-00-orchestration`. Set:

- `outcome`: `pass` if categorization completed and skills loaded; `partial` if
  a conditional skill was deliberately not loaded (record why in `assumptions`).
- `constraints_applied`: `constraint-litellm-config-schema`.
- `risks`: the dimensions the diff touches.
- `assumptions`: any conditional-load decisions and their rationale.
- `next_phase`: `03-check`.
- `next_workflow`: `null`.
