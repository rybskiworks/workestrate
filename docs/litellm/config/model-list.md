# `model_list` Anatomy

## What it controls

Defines deployments: a `model_name` alias + upstream provider params
(`litellm_params`) + optional metadata (`model_info`). Clients request the
alias; LiteLLM routes to the upstream provider.

## Where it appears

- **Config**: `config.yaml` top-level `model_list`.
- **Runtime**: the `model` field in `POST /v1/chat/completions` maps to a
  `model_name` alias (NOT to `litellm_params.model`).

## Verified behavior

- Each entry = `model_name` (alias) + `litellm_params` (required) + `model_info`
  (optional).
- **Multiple entries with the same `model_name` form a load-balancing group.**
- When `tpm`/`rpm` are set **and** `routing_strategy == simple-shuffle`, LiteLLM
  uses a **weighted pick** based on `tpm`/`rpm` (verbatim caveat from the configs
  page).

### `litellm_params` keys (all 18 verified)

Cite: `schemas/config-yaml.option-index.json` → section `model_list.litellm_params`.

| Key | Type | Source pages |
| --- | --- | --- |
| `model` | string | config_settings, configs, deploy, docker_quick_start, quick_start, virtual_keys |
| `api_base` | string | config_settings, configs, deploy, docker_quick_start, quick_start |
| `api_key` | string | config_settings, configs, deploy, docker_quick_start, quick_start, virtual_keys |
| `api_version` | string | config_settings, configs, deploy, docker_quick_start |
| `organization` | string | config_settings |
| `temperature` | float | config_settings |
| `max_tokens` | int | config_settings |
| `seed` | int | config_settings, configs |
| `extra_headers` | dict | config_settings |
| `rpm` | int | config_settings, configs, deploy |
| `tpm` | int | config_settings, configs |
| `weight` | int | config_settings |
| `region_name` | string | config_settings |
| `base_model` | string | config_settings |
| `tags` | list | config_settings |
| `aws_region_name` | string | configs |
| `azure_ad_token` | string | configs |
| `litellm_credential_name` | string | configs |

### `model_info` sub-keys

| Key | Source |
| --- | --- |
| `id` | config_settings |
| `mode` (e.g. `embedding`) | config_settings |
| `input_cost_per_token` | config_settings |
| `output_cost_per_token` | config_settings |
| `max_tokens` | config_settings |
| `base_model` | config_settings |
| `version` | configs |
| `supported_environments` | configs |
| `access_groups` | configs |
| `custom_tokenizer` (`identifier`, `revision`, `auth_token`) | configs |

### `credential_list` (top-level, from configs page)

Top-level `credential_list` entries enable centralized credential reuse, paired
with `litellm_params.litellm_credential_name`.

| Key | Type |
| --- | --- |
| `credential_name` | string |
| `credential_values` | dict |
| `credential_info` | dict |

## Examples

### Minimal entry (verbatim from configs page)

```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: azure/gpt-4o-eu
      api_base: https://my-endpoint-eu.azure.com/
      api_key: os.environ/AZURE_API_KEY_EU
      rpm: 6
```

### Load-balancing group (verbatim pattern from configs page)

Multiple entries with the same `model_name` form a load-balancing group. With
`tpm`/`rpm` set and `routing_strategy: simple-shuffle`, LiteLLM uses weighted
pick:

```yaml
model_list:
  - model_name: zephyr-beta
    litellm_params:
      model: huggingface/HuggingFaceH4/zephyr-7b-beta
      api_base: https://my-endpoint-eu.azure.com
      api_key: os.environ/HUGGINGFACE_API_KEY
      rpm: 6
  - model_name: zephyr-beta
    litellm_params:
      model: huggingface/HuggingFaceH4/zephyr-7b-beta
      api_base: https://my-endpoint-us.azure.com
      api_key: os.environ/HUGGINGFACE_API_KEY
      rpm: 1440
```

## Pitfalls

- The `model` field in the **request** is the alias (`model_name`), **not**
  `litellm_params.model` (the provider model string).
- `api_base` for `openai/`-prefixed (OpenAI-compatible) models **MUST include
  the `/v1` postfix** (e.g. `https://api.neuralwatt.com/v1`).
- `anthropic/` auto-appends `/v1/messages` — do not include that path in
  `api_base`.
- `openrouter/` uses a **nested** format: `openrouter/<provider>/<model>`
  (e.g. `openrouter/z-ai/glm-5.1`).

## Links to local

- [schemas/config-yaml.option-index.json](../schemas/config-yaml.option-index.json) — section `model_list.litellm_params`
- [schemas/config-yaml.normalized.schema.md](../schemas/config-yaml.normalized.schema.md) — normalized schema reference
- [../providers/README.md](../providers/README.md) — per-provider guides (provider prefixes, `api_base` rules)
- [../01-mental-model.md](../01-mental-model.md) — request flow

## Sources

- https://docs.litellm.ai/docs/proxy/config_settings
- https://docs.litellm.ai/docs/proxy/configs
- Local: `extracted/p0-config_settings.md`, `extracted/p0-configs.md`,
  `schemas/config-yaml.option-index.json`

## Workestrator notes

[PROJECT CONTEXT — NOT upstream docs]

The real config (`infra/litellm/config.yaml`) defines 9 aliases across coding
tiers + `neural`. The `coding` + `coding-fallback` pair (verbatim):

```yaml
model_list:
  # --- coding (default: Kimi K2.7 specialist) ---
  - model_name: coding
    litellm_params:
      model: anthropic/kimi-for-coding
      api_base: https://api.kimi.com/coding
      api_key: os.environ/KIMI_CODE_API_KEY

  - model_name: coding-fallback
    litellm_params:
      model: anthropic/MiniMax-M3
      api_base: https://api.minimax.io/anthropic
      api_key: os.environ/MINIMAX_CODING_API_KEY
```

Provider-prefix usage in the real config:

| Prefix | Used for | Notes |
| --- | --- | --- |
| `anthropic/` | Kimi (`kimi-for-coding`), MiniMax (`MiniMax-M3`) | Both endpoints speak the Anthropic Messages API; `api_base` set explicitly |
| `openrouter/` | GLM (`z-ai/glm-5.1`), Qwen (`qwen/qwen3.7-plus`, `qwen/qwen3-coder:free`), Nex (`nex-agi/nex-n2-pro:free`) | Nested `openrouter/<provider>/<model>` format; no `api_base` (uses OpenRouter default) |
| `openai/` | Neuralwatt (`neuralwatt`) | `api_base: https://api.neuralwatt.com/v1` (includes `/v1` postfix) |

Fallbacks are wired in `router_settings` (not `model_list`): `coding` →
`coding-fallback`, `coding.fast` → `coding.fast-fallback`, `coding.pro` →
`coding.pro-fallback`, `coding.free` → `coding.free-fallback`. See
[config-yaml-overview.md](config-yaml-overview.md) → Workestrator notes.
