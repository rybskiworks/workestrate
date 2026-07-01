# Anthropic

> Source: https://docs.litellm.ai/docs/providers/anthropic

## Prefix

`anthropic/` — "add this prefix to the model name, to route any requests to Anthropic - e.g. `anthropic/claude-3-5-sonnet-20240620`"

## Protocol

both (OpenAI-compatible `/chat/completions` + Anthropic native `/v1/messages` passthrough)

## Environment variables

- Required: `ANTHROPIC_API_KEY` — API key for Anthropic
- Optional: `ANTHROPIC_API_BASE` — custom API base URL (for proxy or custom endpoint). `# os.environ['ANTHROPIC_API_BASE'] = '' # [OPTIONAL] or 'ANTHROPIC_BASE_URL'`
- Optional: `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX` — "true" to disable automatic URL suffix appending. `# os.environ['LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX'] = 'true' # [OPTIONAL] Disable automatic URL suffix appending`

## api_base behavior

CRITICAL — Custom API Base IS DOCUMENTED. "When using a custom API base for Anthropic (e.g., a proxy or custom endpoint), LiteLLM automatically appends the appropriate suffix (`/v1/messages` or `/v1/complete`) to your base URL."

`api_base` can be passed as a parameter to `completion()` or via env var `ANTHROPIC_API_BASE`.

URL suffix behavior:

- Without `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX`:
  - `https://my-proxy.com` → `https://my-proxy.com/v1/messages`
  - `https://my-proxy.com/api` → `https://my-proxy.com/api/v1/messages`
- With `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true`:
  - `https://my-proxy.com/custom/path` → `https://my-proxy.com/custom/path` (unchanged)

## api_key behavior

Configured via `ANTHROPIC_API_KEY` env var or `api_key` parameter. `os.environ["ANTHROPIC_API_KEY"] = "your-api-key"`

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

```yaml
model_list:
  - model_name: claude-4 ### RECEIVED MODEL NAME ###
    litellm_params: # all params accepted by litellm.completion() - https://docs.litellm.ai/docs/completion/input
      model: claude-opus-4-20250514 ### MODEL NAME sent to `litellm.completion()` ###
      api_key: "os.environ/ANTHROPIC_API_KEY" # does os.getenv("ANTHROPIC_API_KEY")
```

## Supported endpoints

- `/chat/completions` (OpenAI-compatible)
- `/v1/messages` (Anthropic passthrough)

## Special params / notes

- Anthropic API fails requests when `max_tokens` are not passed. LiteLLM passes `max_tokens=4096` when no `max_tokens` is passed.
- `response_format` fully supported for Claude Sonnet 4.5 and Opus 4.1 models (Structured Outputs).
- `reasoning_effort` mapped to `output_config={"effort": ...}` for Claude 4.6 and Opus 4.5 models.
- Prompt caching via `cache_control: {"type": "ephemeral"}` in message content.
- Thinking/reasoning: `reasoning_effort` ("low"|"medium"|"high") or native `thinking={"type": "enabled", "budget_tokens": 1024}` or `thinking={"type": "adaptive"}`.
- Azure Foundry: use `azure/` prefix or `anthropic/` with `api_base`.

## Caveats

- `max_tokens` is required by Anthropic API; LiteLLM defaults to 4096 if not passed.
- `reasoning_effort` other than "none" automatically turns thinking on for Claude 4.6/4.7 models.
- `budget_tokens` deprecated on 4.6 models, rejected on Opus 4.7 (only adaptive supported).

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]

CRITICAL: The `anthropic/` prefix + custom `api_base` IS a documented pattern. This confirms the repo's `anthropic/kimi-for-coding` with `api_base: api.kimi.com/coding` and `anthropic/MiniMax-M3` with `api_base: api.minimax.io/anthropic` is valid — pointing the `anthropic/` prefix at non-Anthropic endpoints that speak the Anthropic Messages API. IMPORTANT NUANCE: LiteLLM auto-appends `/v1/messages` to the api_base. For Kimi (`api.kimi.com/coding`), the final URL is `api.kimi.com/coding/v1/messages`; for MiniMax (`api.minimax.io/anthropic`), it is `api.minimax.io/anthropic/v1/messages` — which matches the MiniMax docs page exactly. If the endpoint already includes the full path, set `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true` to prevent auto-append. NOTE: The MiniMax provider page documents using `minimax/` prefix (not `anthropic/`) with `litellm.anthropic.messages.acreate()` and `api_base: https://api.minimax.io/anthropic/v1/messages`. The repo's use of `anthropic/MiniMax-M3` may be an alternative valid approach but differs from the documented `minimax/` prefix pattern. See [minimax.md](minimax.md).

## Confidence / uncertainty

- The page was truncated in fetch (6346 bytes truncated) but all critical sections were captured.
- `custom_llm_provider` is NOT documented on this page.
- `api_version` is NOT documented on this page.
- HTTP status inferred from successful full-page content delivery.
