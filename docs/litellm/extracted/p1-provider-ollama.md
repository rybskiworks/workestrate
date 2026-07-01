---
source_url: https://docs.litellm.ai/docs/providers/ollama
canonical_url: unknown
title: Ollama | liteLLM
sidebar_section_path: Supported Models & Providers > Ollama
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: Ollama
litellm_provider_prefix: ollama/
---
# Ollama | liteLLM

## Provider prefix / route
- LiteLLM prefix: `ollama/` (for `/api/generate` endpoint) and `ollama_chat/` (for `/api/chat` endpoint — RECOMMENDED)
- Verbatim: "We recommend using ollama_chat for better responses."
- API protocol: native Ollama (not OpenAI-compatible — uses Ollama's `/api/chat` and `/api/generate` endpoints)

## Example model strings (verbatim)
- `ollama/llama2` — basic usage, streaming, FIM
- `ollama/llama2:13b` — model table
- `ollama/llama2:70b` — model table
- `ollama/llama2-uncensored` — model table
- `ollama/mistral` — model table
- `ollama/mistral-7B-Instruct-v0.1` — model table
- `ollama/mistral-7B-Instruct-v0.2` — model table
- `ollama/mistral-8x7B-Instruct-v0.1` — model table
- `ollama/mixtral-8x22B-Instruct-v0.1` — model table
- `ollama/codellama` — model table
- `ollama/llama3` — model table
- `ollama/llama3:70b` — model table
- `ollama/orca-mini` — model table
- `ollama/vicuna` — model table
- `ollama/nous-hermes` — model table
- `ollama/nous-hermes:13b` — model table
- `ollama/wizard-vicuna` — model table
- `ollama/llava` — vision model
- `ollama_chat/llama3.1` — tool calling, JSON schema
- `ollama_chat/deepseek-r1` — JSON schema support
- `ollama/llama3.1` — FIM example

## Required environment variables
- (none required — Ollama doesn't require an API key)

## Optional environment variables
- (none documented on this page)

## api_base behavior
- Config field: `api_base` in litellm_params or `api_base=` kwarg
- Default: `http://localhost:11434`
- Example: `api_base="http://localhost:11434"`
- In proxy YAML: `api_base: "http://localhost:11434"`

## api_key behavior
- NOT required. Verbatim from proxy example: `api_key="anything" # PROXY KEY (can be anything, if master_key not set)`

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- not documented on this page

## Proxy config example (verbatim YAML)
Tool calling config:
```yaml
model_list:
  - model_name: "llama3.1"                 
    litellm_params:
      model: "ollama_chat/llama3.1"
      keep_alive: "8m" # Optional: Overrides default keep_alive, use -1 for Forever
    model_info:
      supports_function_calling: true
```
(source: https://docs.litellm.ai/docs/providers/ollama)

FIM config:
```yaml
model_list:
  - model_name: "llama3.1"                 
    litellm_params:
      model: "ollama/llama3.1"
      api_base: "http://localhost:11434"
```

JSON Schema config:
```yaml
model_list:
  - model_name: "deepseek-r1"                 
    litellm_params:
      model: "ollama_chat/deepseek-r1"
      api_base: "http://localhost:11434"
```

## SDK example (verbatim, if present)
```python
from litellm import completion
response = completion(
    model="ollama/llama2", 
    messages=[{ "content": "respond in 20 words. who are you?","role": "user"}], 
    api_base="http://localhost:11434"
)
print(response)
```

Using ollama_chat (recommended):
```python
from litellm import completion
response = completion(
    model="ollama_chat/llama2", 
    messages=[{ "content": "respond in 20 words. who are you?","role": "user"}], 
)
print(response)
```

## Supported endpoints (chat/completions, embeddings, etc.)
- `/api/chat` (via `ollama_chat/` prefix — recommended)
- `/api/generate` (via `ollama/` prefix) — also accessible on `/v1/completions`
- `/v1/completions` (FIM — fill-in-the-middle)
- Vision (via `ollama/llava`)
- JSON mode (`format="json"`)
- Tool calling (via `ollama_chat/` prefix)
- JSON Schema / structured outputs (`response_format={"type": "json_schema", ...}`)

## Special params / notes
- `format="json"` — Ollama JSON Mode
- `keep_alive: "8m"` — optional, overrides default keep_alive (use -1 for Forever)
- `model_info: supports_function_calling: true` — register model for function calling support
- `litellm.register_model()` — optional, for models that support function calling
- `ollama_chat/` prefix routes to `/api/chat` (recommended for better responses)
- `ollama/` prefix routes to `/api/generate`
- Vision: `ollama/llava` accepts OpenAI content array with `image_url` blocks

## Custom pricing / context window behavior
- not documented on this page

## Known caveats
- Not all ollama models support function calling — LiteLLM defaults to json mode tool calls if native tool calling not supported.
- `ollama_chat/` is recommended over `ollama/` for better responses.
- Ollama uses native API (not OpenAI-compatible) — uses `/api/chat` and `/api/generate` endpoints.
- `api_base` defaults to `http://localhost:11434`.

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://github.com/ollama/ollama
- https://colab.research.google.com/github/BerriAI/litellm/blob/main/cookbook/liteLLM_Ollama.ipynb

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- Ollama is a local model serving pattern. Uses native Ollama API (not OpenAI-compatible).
- `ollama_chat/` is the recommended prefix for chat; `ollama/` for generate/FIM.
- No API key required — suitable for local development.
- `api_base` defaults to `http://localhost:11434`.
- For OpenAI-compatible local serving, vLLM (`hosted_vllm/`) may be a better choice since it exposes an OpenAI-compatible API.

## Confidence / uncertainty notes
- The page was truncated (38009 bytes truncated) but all critical sections (prefix, api_base, YAML, SDK examples, model table) were captured before truncation.
- HTTP status inferred from successful full-page content delivery.
