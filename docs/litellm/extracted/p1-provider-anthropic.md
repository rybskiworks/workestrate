---
source_url: https://docs.litellm.ai/docs/providers/anthropic
canonical_url: https://docs.litellm.ai/docs/providers/anthropic
title: Anthropic | liteLLM
sidebar_section_path: Supported Models & Providers > Anthropic
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: Anthropic
litellm_provider_prefix: anthropic/
---
# Anthropic | liteLLM

## Provider prefix / route
- LiteLLM prefix: `anthropic/` — "add this prefix to the model name, to route any requests to Anthropic - e.g. `anthropic/claude-3-5-sonnet-20240620`"
- For Azure Foundry deployments: use `azure/claude-*` (see Azure Anthropic documentation)
- API protocol: both (OpenAI-compatible `/chat/completions` + Anthropic native `/v1/messages` passthrough)

## Example model strings (verbatim)
- `anthropic/claude-3-5-sonnet-20240620` — prefix example
- `anthropic/claude-3-haiku-20240307` — proxy curl example
- `anthropic/claude-3-opus-20240229` — tool calling example
- `anthropic/claude-sonnet-4-20250514` — MCP tool calling example
- `anthropic/claude-sonnet-4-5-20250929` — structured outputs / memory example
- `anthropic/claude-3-7-sonnet-20250219` — thinking/reasoning example
- `anthropic/claude-opus-4-7` — adaptive thinking example
- `anthropic/claude-opus-4-6` — adaptive thinking example
- `anthropic/claude-3-5-haiku-20241022` — PDF example
- `claude-opus-4-20250514` — bare model name (no prefix) in SDK usage example
- `claude-3-5-sonnet-20240620` — bare model name in SDK example
- `claude-2.1` — bare model name in pre-fill example

Supported models (from model table):
- claude-opus-4-6 (`claude-opus-4-6-20260205`)
- claude-sonnet-4-6
- claude-sonnet-4-5-20250929
- claude-opus-4-5-20251101
- claude-opus-4-1-20250805
- claude-4 (`claude-opus-4-20250514`, `claude-sonnet-4-20250514`)
- claude-3.7 (`claude-3-7-sonnet-20250219`)
- claude-3.5 (`claude-3-5-sonnet-20240620`)
- claude-3 (`claude-3-haiku-20240307`, `claude-3-opus-20240229`, `claude-3-sonnet-20240229`)
- claude-2, claude-2.1, claude-instant-1.2

## Required environment variables
- `ANTHROPIC_API_KEY` — API key for Anthropic

## Optional environment variables
- `ANTHROPIC_API_BASE` — custom API base URL (for proxy or custom endpoint). Verbatim: "# os.environ['ANTHROPIC_API_BASE'] = '' # [OPTIONAL] or 'ANTHROPIC_BASE_URL'"
- `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX` — "true" to disable automatic URL suffix appending. Verbatim: "# os.environ['LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX'] = 'true' # [OPTIONAL] Disable automatic URL suffix appending"

## api_base behavior
- CRITICAL — Custom API Base IS DOCUMENTED. Verbatim section "### Custom API Base":
  "When using a custom API base for Anthropic (e.g., a proxy or custom endpoint), LiteLLM automatically appends the appropriate suffix (`/v1/messages` or `/v1/complete`) to your base URL."
- `api_base` can be passed as a parameter to `completion()`:
  ```python
  response = completion(
      model="anthropic/claude-sonnet-4-5",
      api_base="https://<your-resource>.services.ai.azure.com/anthropic",
      api_key="<your-azure-api-key>",
      messages=[{"role": "user", "content": "Hello!"}],
  )
  ```
- Or via env var: `os.environ["ANTHROPIC_API_BASE"] = "https://my-custom-endpoint.com/custom/path"`
- URL suffix behavior:
  - Without `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX`:
    - Base URL `https://my-proxy.com` → `https://my-proxy.com/v1/messages`
    - Base URL `https://my-proxy.com/api` → `https://my-proxy.com/api/v1/messages`
  - With `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true`:
    - Base URL `https://my-proxy.com/custom/path` → `https://my-proxy.com/custom/path` (unchanged)

## api_key behavior
- Configured via `ANTHROPIC_API_KEY` env var or `api_key` parameter.
- Verbatim: `os.environ["ANTHROPIC_API_KEY"] = "your-api-key"`

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- not documented on this page (provider selection is implicit via the `anthropic/` prefix)

