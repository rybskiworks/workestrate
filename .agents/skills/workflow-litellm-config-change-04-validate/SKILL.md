---
name: workflow-litellm-config-change-04-validate
description: |
  Use only for the validate phase of the LiteLLM config-change workflow. Run
  `validation-litellm-config-check` (the 9 schema checks against the on-disk
  index files; optionally the co-located `scripts/check_config.py`) on the
  edited `infra/litellm/config.yaml`. Do not use for scoping, design,
  implementation, or runtime verification.
allowed-tools: Read Grep Bash
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-change
  org.phase: validate
  org.phase_order: "04"
---

# Workflow: LiteLLM Config Change — 04 Validate

Run the static config validation gate on the edited config. This phase does
NOT modify the config; it cross-checks every key, provider prefix, fallback
target, env var, and `api_base` against the schema index files.

## Required source files to read first

- `docs/litellm/schemas/config-yaml.option-index.json` — AUTHORITATIVE for config key existence and `section`; flags `deprecated`, `requires_db`, `requires_redis`.
- `docs/litellm/schemas/provider-fields.index.json` — AUTHORITATIVE for provider `litellm_prefix`, `required_env_vars`, `api_base_behavior`, `caveats`.
- `docs/litellm/schemas/env-vars.index.json` — AUTHORITATIVE for env var names, `requires_db`, `deprecated`, `workestrator_used`.
- `docs/litellm/schemas/config-yaml.normalized.schema.md` — precedence rule, env-var syntax, in-memory mapping table, enum values.
- `infra/litellm/config.yaml` — the edited config to validate.

## Exact procedure

Run `validation-litellm-config-check`, which performs the 9 checks (a)–(i).
Record PASS/FAIL/WARN per check with the offending key/path. Optionally run the
co-located script at `.agents/skills/validation-litellm-config-check/scripts/check_config.py`.

1. **(a) Key existence + section.** Every key in the config must exist in `config-yaml.option-index.json` under the correct `section`. Unknown keys or wrong-section placement = FAIL.
2. **(b) Provider prefix.** Every `litellm_params.model` prefix must match a `litellm_prefix` in `provider-fields.index.json`. Unknown prefix = FAIL.
3. **(c) Fallback targets resolve.** Every `router_settings.fallbacks` source and target must exist as a `model_name` in `model_list`. Dangling target = FAIL.
4. **(d) Deprecated keys.** Flag any key with `deprecated: true` as WARN with its `replacement` (e.g. `set_verbose` → `LITELLM_LOG`).
5. **(e) DB-requiring keys (in-memory mode).** Flag every requires_db=true key as FAIL except `disable_spend_logs: true` (intentional — WARN).
6. **(f) Redis-requiring keys.** Flag every requires_redis=true key as FAIL in in-memory mode.
7. **(g) Env var resolution.** Every `os.environ/<VAR>` must resolve to a LiteLLM built-in (in `env-vars.index.json`) or a documented project-defined var. Unrecognized = WARN (or FAIL if clearly misspelled).
8. **(h) openai/ prefix api_base.** `openai/` entries: `api_base` ends in `/v1`, no appended endpoint path, `api_key` present. Violation = FAIL.
9. **(i) anthropic/ prefix api_base.** `anthropic/` entries: `api_base` does not pre-include `/v1/messages` (or `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` is set). Pre-included = WARN.

Aggregate: overall verdict PASS only if no FAIL; WARNs are acceptable but listed.

## Constraints to apply

(Constraints were applied in `02-design`/`03-implement`. This phase re-checks them statically via the 9 checks above, which encode `constraint-litellm-config-schema`, `constraint-litellm-provider-prefix`, `constraint-litellm-openai-compatible-api-base`, `constraint-litellm-anthropic-suffix`, `constraint-litellm-fallback-resolution`, `constraint-litellm-in-memory-no-db`, `constraint-litellm-deprecation-free`, and `constraint-litellm-secret-hygiene`.)

## Validations to run

- `validation-litellm-config-check` — the 9 checks (a)–(i). Optionally `.agents/skills/validation-litellm-config-check/scripts/check_config.py`.

## Handoff

Return the handoff YAML. Set `next_phase: 05-verify`.

```yaml
outcome: pass|fail|partial
files_touched: []
constraints_applied:
  - constraint-litellm-config-schema
  - constraint-litellm-provider-prefix
  - constraint-litellm-openai-compatible-api-base
  - constraint-litellm-anthropic-suffix
  - constraint-litellm-fallback-resolution
  - constraint-litellm-in-memory-no-db
  - constraint-litellm-deprecation-free
  - constraint-litellm-secret-hygiene
assumptions: []
risks: []
tests_run:
  - validation-litellm-config-check
tests_needed:
  - validation-litellm-startup (05-verify)
  - validation-litellm-smoke (05-verify)
  - validation-litellm-no-hardcoded-secrets (05-verify)
next_phase: 05-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```

## Anti-hallucination

Every check must cite the schema index file and field that defines the rule. If a check cannot be grounded in a corpus file, mark TODO rather than asserting.
