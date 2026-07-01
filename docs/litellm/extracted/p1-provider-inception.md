---
source_url: https://docs.litellm.ai/docs/providers/inception
canonical_url: unknown
title: Inception | liteLLM
sidebar_section_path: Supported Models & Providers > Inception
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: Inception
litellm_provider_prefix: inception/
---
# Inception | liteLLM

## Provider prefix / route
- LiteLLM prefix: `inception/` (chat), `text-completion-inception/` (fill-in-the-middle)
- Verbatim: "Provider Route on LiteLLM | `inception/` (chat), `text-completion-inception/` (fill-in-the-middle)"
- API protocol: OpenAI-compatible ("The API is OpenAI-compatible.")
- Base URL: `https://api.inceptionlabs.ai/v1`

## Example model strings (verbatim)
- `inception/mercury-2` — Fast reasoning chat model; supports tool calling and structured outputs (128,000 tokens context)
- `text-completion-inception/mercury-edit-2` — Code model for fill-in-the-middle (FIM) autocomplete (32,000 tokens context)

## Required environment variables
- `INCEPTION_API_KEY` — "your Inception API key"

## Optional environment variables
- (none documented)

## api_base behavior
- NOT explicitly configurable in examples — fixed base URL is `https://api.inceptionlabs.ai/v1`
- Verbatim: "Base URL | `https://api.inceptionlabs.ai/v1`"

## api_key behavior
- Env var: `INCEPTION_API_KEY`
- Verbatim: `os.environ["INCEPTION_API_KEY"] = ""  # your Inception API key`
- In proxy YAML: `api_key: os.environ/INCEPTION_API_KEY`

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- not documented on this page

## Proxy config example (verbatim YAML)
```yaml
model_list:
  - model_name: mercury-2
    litellm_params:
      model: inception/mercury-2
      api_key: os.environ/INCEPTION_API_KEY
  - model_name: mercury-edit-2
    litellm_params:
      model: text-completion-inception/mercury-edit-2
      api_key: os.environ/INCEPTION_API_KEY
```
(source: https://docs.litellm.ai/docs/providers/inception)

## SDK example (verbatim, if present)
```python
import os
import litellm
from litellm import completion

os.environ["INCEPTION_API_KEY"] = ""  # your Inception API key

messages = [{"content": "Hello, how are you?", "role": "user"}]

# Inception call
response = completion(
    model="inception/mercury-2",
    messages=messages,
)

print(response)
```

FIM (fill-in-the-middle):
```python
import os
from litellm import text_completion

os.environ["INCEPTION_API_KEY"] = ""  # your Inception API key

response = text_completion(
    model="text-completion-inception/mercury-edit-2",
    prompt="def add(a, b):\n    return ",
    suffix="\n",
    max_tokens=64,
)

print(response.choices[0].text)
```

## Supported endpoints (chat/completions, embeddings, etc.)
- `/chat/completions` (via `inception/` prefix)
- `/fim/completions` (via `text-completion-inception/` prefix — fill-in-the-middle)

## Special params / notes
- Inception-specific parameters (passed through to chat API):
  - `reasoning_effort` (`instant` | `low` | `medium` | `high`) — `instant` is Inception-specific for near real-time
  - `reasoning_summary` (bool) — return summary of model's reasoning
  - `reasoning_summary_wait` (bool) — wait for summary to complete before returning
  - `diffusing` (bool) — stream intermediate denoising steps
  - `realtime` (bool) — optimize for lowest latency
- Supported OpenAI parameters: `max_tokens`, `max_completion_tokens`, `temperature`, `stop`, `tools`, `tool_choice`, `stream`, `stream_options`, `response_format`
- Inception serves the Mercury family of diffusion LLMs (dLLMs)

## Custom pricing / context window behavior
- Context windows: mercury-2 = 128,000 tokens; mercury-edit-2 = 32,000 tokens
- Pricing: not documented on this page

## Known caveats
- Inception serves diffusion LLMs (dLLMs) — the Mercury family.
- `api_base` is NOT configurable — fixed at `https://api.inceptionlabs.ai/v1`.
- `reasoning_effort` has an Inception-specific `instant` value not found in other providers.

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://docs.inceptionlabs.ai/

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- Inception is a provider of diffusion LLMs (Mercury family). OpenAI-compatible API.
- Pattern: `model: inception/mercury-2` + `api_key: os.environ/INCEPTION_API_KEY`.
- `api_base` is fixed at `https://api.inceptionlabs.ai/v1` — not configurable.
- Supports FIM (fill-in-the-middle) via `text-completion-inception/mercury-edit-2` prefix.
- Inception-specific `reasoning_effort="instant"` for near real-time responses.

## Confidence / uncertainty notes
- `api_base` is NOT documented as configurable — the fixed URL is the only one shown.
- HTTP status confirmed as 200 from successful page fetch.