## Proxy config example (verbatim YAML)
```yaml
model_list:
  - model_name: claude-4 ### RECEIVED MODEL NAME ###
    litellm_params: # all params accepted by litellm.completion() - https://docs.litellm.ai/docs/completion/input
      model: claude-opus-4-20250514 ### MODEL NAME sent to `litellm.completion()` ###
      api_key: "os.environ/ANTHROPIC_API_KEY" # does os.getenv("ANTHROPIC_API_KEY")
```
(source: https://docs.litellm.ai/docs/providers/anthropic)

Wildcard config (default all Anthropic models):
```yaml
model_list:
  - model_name: "*"
     litellm_params:
      model: "*"
```
Required env: `ANTHROPIC_API_KEY=sk-ant****`

Structured outputs config:
```yaml
model_list:
  - model_name: claude-sonnet-4-5
    litellm_params:
      model: anthropic/claude-sonnet-4-5-20250929
      api_key: os.environ/ANTHROPIC_API_KEY
```

## SDK example (verbatim, if present)
```python
import os
from litellm import completion

# set env - [OPTIONAL] replace with your anthropic key
os.environ["ANTHROPIC_API_KEY"] = "your-api-key"

messages = [{"role": "user", "content": "Hey! how's it going?"}]

response = completion(model="claude-opus-4-20250514", messages=messages)
print(response)
```

Custom API Base SDK example:
```python
import os
os.environ["ANTHROPIC_API_BASE"] = "https://my-custom-endpoint.com/custom/path"
os.environ["LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX"] = "true"  # Prevents automatic suffix
```

## Supported endpoints (chat/completions, embeddings, etc.)
- `/chat/completions` (OpenAI-compatible)
- `/v1/messages` (Anthropic passthrough)

## Special params / notes
- Anthropic API fails requests when `max_tokens` are not passed. LiteLLM passes `max_tokens=4096` when no `max_tokens` is passed.
- `response_format` fully supported for Claude Sonnet 4.5 and Opus 4.1 models (Structured Outputs).
- `reasoning_effort` mapped to `output_config={"effort": ...}` for Claude 4.6 and Opus 4.5 models.
- Supported OpenAI params: `stream`, `stop`, `temperature`, `top_p`, `max_tokens`, `max_completion_tokens`, `tools`, `tool_choice`, `extra_headers`, `parallel_tool_calls`, `response_format`, `user`, `reasoning_effort`.
- Prompt caching via `cache_control: {"type": "ephemeral"}` in message content.
- Thinking/reasoning: `reasoning_effort` ("low"|"medium"|"high") or native `thinking={"type": "enabled", "budget_tokens": 1024}` or `thinking={"type": "adaptive"}`.
- Azure Foundry: use `azure/` prefix or `anthropic/` with `api_base`.

## Custom pricing / context window behavior
- not documented on this page (see model_prices_and_context_window.json)

## Known caveats
- `max_tokens` is required by Anthropic API; LiteLLM defaults to 4096 if not passed.
- `reasoning_effort` other than "none" automatically turns thinking on for Claude 4.6/4.7 models.
- `budget_tokens` deprecated on 4.6 models, rejected on Opus 4.7 (only adaptive supported).

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://docs.anthropic.com/en/docs/build-with-claude/overview
- https://docs.litellm.ai/docs/providers/azure/azure_anthropic (Azure Anthropic)
- https://docs.litellm.ai/docs/providers/anthropic_effort (Effort Parameter)
- https://docs.litellm.ai/docs/completion/input#translated-openai-params

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- CRITICAL: The `anthropic/` prefix + custom `api_base` IS a documented pattern. The page has a dedicated "Custom API Base" section.
- This confirms the repo's pattern of `anthropic/kimi-for-coding` with `api_base: api.kimi.com/coding` and `anthropic/MiniMax-M3` with `api_base: api.minimax.io/anthropic` is valid — pointing the `anthropic/` prefix at a non-Anthropic endpoint that speaks the Anthropic Messages API.
- IMPORTANT NUANCE: LiteLLM auto-appends `/v1/messages` to the api_base. For Kimi (api.kimi.com/coding), the final URL would be `api.kimi.com/coding/v1/messages`. For MiniMax (api.minimax.io/anthropic), it would be `api.minimax.io/anthropic/v1/messages` — which matches the MiniMax docs page exactly.
- If the endpoint already includes the full path, set `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` to prevent auto-append.
- NOTE: The MiniMax provider page documents using `minimax/` prefix (not `anthropic/`) with `litellm.anthropic.messages.acreate()` and `api_base: https://api.minimax.io/anthropic/v1/messages`. The repo's use of `anthropic/MiniMax-M3` may be an alternative valid approach but differs from the documented `minimax/` prefix pattern. See p1-provider-minimax.md for details.

## Confidence / uncertainty notes
- The page was truncated in fetch (6346 bytes truncated) but all critical sections (prefix, custom API base, env vars, YAML, SDK examples, model table) were captured before truncation.
- `custom_llm_provider` is NOT documented on this page.
- `api_version` is NOT documented on this page.
- HTTP status inferred from successful full-page content delivery.
