---
source_url: https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies
canonical_url: https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies
title: "[Beta] Guardrail Policies"
sidebar_section_path: proxy > guardrails (Policies)
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: guardrails
---
# [Beta] Guardrail Policies

## Headings
- Why use policies?
- Quick Start
- Add guardrails for a specific team
- Remove guardrails for a specific team
- Inheritance
- Model Conditions
- Attachments
- Test Policy Matching
- Policy Flow Builder
- Config Reference
  - policies
  - policy_attachments
  - Response Headers
- How it works

## Exact config keys found (full path, verbatim spelling)
- `guardrails[].guardrail_name` (guardrails)
- `guardrails[].litellm_params.guardrail` (guardrails)
- `guardrails[].litellm_params.mode` (guardrails)
- `guardrails[].litellm_params.api_key` (guardrails)
- `policies.<policy-name>.description` (policies)
- `policies.<policy-name>.inherit` (policies)
- `policies.<policy-name>.guardrails.add` (policies) — list of guardrail names
- `policies.<policy-name>.guardrails.remove` (policies) — list of guardrail names
- `policies.<policy-name>.condition.model` (policies) — regex or exact match list
- `policies.<policy-name>.pipeline` (policies) — optional; see Policy Flow Builder
- `policy_attachments[].policy` (policy_attachments) — Required; name of the policy to attach
- `policy_attachments[].scope` (policy_attachments) — Use `"*"` to apply globally
- `policy_attachments[].teams` (policy_attachments) — list of team aliases
- `policy_attachments[].keys` (policy_attachments) — list of key alias patterns
- `policy_attachments[].models` (policy_attachments) — list
- `policy_attachments[].tags` (policy_attachments) — list (supports `*` wildcard)

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Quick Start
model_list:
  - model_name: gpt-4
    litellm_params:
      model: openai/gpt-4

# 1. Define your guardrails
guardrails:
  - guardrail_name: pii_masking
    litellm_params:
      guardrail: presidio
      mode: pre_call

  - guardrail_name: prompt_injection
    litellm_params:
      guardrail: lakera
      mode: pre_call
      api_key: os.environ/LAKERA_API_KEY

# 2. Create a policy
policies:
  my-policy:
    guardrails:
      add:
        - pii_masking
        - prompt_injection

# 3. Attach the policy
policy_attachments:
  - policy: my-policy
    scope: "*"  # apply to all requests
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies)

```yaml
# Add guardrails for a specific team
policies:
  global-baseline:
    guardrails:
      add:
        - pii_masking
  finance-team-policy:
    inherit: global-baseline
    guardrails:
      add:
        - strict_compliance_check
        - audit_logger

policy_attachments:
  - policy: global-baseline
    scope: "*"

  - policy: finance-team-policy
    teams:
      - finance  # team alias from /team/new
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies)

```yaml
# Inheritance
policies:
  base:
    guardrails:
      add:
        - pii_masking
        - toxicity_filter

  strict:
    inherit: base
    guardrails:
      add:
        - prompt_injection

  relaxed:
    inherit: base
    guardrails:
      remove:
        - toxicity_filter
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies)

```yaml
# Model Conditions
policies:
  gpt4-safety:
    guardrails:
      add:
        - strict_content_filter
    condition:
      model: "gpt-4.*"  # regex - matches gpt-4, gpt-4-turbo, gpt-4o

  bedrock-compliance:
    guardrails:
      add:
        - audit_logger
    condition:
      model:  # exact match list
        - bedrock/claude-3
        - bedrock/claude-2
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies)

```yaml
# Attachments - Tag-based
policy_attachments:
  - policy: hipaa-compliance
    tags:
      - "healthcare"
      - "health-*"  # wildcard - matches health-team, health-dev, etc.
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies)

```yaml
# Config Reference - policies
policies:
  <policy-name>:
    description: ...
    inherit: ...
    guardrails:
      add: [...]
      remove: [...]
    condition:
      model: ...
    pipeline: ...  # optional; see Policy Flow Builder
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies)

```yaml
# Config Reference - policy_attachments
policy_attachments:
  - policy: ...
    scope: ...
    teams: [...]
    keys: [...]
    models: [...]
    tags: [...]
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/guardrail_policies)

## Exact environment variables
- `LAKERA_API_KEY` — Lakera guardrail API key (Quick Start example)

## Exact endpoint paths / API routes (runtime + management)
- `POST /policies/resolve` — Debug which policies and guardrails apply for a given context. Body: `{"tags": ["healthcare"], "model": "gpt-4"}`
- `POST /team/new` — referenced for team alias
- `POST /key/generate` — referenced for key alias

## Exact CLI commands
- none documented (only curl examples)

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: required for team/key-based policy attachments — verbatim: "✨ Enterprise only feature for team/key-based policy attachments." (appears in "Add guardrails for a specific team" and "Remove guardrails for a specific team")
- admin_ui: not documented on this page

## Deprecations
- none documented (page itself is marked `[Beta]` in the title)

## Caveats / pitfalls
- Page title is "[Beta] Guardrail Policies"
- "`scope`: Use `\"*\"` to apply globally."
- "`condition.model`: Optional. Only apply when model matches. Supports regex."
- "`policy` (**Required**) Name of the policy to attach."
- Tags are read from key and team `metadata.tags`. A key created with `metadata: {"tags": ["healthcare"]}` would match the attachment.
- `tags` / `keys` / `teams` all support `*` wildcard

## Related links
- /docs/proxy/guardrails/quick_start
- /docs/proxy/guardrails/policy_flow_builder
- /docs/proxy/guardrails/policy_templates
- /docs/proxy/guardrails/policy_tags
- /docs/adding_provider/adding_guardrail_support
- /docs/enterprise

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- Guardrail policies (`policies:` + `policy_attachments:` in config.yaml) work WITHOUT a database — they're static config. The workestrate can use `scope: "*"` (global) policies and `condition.model` (model-based) policies in-memory. Team-based and key-based attachments are Enterprise (unavailable). Tag-based attachments read from `metadata.tags` on keys/teams — without virtual keys/teams (no DB), tag-based attachments are moot. `POST /policies/resolve` works without DB (resolves static config). The `inherit` mechanism and `guardrails.add`/`guardrails.remove` work with static config. This is a [Beta] feature.

## Confidence / uncertainty notes
- high confidence on policy config format (verbatim YAML). DB requirement is "not documented" — policies are static config (inferred they work without DB). Enterprise requirement for team/key attachments is verbatim. Page is [Beta] — config format may change.
