# MiniMax

> Source: https://docs.litellm.ai/docs/providers/minimax

## Prefix

`minimax/` — used for BOTH Anthropic Messages API and OpenAI-compatible API. "Litellm provides anthropic specs compatible support for minmax"

## Protocol

both — Anthropic Messages-compatible (`/v1/messages`) AND OpenAI-compatible (`/v1/chat/completions`), plus TTS (`/audio/speech`)

## Environment variables

- Required: `MINIMAX_API_KEY` — API key for MiniMax
- Optional: `MINIMAX_API_BASE` — custom API base URL

## api_base behavior

Anthropic Messages endpoint: `api_base="https://api.minimax.io/anthropic/v1/messages"` (verbatim). OpenAI-compatible endpoint: `api_base="https://api.minimax.io/v1"` (verbatim). Env var: `MINIMAX_API_BASE` — set to `https://api.minimax.io/anthropic/v1/messages` or `https://api.minimax.io/v1`. In proxy YAML: `api_base: https://api.minimax.io/anthropic/v1/messages` or `api_base: https://api.minimax.io/v1`.

## api_key behavior

Direct param: `api_key="your-minimax-api-key"`. Env var: `MINIMAX_API_KEY`. In proxy YAML: `api_key: os.environ/MINIMAX_API_KEY`

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

Anthropic Messages endpoint:

```yaml
model_list:
  - model_name: minimax/MiniMax-M2.1
    litellm_params:
      model: minimax/MiniMax-M2.1
      api_key: os.environ/MINIMAX_API_KEY
      api_base: https://api.minimax.io/anthropic/v1/messages
```

OpenAI-compatible endpoint:

```yaml
model_list:
  - model_name: minimax/MiniMax-M2.1
    litellm_params:
      model: minimax/MiniMax-M2.1
      api_key: os.environ/MINIMAX_API_KEY
      api_base: https://api.minimax.io/v1
```

## Supported endpoints

- Anthropic Messages (`/v1/messages`) — via `litellm.anthropic.messages.acreate()`
- OpenAI Chat Completions (`/v1/chat/completions`) — via `litellm.completion()`
- Audio/Speech (`/v1/audio/speech`) — via `litellm.speech()` / `litellm.aspeech()`

## Special params / notes

- `reasoning_split: True` (via `extra_body`) — separates thinking content on OpenAI-compatible endpoint.
- `thinking={"type": "enabled", "budget_tokens": 1000}` — on Anthropic Messages endpoint (M2.1 feature).
- TTS voice mappings: alloy→male-qn-qingse, echo→male-qn-jingying, fable→female-shaonv, onyx→male-qn-badao, nova→female-yujie, shimmer→female-tianmei.
- TTS extra_body params: `vol` (0.1-10), `pitch` (-12 to 12), `sample_rate` (16000/24000/32000), `bitrate` (64000/128000/192000/256000), `channel` (1/2), `output_format` ("hex"/"url").
- Cost calculation works automatically using model_prices_and_context_window.json.

## Caveats

- MiniMax exposes BOTH an Anthropic Messages-compatible endpoint AND an OpenAI Chat Completions-compatible endpoint.
- The Anthropic-compatible endpoint is documented first and is the primary path for M2-series models (thinking, tool-use).
- The OpenAI-compatible endpoint uses `reasoning_split` instead of native Anthropic thinking blocks.
- `minimax/MiniMax-M3` is NOT documented — only M2-series models (M2, M2.1, M2.1-lightning).

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]

CRITICAL FINDING: The repo uses `anthropic/MiniMax-M3` with `api_base: api.minimax.io/anthropic`, but the documented pattern on this page is `minimax/MiniMax-M2.1` with `api_base: https://api.minimax.io/anthropic/v1/messages`. The documented approach uses the `minimax/` prefix (NOT `anthropic/`) even when calling the Anthropic Messages endpoint, via `litellm.anthropic.messages.acreate(model="minimax/MiniMax-M2.1", api_base="https://api.minimax.io/anthropic/v1/messages")`. The repo's `anthropic/MiniMax-M3` approach may work (since the Anthropic page documents custom api_base), but it differs from the documented `minimax/` prefix pattern. `MiniMax-M3` is NOT in the docs — only M2-series. The repo may be using a newer model not yet documented, or the model name may need verification. The api_base `api.minimax.io/anthropic` in the repo matches the documented `https://api.minimax.io/anthropic/v1/messages` (the repo omits the `/v1/messages` suffix, which LiteLLM would auto-append via the `anthropic/` prefix).

## Confidence / uncertainty

- `minimax/MiniMax-M3` is NOT documented — only M2-series models. The M3 model may be newer than the docs.
- The repo's use of `anthropic/` prefix (vs documented `minimax/` prefix) for MiniMax is a deviation from the documented pattern. Both may work but the documented approach is `minimax/`.
- HTTP status inferred from successful full-page content delivery.
