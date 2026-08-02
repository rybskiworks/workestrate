# OpenAI-Compatible Endpoints

> Source: https://docs.litellm.ai/docs/providers/openai_compatible

## Prefix

`openai/` — "Selecting `openai` as the provider routes your request to an OpenAI-compatible endpoint using the upstream official OpenAI Python API library."

## Protocol

OpenAI-compatible

## Environment variables

- Required: `OPENAI_API_KEY` — "This library requires an API key for all requests, either through the `api_key` parameter or the `OPENAI_API_KEY` environment variable."
- Optional: (none documented on this page beyond `OPENAI_API_KEY`)

## api_base behavior

Configured as `api_base` in both Python SDK (`litellm.completion(model=..., api_base=...)`) and Proxy `litellm_params`. Example values: `http://0.0.0.0:4000`, `http://my-custom-base`.

CRITICAL (verbatim):

- "If you see `Not Found Error` when testing make sure your `api_base` has the `/v1` postfix. Example: `http://vllm-endpoint.xyz/v1`"
- "Do NOT add anything additional to the base url e.g. `/v1/embedding`. LiteLLM uses the openai-client to make these calls, and that automatically adds the relevant endpoints."

## api_key behavior

Configured as `api_key` parameter or `OPENAI_API_KEY` env var. "This library requires an API key for all requests, either through the `api_key` parameter or the `OPENAI_API_KEY` environment variable." If you don't want to provide a fake API key in each request, consider using a provider that directly matches your OpenAI-compatible endpoint, such as `hosted_vllm` or `llamafile`.

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

```yaml
model_list:
  - model_name: my-model
    litellm_params:
      model: openai/<your-model-name>  # add openai/ prefix to route as OpenAI provider
      api_base: <model-api-base>       # add api base for OpenAI compatible provider
      api_key: api-key                 # api key to send your model
```

## Supported endpoints

- `/chat/completions`
- `/embeddings`
- `/completions`

## Special params / notes

- `supports_system_message: False` — under `litellm_params`, maps system messages to 'user' messages (for VLLM models like gemma that don't support system messages).
- Sibling page: `/docs/contributing/adding_openai_compatible_providers` — newer "Add OpenAI-Compatible Provider (JSON)" workflow.

## Caveats

- The openai-client library requires an API key for all requests (even for endpoints that don't need one — use `hosted_vllm` or `llamafile` as alternatives).
- `api_base` must include `/v1` postfix or you'll get "Not Found Error".
- Do NOT append endpoint paths like `/v1/embedding` to `api_base` — the openai-client adds them automatically.

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]

This is the CANONICAL pattern for Neuralwatt (`api.neuralwatt.com/v1`) and any self-hosted vLLM/Ollama OpenAI-compatible endpoint. Pattern: `model: openai/<name>` + `api_base: https://<host>/v1` + `api_key: <key>` in `litellm_params`. The repo's `openai/neuralwatt` with `api_base: https://api.neuralwatt.com/v1` follows this exact documented pattern. For endpoints that don't require an API key, a fake key must still be passed (or use `hosted_vllm/` prefix instead).

## Confidence / uncertainty

- `custom_llm_provider` is NOT documented on this page — provider selection is implicit via the `openai/` prefix.
- `OPENAI_BASE_URL` env var is NOT documented on this page (only `OPENAI_API_KEY`). The OpenAI provider page documents `OPENAI_BASE_URL`.
- HTTP status inferred from successful full-page content delivery.
