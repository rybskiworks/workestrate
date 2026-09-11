# Inception

> Source: https://docs.litellm.ai/docs/providers/inception

## Prefix

`inception/` (chat), `text-completion-inception/` (fill-in-the-middle). "Provider Route on LiteLLM | `inception/` (chat), `text-completion-inception/` (fill-in-the-middle)"

## Protocol

OpenAI-compatible ("The API is OpenAI-compatible.")

## Environment variables

- Required: `INCEPTION_API_KEY` — "your Inception API key"
- Optional: (none documented)

## api_base behavior

NOT explicitly configurable in examples — fixed base URL is `https://api.inceptionlabs.ai/v1`. Base URL: `https://api.inceptionlabs.ai/v1`

## api_key behavior

Env var: `INCEPTION_API_KEY`. `os.environ["INCEPTION_API_KEY"] = ""  # your Inception API key`. In proxy YAML: `api_key: os.environ/INCEPTION_API_KEY`

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

```yaml
model_list:
  - model_name: mercury-2
    litellm_params:
      model: inception/mercury-2
      api_key: os.environ/INCEPTION_API_KEY
  - model_name: mercury-edit-2
    litellm_params:
      model: text-completion-inception/mercury-edit-2
      api_key: os.environ/INCEPTION_API_KEY
```

## Supported endpoints

- `/chat/completions` (via `inception/` prefix)
- `/fim/completions` (via `text-completion-inception/` prefix — fill-in-the-middle)

## Special params / notes

- Inception-specific parameters (passed through to chat API):
  - `reasoning_effort` (`instant` | `low` | `medium` | `high`) — `instant` is Inception-specific for near real-time
  - `reasoning_summary` (bool)
  - `reasoning_summary_wait` (bool)
  - `diffusing` (bool)
  - `realtime` (bool)
- Supported OpenAI parameters: `max_tokens`, `max_completion_tokens`, `temperature`, `stop`, `tools`, `tool_choice`, `stream`, `stream_options`, `response_format`
- Inception serves the Mercury family of diffusion LLMs (dLLMs)

## Caveats

- Inception serves diffusion LLMs (dLLMs) — the Mercury family.
- `api_base` is NOT configurable — fixed at `https://api.inceptionlabs.ai/v1`.
- `reasoning_effort` has an Inception-specific `instant` value not found in other providers.

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]

Not currently used in `infra/litellm/config.yaml`. Inception is a provider of diffusion LLMs (Mercury family). OpenAI-compatible API. Pattern: `model: inception/mercury-2` + `api_key: os.environ/INCEPTION_API_KEY`. `api_base` is fixed at `https://api.inceptionlabs.ai/v1` — not configurable. Supports FIM (fill-in-the-middle) via `text-completion-inception/mercury-edit-2` prefix. Inception-specific `reasoning_effort="instant"` for near real-time responses.

## Confidence / uncertainty

- `api_base` is NOT documented as configurable — the fixed URL is the only one shown.
- HTTP status confirmed as 200 from successful page fetch.
