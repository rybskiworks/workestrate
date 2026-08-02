---
source_url: https://docs.litellm.ai/docs/providers/minimax
canonical_url: unknown
title: MiniMax | liteLLM
sidebar_section_path: Supported Models & Providers > MiniMax
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: MiniMax
litellm_provider_prefix: minimax/
---
# MiniMax | liteLLM

## Provider prefix / route
- LiteLLM prefix: `minimax/` — used for BOTH Anthropic Messages API and OpenAI-compatible API.
- API protocol: BOTH — Anthropic Messages-compatible (`/v1/messages`) AND OpenAI-compatible (`/v1/chat/completions`), plus TTS (`/audio/speech`).
- Verbatim: "Litellm provides anthropic specs compatible support for minmax"

## Example model strings (verbatim)
- `minimax/MiniMax-M2.1` — chat (Anthropic + OpenAI), tool calling, thinking, streaming
- `minimax/MiniMax-M2.1-lightning` — model table (faster variant)
- `minimax/MiniMax-M2` — model table (agentic capabilities)
- `minimax/speech-2.6-hd` — TTS
- `minimax/speech-2.6-turbo` — TTS

NOTE: `minimax/MiniMax-M3` is NOT on this page. The page documents M2-series models only. The repo's use of `anthropic/MiniMax-M3` references a model not yet in the docs.

## Required environment variables
- `MINIMAX_API_KEY` — API key for MiniMax

## Optional environment variables
- `MINIMAX_API_BASE` — custom API base URL

## api_base behavior
- Anthropic Messages endpoint: `api_base="https://api.minimax.io/anthropic/v1/messages"` (verbatim)
- OpenAI-compatible endpoint: `api_base="https://api.minimax.io/v1"` (verbatim)
- Env var: `MINIMAX_API_BASE` — set to `https://api.minimax.io/anthropic/v1/messages` or `https://api.minimax.io/v1`
- In proxy YAML: `api_base: https://api.minimax.io/anthropic/v1/messages` or `api_base: https://api.minimax.io/v1`

## api_key behavior
- Direct param: `api_key="your-minimax-api-key"`
- Env var: `MINIMAX_API_KEY`
- In proxy YAML: `api_key: os.environ/MINIMAX_API_KEY`

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- not documented on this page

