---
name: constraint-litellm-anthropic-suffix
description: |
  Enforces api_base suffix rules for anthropic/ prefix entries with a custom
  api_base in a LiteLLM config.yaml. Load when authoring or reviewing an
  anthropic/ model_list entry that sets a custom api_base. Does NOT cover the
  anthropic/ prefix validity itself (see constraint-litellm-provider-prefix) or
  openai/ api_base rules (see constraint-litellm-openai-compatible-api-base).
metadata:
  org.kind: constraint
---

# Constraint: Anthropic Custom api_base Must Not Pre-Include /v1/messages

This constraint enforces the auto-suffix behavior documented on the Anthropic
provider page for any `model_list` entry whose `litellm_params.model` uses the
`anthropic/` prefix with a custom `api_base`. LiteLLM automatically appends
`/v1/messages` (or `/v1/complete`) to the base URL; an `api_base` that already
ends in `/v1/messages` would cause a double-append (e.g.
`.../v1/messages/v1/messages`) unless the auto-suffix is explicitly disabled.

## Triggers

Load this skill when:

- Adding or reviewing an `anthropic/`-prefixed `model_list` entry with a custom
  `api_base` (e.g. pointing `anthropic/` at a non-Anthropic endpoint that speaks
  the Anthropic Messages API).
- Triaging a 404 / double-path error from an `anthropic/` model call.
- Reviewing use of `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX`.

## Rules

1. For `anthropic/`-prefix entries with a custom `api_base`, LiteLLM
   automatically appends `/v1/messages` (or `/v1/complete`). Verbatim from
   `providers/anthropic.md`: "When using a custom API base for Anthropic (e.g., a
   proxy or custom endpoint), LiteLLM automatically appends the appropriate suffix
   (`/v1/messages` or `/v1/complete`) to your base URL."
2. The `api_base` MUST NOT already end in `/v1/messages` or `/v1/complete`,
   because that would double-append (e.g. `https://my-proxy.com/v1/messages` →
   `https://my-proxy.com/v1/messages/v1/messages`).
3. If the endpoint requires the full path in `api_base` (i.e. auto-append must be
  disabled), the env var `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` MUST be set.
   Verbatim: "With `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true`: Base URL
   `https://my-proxy.com/custom/path` → `https://my-proxy.com/custom/path`
   (unchanged)."
4. The recommended form is a bare base URL (e.g.
   `https://api.minimax.io/anthropic`) that LiteLLM appends `/v1/messages` to,
   yielding `https://api.minimax.io/anthropic/v1/messages` (matches the MiniMax
   provider page verbatim).
5. `api_key` must still use `os.environ/<VAR>` indirection (see
   `constraint-litellm-secret-hygiene`).

## References

- Docs: `docs/litellm/providers/anthropic.md` (verbatim auto-suffix behavior and
  `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX` toggle).
- Schema: `docs/litellm/schemas/provider-fields.index.json` (Anthropic entry:
  `api_base_behavior`, `optional_env_vars` including
  `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX`).

## Out of scope

- Whether the `anthropic/` prefix itself is valid — see
  `constraint-litellm-provider-prefix`.
- Whether `api_key` is hardcoded — see `constraint-litellm-secret-hygiene`.
- `openai/` api_base `/v1` postfix rules — see
  `constraint-litellm-openai-compatible-api-base`.

## Violation examples

### api_base already ends in /v1/messages (double-append)

```yaml
model_list:
  - model_name: minimax
    litellm_params:
      model: anthropic/MiniMax-M3
      api_base: https://api.minimax.io/anthropic/v1/messages   # FORBIDDEN: LiteLLM appends /v1/messages again
      api_key: os.environ/MINIMAX_API_KEY
```

### Full path in api_base without disabling suffix

```yaml
model_list:
  - model_name: custom-anthropic
    litellm_params:
      model: anthropic/my-model
      api_base: https://my-proxy.com/custom/path/v1/messages  # FORBIDDEN: no LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true set
      api_key: os.environ/MY_API_KEY
```

## How to check

Run `validation-litellm-config-check` (check i: for every `anthropic/`-prefix
entry with a custom `api_base`, verify `api_base` does not end in
`/v1/messages` or `/v1/complete` unless `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX`
is set to true). The check uses the verbatim suffix rules from
`providers/anthropic.md`.
