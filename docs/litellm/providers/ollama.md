# Ollama

> Source: https://docs.litellm.ai/docs/providers/ollama

## Prefix

`ollama/` (for `/api/generate` endpoint) and `ollama_chat/` (for `/api/chat` endpoint — RECOMMENDED). "We recommend using ollama_chat for better responses."

## Protocol

native Ollama (not OpenAI-compatible — uses Ollama's `/api/chat` and `/api/generate` endpoints)

## Environment variables

- Required: (none required — Ollama doesn't require an API key)
- Optional: (none documented on this page)

## api_base behavior

Config field: `api_base` in litellm_params or `api_base=` kwarg. Default: `http://localhost:11434`. Example: `api_base="http://localhost:11434"`. In proxy YAML: `api_base: "http://localhost:11434"`

## api_key behavior

NOT required. `api_key="anything" # PROXY KEY (can be anything, if master_key not set)`

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

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

## Supported endpoints

- `/api/chat` (via `ollama_chat/` prefix — recommended)
- `/api/generate` (via `ollama/` prefix)
- `/v1/completions` (FIM — fill-in-the-middle)
- Vision (via `ollama/llava`)
- JSON mode (`format="json"`)
- Tool calling (via `ollama_chat/` prefix)
- JSON Schema / structured outputs

## Special params / notes

- `format="json"` — Ollama JSON Mode
- `keep_alive: "8m"` — optional, overrides default keep_alive (use -1 for Forever)
- `model_info: supports_function_calling: true` — register model for function calling support
- `litellm.register_model()` — optional, for models that support function calling
- `ollama_chat/` prefix routes to `/api/chat` (recommended for better responses)
- `ollama/` prefix routes to `/api/generate`
- Vision: `ollama/llava` accepts OpenAI content array with `image_url` blocks

## Caveats

- Not all ollama models support function calling — LiteLLM defaults to json mode tool calls if native tool calling not supported.
- `ollama_chat/` is recommended over `ollama/` for better responses.
- Ollama uses native API (not OpenAI-compatible) — uses `/api/chat` and `/api/generate` endpoints.
- `api_base` defaults to `http://localhost:11434`.

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]

Ollama is a local model serving pattern. Uses native Ollama API (not OpenAI-compatible). `ollama_chat/` is the recommended prefix for chat; `ollama/` for generate/FIM. No API key required — suitable for local development. `api_base` defaults to `http://localhost:11434`. For OpenAI-compatible local serving, vLLM (`hosted_vllm/`) may be a better choice since it exposes an OpenAI-compatible API.

## Confidence / uncertainty

- The page was truncated (38009 bytes truncated) but all critical sections (prefix, api_base, YAML, SDK examples, model table) were captured before truncation.
- HTTP status inferred from successful full-page content delivery.
