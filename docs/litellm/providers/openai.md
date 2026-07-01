# OpenAI

> Source: https://docs.litellm.ai/docs/providers/openai

## Prefix

`openai/` — "The `openai/` prefix will call openai.chat.completions.create"

For legacy completions: `text-completion-openai/` prefix calls `openai.completions.create`. For Responses API: `openai/responses/` prefix routes through Responses API.

## Protocol

OpenAI native (chat completions JSON over HTTPS)

## Environment variables

- Required: `OPENAI_API_KEY` — API key for OpenAI
- Optional: `OPENAI_BASE_URL` — custom API endpoint. `os.environ["OPENAI_BASE_URL"] = "https://your_host/v1" # OPTIONAL`
- Optional: `OPENAI_ORGANIZATION` — OpenAI organization ID. `os.environ["OPENAI_ORGANIZATION"] = "your-org-id" # OPTIONAL`
- Optional: `LITELLM_ROUTE_ALL_CHAT_OPENAI_TO_RESPONSES` — "true" to route all OpenAI chat through Responses API

## api_base behavior

Env var: `OPENAI_BASE_URL` — "These also support the `OPENAI_BASE_URL` environment variable, which can be used to specify a custom API endpoint." Global attribute: `litellm.api_base = "https://your_host/v1"`. Per-request: `api_base="your-proxy-api-base"` kwarg. "If you need to set api_base dynamically, just pass it in completions instead - `completions(...,api_base='your-proxy-api-base')`". NOTE: `api_base` as a YAML `litellm_params` key is NOT shown on this page (see openai_compatible page for that form).

## api_key behavior

Env var: `OPENAI_API_KEY`. In proxy YAML: `api_key: os.environ/OPENAI_API_KEY`

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

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

## Supported endpoints

- `/chat/completions`
- `/completions`
- `/v1/responses`
- Embeddings
- Audio Transcription
- Vision
- PDF File Parsing
- Image Generation
- Web Search
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

## Caveats

- `openai/` prefix is used in proxy YAML to disambiguate the OpenAI route. In SDK calls, bare model names work when calling api.openai.com directly.
- `OPENAI_BASE_URL` env var is documented for custom endpoints — this is the mechanism for pointing at OpenAI-compatible endpoints.
- `api_base` as a YAML `litellm_params` key is NOT shown on this page (see openai_compatible page for that form).
- GPT-5.4+ drops `reasoning_effort` from requests that include tools (only supported in Responses API).
- `reasoning_effort` summary field requires OpenAI org verification.

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]

Not currently used in `infra/litellm/config.yaml` for direct OpenAI API calls. The repo uses `openai/neuralwatt` with `api_base: https://api.neuralwatt.com/v1` — this follows the documented pattern. `openai/` prefix + `OPENAI_BASE_URL` (or `api_base` in litellm_params) is the canonical way to point LiteLLM at a custom OpenAI-compatible endpoint. The `openai/` prefix is shared between the OpenAI provider page and the openai_compatible page — both use the same prefix for OpenAI-protocol chat-completions endpoints. For Neuralwatt: `model: openai/neuralwatt` + `api_base: https://api.neuralwatt.com/v1` + `api_key: <key>` is correct. The `openai/responses/` sub-prefix is for OpenAI's Responses API (not relevant for Neuralwatt).

## Confidence / uncertainty

- `api_base` as a YAML `litellm_params` key is NOT shown on this page — it IS shown on the openai_compatible page. Both pages use the same `openai/` prefix.
- The page is very large — all critical sections captured.
- HTTP status inferred from successful full-page content delivery.
