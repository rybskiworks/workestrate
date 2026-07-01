---
source_url: https://docs.litellm.ai/docs/providers/openai
canonical_url: unknown
title: OpenAI | liteLLM
sidebar_section_path: Supported Models & Providers > OpenAI > OpenAI
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: OpenAI
litellm_provider_prefix: openai/
---
# OpenAI | liteLLM

## Provider prefix / route
- LiteLLM prefix: `openai/` — "The `openai/` prefix will call openai.chat.completions.create"
- For legacy completions: `text-completion-openai/` prefix calls `openai.completions.create`
- For Responses API: `openai/responses/` prefix routes through Responses API
- API protocol: OpenAI native (chat completions JSON over HTTPS)
- NOTE: In SDK calls, bare model names (e.g. `gpt-4o`) work without prefix when calling api.openai.com directly. In proxy YAML config, the `openai/` prefix is used to disambiguate the OpenAI route.

## Example model strings (verbatim)
- `gpt-4o` — bare model name in SDK example
- `gpt-3.5-turbo` — bare model name in SDK example
- `openai/gpt-3.5-turbo` — proxy YAML (with prefix)
- `openai/gpt-3.5-turbo-instruct` → `text-completion-openai/gpt-3.5-turbo-instruct` — proxy YAML
- `openai/*` — wildcard proxy YAML
- `openai/gpt-5-search-api` — web search model
- `openai/gpt-5` — web search / responses
- `openai/responses/gpt-5-mini` — Responses API prefix
- `openai/responses/gpt-4o` — Responses API prefix for built-in tools
- `openai/your-model-name` — custom proxy example

Full model list (selection — see page for complete list):
- gpt-5, gpt-5-mini, gpt-5-nano, gpt-5-chat, gpt-5-chat-latest, gpt-5-pro
- gpt-5.1, gpt-5.1-codex, gpt-5.1-codex-mini, gpt-5.1-codex-max
- gpt-5.2, gpt-5.2-pro, gpt-5.2-chat-latest
- gpt-5.3-chat-latest, gpt-5.4, gpt-5.4-pro, gpt-5.5, gpt-5.5-pro
- gpt-4.1, gpt-4.1-mini, gpt-4.1-nano
- o4-mini, o3-mini, o3, o1-mini, o1-preview
- gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-4, gpt-4-32k
- gpt-3.5-turbo, gpt-3.5-turbo-16k
- whisper-1, gpt-4o-transcribe, gpt-4o-mini-transcribe (audio)
- ft:gpt-4-0613, ft:gpt-4o-2024-05-13 (fine-tuned)

## Required environment variables
- `OPENAI_API_KEY` — API key for OpenAI

## Optional environment variables
- `OPENAI_BASE_URL` — custom API endpoint. Verbatim: `os.environ["OPENAI_BASE_URL"] = "https://your_host/v1" # OPTIONAL`
- `OPENAI_ORGANIZATION` — OpenAI organization ID. Verbatim: `os.environ["OPENAI_ORGANIZATION"] = "your-org-id" # OPTIONAL`
- `LITELLM_ROUTE_ALL_CHAT_OPENAI_TO_RESPONSES` — "true" to route all OpenAI chat through Responses API

## api_base behavior
- Env var: `OPENAI_BASE_URL` — "These also support the `OPENAI_BASE_URL` environment variable, which can be used to specify a custom API endpoint."
- Global attribute: `litellm.api_base = "https://your_host/v1"`
- Per-request: `api_base="your-proxy-api-base"` kwarg
- Verbatim: "If you need to set api_base dynamically, just pass it in completions instead - `completions(...,api_base='your-proxy-api-base')`"
- NOTE: `api_base` as a YAML `litellm_params` key is NOT shown on this page (see openai_compatible page for that form).

## api_key behavior
- Env var: `OPENAI_API_KEY`
- In proxy YAML: `api_key: os.environ/OPENAI_API_KEY`

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- not documented on this page (provider selection is implicit via the `openai/` prefix)

