---
source_url: https://docs.litellm.ai/docs/providers/zai
canonical_url: unknown
title: Z.AI (Zhipu AI) | liteLLM
sidebar_section_path: Supported Models & Providers > Z.AI (Zhipu AI)
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: Z.AI (Zhipu AI)
litellm_provider_prefix: zai/
---
# Z.AI (Zhipu AI) | liteLLM

## Provider prefix / route
- LiteLLM prefix: `zai/` — "We support Z.AI GLM text/chat models, just set `zai/` as a prefix when sending completion requests"
- CRITICAL: The prefix is `zai/` (no hyphen). This differs from OpenRouter's `z-ai/` (hyphenated) namespace.
- API protocol: OpenAI-compatible (chat completions via `/v1/chat/completions`)

## Example model strings (verbatim)
- `zai/glm-4.7` — Latest flagship, 200K context, Reasoning
- `zai/glm-4.6` — 200K context
- `zai/glm-4.5` — 128K context
- `zai/glm-4.5v` — Vision model
- `zai/glm-4.5-x` — Premium tier
- `zai/glm-4.5-air` — Lightweight
- `zai/glm-4.5-airx` — Fast lightweight
- `zai/glm-4-32b-0414-128k` — 32B parameter model
- `zai/glm-4.5-flash` — FREE tier

NOTE: `zai/glm-5.1` is NOT documented on this page. Only GLM-4.x family models are listed. GLM-5.1 must be accessed via OpenRouter (`openrouter/z-ai/glm-5.1`).

## Required environment variables
- `ZAI_API_KEY` — API key for Z.AI

## Optional environment variables
- (none documented)

## api_base behavior
- NOT documented on this page. No `api_base` is shown in any example. The URL `https://z.ai/` appears as the provider homepage link only.

## api_key behavior
- Env var: `ZAI_API_KEY`
- Verbatim: `os.environ['ZAI_API_KEY'] = ""`
- In proxy YAML: `api_key: os.environ/ZAI_API_KEY`

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- not documented on this page

## Proxy config example (verbatim YAML)
```yaml
model_list:
  - model_name: glm-4.7
    litellm_params:
        model: zai/glm-4.7
        api_key: os.environ/ZAI_API_KEY

  - model_name: glm-4.5-flash  # Free tier
    litellm_params:
        model: zai/glm-4.5-flash
        api_key: os.environ/ZAI_API_KEY
```
(source: https://docs.litellm.ai/docs/providers/zai)

## SDK example (verbatim, if present)
```python
from litellm import completion
import os
os.environ['ZAI_API_KEY'] = ""
response = completion(
    model="zai/glm-4.7",
    messages=[
       {"role": "user", "content": "hello from litellm"}
   ],
)
print(response)
```

Streaming:
```python
from litellm import completion
import os
os.environ['ZAI_API_KEY'] = ""
response = completion(
    model="zai/glm-4.7",
    messages=[
       {"role": "user", "content": "hello from litellm"}
   ],
    stream=True
)
for chunk in response:
    print(chunk)
```

## Supported endpoints (chat/completions, embeddings, etc.)
- `/v1/chat/completions` (chat/completion via `litellm.completion`) — only endpoint demonstrated

## Special params / notes
- Model pricing table (verbatim):
  - glm-4.7: $0.60/M input, $2.20/M output, $0.11/M cached, 200K context
  - glm-4.6: $0.60/M input, $2.20/M output, 200K context
  - glm-4.5: $0.60/M input, $2.20/M output, 128K context
  - glm-4.5v: $0.60/M input, $1.80/M output, 128K context
  - glm-4.5-x: $2.20/M input, $8.90/M output, 128K context
  - glm-4.5-air: $0.20/M input, $1.10/M output, 128K context
  - glm-4.5-airx: $1.10/M input, $4.50/M output, 128K context
  - glm-4-32b-0414-128k: $0.10/M input, $0.10/M output, 128K context
  - glm-4.5-flash: FREE, 128K context

## Custom pricing / context window behavior
- Documented in model pricing table above. Context windows: 200K (glm-4.7, glm-4.6), 128K (all others).

## Known caveats
- The prefix is `zai/` (no hyphen) — NOT `z-ai/` (hyphenated). OpenRouter uses `z-ai/` for the same provider.
- GLM-5.1 is NOT documented on this page — only GLM-4.x models.
- `api_base` is NOT documented — the exact Z.AI API base URL is not stated on this page.
- Provider name variations: "Z.AI" (primary), "Zhipu AI" (parenthesized alias).

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://z.ai/

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- The repo uses `openrouter/z-ai/glm-5.1` (via OpenRouter). This is the CORRECT path for GLM-5.1 since it's not documented on the native Z.AI provider page.
- CRITICAL SPELLING DIFFERENCE: LiteLLM native prefix is `zai/` (no hyphen); OpenRouter namespace is `z-ai/` (hyphenated). These are distinct routing paths.
- Do NOT change `openrouter/z-ai/glm-5.1` to `zai/glm-5.1` — GLM-5.1 is not in the native Z.AI docs and would likely fail routing.
- If direct Z.AI access is desired, use `zai/glm-4.7` (latest flagship) or `zai/glm-4.5-flash` (free tier).
- The repo's GLM-5.1 access via OpenRouter is the correct approach since GLM-5.1 is not yet in LiteLLM's native Z.AI docs.

## Confidence / uncertainty notes
- GLM-5.1 is NOT documented — only GLM-4.x. The model may be newer than the docs or only available via OpenRouter.
- `api_base` is NOT documented on this page — the exact Z.AI API endpoint URL is unknown from this page alone.
- HTTP status inferred from successful full-page content delivery.
