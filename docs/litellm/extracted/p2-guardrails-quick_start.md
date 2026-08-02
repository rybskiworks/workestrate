---
source_url: https://docs.litellm.ai/docs/proxy/guardrails/quick_start
canonical_url: https://docs.litellm.ai/docs/proxy/guardrails/quick_start
title: "Guardrails - Quick Start"
sidebar_section_path: proxy > guardrails
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: guardrails
---
# Guardrails - Quick Start

## Headings
- 1. Define guardrails on your LiteLLM config.yaml
  - Supported values for `mode` (Event Hooks)
  - Skip system messages in guardrail evaluation
  - Load Balancing Guardrails
- 2. Start LiteLLM Gateway
- 3. Test request
- Default On Guardrails
  - Guardrail Policies
- Using Guardrails Client Side
  - Test yourself (OSS)
  - Expose to your users (Enterprise)
- Proxy Admin Controls
  - Monitoring Guardrails
  - Control Guardrails per API Key
  - Tag-based Guardrail Modes
  - Model-level Guardrails
  - Disable team from turning on/off guardrails
- Specification
  - guardrails Configuration on YAML
  - guardrails Request Parameter

## Exact config keys found (full path, verbatim spelling)
- `guardrails[].guardrail_name` (guardrails) — Required: Name of the guardrail
- `guardrails[].litellm_params.guardrail` (guardrails) — Required: provider id
- `guardrails[].litellm_params.mode` (guardrails) — Required: string, list, or Mode object; values: `"pre_call"`, `"post_call"`, `"during_call"`, `"logging_only"`
- `guardrails[].litellm_params.api_key` (guardrails) — Required: API key for the guardrail service
- `guardrails[].litellm_params.api_base` (guardrails) — Optional: Base URL
- `guardrails[].litellm_params.default_on` (guardrails) — Optional (Default False)
- `guardrails[].litellm_params.presidio_language` (guardrails) — Presidio
- `guardrails[].litellm_params.pii_entities_config` (guardrails) — Presidio
- `guardrails[].litellm_params.presidio_score_thresholds` (guardrails) — Presidio
- `guardrails[].litellm_params.additional_provider_specific_params` (guardrails) — Generic Guardrail API
- `guardrails[].litellm_params.guard_name` (guardrails) — Guardrails AI / Tag-based
- `guardrails[].litellm_params.skip_system_message_in_guardrail` (guardrails) — per guardrail
- `guardrails[].guardrail_info` (guardrails) — Optional[Dict]; returned on GET /guardrails/list
- `guardrails[].guardrail_info.params[].name` / `.type` / `.description` (guardrails)
- `model_list[].litellm_params.guardrails` (model_list) — list of guardrail names (Model-level Guardrails)
- `litellm_settings.skip_system_message_in_guardrail` (litellm_settings) — global

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# 1. Define guardrails on your LiteLLM config.yaml
model_list:
  - model_name: gpt-3.5-turbo
    litellm_params:
      model: openai/gpt-3.5-turbo
      api_key: os.environ/OPENAI_API_KEY

guardrails:
  - guardrail_name: general-guard
    litellm_params:
      guardrail: cato_networks
      mode: [pre_call, post_call]
      api_key: os.environ/CATO_API_KEY
      api_base: os.environ/CATO_API_BASE
      default_on: true # Optional

  - guardrail_name: "aporia-pre-guard"
    litellm_params:
      guardrail: aporia  # supported values: "aporia", "lakera"
      mode: "during_call"
      api_key: os.environ/APORIA_API_KEY_1
      api_base: os.environ/APORIA_API_BASE_1

  - guardrail_name: "aporia-post-guard"
    litellm_params:
      guardrail: aporia  # supported values: "aporia", "lakera"
      mode: "post_call"
      api_key: os.environ/APORIA_API_KEY_2
      api_base: os.environ/APORIA_API_BASE_2
    guardrail_info: # Optional field, info is returned on GET /guardrails/list
      params:
        - name: "toxicity_score"
          type: "float"
          description: "Score between 0-1 indicating content toxicity level"
        - name: "pii_detection"
          type: "boolean"

  # Example Presidio guardrail config
  - guardrail_name: "presidio-pii"
    litellm_params:
      guardrail: presidio
      mode: "pre_call"
      presidio_language: "en"
      pii_entities_config:
        CREDIT_CARD: "MASK"
        EMAIL_ADDRESS: "MASK"
        US_SSN: "MASK"
      presidio_score_thresholds:
        CREDIT_CARD: 0.8
        EMAIL_ADDRESS: 0.6

  # Example Pillar Security config via Generic Guardrail API
  - guardrail_name: "pillar-security"
    litellm_params:
      guardrail: generic_guardrail_api
      mode: [pre_call, post_call]
      api_base: https://api.pillar.security/api/v1/integrations/litellm
      api_key: os.environ/PILLAR_API_KEY
      additional_provider_specific_params:
        plr_mask: true
        plr_evidence: true
        plr_scanners: true
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/quick_start)

```yaml
# Skip system messages in guardrail evaluation (Global)
litellm_settings:
  skip_system_message_in_guardrail: true
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/quick_start)

```yaml
# Default On Guardrails
guardrails:
  - guardrail_name: "aporia-pre-guard"
    litellm_params:
      guardrail: aporia
      mode: "pre_call"
      default_on: true
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/quick_start)

