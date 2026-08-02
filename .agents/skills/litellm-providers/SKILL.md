---
name: litellm-providers
description: |
  Operational reference for LiteLLM provider onboarding: provider prefix
  selection, required env vars per provider, and `api_base`/`api_key` rules.
  Load when adding a provider to `model_list`, choosing a prefix, diagnosing
  a "Not Found Error" or auth failure, or reviewing `litellm_params` for a
  new upstream. Does NOT cover the OpenAI-compatible `/v1` gotcha in depth
  (see litellm-openai-compatible), routing/fallbacks (see litellm-routing-
  fallbacks), or config top-level structure (see litellm-config-anatomy).
  Full detail lives in docs/litellm/providers/*.md.
---

# LiteLLM Providers

Distilled operational reference for selecting a LiteLLM provider prefix and
wiring its `litellm_params`. The provider prefix is the first path segment of
`litellm_params.model` (e.g. `anthropic/kimi-for-coding` → prefix `anthropic/`).
Full detail lives in:

- `docs/litellm/providers/README.md` — provider index + workestrate usage.
- `docs/litellm/providers/<provider>.md` — per-provider extraction.
- `docs/litellm/schemas/provider-fields.index.json` — per-provider
  `litellm_prefix`, `required_env_vars`, `api_base_behavior`,
  `api_key_behavior`, `caveats`, `requires_db`, `requires_redis`.

> **Do not hallucinate.** Every prefix/env-var/rule below traces to
> `docs/litellm/providers/*.md` or `docs/litellm/schemas/provider-fields.index.json`.
> Project-specific choices are marked **[PROJECT]**.

## Triggers

Load this skill when:

- Adding a provider deployment to `model_list`.
- Choosing between `openai/`, `hosted_vllm/`, `anthropic/`, `openrouter/`.
- Diagnosing a "Not Found Error", auth failure, or wrong-protocol call.
- Reviewing whether a custom `api_base` is documented for a prefix.
- Deciding env-var names for a new upstream.

## Prefix selection table

| Prefix | Protocol | Required env var | `api_base` configurable? | Notes |
|--------|----------|------------------|--------------------------|-------|
| `openai/` | OpenAI-compatible | `OPENAI_API_KEY` | yes (`api_base` in `litellm_params`) | Shared by OpenAI-native + OpenAI-compatible. `/v1` postfix REQUIRED. Key required even for keyless endpoints. |
| `anthropic/` | both (OpenAI `/chat/completions` + Anthropic `/v1/messages`) | `ANTHROPIC_API_KEY` | yes (`ANTHROPIC_API_BASE` or `api_base`) | Auto-appends `/v1/messages` unless `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true`. `max_tokens` required (LiteLLM defaults 4096). |
| `openrouter/` | OpenAI-compatible | `OPENROUTER_API_KEY` | optional (`OPENROUTER_API_BASE`, defaults `https://openrouter.ai/api/v1`) | Nested form `openrouter/<provider>/<model>`. |
| `moonshot/` | OpenAI-compatible | `MOONSHOT_API_KEY` | optional (`MOONSHOT_API_BASE`) | Default `https://api.moonshot.ai/v1`; China endpoint `https://api.moonshot.cn/v1`. Temp range [0,1] (auto-clamped). |
| `minimax/` | both (Anthropic Messages + OpenAI-compatible) + TTS | `MINIMAX_API_KEY` | yes (`MINIMAX_API_BASE`) | Anthropic endpoint `https://api.minimax.io/anthropic/v1/messages`; OpenAI endpoint `https://api.minimax.io/v1`. Only M2-series documented. |
| `zai/` | OpenAI-compatible | `ZAI_API_KEY` | NOT documented | Prefix is `zai/` (NO hyphen). Only GLM-4.x documented; GLM-5.1 NOT here. |
| `hosted_vllm/` | OpenAI-compatible | none required (`HOSTED_VLLM_API_KEY` optional) | yes (`HOSTED_VLLM_API_BASE` or `api_base`) | CURRENT/RECOMMENDED for vLLM HTTP servers. `vllm/` is DEPRECATED (in-process SDK). Keyless alternative to `openai/`. |
| `ollama/` | native Ollama (`/api/generate`) | none required | yes (default `http://localhost:11434`) | `ollama_chat/` recommended for `/api/chat`. No API key needed. |
| `ollama_chat/` | native Ollama (`/api/chat`) | none required | yes (default `http://localhost:11434`) | RECOMMENDED over `ollama/` for chat. |
| `litellm_proxy/` | OpenAI-compatible | `LITELLM_PROXY_API_KEY`, `LITELLM_PROXY_API_BASE` | yes (`api_base`) | Chained-proxy pattern. `USE_LITELLM_PROXY=True` (v1.72.1+) routes all SDK requests through proxy. |
| `inception/` | OpenAI-compatible | `INCEPTION_API_KEY` | NOT configurable (fixed `https://api.inceptionlabs.ai/v1`) | Diffusion LLMs (Mercury). `text-completion-inception/` for FIM. `reasoning_effort="instant"` is Inception-specific. |

## `api_base` rules by prefix

| Prefix | `api_base` rule |
|--------|-----------------|
| `openai/` | MUST include `/v1` postfix (e.g. `https://api.neuralwatt.com/v1`). Do NOT append endpoint paths (`/v1/embedding`) — the openai-client adds them. |
| `anthropic/` | LiteLLM auto-appends `/v1/messages` (or `/v1/complete`). Set `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` to prevent auto-append. Do NOT include `/v1/messages` in `api_base` unless disabling the suffix. |
| `openrouter/` | Defaults to `https://openrouter.ai/api/v1`; usually omit `api_base`. |
| `moonshot/` | Default `https://api.moonshot.ai/v1`; override via `MOONSHOT_API_BASE` for China endpoint. |
| `minimax/` | Full path: `https://api.minimax.io/anthropic/v1/messages` (Anthropic) or `https://api.minimax.io/v1` (OpenAI). |
| `hosted_vllm/` | Server URL, e.g. `http://localhost:8000`. No `/v1` requirement documented. |
| `ollama/` / `ollama_chat/` | Default `http://localhost:11434`. |
| `inception/` | Fixed `https://api.inceptionlabs.ai/v1` — not configurable. |

## `api_key` rules

- Most providers: `api_key: os.environ/<PROVIDER>_API_KEY` in `litellm_params`,
  or the `<PROVIDER>_API_KEY` env var.
- `openai/` requires a key for EVERY request (even keyless endpoints — pass a
  fake key, or use `hosted_vllm/` instead).
- `hosted_vllm/`, `ollama/`, `ollama_chat/` do NOT require a key.
- `anthropic/` defaults `max_tokens=4096` if not passed (Anthropic API requires it).

## OpenRouter nested model format

```text
openrouter/<provider>/<model>
```

The middle segment is OpenRouter's provider namespace, which may differ from
LiteLLM's native prefix:

- OpenRouter `z-ai/` (hyphenated) vs LiteLLM native `zai/` (no hyphen) —
  **distinct routing paths**.
- Example: `openrouter/z-ai/glm-5.1` (correct for GLM-5.1, which is NOT on the
  native `zai/` page — only GLM-4.x).

## Common Mistakes

| Mistake | Cause | Fix |
|---------|-------|-----|
| "Not Found Error" on `openai/` | `api_base` missing `/v1` | Add `/v1` postfix. |
| Appended `/v1/embedding` to `api_base` | Misunderstanding openai-client | Remove; client adds endpoints. |
| `anthropic/` `api_base` includes `/v1/messages` | Double path | Remove suffix; LiteLLM auto-appends (or set `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true`). |
| `zai/glm-5.1` used | GLM-5.1 not on native Z.AI page | Use `openrouter/z-ai/glm-5.1`. |
| `z-ai/` used as native prefix | Hyphen confusion | Native is `zai/` (no hyphen); `z-ai/` is OpenRouter-only. |
| `vllm/` prefix used | Deprecated | Use `hosted_vllm/` for HTTP servers. |
| `ollama/` for chat | Suboptimal responses | Use `ollama_chat/` for `/api/chat`. |
| Fake key omitted on keyless `openai/` endpoint | openai-client requires a key | Pass any non-empty string, or switch to `hosted_vllm/`. |
| `minimax/MiniMax-M3` | M3 not documented (only M2-series) | Verify model name; M3 may be newer than docs. |
| `moonshot/` for Kimi coding endpoint | Wrong protocol | Kimi coding endpoint speaks Anthropic Messages API; use `anthropic/` + custom `api_base` **[PROJECT]**. |

## Project context [PROJECT — not upstream docs]

The workestrate config (`infra/litellm/config.yaml`) uses these prefixes:

| Prefix | Used for | Notes |
|--------|----------|-------|
| `anthropic/` | `kimi-for-coding`, `MiniMax-M3` | Custom `api_base`; both endpoints speak Anthropic Messages API. Deviation: docs use `minimax/` for MiniMax. |
| `openrouter/` | `z-ai/glm-5.1`, `qwen/qwen3.7-plus`, `qwen/qwen3-coder:free`, `nex-agi/nex-n2-pro:free` | Nested form; no `api_base` (uses OpenRouter default). |
| `openai/` | `neuralwatt` | `api_base: https://api.neuralwatt.com/v1` (includes `/v1`). |

Project-defined env vars (NOT LiteLLM built-ins; empty `source_urls` in
`env-vars.index.json`): `KIMI_CODE_API_KEY`, `MINIMAX_CODING_API_KEY`,
`NEURALWATT_API_KEY`. Plus `OPENROUTER_API_KEY` (provider env, built-in).

## Related Docs

- `docs/litellm/providers/README.md`
- `docs/litellm/providers/openai-compatible.md`
- `docs/litellm/providers/openai.md`
- `docs/litellm/providers/anthropic.md`
- `docs/litellm/providers/openrouter.md`
- `docs/litellm/providers/moonshot.md`
- `docs/litellm/providers/minimax.md`
- `docs/litellm/providers/zai-glm.md`
- `docs/litellm/providers/vllm.md`
- `docs/litellm/providers/ollama.md`
- `docs/litellm/providers/litellm-proxy.md`
- `docs/litellm/providers/inception.md`
- `docs/litellm/schemas/provider-fields.index.json`
