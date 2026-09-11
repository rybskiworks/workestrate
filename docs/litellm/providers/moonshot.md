# Moonshot AI

> Source: https://docs.litellm.ai/docs/providers/moonshot

## Prefix

`moonshot/` — "We support ALL Moonshot AI models, just set `moonshot/` as a prefix when sending completion requests"

## Protocol

OpenAI-compatible (page mentions "seamless OpenAI compatibility")

## Environment variables

- Required: `MOONSHOT_API_KEY` — "your Moonshot AI API key"
- Optional: `MOONSHOT_API_BASE` — override base URL (for China endpoint). `os.environ["MOONSHOT_API_BASE"] = "https://api.moonshot.cn/v1"`

## api_base behavior

Default Global API Base URL: `https://api.moonshot.ai/v1` (verbatim: "This is the one currently implemented"). China API Base URL: `https://api.moonshot.cn/v1`. Override via `MOONSHOT_API_BASE` env var. "Moonshot AI offers two distinct API endpoints: a global one and a China-specific one."

## api_key behavior

Env var: `MOONSHOT_API_KEY`. In proxy YAML: `api_key: os.environ/MOONSHOT_API_KEY`

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

```yaml
model_list:
  - model_name: moonshot-v1-8k
    litellm_params:
      model: moonshot/moonshot-v1-8k
      api_key: os.environ/MOONSHOT_API_KEY
  - model_name: moonshot-v1-32k
    litellm_params:
      model: moonshot/moonshot-v1-32k
      api_key: os.environ/MOONSHOT_API_KEY
  - model_name: moonshot-v1-128k
    litellm_params:
      model: moonshot/moonshot-v1-128k
      api_key: os.environ/MOONSHOT_API_KEY
```

## Supported endpoints

- `/chat/completions`

## Special params / notes

- Temperature range limitation: Moonshot only supports [0, 1] (vs OpenAI's [0, 2]). LiteLLM auto-clamps temperature > 1 to 1.
- Temperature + multiple outputs: if temperature < 0.3 and n > 1, Moonshot raises exception. LiteLLM auto-sets temperature to 0.3.
- Tool choice "required" not supported. LiteLLM converts by adding message "Please select a tool to handle the current issue." and removing tool_choice param.
- Vision models: `kimi-k2.5`, `kimi-latest`, `moonshot-v1-*-vision-preview` accept OpenAI content array with `image_url` blocks.

## Caveats

- Moonshot AI = the company; Kimi = their model line (e.g. kimi-k2.5, kimi-latest).
- The page does NOT document using Moonshot via the `anthropic/` prefix with a custom api_base.
- The page does NOT mention `api.kimi.com/coding` or any Kimi-specific coding endpoint.
- Global endpoint (`api.moonshot.ai/v1`) is "the one currently implemented" — China endpoint requires manual override.

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]

Moonshot AI = the company; Kimi = their model line. The `moonshot/` prefix uses the OpenAI-compatible Moonshot API at `https://api.moonshot.ai/`. The repo's `anthropic/kimi-for-coding` with `api_base: api.kimi.com/coding` is a DIFFERENT integration — it uses the `anthropic/` prefix (Anthropic Messages API protocol) pointed at Kimi's coding endpoint. This pattern is NOT documented on the moonshot page; it's documented on the Anthropic provider page (Custom API Base section). Kimi's coding endpoint (api.kimi.com/coding) speaks the Anthropic Messages API, NOT the OpenAI-compatible API. That's why the repo uses `anthropic/` prefix + custom api_base instead of `moonshot/` prefix. If using the standard Moonshot OpenAI-compatible API, use `moonshot/kimi-k2.5` or `moonshot/moonshot-v1-128k` with `MOONSHOT_API_KEY`.

## Confidence / uncertainty

- The `anthropic/kimi-for-coding` pattern is NOT documented on this page — it's inferred from the Anthropic provider page's Custom API Base section.
- `moonshot/kimi-k2` (without .5) is NOT on the page — only `moonshot/kimi-k2.5` and `moonshot/kimi-latest` are documented.
- HTTP status inferred from successful full-page content delivery.
