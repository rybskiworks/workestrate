# vLLM

> Source: https://docs.litellm.ai/docs/providers/vllm

## Prefix

`hosted_vllm/` (for OpenAI compatible server) — CURRENT/RECOMMENDED. `vllm/` — [DEPRECATED] for vllm sdk usage (packaged vllm installs, in-process). "Provider Route on LiteLLM | `hosted_vllm/` (for OpenAI compatible server), `vllm/` ([DEPRECATED] for vllm sdk usage)"

## Protocol

OpenAI-compatible ("vLLM Provides an OpenAI compatible endpoints")

## Environment variables

- Required: (none required — api_key is optional for vLLM)
- Optional: `HOSTED_VLLM_API_BASE` — base URL for vLLM server (e.g. `http://localhost:8000`)
- Optional: `HOSTED_VLLM_API_KEY` — "[optional], if your VLLM server requires an API key"
- Optional: `LITELLM_DEFAULT_EMBEDDING_ENCODING_FORMAT` — defaults to `float` (for embeddings)

## api_base behavior

Config field: `api_base` in litellm_params or `api_base=` kwarg. Env var: `HOSTED_VLLM_API_BASE`. Example values: `https://hosted-vllm-api.co`, `http://localhost:8000`. "In order to use litellm to call a hosted vllm server add the following to your completion call: `model='hosted_vllm/<your-vllm-model-name>'`, `api_base = 'your-hosted-vllm-server'`"

## api_key behavior

Optional. Env var: `HOSTED_VLLM_API_KEY`. `os.environ['HOSTED_VLLM_API_KEY'] = '' # [optional], if your VLLM server requires an API key`. In proxy YAML: `# api_key: your-api-key # [optional] if your VLLM server requires authentication`

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

```yaml
model_list:
  - model_name: my-model
    litellm_params:
      model: hosted_vllm/facebook/opt-125m  # add hosted_vllm/ prefix to route as OpenAI provider
      api_base: https://hosted-vllm-api.co      # add api base for OpenAI compatible provider
```

## Supported endpoints

- `/chat/completions`
- `/embeddings`
- `/completions`
- `/rerank`
- `/audio/transcriptions`

## Special params / notes

- `reasoning_effort` supported (e.g. `reasoning_effort="high"`)
- Video URL support: `{"type": "file", "file": {"file_id": video_url}}` or `{"type": "video_url", "video_url": {"url": video_url}}`
- `litellm.register_prompt_template()` for custom prompt templates (deprecated vllm/ path)
- Embeddings: when clients omit `encoding_format`, LiteLLM defaults it (request → model litellm_params → `LITELLM_DEFAULT_EMBEDDING_ENCODING_FORMAT` → `float`)

## Caveats

- `vllm/` prefix is DEPRECATED — use `hosted_vllm/` for OpenAI-compatible vLLM servers.
- The `vllm/` prefix is for in-process vLLM SDK usage (packaged installs via `uv add litellm vllm`), not HTTP server calls.
- `hosted_vllm/` is the recommended alternative to `openai/` prefix when you don't want to pass a fake API key (referenced by openai_compatible page).

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]

`hosted_vllm/` is the recommended prefix for self-hosted OpenAI-compatible vLLM servers. This is the alternative to `openai/` prefix when the endpoint doesn't require an API key (no need for fake key). Pattern: `model: hosted_vllm/<model-name>` + `api_base: https://<vllm-host>` + optional `api_key`. Supports more endpoints than openai_compatible: `/rerank`, `/audio/transcriptions` in addition to chat/completions and embeddings. For Neuralwatt (which requires an API key), `openai/` prefix is more appropriate. For keyless local vLLM, `hosted_vllm/` is better.

## Confidence / uncertainty

- The `vllm/` deprecated path documents `custom_llm_provider` usage, but the current `hosted_vllm/` path does not explicitly document it.
- HTTP status inferred from successful full-page content delivery.
