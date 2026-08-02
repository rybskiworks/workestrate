---
source_url: https://docs.litellm.ai/docs/providers/openai_compatible
canonical_url: unknown
title: OpenAI-Compatible Endpoints | liteLLM
sidebar_section_path: Supported Models & Providers > OpenAI-Compatible Endpoints
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: OpenAI-Compatible Endpoints
litellm_provider_prefix: openai/
---
# OpenAI-Compatible Endpoints | liteLLM

## Provider prefix / route
- LiteLLM prefix: `openai/` — "Selecting `openai` as the provider routes your request to an OpenAI-compatible endpoint using the upstream official OpenAI Python API library."
- For `/completions` (legacy): use `text-completion-openai/` prefix (NOT required for `openai/` endpoints called via `/v1/completions` route).
- API protocol: OpenAI-compatible

## Example model strings (verbatim)
- `openai/mistral` — SDK completion example
- `openai/GPT-J` — SDK embedding example
- `openai/<your-model-name>` — proxy YAML placeholder
- `openai/google/gemma` — advanced example with supports_system_message flag

## Required environment variables
- `OPENAI_API_KEY` — "This library requires an API key for all requests, either through the `api_key` parameter or the `OPENAI_API_KEY` environment variable."

## Optional environment variables
- (none documented on this page beyond OPENAI_API_KEY)

## api_base behavior
- Configured as `api_base` in both Python SDK (`litellm.completion(model=..., api_base=...)`) and Proxy `litellm_params`.
- Example values: `http://0.0.0.0:4000`, `http://my-custom-base`
- CRITICAL NOTE (verbatim): "If you see `Not Found Error` when testing make sure your `api_base` has the `/v1` postfix. Example: `http://vllm-endpoint.xyz/v1`"
- CRITICAL NOTE (verbatim): "Do NOT add anything additional to the base url e.g. `/v1/embedding`. LiteLLM uses the openai-client to make these calls, and that automatically adds the relevant endpoints."

## api_key behavior
- Configured as `api_key` parameter or `OPENAI_API_KEY` env var.
- Verbatim: "This library requires an API key for all requests, either through the `api_key` parameter or the `OPENAI_API_KEY` environment variable."
- Verbatim: "If you don't want to provide a fake API key in each request, consider using a provider that directly matches your OpenAI-compatible endpoint, such as `hosted_vllm` or `llamafile`."

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- not documented on this page (provider selection is implicit via the `openai/` prefix in the model string)

## Proxy config example (verbatim YAML)
```yaml
model_list:
  - model_name: my-model
    litellm_params:
      model: openai/<your-model-name>  # add openai/ prefix to route as OpenAI provider
      api_base: <model-api-base>       # add api base for OpenAI compatible provider
      api_key: api-key                 # api key to send your model
```
(source: https://docs.litellm.ai/docs/providers/openai_compatible)

Advanced — Disable System Messages:
```yaml
model_list:
- model_name: my-custom-model
   litellm_params:
      model: openai/google/gemma
      api_base: http://my-custom-base
      api_key: ""
      supports_system_message: False # KEY CHANGE
```
(source: https://docs.litellm.ai/docs/providers/openai_compatible)

## SDK example (verbatim, if present)
```python
import litellm
import os

response = litellm.completion(
    model="openai/mistral",               # add `openai/` prefix to model so litellm knows to route to OpenAI
    api_key="sk-1234",                  # api key to your openai compatible endpoint
    api_base="http://0.0.0.0:4000",     # set API Base of your Custom OpenAI Endpoint
    messages=[
                {
                    "role": "user",
                    "content": "Hey, how's it going?",
                }
    ],
)
print(response)
```

Embedding SDK example:
```python
import litellm
import os

response = litellm.embedding(
    model="openai/GPT-J",               # add `openai/` prefix to model so litellm knows to route to OpenAI
    api_key="sk-1234",                  # api key to your openai compatible endpoint
    api_base="http://0.0.0.0:4000",     # set API Base of your Custom OpenAI Endpoint
    input=["good morning from litellm"]
)
print(response)
```

## Supported endpoints (chat/completions, embeddings, etc.)
- `/chat/completions` (via `litellm.completion`)
- `/embeddings` (via `litellm.embedding`)
- `/completions` (via `text-completion-openai/` prefix)

## Special params / notes
- `supports_system_message: False` — under `litellm_params`, maps system messages to 'user' messages (for VLLM models like gemma that don't support system messages).
- Sibling page: `/docs/contributing/adding_openai_compatible_providers` — newer "Add OpenAI-Compatible Provider (JSON)" workflow (alternative integration path).

## Custom pricing / context window behavior
- not documented on this page

## Known caveats
- The openai-client library requires an API key for all requests (even for endpoints that don't need one — use `hosted_vllm` or `llamafile` as alternatives).
- `api_base` must include `/v1` postfix or you'll get "Not Found Error".
- Do NOT append endpoint paths like `/v1/embedding` to `api_base` — the openai-client adds them automatically.

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://docs.litellm.ai/docs/providers/vllm (hosted_vllm alternative)
- https://docs.litellm.ai/docs/providers/llamafile (llamafile alternative)
- https://docs.litellm.ai/docs/contributing/adding_openai_compatible_providers (JSON registration alternative)

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- This is the CANONICAL pattern for Neuralwatt (api.neuralwatt.com/v1) and any self-hosted vLLM/Ollama OpenAI-compatible endpoint.
- Pattern: `model: openai/<name>` + `api_base: https://<host>/v1` + `api_key: <key>` in litellm_params.
- The repo's `openai/neuralwatt` with `api_base: https://api.neuralwatt.com/v1` follows this exact documented pattern.
- For endpoints that don't require an API key, a fake key must still be passed (or use `hosted_vllm/` prefix instead).

## Confidence / uncertainty notes
- `custom_llm_provider` is NOT documented on this page — provider selection is implicit via the `openai/` prefix.
- `OPENAI_BASE_URL` env var is NOT documented on this page (only `OPENAI_API_KEY`). The OpenAI provider page documents `OPENAI_BASE_URL`.
- HTTP status inferred from successful full-page content delivery (webfetch tool does not expose explicit status header).
