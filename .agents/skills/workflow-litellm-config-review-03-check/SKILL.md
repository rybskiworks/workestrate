---
name: workflow-litellm-config-review-03-check
description: |
  Use only for the check phase of the LiteLLM config-review workflow. Run the
  automated config checks (validation-litellm-config-check checks a–i +
  validation-litellm-no-hardcoded-secrets) and record pass/fail. Do not use for
  scoping, analysis, manual review, or issuing a verdict.
allowed-tools: Read Bash(git:*) Bash(python:*)
metadata:
  org.kind: workflow-phase
  org.workflow: litellm-config-review
  org.phase: check
  org.phase_order: "03"
---

# Phase 03: check (LiteLLM config review)

## Phase purpose

Run the automated config checks against the `config.yaml` under review and
record their pass/fail status. A clean config check is the baseline; a FAIL on
any check (a)–(i) or a hardcoded-secret finding is a blocker that stops the
workflow.

## Steps to perform

1. Run `validation-litellm-config-check`. This validation runs the checks (a)–(i)
   against the config, cross-referencing `docs/litellm/schemas/config-yaml.option-index.json`,
   `provider-fields.index.json`, and `env-vars.index.json`:
   - **(a) Key existence + section** — every key exists in the option index under
     the correct `section`; unknown keys are FAIL.
   - **(b) Provider prefix** — every `litellm_params.model` prefix matches a
     `litellm_prefix` in `provider-fields.index.json`.
   - **(c) Fallback targets resolve** — every `router_settings.fallbacks` source
     and target exists as a `model_name` in `model_list`.
   - **(d) Deprecated keys** — flag any `deprecated: true` key (e.g.
     `set_verbose`) as WARN with its replacement.
   - **(e) DB-requiring keys (in-memory mode)** — flag `requires_db: true` keys
     (`database_url`, `store_model_in_db`, `disable_spend_*`, etc.) as FAIL
     unless the intentional compensating control.
   - **(f) Redis-requiring keys** — flag `requires_redis: true` keys
     (`enable_redis_auth_cache`, redis `cache_params`, `router_settings.redis_*`)
     as FAIL in in-memory mode.
   - **(g) Env var resolution** — every `os.environ/<VAR>` resolves to a LiteLLM
     built-in or a documented project-defined var.
   - **(h) openai/ prefix api_base** — `api_base` ends in `/v1`, no appended
     endpoint path, and `api_key` is present.
   - **(i) anthropic/ prefix api_base** — `api_base` does not pre-include
     `/v1/messages` (or `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX` is set).
2. Run `validation-litellm-no-hardcoded-secrets`. This validation scans the
   config for hardcoded secret literals in secret-bearing keys (`api_key`,
   `master_key`, `credential_values`, `custom_tokenizer.auth_token`) — any value
   not of the form `os.environ/<VAR>` is FAIL. Confirm `master_key` resolves to
   `os.environ/LITELLM_MASTER_KEY`.
3. Apply `constraint-litellm-config-schema`: check failures are reported only
   for the diff under review; do not request fixes for pre-existing failures in
   untouched config (record them as follow-ups).
4. If any check returns FAIL, report a blocker, set `outcome: fail`, and stop —
   do not proceed to 04-review against config that fails automated checks.

## Docs to consult

- `docs/litellm/schemas/config-yaml.option-index.json`
- `docs/litellm/schemas/provider-fields.index.json`
- `docs/litellm/schemas/env-vars.index.json`
- `docs/litellm/schemas/config-yaml.normalized.schema.md`
- `docs/litellm/config/config-validation.md`

## Operational skills to load

None new — the validation skills encapsulate the check procedure. (Regular
skills were loaded in 02-analyze.)

## Constraints to apply

- `constraint-litellm-config-schema` — check failures are reported only for the
  diff under review; do not request fixes for pre-existing failures in untouched
  config (record them as follow-ups).

## Validations to run

- `validation-litellm-config-check` — checks (a)–(i).
- `validation-litellm-no-hardcoded-secrets` — hardcoded-secret scan.

## Handoff output

Return the handoff YAML block per the schema in
`workflow-litellm-config-review-00-orchestration`. Set:

- `outcome`: `pass` if both validations are green (WARNs acceptable); `fail` if
  any check returned FAIL; `partial` if a validation was skipped with attached CI
  evidence.
- `constraints_applied`: `constraint-litellm-config-schema`.
- `tests_run`: one entry per validation with result, e.g.
  `validation-litellm-config-check: pass (0 FAIL, 1 WARN: set_verbose deprecated)`,
  `validation-litellm-no-hardcoded-secrets: pass`.
- `blockers`: any FAIL check, with the offending key/path and the check letter.
- `assumptions`: any skipped validation and the attached CI evidence.
- `next_phase`: `04-review` (or `null` if a blocker stopped the workflow).
- `next_workflow`: `null`.
