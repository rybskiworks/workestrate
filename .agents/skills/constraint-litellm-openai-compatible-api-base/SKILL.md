---
name: constraint-litellm-openai-compatible-api-base
description: |
  Enforces api_base and api_key rules for openai/ prefix entries in a LiteLLM
  config.yaml. Load when authoring or reviewing an openai/ model_list entry that
  sets a custom api_base. Does NOT cover the openai/ prefix validity itself (see
  constraint-litellm-provider-prefix) or anthropic/ suffix behavior (see
  constraint-litellm-anthropic-suffix).
metadata:
  org.kind: constraint
---

# Constraint: OpenAI-Compatible api_base Has /v1 Postfix, No Endpoint Path, With api_key

This constraint enforces the verbatim caveats from the OpenAI-Compatible provider
page for any `model_list` entry whose `litellm_params.model` uses the `openai/`
prefix with a custom `api_base`. The `openai/` prefix routes through the upstream
OpenAI Python client, which (a) requires an API key for every request, (b)
requires the `/v1` postfix on `api_base` or returns "Not Found Error", and (c)
auto-appends endpoint paths so manual endpoint paths must not be added.

## Triggers

Load this skill when:

- Adding or reviewing an `openai/`-prefixed `model_list` entry with a custom
  `api_base` (e.g. self-hosted vLLM, Neuralwatt, Ollama OpenAI-compatible).
- Triaging a "Not Found Error" from an `openai/` model call.
- Triaging an "API key required" error from a keyless local endpoint.

## Rules

1. For `openai/`-prefix entries with a custom `api_base`, the `api_base` MUST
   include the `/v1` postfix. Verbatim from `providers/openai-compatible.md`: "If
   you see `Not Found Error` when testing make sure your `api_base` has the `/v1`
   postfix. Example: `http://vllm-endpoint.xyz/v1`".
2. The `api_base` MUST NOT append endpoint paths. Verbatim: "Do NOT add anything
   additional to the base url e.g. `/v1/embedding`. LiteLLM uses the openai-client
   to make these calls, and that automatically adds the relevant endpoints."
3. `api_key` MUST be present (the openai-client requires a key for all requests).
   Verbatim: "This library requires an API key for all requests, either through
   the `api_key` parameter or the `OPENAI_API_KEY` environment variable."
4. If the endpoint does not require an API key, use the `hosted_vllm/` prefix
   instead of `openai/` (verbatim: "If you don't want to provide a fake API key in
   each request, consider using a provider that directly matches your
   OpenAI-compatible endpoint, such as `hosted_vllm` or `llamafile`.").
5. `api_key` must still use `os.environ/<VAR>` indirection (see
   `constraint-litellm-secret-hygiene`).

## References

- Docs: `docs/litellm/providers/openai-compatible.md` (verbatim CRITICAL caveats
  on `/v1` postfix, no endpoint paths, api_key required).
- Schema: `docs/litellm/schemas/provider-fields.index.json` (OpenAI-Compatible
  entry: `api_base_behavior`, `api_key_behavior`, `caveats`).

## Out of scope

- Whether the `openai/` prefix itself is valid — see
  `constraint-litellm-provider-prefix`.
- Whether `api_key` is hardcoded — see `constraint-litellm-secret-hygiene`.
- `anthropic/` custom api_base suffix behavior — see
  `constraint-litellm-anthropic-suffix`.

## Violation examples

### Missing /v1 postfix

```yaml
model_list:
  - model_name: neuralwatt
    litellm_params:
      model: openai/neuralwatt
      api_base: https://api.neuralwatt.com      # FORBIDDEN: missing /v1 postfix -> Not Found Error
      api_key: os.environ/NEURALWATT_API_KEY
```

### Endpoint path appended to api_base

```yaml
model_list:
  - model_name: my-model
    litellm_params:
      model: openai/my-model
      api_base: https://my-host/v1/embedding   # FORBIDDEN: openai-client auto-appends endpoints
      api_key: os.environ/MY_API_KEY
```

### Missing api_key on openai/ prefix

```yaml
model_list:
  - model_name: local-vllm
    litellm_params:
      model: openai/my-model
      api_base: https://my-vllm-host/v1
      # api_key omitted              # FORBIDDEN: openai-client requires a key; use hosted_vllm/ instead
```

## How to check

Run `validation-litellm-config-check` (check h: for every `openai/`-prefix entry
with a custom `api_base`, verify `api_base` ends in `/v1`, contains no
`/v1/<endpoint>` suffix, and `api_key` is present). The check uses the verbatim
rules from `providers/openai-compatible.md`.
