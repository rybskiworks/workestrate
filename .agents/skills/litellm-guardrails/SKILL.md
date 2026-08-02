---
name: litellm-guardrails
description: |
  Operational reference for LiteLLM proxy guardrails: static guardrails config
  (no DB), verbatim guardrail provider names, modes (pre_call/post_call/
  during_call/logging_only), default_on, model-level guardrails via
  litellm_params.guardrails, and guardrail policies (Beta). Per-API-key
  guardrail control is Enterprise. Load when configuring or reviewing
  guardrails in config.yaml. Distilled from docs/litellm/; does NOT re-teach
  provider-specific guardrail internals.
---

# LiteLLM Guardrails

Distilled operational guidance for LiteLLM proxy guardrails (PII / content
filtering pre/post/during call). Full detail lives in:

- `docs/litellm/observability-cache-guardrails/README.md` — guardrails section,
  DB/Enterprise requirements, pitfalls.
- `docs/litellm/extracted/p2-guardrails-quick_start.md` — verbatim config keys,
  YAML examples, provider names, endpoints.
- `docs/litellm/extracted/p2-guardrails-guardrail_policies.md` — policies (Beta).

Guardrails config works **WITHOUT a database** (static `config.yaml` + per-request
param). The DB-gated surface is per-API-key control (Enterprise + DB).

## Triggers

Load this skill when:

- Adding or reviewing a `guardrails:` top-level block in `config.yaml`.
- Choosing a guardrail provider id (`litellm_params.guardrail`).
- Setting `mode` (pre_call / post_call / during_call / logging_only).
- Configuring `default_on: true` or model-level guardrails.
- Reviewing guardrail policies (`policies:` / `policy_attachments:`).
- Deciding whether a guardrail feature needs DB / Enterprise.

## Guardrail provider names (verbatim)

`cato_networks`, `aporia`, `lakera`, `presidio`, `generic_guardrail_api`,
`guardrails_ai`, `azure/text_moderations`, `bedrock`, `hide-secrets`,
`litellm_content_filter`, `OpenAI Moderation`.

The Specification block lists a subset verbatim: `"aporia"`, `"bedrock"`,
`"guardrails_ai"`, `"lakera"`, `"presidio"`, `"hide-secrets"`. The examples
show the fuller set above — a documented inconsistency.

## Config shape

Top-level `guardrails:` list (sibling to `model_list`). Each entry:

| Key | Required | Notes |
|-----|----------|-------|
| `guardrail_name` | yes | Name used in per-request `guardrails` param / model-level lists |
| `litellm_params.guardrail` | yes | Provider id (see list above) |
| `litellm_params.mode` | yes | string, list, or Mode object |
| `litellm_params.api_key` | yes | Guardrail service API key |
| `litellm_params.api_base` | no | Base URL |
| `litellm_params.default_on` | no | default `False` |
| `litellm_params.presidio_language` / `pii_entities_config` / `presidio_score_thresholds` | no | Presidio-specific |
| `litellm_params.additional_provider_specific_params` | no | Generic Guardrail API |
| `litellm_params.guard_name` | no | Guardrails AI / tag-based |
| `litellm_params.skip_system_message_in_guardrail` | no | per-guardrail |
| `guardrail_info` | no | dict returned on `GET /guardrails/list` |

## Modes (verbatim)

`"pre_call"`, `"post_call"`, `"during_call"`, `"logging_only"`. Accepts a
string, a list, or a `Mode` object.

## Unified guardrail path (applies on)

`/v1/chat/completions` and `/v1/messages` (Anthropic). Verbatim: Presidio,
Bedrock guardrails, `litellm_content_filter`, OpenAI Moderation, Generic
Guardrail API, and custom code guardrails that define `apply_guardrail`.

## Does NOT apply (direct hooks only)

Verbatim: Lakera v2, Aporia, DynamoAI, Javelin, Lasso, Pangea, Model Armor,
Azure Content Safety hooks, Guardrails AI, AIM, Cato Networks, tool
permission, MCP security.

## Model-level guardrails

`model_list[].litellm_params.guardrails` — list of guardrail names applied to
that model. Listed Enterprise in "Proxy Admin Controls" but shown in the spec
example — **inconsistency flag**; verify before relying on it.

## default_on caveat (verbatim)

`default_on: true` "will run even if user specifies a different guardrail or
empty guardrails array." A `default_on` guardrail cannot be opted out per
request.

## Endpoints

- `GET /guardrails/list` — returns guardrails + `guardrail_info` (no DB).
- `POST /v1/chat/completions` — `guardrails` param (list or dict), no DB.
- `POST /key/generate` / `POST /key/update` — `guardrails` list — **DB-backed**.
- `POST /team/update` — `metadata {"guardrails":{"modify_guardrails": false}}`.

## Guardrail Policies ([Beta])

`policies:` + `policy_attachments:` top-level config.

- `policies.<name>.description` / `inherit` / `guardrails.add` /
  `guardrails.remove` / `condition.model` / `pipeline`
- `policy_attachments[].policy` / `scope` / `teams` / `keys` / `models` / `tags`

`scope:"*"` (global) and `condition.model` work without DB. Team/key-based
attachments are Enterprise. Tag-based reads `metadata.tags` on keys/teams —
moot without virtual keys/teams. `POST /policies/resolve` works without DB.
Page is **[Beta]**.

## Enterprise (verbatim)

Pass Dynamic Parameters to Guardrail, Control Guardrails per API Key,
Tag-based Guardrail Modes, Model-level Guardrails (inconsistency flagged
above), Disable team from turning on/off guardrails.

## DB / Enterprise requirements

| Feature | Requirement | In-memory? |
|----------|-------------|------------|
| Static `guardrails:` config | none | **YES** |
| `GET /guardrails/list` | none | **YES** |
| Per-request `guardrails` param | none | **YES** |
| `default_on: true` | none | **YES** |
| Model-level guardrails | Enterprise (flagged) | unclear |
| Global `scope:"*"` + `condition.model` policies | none | **YES** |
| Per-API-key guardrail control (`/key/generate` `guardrails`) | Enterprise + DB | **NO** |
| Team/key-based policy attachments | Enterprise + DB | **NO** |
| Tag-based guardrail modes | virtual keys/teams (DB) | moot |

## Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| `default_on` guardrail fires on empty `guardrails` array | verbatim behavior | Accept; cannot opt out per request. |
| Per-key guardrail config rejected | `/key/generate` `guardrails` needs DB | Add Postgres (M4) or use static config + `default_on`. |
| Model-level guardrails not applied | Enterprise inconsistency | Verify entitlement; fall back to `default_on` + per-request param. |
| Lakera/Aporia not on unified path | direct-hook guardrails only | They do not run via the unified `/v1/chat/completions` path; configure per provider docs. |
| Policy team/key attachment ignored | Enterprise + DB | Use `scope:"*"` / `condition.model` (DB-free) instead. |

## Related Docs

- `docs/litellm/observability-cache-guardrails/README.md`
- `docs/litellm/extracted/p2-guardrails-quick_start.md`
- `docs/litellm/extracted/p2-guardrails-guardrail_policies.md`
- `docs/litellm/schemas/config-yaml.option-index.json`

> Do not hallucinate guardrail provider names, modes, or endpoints. Every
> name above is verbatim from `p2-guardrails-quick_start.md`; every DB/Enterprise
> flag traces to the cited corpus. Items not in the fetched source are marked
> "(inferred)" or "not documented in fetched source".
