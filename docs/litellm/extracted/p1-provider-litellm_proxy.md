---
source_url: https://docs.litellm.ai/docs/providers/litellm_proxy
canonical_url: unknown
title: LiteLLM Proxy (LLM Gateway) | liteLLM
sidebar_section_path: Supported Models & Providers > LiteLLM Proxy (LLM Gateway)
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
provider_name: LiteLLM Proxy (LLM Gateway)
litellm_provider_prefix: litellm_proxy/
---
# LiteLLM Proxy (LLM Gateway) | liteLLM

## Provider prefix / route
- LiteLLM prefix: `litellm_proxy/` — "Simply use the `litellm_proxy/` prefix before the model name to route your requests through the proxy."
- Verbatim: "add this prefix to the model name, to route any requests to litellm_proxy - e.g. `litellm_proxy/your-model-name`"
- API protocol: OpenAI-compatible ("LiteLLM Proxy is an OpenAI-compatible gateway")
- This is the CHAINED-PROXY pattern: one LiteLLM SDK/proxy calling another LiteLLM proxy.

## Example model strings (verbatim)
- `litellm_proxy/your-model-name` — chat/completions
- `litellm_proxy/your-embedding-model` — embeddings
- `litellm_proxy/dall-e-3` — image generation
- `litellm_proxy/gpt-image-1` — image edit
- `litellm_proxy/whisper-1` — audio transcription
- `litellm_proxy/tts-1` — text-to-speech
- `litellm_proxy/rerank-english-v2.0` — rerank
- `vertex_ai/gemini-2.0-flash-001` — used in "Send all SDK requests" example (non-proxy model routed through proxy)
- `gpt-4` — used in OAuth2/JWT and tags examples (raw model, proxied via api_base)

## Required environment variables
- `LITELLM_PROXY_API_KEY` — "your litellm proxy api key" (e.g. "sk-1234")
- `LITELLM_PROXY_API_BASE` — "your litellm proxy api base" (e.g. "http://localhost:4000")

## Optional environment variables
- `USE_LITELLM_PROXY` — "True" — controls SDK-wide proxy routing (requires v1.72.1+)

## api_base behavior
- Env var: `LITELLM_PROXY_API_BASE` (e.g. "http://localhost:4000")
- Global attribute: `litellm.api_base = "your-openai-proxy-url"`
- Per-request: `api_base = "your-litellm-proxy-url"` kwarg
- Verbatim: "If you need to set api_base dynamically, just pass it in completions instead - completions(...,api_base='your-proxy-api-base')"

## api_key behavior
- Env var: `LITELLM_PROXY_API_KEY`
- Per-request: `api_key = "your-litellm-proxy-api-key"` kwarg

## api_version behavior
- not documented on this page

## custom_llm_provider behavior
- not documented on this page

## Proxy config example (verbatim YAML)
- NOT documented on this page. The page documents only Python SDK usage. No `config.yaml` / `model_list` YAML examples are present.
- (inferred) YAML would follow standard pattern: `model: litellm_proxy/<model-name>`, `api_key: os.environ/LITELLM_PROXY_API_KEY`, `api_base: <proxy-url>`

## SDK example (verbatim, if present)
```python
import os 
import litellm
from litellm import completion

os.environ["LITELLM_PROXY_API_KEY"] = ""

# set custom api base to your proxy
# either set .env or litellm.api_base
# os.environ["LITELLM_PROXY_API_BASE"] = ""
litellm.api_base = "your-openai-proxy-url"

messages = [{ "content": "Hello, how are you?","role": "user"}]

# litellm proxy call
response = completion(model="litellm_proxy/your-model-name", messages)
```

Per-request api_base/api_key:
```python
response = completion(
    model="litellm_proxy/your-model-name", 
    messages=messages, 
    api_base = "your-litellm-proxy-url",
    api_key = "your-litellm-proxy-api-key"
)
```

Send all SDK requests to LiteLLM Proxy (v1.72.1+):
```python
# Option 1: Global flag
litellm.use_litellm_proxy = True
response = litellm.completion(
    model="vertex_ai/gemini-2.0-flash-001",
    messages=[{"role": "user", "content": "Hello, how are you?"}]
)

# Option 2: Environment variable
os.environ["USE_LITELLM_PROXY"] = "True"

# Option 3: Per request
response = litellm.completion(
    model="vertex_ai/gemini-2.0-flash-001",
    messages=[{"role": "user", "content": "Hello, how are you?"}],
    use_litellm_proxy=True
)
```

## Supported endpoints (chat/completions, embeddings, etc.)
- `/chat/completions`
- `/completions`
- `/embeddings`
- `/audio/speech`
- `/audio/transcriptions`
- `/images`
- `/images/edits`
- `/rerank`

## Special params / notes
- `use_litellm_proxy=True` — per-request flag to route through proxy (v1.72.1+)
- `litellm.use_litellm_proxy = True` — global flag
- `USE_LITELLM_PROXY` env var — controls SDK-wide proxy routing
- `extra_body={"tags": [...]}` — send tags for categorization/tracking
- OAuth2/JWT authentication: `litellm.proxy_auth = ProxyAuthHandler(credential=AzureADCredential(), scope="...")`
- When `USE_LITELLM_PROXY=True`, requests use `LITELLM_PROXY_API_BASE` with `LITELLM_PROXY_API_KEY` as auth, regardless of model specified.

## Custom pricing / context window behavior
- not documented on this page

## Known caveats
- No YAML config example on this page — only Python SDK usage documented.
- The `USE_LITELLM_PROXY` flag requires v1.72.1 or higher.
- OAuth2/JWT auto-refresh is available via `ProxyAuthHandler`.

## Requirements
- database: not required | redis: not required | enterprise: not required | admin_ui: not required

## Related links
- https://docs.litellm.ai/docs/simple_proxy (Setup LiteLLM Gateway)
- https://docs.litellm.ai/docs/proxy/user_keys (Integration with other libraries)
- https://docs.litellm.ai/docs/proxy_auth (SDK Proxy Authentication)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- This is the chained-proxy pattern for one LiteLLM proxy calling another LiteLLM proxy.
- Pattern: `model: litellm_proxy/<model-name>` + `api_base: <upstream-proxy-url>` + `api_key: <upstream-proxy-key>`.
- The `USE_LITELLM_PROXY` flag (v1.72.1+) routes ALL SDK requests through the proxy regardless of model — useful for centralized proxy management.
- Tags (`extra_body={"tags": [...]}`) enable request categorization for monitoring/analytics.

## Confidence / uncertainty notes
- No YAML config example on this page — YAML shape is inferred from standard LiteLLM patterns.
- HTTP status inferred from successful full-page content delivery (webfetch tool does not expose explicit status header).
