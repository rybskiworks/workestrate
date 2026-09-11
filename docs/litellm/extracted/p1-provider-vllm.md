---
source_url: https://docs.litellm.ai/docs/providers/vllm
canonical_url: unknown
title: VLLM | liteLLM
sidebar_section_path: Supported Models & Providers > vLLM > VLLM
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: vLLM
litellm_provider_prefix: hosted_vllm/
---
# VLLM | liteLLM

## Provider prefix / route
- LiteLLM prefix: `hosted_vllm/` (for OpenAI compatible server) — CURRENT/RECOMMENDED
- `vllm/` — [DEPRECATED] for vllm sdk usage (packaged vllm installs, in-process)
- Verbatim: "Provider Route on LiteLLM | `hosted_vllm/` (for OpenAI compatible server), `vllm/` ([DEPRECATED] for vllm sdk usage)"
- API protocol: OpenAI-compatible ("vLLM Provides an OpenAI compatible endpoints")

## Example model strings (verbatim)
- `hosted_vllm/facebook/opt-125m` — completion + embedding example
- `hosted_vllm/gpt-oss-120b` — reasoning effort example
- `hosted_vllm/qwen` — video URL example
- `hosted_vllm/your-rerank-model` — rerank example
- (deprecated) `vllm/facebook/opt-125m`, `vllm/meta-llama/Llama-2-7b`, `vllm/tiiuae/falcon-7b-instruct`, `vllm/mosaicml/mpt-7b-chat`, `vllm/codellama/CodeLlama-34b-Instruct-hf`, `vllm/WizardLM/WizardCoder-Python-34B-V1.0`, `vllm/Phind/Phind-CodeLlama-34B-v2`, `vllm/togethercomputer/LLaMA-2-7B-32K`

## Required environment variables
- (none required — api_key is optional for vLLM)

## Optional environment variables
- `HOSTED_VLLM_API_BASE` — base URL for vLLM server (e.g. `http://localhost:8000`)
- `HOSTED_VLLM_API_KEY` — "[optional], if your VLLM server requires an API key"
- `LITELLM_DEFAULT_EMBEDDING_ENCODING_FORMAT` — defaults to `float` (for embeddings)

## api_base behavior
- Config field: `api_base` in litellm_params or `api_base=` kwarg
- Env var: `HOSTED_VLLM_API_BASE`
- Example values: `https://hosted-vllm-api.co`, `http://localhost:8000`
- Verbatim: "In order to use litellm to call a hosted vllm server add the following to your completion call: `model='hosted_vllm/<your-vllm-model-name>'`, `api_base = 'your-hosted-vllm-server'`"

## api_key behavior
- Optional. Env var: `HOSTED_VLLM_API_KEY`
- Verbatim: "os.environ['HOSTED_VLLM_API_KEY'] = '' # [optional], if your VLLM server requires an API key"
- In proxy YAML: `# api_key: your-api-key # [optional] if your VLLM server requires authentication`

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- Documented in deprecated section: `custom_llm_provider=provider` (where provider="vllm") for batch_completion. Verbatim: "custom_llm_provider=provider, # can easily switch to huggingface, replicate, together ai, sagemaker, etc."

## Proxy config example (verbatim YAML)
```yaml
model_list:
  - model_name: my-model
    litellm_params:
      model: hosted_vllm/facebook/opt-125m  # add hosted_vllm/ prefix to route as OpenAI provider
      api_base: https://hosted-vllm-api.co      # add api base for OpenAI compatible provider
```
(source: https://docs.litellm.ai/docs/providers/vllm)

Reasoning effort config:
```yaml
model_list:
  - model_name: gpt-oss-120b
    litellm_params:
      model: hosted_vllm/gpt-oss-120b
      api_base: https://hosted-vllm-api.co
```

Rerank config:
```yaml
model_list:
    - model_name: my-rerank-model
      litellm_params:
        model: hosted_vllm/your-rerank-model  # add hosted_vllm/ prefix to route as VLLM provider
        api_base: http://localhost:8000      # add api base for your VLLM server
        # api_key: your-api-key             # [optional] if your VLLM server requires authentication
```

## SDK example (verbatim, if present)
```python
import litellm

response = litellm.completion(
            model="hosted_vllm/facebook/opt-125m", # pass the vllm model name
            messages=messages,
            api_base="https://hosted-vllm-api.co",
            temperature=0.2,
            max_tokens=80
)
print(response)
```

Embedding:
```python
from litellm import embedding   
import os

os.environ["HOSTED_VLLM_API_BASE"] = "http://localhost:8000"

embedding = embedding(model="hosted_vllm/facebook/opt-125m", input=["Hello world"])

print(embedding)
```

## Supported endpoints (chat/completions, embeddings, etc.)
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

## Custom pricing / context window behavior
- not documented on this page

## Known caveats
- `vllm/` prefix is DEPRECATED — use `hosted_vllm/` for OpenAI-compatible vLLM servers.
- The `vllm/` prefix is for in-process vLLM SDK usage (packaged installs via `uv add litellm vllm`), not HTTP server calls.
- `hosted_vllm/` is the recommended alternative to `openai/` prefix when you don't want to pass a fake API key (referenced by openai_compatible page).

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://docs.vllm.ai/en/latest/index.html
- https://docs.litellm.ai/docs/providers/vllm_batches (vLLM Batch + Files API)

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- `hosted_vllm/` is the recommended prefix for self-hosted OpenAI-compatible vLLM servers.
- This is the alternative to `openai/` prefix when the endpoint doesn't require an API key (no need for fake key).
- Pattern: `model: hosted_vllm/<model-name>` + `api_base: https://<vllm-host>` + optional `api_key`.
- Supports more endpoints than openai_compatible: `/rerank`, `/audio/transcriptions` in addition to chat/completions and embeddings.
- For Neuralwatt (which requires an API key), `openai/` prefix is more appropriate. For keyless local vLLM, `hosted_vllm/` is better.

## Confidence / uncertainty notes
- The `vllm/` deprecated path documents `custom_llm_provider` usage, but the current `hosted_vllm/` path does not explicitly document it.
- HTTP status inferred from successful full-page content delivery.