```yaml
# Model-level Guardrails
model_list:
  - model_name: claude-sonnet-4
    litellm_params:
      model: anthropic/claude-sonnet-4-20250514
      api_key: os.environ/ANTHROPIC_API_KEY
      api_base: https://api.anthropic.com/v1
      guardrails: ["azure-text-moderation"]

  - model_name: openai-gpt-4o
    litellm_params:
      model: openai/gpt-4o

guardrails:
  - guardrail_name: "presidio-pii"
    litellm_params:
      guardrail: presidio
      mode: "pre_call"
      presidio_language: "en"
      pii_entities_config:
        PERSON: "BLOCK"

  - guardrail_name: azure-text-moderation
    litellm_params:
      guardrail: azure/text_moderations
      mode: "post_call" 
      api_key: os.environ/AZURE_GUARDRAIL_API_KEY
      api_base: os.environ/AZURE_GUARDRAIL_API_BASE 
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/quick_start)

```yaml
# Specification - guardrails Configuration on YAML
guardrails:
  - guardrail_name: string     # Required: Name of the guardrail
    litellm_params:            # Required: Configuration parameters
      guardrail: string        # Required: One of "aporia", "bedrock", "guardrails_ai", "lakera", "presidio", "hide-secrets"
      mode: Union[string, List[string], Mode]  # Required: One or more of "pre_call", "post_call", "during_call", "logging_only"
      api_key: string          # Required: API key for the guardrail service
      api_base: string         # Optional: Base URL for the guardrail service
      default_on: boolean      # Optional: Default False. When set to True, will run on every request
    guardrail_info:            # Optional[Dict]: Additional information about the guardrail
```
(source: https://docs.litellm.ai/docs/proxy/guardrails/quick_start)

## Exact environment variables
- `OPENAI_API_KEY` — OpenAI API key for model
- `CATO_API_KEY` / `CATO_API_BASE` — Cato Networks guardrail
- `APORIA_API_KEY_1` / `APORIA_API_BASE_1` / `APORIA_API_KEY_2` / `APORIA_API_BASE_2` — Aporia guardrail
- `PILLAR_API_KEY` — Pillar Security
- `LAKERA_API_KEY` — Lakera guardrail
- `GUARDRAILS_AI_API_BASE` — Guardrails AI API base (defaults to `http://0.0.0.0:8000`)
- `ANTHROPIC_API_KEY` — Anthropic API key
- `AZURE_GUARDRAIL_API_KEY` / `AZURE_GUARDRAIL_API_BASE` — Azure Content Safety

## Exact endpoint paths / API routes (runtime + management)
- `GET /guardrails/list` — Returns available guardrails and their `guardrail_info`
- `POST /v1/chat/completions` (alias `/chat/completions`) — LLM request; accepts `guardrails` parameter (list or dict)
- `POST /key/generate` — Create key with `guardrails` list
- `POST /key/update` — Update key with `guardrails` list
- `POST /team/update` — Update team metadata `{"guardrails": {"modify_guardrails": false}}`

## Exact CLI commands
- `litellm --config config.yaml --detailed_debug` — Start LiteLLM Gateway

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: required for — "✨ This is an Enterprise only feature" appears for: Pass Dynamic Parameters to Guardrail, Control Guardrails per API Key, Tag-based Guardrail Modes, Model-level Guardrails, Disable team from turning on/off guardrails
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- "These will run even if user specifies a different guardrail or empty guardrails array." (re `default_on: true`)
- "Where this applies: Only the **unified** guardrail path... on **OpenAI Chat Completions** (`/v1/chat/completions`) and **Anthropic Messages** (`/v1/messages`). Examples include Presidio, Bedrock guardrails, `litellm_content_filter`, OpenAI Moderation, Generic Guardrail API, and custom code guardrails that define `apply_guardrail`."
- "Where this does *not* apply: Guardrails that run only via direct hooks on the raw request (e.g. Lakera v2, Aporia, DynamoAI, Javelin, Lasso, Pangea, Model Armor, Azure Content Safety hooks, Guardrails AI, AIM, Cato Networks, tool permission, MCP security)."
- Guardrail provider names mentioned verbatim: `cato_networks`, `aporia`, `lakera`, `presidio`, `generic_guardrail_api`, `guardrails_ai`, `azure/text_moderations`, `bedrock`, `hide-secrets`, `litellm_content_filter`, `OpenAI Moderation`

## Related links
- /docs/proxy/guardrails/team_based_guardrails
- /docs/proxy/guardrails/guardrail_load_balancing
- /docs/proxy/guardrails/test_playground
- /docs/proxy/guardrails/litellm_content_filter
- /docs/guardrail_providers
- /docs/adding_provider/generic_guardrail_api
- /docs/proxy/guardrails/guardrail_policies
- /docs/enterprise

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- Guardrails config (`guardrails:` list in config.yaml) works WITHOUT a database — it's static config. The workestrate can define guardrails (e.g. `presidio`, `litellm_content_filter`, `generic_guardrail_api`) in config.yaml and apply them via `default_on: true` or per-request `guardrails` param. `GET /guardrails/list` works without DB. Model-level guardrails (`model_list[].litellm_params.guardrails`) work without DB. Per-API-key guardrail control and tag-based modes are Enterprise (unavailable). `litellm_content_filter` and `hide-secrets` are built-in guardrails that may work without external services (inferred). The `guardrails` request parameter on `/v1/chat/completions` works without DB.

## Confidence / uncertainty notes
- high confidence on guardrails config format and provider names (verbatim YAML). DB requirement is "not documented" — guardrails are static config + request params (inferred they work without DB). Enterprise requirements are verbatim. The Specification block lists fewer provider names than the examples show — noted as a doc inconsistency.
