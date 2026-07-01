---
name: constraint-litellm-provider-prefix
description: |
  Enforces that every model_list[].litellm_params.model prefix matches a
  documented litellm_prefix in the provider-fields index. Load when authoring or
  reviewing model_list entries in a LiteLLM config.yaml. Does NOT cover api_base
  suffix rules for openai/ (see constraint-litellm-openai-compatible-api-base) or
  anthropic/ (see constraint-litellm-anthropic-suffix), or schema validity of
  non-model keys (see constraint-litellm-config-schema).
metadata:
  org.kind: constraint
---

# Constraint: Provider Prefix Must Be Documented

This constraint enforces that the `model` string in every
`model_list[].litellm_params` entry begins with a provider prefix documented in
`docs/litellm/schemas/provider-fields.index.json` (`litellm_prefix` field). An
unknown prefix means LiteLLM cannot route the request to a provider and will fail
at call time.

## Triggers

Load this skill when:

- Adding or editing a `model_list` entry in `config.yaml`.
- Reviewing a PR that introduces a new model alias or provider.
- Triaging a "model not found" / "unknown provider" routing error.

## Rules

1. Every `model_list[].litellm_params.model` MUST begin with one of the
   documented `litellm_prefix` values from `provider-fields.index.json`.
2. Known valid prefixes (verbatim from the index):
   `openai/`, `anthropic/`, `openrouter/`, `moonshot/`, `minimax/`, `zai/`,
   `hosted_vllm/`, `ollama/`, `ollama_chat/`, `litellm_proxy/`, `inception/`.
3. A bare model name with no prefix (e.g. `model: gpt-4o`) is permitted ONLY for
   the OpenAI native provider when calling api.openai.com directly; in proxy YAML
   the `openai/` prefix is the documented disambiguation form. A bare name
   pointing at a custom endpoint is a violation (use `openai/` + `api_base`).
4. The deprecated `vllm/` prefix (in-process vLLM SDK) is a violation for HTTP
   server calls — use `hosted_vllm/` instead (verbatim from the index: "`vllm/`
   prefix is DEPRECATED — use `hosted_vllm/` for OpenAI-compatible vLLM
   servers.").
5. Spelling matters: `zai/` (no hyphen) is the LiteLLM native prefix; `z-ai/`
   (hyphenated) is the OpenRouter namespace — they are distinct routing paths and
   must not be conflated.

## References

- Schema: `docs/litellm/schemas/provider-fields.index.json` (`providers[].litellm_prefix`,
  `providers[].example_model_strings`, `providers[].caveats`).

## Out of scope

- Whether an `openai/` entry's `api_base` has the `/v1` postfix — see
  `constraint-litellm-openai-compatible-api-base`.
- Whether an `anthropic/` entry's `api_base` will double-append `/v1/messages` —
  see `constraint-litellm-anthropic-suffix`.
- Whether the model name after the prefix is a real upstream model — that is a
  runtime concern, not a config constraint.

## Violation examples

### Unknown prefix

```yaml
model_list:
  - model_name: my-model
    litellm_params:
      model: openaai/gpt-4o          # FORBIDDEN: misspelled prefix (openaai/ not in index)
```

### Deprecated vllm/ prefix for HTTP server

```yaml
model_list:
  - model_name: my-model
    litellm_params:
      model: vllm/facebook/opt-125m  # FORBIDDEN: vllm/ deprecated for HTTP; use hosted_vllm/
      api_base: https://my-vllm-host
```

### Wrong namespace spelling

```yaml
model_list:
  - model_name: glm
    litellm_params:
      model: z-ai/glm-4.7            # FORBIDDEN: z-ai/ is OpenRouter namespace; native prefix is zai/
```

## How to check

Run `validation-litellm-config-check` (check b: every
`model_list[].litellm_params.model` prefix matches a `litellm_prefix` in
`provider-fields.index.json`). The check extracts the prefix (text before the
first `/`) and matches it against the index's prefix set.
