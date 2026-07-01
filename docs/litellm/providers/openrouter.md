# OpenRouter

> Source: https://docs.litellm.ai/docs/providers/openrouter

## Prefix

`openrouter/` — "LiteLLM supports ALL OpenRouter models, send `model=openrouter/<your-openrouter-model>` to send it to open router."

## Protocol

OpenAI-compatible

## Environment variables

- Required: `OPENROUTER_API_KEY` — API key for OpenRouter
- Optional: `OPENROUTER_API_BASE` — "[OPTIONAL] defaults to https://openrouter.ai/api/v1"
- Optional: `OR_SITE_URL`
- Optional: `OR_APP_NAME`

## api_base behavior

Env var: `OPENROUTER_API_BASE` — defaults to `https://openrouter.ai/api/v1`. Can also be passed as `base_url=` kwarg to `completion()`. `os.environ["OPENROUTER_API_BASE"] = "" # [OPTIONAL] defaults to https://openrouter.ai/api/v1`

## api_key behavior

Env var: `OPENROUTER_API_KEY`. `os.environ["OPENROUTER_API_KEY"] = ""`

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

NOT documented on this page. The page shows only Python SDK examples. No `config.yaml` / `model_list` YAML is present. (inferred) YAML would follow standard pattern:

```yaml
model_list:
  - model_name: <alias>
    litellm_params:
      model: openrouter/<provider>/<model>
      api_key: os.environ/OPENROUTER_API_KEY
```

## Supported endpoints

- `litellm.completion()` (chat/text)
- `litellm.embedding()` (embeddings)
- `litellm.image_generation()` (image generation)
- `litellm.image_edit()` (image editing)
- Pass-through params: `transforms`, `models`, `route`
- Image-config params: `image_config={aspect_ratio, image_size}`

## Special params / notes

- OpenRouter-specific params: `transforms`, `models`, `route` — passed as arguments to `litellm.completion()`.
- Image generation: `size` maps to `aspect_ratio` (1024x1024 → 1:1, etc.), `quality` maps to `image_size` (low/standard → 1K, medium → 2K, high/hd → 4K).
- `image_config` dict for native OpenRouter params: `aspect_ratio`, `image_size`.
- Cost tracking: `response._hidden_params['additional_headers']['llm_provider-x-litellm-response-cost']`.

## Caveats

- The model table on the page is small/legacy and does not enumerate current OpenRouter offerings.
- LiteLLM claims to support "ALL OpenRouter models" via the `openrouter/<any-model>` generic pattern.
- No proxy YAML config is shown on this page.

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]

The repo uses `openrouter/z-ai/glm-5.1`, `openrouter/qwen/...`, and `openrouter/nex/...` style model strings. CONFIRMED: The nested format `openrouter/<provider>/<model>` is the correct documented pattern. `openrouter/z-ai/glm-5.1` is structurally valid (`z-ai` is OpenRouter's namespace for Z.AI/Zhipu). Note: OpenRouter uses `z-ai/` (hyphenated) while LiteLLM's native Z.AI provider uses `zai/` (no hyphen) — distinct paths. `openrouter/qwen/...` and `openrouter/nex/...` are structurally consistent with the documented nested pattern, though not explicitly shown on the page. GLM-5.1 is NOT documented on the native Z.AI provider page (only GLM-4.x). OpenRouter is the correct path for GLM-5.1.

## Confidence / uncertainty

- No YAML config example on this page — YAML shape is inferred from standard LiteLLM patterns.
- `openrouter/z-ai/glm-5.1`, `openrouter/qwen/...`, `openrouter/nex/...` are NOT verbatim on the page but follow the documented nested format.
- HTTP status inferred from successful full-page content delivery.
