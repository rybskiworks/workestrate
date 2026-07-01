---
name: litellm-openai-compatible
description: |
  Operational reference for the OpenAI-compatible provider pattern (Neuralwatt,
  vLLM, Ollama-OpenAI, self-hosted): `openai/<name>` + `api_base` WITH `/v1` +
  `api_key` required, the `/v1` gotcha, and `hosted_vllm/` as the keyless
  alternative. Load when wiring a Neuralwatt/vLLM/Ollama-OpenAI endpoint,
  diagnosing a "Not Found Error", or deciding `openai/` vs `hosted_vllm/`.
  Does NOT cover non-OpenAI protocols (see litellm-providers), routing
  (see litellm-routing-fallbacks), or config anatomy (see litellm-config-
  anatomy). Full detail lives in docs/litellm/providers/openai-compatible.md.
---

# LiteLLM OpenAI-Compatible Pattern

Distilled operational reference for the `openai/` prefix pointed at an
OpenAI-compatible endpoint. This is the canonical pattern for Neuralwatt,
self-hosted vLLM, and any HTTP server speaking the OpenAI chat-completions
protocol. Full detail lives in:

- `docs/litellm/providers/openai-compatible.md` — the upstream extraction.
- `docs/litellm/providers/openai.md` — the OpenAI-native variant (same prefix).
- `docs/litellm/providers/vllm.md` — `hosted_vllm/` keyless alternative.
- `docs/litellm/examples/minimal-openai-compatible.yaml` — worked example.
- `docs/litellm/schemas/provider-fields.index.json` — `openai/` entry.

> **Do not hallucinate.** Every rule below traces verbatim to
> `docs/litellm/providers/openai-compatible.md` or `provider-fields.index.json`.
> Project context is marked **[PROJECT]**.

## Triggers

Load this skill when:

- Wiring a Neuralwatt, vLLM, or Ollama-OpenAI-compatible endpoint.
- Diagnosing a `Not Found Error` from an `openai/`-prefixed model.
- Deciding between `openai/` (key required) and `hosted_vllm/` (keyless).
- Reviewing an `api_base` value for the `/v1` postfix.

## The pattern

```yaml
model_list:
  - model_name: <alias>
    litellm_params:
      model: openai/<your-model-name>   # openai/ prefix routes via openai-client
      api_base: <model-api-base>        # MUST include /v1 postfix
      api_key: <api-key>                 # REQUIRED for all requests
```

Verbatim from the docs: "Selecting `openai` as the provider routes your
request to an OpenAI-compatible endpoint using the upstream official OpenAI
Python API library."

## The `/v1` gotcha (CRITICAL)

> "If you see `Not Found Error` when testing make sure your `api_base` has
> the `/v1` postfix. Example: `http://vllm-endpoint.xyz/v1`"

- `api_base` **MUST** include the `/v1` postfix.
- Do **NOT** append endpoint paths like `/v1/embedding` to `api_base` —
  LiteLLM uses the openai-client, which automatically adds the relevant
  endpoints.

## `api_key` requirement

> "This library requires an API key for all requests, either through the
> `api_key` parameter or the `OPENAI_API_KEY` environment variable."

The openai-client requires a key for **every** request, even for endpoints
that don't need one. For keyless endpoints:

- Pass a fake/non-empty `api_key`, OR
- Use `hosted_vllm/` instead (keyless alternative).

## `openai/` vs `hosted_vllm/`

| Concern | `openai/` | `hosted_vllm/` |
|---------|-----------|-----------------|
| API key | REQUIRED (even fake) | optional (`HOSTED_VLLM_API_KEY`) |
| `api_base` `/v1` postfix | REQUIRED | not documented as required |
| Endpoints | `/chat/completions`, `/embeddings`, `/completions` | adds `/rerank`, `/audio/transcriptions` |
| Use when | endpoint requires a key (e.g. Neuralwatt) | keyless local vLLM |

`hosted_vllm/` is the recommended alternative to `openai/` when you don't
want to pass a fake API key (verbatim, referenced by the openai-compatible
page). `vllm/` (without `hosted_`) is DEPRECATED — for in-process vLLM SDK
usage, not HTTP server calls.

## Supported endpoints (openai/)

- `/chat/completions`
- `/embeddings`
- `/completions`

## Special params

- `supports_system_message: False` (under `litellm_params`) — maps system
  messages to 'user' messages (for vLLM models like gemma that don't support
  system messages).

## OpenAI-native variant (same prefix)

The `openai/` prefix is shared between the OpenAI-native page and the
openai-compatible page. OpenAI-native adds:

- `OPENAI_BASE_URL` env var (custom endpoint) — the OpenAI-native mechanism
  for pointing at OpenAI-compatible endpoints.
- `OPENAI_ORGANIZATION` env var (optional).
- `openai/responses/` sub-prefix — routes through OpenAI's Responses API
  (built-in tools like `web_search_preview`, `code_interpreter`). NOT
  relevant for Neuralwatt.
- `text-completion-openai/` prefix — legacy completions
  (`openai.completions.create`).

> `api_base` as a YAML `litellm_params` key is shown on the openai-compatible
> page; the OpenAI-native page documents `OPENAI_BASE_URL` env var instead.
> Both use the same `openai/` prefix.

## Common Mistakes

| Mistake | Cause | Fix |
|---------|-------|-----|
| `Not Found Error` | `api_base` missing `/v1` | Add `/v1` postfix (e.g. `https://api.neuralwatt.com/v1`). |
| Appended `/v1/embedding` to `api_base` | Misunderstanding openai-client | Remove; client adds endpoints automatically. |
| Auth failure on keyless endpoint | openai-client requires a key | Pass a fake key, or switch to `hosted_vllm/`. |
| `OPENAI_BASE_URL` set but `api_base` also in `litellm_params` | Two mechanisms | Prefer `api_base` in `litellm_params` for proxy YAML. |
| `vllm/` prefix used for HTTP server | Deprecated | Use `hosted_vllm/` for HTTP servers; `vllm/` is in-process SDK only. |
| `openai/responses/` used for Neuralwatt | Wrong sub-prefix | Use plain `openai/<name>`; `responses/` is for OpenAI's Responses API. |

## Project context [PROJECT — not upstream docs]

The workestrator uses this exact pattern for the `neural` tier:

```yaml
model_list:
  - model_name: neural
    litellm_params:
      model: openai/neuralwatt
      api_base: https://api.neuralwatt.com/v1   # /v1 postfix present
      api_key: os.environ/NEURALWATT_API_KEY
```

- `neuralwatt` model name, `https://api.neuralwatt.com/v1` host, and
  `NEURALWATT_API_KEY` env var are **project choices** — they follow the
  documented pattern but the specific host/key-name are NOT in upstream docs.
- `NEURALWATT_API_KEY` is a project-defined env var (empty `source_urls` in
  `env-vars.index.json`), injected by the sandbox `env()`/`secret_env()`.
- The `neural` tier has **no fallback** configured (single OpenAI-compatible
  deployment, no redundancy).
- Egress to `api.neuralwatt.com` is allowlisted at the microsandbox runtime
  (default-deny; DNS + tcp/443 only).

## Related Docs

- `docs/litellm/providers/openai-compatible.md`
- `docs/litellm/providers/openai.md`
- `docs/litellm/providers/vllm.md`
- `docs/litellm/examples/minimal-openai-compatible.yaml`
- `docs/litellm/schemas/provider-fields.index.json`