## Proxy config example (verbatim YAML)
```yaml
model_list:
  - model_name: gpt-3.5-turbo
    litellm_params:
      model: openai/gpt-3.5-turbo                          # The `openai/` prefix will call openai.chat.completions.create
      api_key: os.environ/OPENAI_API_KEY
  - model_name: gpt-3.5-turbo-instruct
    litellm_params:
      model: text-completion-openai/gpt-3.5-turbo-instruct # The `text-completion-openai/` prefix will call openai.completions.create
      api_key: os.environ/OPENAI_API_KEY
```
(source: https://docs.litellm.ai/docs/providers/openai)

Wildcard (proxy all OpenAI models):
```yaml
model_list:
  - model_name: "*"             # all requests where model not in your config go to this deployment
    litellm_params:
      model: openai/*           # set `openai/` to use the openai route
      api_key: os.environ/OPENAI_API_KEY
```

Responses API routing:
```yaml
litellm_settings:
  route_all_chat_openai_to_responses: true
```

## SDK example (verbatim, if present)
```python
import os 
from litellm import completion
os.environ["OPENAI_API_KEY"] = "your-api-key"
# openai call
response = completion(
    model = "gpt-4o", 
    messages=[{ "content": "Hello, how are you?","role": "user"}])
```

Using OpenAI Proxy with LiteLLM (custom api_base):
```python
import os 
import litellm
from litellm import completion
os.environ["OPENAI_API_KEY"] = ""

# set custom api base to your proxy
# either set .env or litellm.api_base
# os.environ["OPENAI_BASE_URL"] = "https://your_host/v1"

litellm.api_base = "https://your_host/v1"

messages = [{ "content": "Hello, how are you?","role": "user"}]

# openai call
response = completion("openai/your-model-name", messages)
```

## Supported endpoints (chat/completions, embeddings, etc.)
- `/chat/completions` (via `completion()`)
- `/completions` (via `text-completion-openai/` prefix)
- `/v1/responses` (via `openai/responses/` prefix or `route_all_chat_openai_to_responses` flag)
- Embeddings (`text-embedding-ada-002`)
- Audio Transcription (`transcription()`)
- Vision (gpt-4o, gpt-4-turbo, gpt-4-vision-preview)
- PDF File Parsing (`file` message type)
- Image Generation (Sora — linked to separate page)
- Web Search (`web_search_options` or `web_search_preview` tool)
- Function/Tool calling
- Streaming

## Special params / notes
- `openai/responses/` prefix — routes through Responses API (for built-in tools like `web_search_preview`, `code_interpreter`)
- `route_all_chat_openai_to_responses: true` — global flag to route all OpenAI chat through Responses API
- `reasoning_effort` — supports "none", "minimal", "low", "medium", "high", "xhigh" (xhigh only on gpt-5.1-codex-max and gpt-5.2)
- `reasoning_effort` as dict: `{"effort": "high", "summary": "auto"}` — requires org verification
- `verbosity` — "low", "medium", "high" (for GPT-5 family; NOT for GPT-5-Codex)
- `reasoning_items` — for multi-turn conversations with encrypted_content
- `include=["reasoning.encrypted_content"]` — to get reasoning token for next turn
- `forward_openai_org_id: true` — in general_settings, forwards OpenAI Org ID from client
- `litellm.return_response_headers = True` — get raw response headers
- `litellm.client_session = httpx.Client(verify=False)` — disable SSL verification
- Model `mode` property: `mode: responses` (auto Responses API) vs `mode: chat` (Chat Completions, needs prefix for built-in tools)
- GPT-5 Pro: Responses API only, no streaming, 400K input / 272K output tokens

## Custom pricing / context window behavior
- GPT-5 Pro: $15.00 input / $120.00 output per 1M tokens (Standard), $7.50/$60.00 (Batch)
- GPT-5 Pro context: 400,000 input, 272,000 output tokens
- Other models: see model_prices_and_context_window.json

## Known caveats
- `openai/` prefix is used in proxy YAML to disambiguate the OpenAI route. In SDK calls, bare model names work when calling api.openai.com directly.
- `OPENAI_BASE_URL` env var is documented for custom endpoints — this is the mechanism for pointing at OpenAI-compatible endpoints.
- `api_base` as a YAML `litellm_params` key is NOT shown on this page (see openai_compatible page for that form).
- GPT-5.4+ drops `reasoning_effort` from requests that include tools (only supported in Responses API).
- `reasoning_effort` summary field requires OpenAI org verification.

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://docs.litellm.ai/docs/providers/openai/videos (Video Generation)
- https://docs.litellm.ai/docs/set_keys (Setting API Base/Keys)
- https://platform.openai.com/docs/api-reference/responses (Responses API)
- https://github.com/BerriAI/litellm/blob/main/model_prices_and_context_window.json

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- The repo uses `openai/neuralwatt` with `api_base: https://api.neuralwatt.com/v1` — this follows the documented pattern.
- `openai/` prefix + `OPENAI_BASE_URL` (or `api_base` in litellm_params) is the canonical way to point LiteLLM at a custom OpenAI-compatible endpoint.
- The `openai/` prefix is shared between the OpenAI provider page and the openai_compatible page — both use the same prefix for OpenAI-protocol chat-completions endpoints.
- For Neuralwatt: `model: openai/neuralwatt` + `api_base: https://api.neuralwatt.com/v1` + `api_key: <key>` is correct.
- The `openai/responses/` sub-prefix is for OpenAI's Responses API (not relevant for Neuralwatt).

## Confidence / uncertainty notes
- `api_base` as a YAML `litellm_params` key is NOT shown on this page — it IS shown on the openai_compatible page. Both pages use the same `openai/` prefix.
- The page is very large (includes extensive GPT-5 model tables and Responses API docs) — all critical sections captured.
- HTTP status inferred from successful full-page content delivery.
