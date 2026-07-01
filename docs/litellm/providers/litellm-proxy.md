# LiteLLM Proxy (LLM Gateway)

> Source: https://docs.litellm.ai/docs/providers/litellm_proxy

## Prefix

`litellm_proxy/` — "Simply use the `litellm_proxy/` prefix before the model name to route your requests through the proxy." "add this prefix to the model name, to route any requests to litellm_proxy - e.g. `litellm_proxy/your-model-name`"

## Protocol

OpenAI-compatible ("LiteLLM Proxy is an OpenAI-compatible gateway")

## Environment variables

- Required: `LITELLM_PROXY_API_KEY` — "your litellm proxy api key" (e.g. "sk-1234")
- Required: `LITELLM_PROXY_API_BASE` — "your litellm proxy api base" (e.g. "http://localhost:4000")
- Optional: `USE_LITELLM_PROXY` — "True" — controls SDK-wide proxy routing (requires v1.72.1+)

## api_base behavior

Env var: `LITELLM_PROXY_API_BASE` (e.g. "http://localhost:4000"). Global attribute: `litellm.api_base = "your-openai-proxy-url"`. Per-request: `api_base = "your-litellm-proxy-url"` kwarg. "If you need to set api_base dynamically, just pass it in completions instead - completions(...,api_base='your-proxy-api-base')"

## api_key behavior

Env var: `LITELLM_PROXY_API_KEY`. Per-request: `api_key = "your-litellm-proxy-api-key"` kwarg

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

NOT documented on this page. The page documents only Python SDK usage. No `config.yaml` / `model_list` YAML examples are present. (inferred) YAML would follow standard pattern:

```yaml
model_list:
  - model_name: <alias>
    litellm_params:
      model: litellm_proxy/<model-name>
      api_key: os.environ/LITELLM_PROXY_API_KEY
      api_base: <proxy-url>
```

## Supported endpoints

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

## Caveats

- No YAML config example on this page — only Python SDK usage documented.
- The `USE_LITELLM_PROXY` flag requires v1.72.1 or higher.
- OAuth2/JWT auto-refresh is available via `ProxyAuthHandler`.

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]

This is the chained-proxy pattern for one LiteLLM proxy calling another LiteLLM proxy. Pattern: `model: litellm_proxy/<model-name>` + `api_base: <upstream-proxy-url>` + `api_key: <upstream-proxy-key>`. The `USE_LITELLM_PROXY` flag (v1.72.1+) routes ALL SDK requests through the proxy regardless of model — useful for centralized proxy management. Tags (`extra_body={"tags": [...]}`) enable request categorization for monitoring/analytics.

## Confidence / uncertainty

- No YAML config example on this page — YAML shape is inferred from standard LiteLLM patterns.
- HTTP status inferred from successful full-page content delivery.
