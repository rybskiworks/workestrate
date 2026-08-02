# LiteLLM Provider Index

This index covers 11 LiteLLM providers extracted from on-disk source files under `docs/litellm/extracted/`. No web fetches were performed. Each row links to a per-provider doc and the upstream LiteLLM source URL.

| Provider | LiteLLM Prefix | API Protocol | Workestrate Usage |
|---|---|---|---|
| [OpenAI-Compatible Endpoints](openai-compatible.md) | `openai/` | OpenAI-compatible | neural tier (`openai/neuralwatt`) |
| [Anthropic](anthropic.md) | `anthropic/` | both (OpenAI-compatible + Anthropic Messages) | `coding`, `coding.fast`, `coding.pro-fallback` (`kimi-for-coding`, `MiniMax-M3`) |
| [OpenRouter](openrouter.md) | `openrouter/` | OpenAI-compatible | `coding.pro`, `coding.fast-fallback`, `coding.free` (GLM-5.1, Qwen, Nex) |
| [Moonshot AI](moonshot.md) | `moonshot/` | OpenAI-compatible | NOT used (`kimi-for-coding` uses `anthropic/` instead) |
| [MiniMax](minimax.md) | `minimax/` | both | NOT used directly (uses `anthropic/MiniMax-M3` — see deviation) |
| [Z.AI (Zhipu AI)](zai-glm.md) | `zai/` | OpenAI-compatible | NOT used (GLM-5.1 via `openrouter/z-ai/`) |
| [vLLM](vllm.md) | `hosted_vllm/` (current), `vllm/` deprecated | OpenAI-compatible | Not used |
| [Ollama](ollama.md) | `ollama/`, `ollama_chat/` (recommended) | native Ollama | Not used |
| [LiteLLM Proxy](litellm-proxy.md) | `litellm_proxy/` | OpenAI-compatible | Not used |
| [Inception](inception.md) | `inception/` | OpenAI-compatible | Not used |
| [OpenAI](openai.md) | `openai/` | OpenAI native | Not used |

## OpenAI-compatible endpoints (the general pattern)

The `openai/` prefix routes to an OpenAI-compatible endpoint using the upstream official OpenAI Python API library. This is the Neuralwatt pattern used in this repo.

CRITICAL gotchas from the extraction:

- `api_base` **MUST** include the `/v1` postfix. Example: `https://api.neuralwatt.com/v1`. If you see a "Not Found Error", the base URL is missing `/v1`.
- Do **NOT** append endpoint paths like `/v1/embedding` to `api_base`. LiteLLM uses the openai-client, which automatically adds the relevant endpoints.
- The openai-client requires an API key for every request. For keyless endpoints, use `hosted_vllm/` instead.

See [openai-compatible.md](openai-compatible.md) for the full extraction and [openai.md](openai.md) for the OpenAI-specific variant.

## Anthropic-Messages-compatible endpoints via custom api_base

The `anthropic/` prefix + custom `api_base` is the documented way to reach endpoints that speak the Anthropic Messages API, including non-Anthropic endpoints such as Kimi's coding endpoint and MiniMax's Anthropic endpoint.

Key points:

- Set `ANTHROPIC_API_BASE` (or pass `api_base=...`) to the custom endpoint.
- LiteLLM auto-appends `/v1/messages` to the base URL unless `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` is set.
- This is how the repo reaches `anthropic/kimi-for-coding` (`https://api.kimi.com/coding`) and `anthropic/MiniMax-M3` (`https://api.minimax.io/anthropic`).

See [anthropic.md](anthropic.md).

## Direct native providers

These providers use their own prefixes and are documented directly by LiteLLM:

- [Moonshot AI](moonshot.md) — `moonshot/` prefix; OpenAI-compatible endpoint at `https://api.moonshot.ai/v1`.
- [MiniMax](minimax.md) — `minimax/` prefix; supports BOTH Anthropic Messages (`/v1/messages`) and OpenAI-compatible (`/v1/chat/completions`) endpoints, plus TTS.
- [Z.AI (Zhipu AI)](zai-glm.md) — `zai/` prefix (no hyphen); OpenAI-compatible; GLM-4.x family only.
- [Inception](inception.md) — `inception/` prefix; OpenAI-compatible endpoint with fixed base URL `https://api.inceptionlabs.ai/v1`.

## OpenRouter

[OpenRouter](openrouter.md) uses a **nested** model string format:

```text
openrouter/<provider>/<model>
```

For example, `openrouter/z-ai/glm-5.1`. The middle segment is OpenRouter's provider namespace, which may differ from LiteLLM's native provider prefix (e.g. OpenRouter `z-ai/` vs native LiteLLM `zai/`).

## Local/self-hosted (vLLM/Ollama)

- [vLLM](vllm.md): use `hosted_vllm/` for OpenAI-compatible HTTP servers. `vllm/` is deprecated and meant for in-process vLLM SDK usage.
- [Ollama](ollama.md): use `ollama_chat/` for `/api/chat` (recommended), `ollama/` for `/api/generate` and FIM. Uses the native Ollama API, not OpenAI-compatible.

## Chained proxy

[LiteLLM Proxy](litellm-proxy.md) (`litellm_proxy/`) is the chained-proxy pattern: one LiteLLM proxy/SDK calling another LiteLLM proxy. Useful for centralized gateway management.

## Workestrate notes

The following are project-specific choices in `infra/litellm/config.yaml`. They are not necessarily the only upstream-documented way to reach these upstreams.

1. **`kimi-for-coding` uses `anthropic/`, not `moonshot/`**  
   The repo routes `anthropic/kimi-for-coding` to `https://api.kimi.com/coding`. Kimi's coding endpoint speaks the Anthropic Messages API, not the OpenAI-compatible Moonshot API. Therefore the repo intentionally uses the `anthropic/` prefix with a custom `api_base` instead of the documented `moonshot/` prefix. LiteLLM auto-appends `/v1/messages`, resulting in `https://api.kimi.com/coding/v1/messages`. This is a project choice.

2. **`MiniMax-M3` uses `anthropic/`, not documented `minimax/`**  
   The repo routes `anthropic/MiniMax-M3` to `https://api.minimax.io/anthropic`. The documented MiniMax page uses `minimax/MiniMax-M2.1` with `api_base: https://api.minimax.io/anthropic/v1/messages`. The repo omits the `/v1/messages` suffix because the `anthropic/` prefix auto-appends it, and it uses a model name (`MiniMax-M3`) that is not in the extracted docs. Both approaches may work; the documented approach is `minimax/`. This is a project choice.

3. **GLM-5.1 is accessed via `openrouter/z-ai/glm-5.1`**  
   GLM-5.1 is not documented on the native Z.AI provider page, which only lists GLM-4.x models. The repo correctly uses OpenRouter's `z-ai/` namespace. Do not change this to native `zai/glm-5.1`.

---

Sources: upstream URLs are recorded in each per-provider doc and in [`provider-fields.index.json`](../schemas/provider-fields.index.json).