## Proxy config example (verbatim YAML)
Anthropic Messages endpoint:
```yaml
model_list:
  - model_name: minimax/MiniMax-M2.1
    litellm_params:
      model: minimax/MiniMax-M2.1
      api_key: os.environ/MINIMAX_API_KEY
      api_base: https://api.minimax.io/anthropic/v1/messages
```
(source: https://docs.litellm.ai/docs/providers/minimax)

OpenAI-compatible endpoint:
```yaml
model_list:
  - model_name: minimax/MiniMax-M2.1
    litellm_params:
      model: minimax/MiniMax-M2.1
      api_key: os.environ/MINIMAX_API_KEY
      api_base: https://api.minimax.io/v1
```
(source: https://docs.litellm.ai/docs/providers/minimax)

TTS:
```yaml
model_list:
  - model_name: tts
    litellm_params:
      model: minimax/speech-2.6-hd
      api_key: os.environ/MINIMAX_API_KEY
  - model_name: tts-turbo
    litellm_params:
      model: minimax/speech-2.6-turbo
      api_key: os.environ/MINIMAX_API_KEY
```
(source: https://docs.litellm.ai/docs/providers/minimax)

## SDK example (verbatim, if present)
Anthropic Messages API (via litellm.anthropic.messages.acreate):
```python
import litellm
response = litellm.anthropic.messages.acreate(
    model="minimax/MiniMax-M2.1",
    messages=[{"role": "user", "content": "Hello, how are you?"}],
    api_key="your-minimax-api-key",
    api_base="https://api.minimax.io/anthropic/v1/messages",
    max_tokens=1000
)
print(response.choices[0].message.content)
```

OpenAI-compatible (via litellm.completion):
```python
import litellm
response = litellm.completion(
    model="minimax/MiniMax-M2.1",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello, how are you?"}
    ],
    api_key="your-minimax-api-key",
    api_base="https://api.minimax.io/v1"
)
print(response.choices[0].message.content)
```

## Supported endpoints (chat/completions, embeddings, etc.)
- Anthropic Messages (`/v1/messages`) — via `litellm.anthropic.messages.acreate()`
- OpenAI Chat Completions (`/v1/chat/completions`) — via `litellm.completion()`
- Audio/Speech (`/v1/audio/speech`) — via `litellm.speech()` / `litellm.aspeech()`
- WebSocket TTS — NOT supported by LiteLLM (refers to platform.minimax.io docs)

## Special params / notes
- `reasoning_split: True` (via `extra_body`) — separates thinking content on OpenAI-compatible endpoint.
- `thinking={"type": "enabled", "budget_tokens": 1000}` — on Anthropic Messages endpoint (M2.1 feature).
- TTS voice mappings: alloy→male-qn-qingse, echo→male-qn-jingying, fable→female-shaonv, onyx→male-qn-badao, nova→female-yujie, shimmer→female-tianmei.
- TTS extra_body params: `vol` (0.1-10), `pitch` (-12 to 12), `sample_rate` (16000/24000/32000), `bitrate` (64000/128000/192000/256000), `channel` (1/2), `output_format` ("hex"/"url").
- Cost calculation works automatically using model_prices_and_context_window.json.

## Custom pricing / context window behavior
- Model pricing table (verbatim):
  - MiniMax-M2.1: $0.3/M input, $1.2/M output, $0.03/M cached read, $0.375/M cached write
  - MiniMax-M2.1-lightning: $0.3/M input, $2.4/M output
  - MiniMax-M2: $0.3/M input, $1.2/M output

## Known caveats
- MiniMax exposes BOTH an Anthropic Messages-compatible endpoint AND an OpenAI Chat Completions-compatible endpoint.
- The Anthropic-compatible endpoint is documented first and is the primary path for M2-series models (thinking, tool-use).
- The OpenAI-compatible endpoint uses `reasoning_split` instead of native Anthropic thinking blocks.
- `minimax/MiniMax-M3` is NOT documented — only M2-series models (M2, M2.1, M2.1-lightning).

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://platform.minimax.io/docs

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- CRITICAL FINDING: The repo uses `anthropic/MiniMax-M3` with `api_base: api.minimax.io/anthropic`, but the documented pattern on this page is `minimax/MiniMax-M2.1` with `api_base: https://api.minimax.io/anthropic/v1/messages`.
- The documented approach uses the `minimax/` prefix (NOT `anthropic/`) even when calling the Anthropic Messages endpoint, via `litellm.anthropic.messages.acreate(model="minimax/MiniMax-M2.1", api_base="https://api.minimax.io/anthropic/v1/messages")`.
- The repo's `anthropic/MiniMax-M3` approach may work (since the Anthropic page documents custom api_base), but it differs from the documented `minimax/` prefix pattern.
- `MiniMax-M3` is NOT in the docs — only M2-series. The repo may be using a newer model not yet documented, or the model name may need verification.
- The api_base `api.minimax.io/anthropic` in the repo matches the documented `https://api.minimax.io/anthropic/v1/messages` (the repo omits the `/v1/messages` suffix, which LiteLLM would auto-append via the anthropic/ prefix — see p1-provider-anthropic.md).

## Confidence / uncertainty notes
- `minimax/MiniMax-M3` is NOT documented — only M2-series models. The M3 model may be newer than the docs.
- The repo's use of `anthropic/` prefix (vs documented `minimax/` prefix) for MiniMax is a deviation from the documented pattern. Both may work but the documented approach is `minimax/`.
- HTTP status inferred from successful full-page content delivery.
