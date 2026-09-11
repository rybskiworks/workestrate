---
source_url: https://docs.litellm.ai/docs/providers/openrouter
canonical_url: unknown
title: OpenRouter | liteLLM
sidebar_section_path: Supported Models & Providers > OpenRouter
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: OpenRouter
litellm_provider_prefix: openrouter/
---
# OpenRouter | liteLLM

## Provider prefix / route
- LiteLLM prefix: `openrouter/` — "LiteLLM supports ALL OpenRouter models, send `model=openrouter/<your-openrouter-model>` to send it to open router."
- API protocol: OpenAI-compatible

## Example model strings (verbatim)
- `openrouter/openai/gpt-3.5-turbo` — model table
- `openrouter/openai/gpt-3.5-turbo-16k` — model table
- `openrouter/openai/gpt-4` — model table
- `openrouter/openai/gpt-4-32k` — model table
- `openrouter/anthropic/claude-2` — model table
- `openrouter/anthropic/claude-instant-v1` — model table
- `openrouter/google/palm-2-chat-bison` — model table + usage example
- `openrouter/google/palm-2-codechat-bison` — model table
- `openrouter/meta-llama/llama-2-13b-chat` — model table
- `openrouter/meta-llama/llama-2-70b-chat` — model table
- `openrouter/openai/text-embedding-3-small` — embedding example
- `openrouter/google/gemini-2.5-flash-image` — image generation/edit example

CRITICAL: The model string format is NESTED: `openrouter/<provider>/<model>` (three segments). The middle segment is the OpenRouter provider namespace (e.g. `openai`, `anthropic`, `google`, `meta-llama`).

## Required environment variables
- `OPENROUTER_API_KEY` — API key for OpenRouter

## Optional environment variables
- `OPENROUTER_API_BASE` — "[OPTIONAL] defaults to https://openrouter.ai/api/v1"
- `OR_SITE_URL` — [OPTIONAL]
- `OR_APP_NAME` — [OPTIONAL]

## api_base behavior
- Env var: `OPENROUTER_API_BASE` — defaults to `https://openrouter.ai/api/v1`
- Can also be passed as `base_url=` kwarg to `completion()`
- Verbatim: `os.environ["OPENROUTER_API_BASE"] = "" # [OPTIONAL] defaults to https://openrouter.ai/api/v1`

## api_key behavior
- Env var: `OPENROUTER_API_KEY`
- Verbatim: `os.environ["OPENROUTER_API_KEY"] = ""`

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- not documented on this page

## Proxy config example (verbatim YAML)
- NOT documented on this page. The page shows only Python SDK examples. No `config.yaml` / `model_list` YAML is present.
- (inferred) YAML would follow standard pattern: `model: openrouter/<provider>/<model>`, `api_key: os.environ/OPENROUTER_API_KEY`

## SDK example (verbatim, if present)
```python
import os
from litellm import completion
os.environ["OPENROUTER_API_KEY"] = ""
os.environ["OPENROUTER_API_BASE"] = "" # [OPTIONAL] defaults to https://openrouter.ai/api/v1
os.environ["OR_SITE_URL"] = "" # [OPTIONAL]
os.environ["OR_APP_NAME"] = "" # [OPTIONAL]

response = completion(
            model="openrouter/google/palm-2-chat-bison",
            messages=messages,
        )
```

Embedding SDK example:
```python
from litellm import embedding
import os
os.environ["OPENROUTER_API_KEY"] = "your-api-key"

response = embedding(
    model="openrouter/openai/text-embedding-3-small",
    input=["good morning from litellm", "this is another item"],
)
print(response)
```

## Supported endpoints (chat/completions, embeddings, etc.)
- `litellm.completion()` (chat/text)
- `litellm.embedding()` (embeddings)
- `litellm.image_generation()` (image generation)
- `litellm.image_edit()` (image editing)
- Pass-through params: `transforms`, `models`, `route`
- Image-config params: `image_config={aspect_ratio, image_size}`

## Special params / notes
- OpenRouter-specific params: `transforms`, `models`, `route` — passed as arguments to `litellm.completion()`
- Image generation: `size` maps to `aspect_ratio` (1024x1024 → 1:1, etc.), `quality` maps to `image_size` (low/standard → 1K, medium → 2K, high/hd → 4K)
- `image_config` dict for native OpenRouter params: `aspect_ratio`, `image_size`
- Cost tracking: `response._hidden_params['additional_headers']['llm_provider-x-litellm-response-cost']`

## Custom pricing / context window behavior
- not documented on this page (OpenRouter provides cost info in response, LiteLLM tracks automatically)

## Known caveats
- The model table on the page is small/legacy and does not enumerate current OpenRouter offerings.
- LiteLLM claims to support "ALL OpenRouter models" via the `openrouter/<any-model>` generic pattern.
- No proxy YAML config is shown on this page.

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://openrouter.ai/docs
- https://openrouter.ai/models (all models)

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- The repo uses `openrouter/z-ai/glm-5.1`, `openrouter/qwen/...`, and `openrouter/nex/...` style model strings.
- CONFIRMED: The nested format `openrouter/<provider>/<model>` is the correct documented pattern.
- `openrouter/z-ai/glm-5.1` is structurally valid (z-ai is OpenRouter's namespace for Z.AI/Zhipu). Note: OpenRouter uses `z-ai/` (hyphenated) while LiteLLM's native Z.AI provider uses `zai/` (no hyphen) — see p1-provider-zai.md.
- `openrouter/qwen/...` and `openrouter/nex/...` are structurally consistent with the documented nested pattern, though not explicitly shown on the page.
- GLM-5.1 is NOT documented on the native Z.AI provider page (only GLM-4.x). OpenRouter is the correct path for GLM-5.1.

## Confidence / uncertainty notes
- No YAML config example on this page — YAML shape is inferred from standard LiteLLM patterns.
- `openrouter/z-ai/glm-5.1`, `openrouter/qwen/...`, `openrouter/nex/...` are NOT verbatim on the page but follow the documented nested format.
- HTTP status inferred from successful full-page content delivery.
