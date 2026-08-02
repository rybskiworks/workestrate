# Z.AI (Zhipu AI)

> Source: https://docs.litellm.ai/docs/providers/zai

## Prefix

`zai/` — "We support Z.AI GLM text/chat models, just set `zai/` as a prefix when sending completion requests"

CRITICAL: The prefix is `zai/` (no hyphen). This differs from OpenRouter's `z-ai/` (hyphenated) namespace.

## Protocol

OpenAI-compatible (chat completions via `/v1/chat/completions`)

## Environment variables

- Required: `ZAI_API_KEY` — API key for Z.AI
- Optional: (none documented)

## api_base behavior

NOT documented on this page. No `api_base` is shown in any example. The URL `https://z.ai/` appears as the provider homepage link only.

## api_key behavior

Env var: `ZAI_API_KEY`. `os.environ['ZAI_API_KEY'] = ""`. In proxy YAML: `api_key: os.environ/ZAI_API_KEY`

## api_version behavior

not documented on this page

## Proxy YAML example (verbatim)

```yaml
model_list:
  - model_name: glm-4.7
    litellm_params:
        model: zai/glm-4.7
        api_key: os.environ/ZAI_API_KEY

  - model_name: glm-4.5-flash  # Free tier
    litellm_params:
        model: zai/glm-4.5-flash
        api_key: os.environ/ZAI_API_KEY
```

## Supported endpoints

- `/v1/chat/completions` — only endpoint demonstrated

## Special params / notes

Model pricing table (verbatim):

- glm-4.7: $0.60/M input, $2.20/M output, $0.11/M cached, 200K context
- glm-4.6: $0.60/M input, $2.20/M output, 200K context
- glm-4.5: $0.60/M input, $2.20/M output, 128K context
- glm-4.5v: $0.60/M input, $1.80/M output, 128K context
- glm-4.5-x: $2.20/M input, $8.90/M output, 128K context
- glm-4.5-air: $0.20/M input, $1.10/M output, 128K context
- glm-4.5-airx: $1.10/M input, $4.50/M output, 128K context
- glm-4-32b-0414-128k: $0.10/M input, $0.10/M output, 128K context
- glm-4.5-flash: FREE, 128K context

## Caveats

- The prefix is `zai/` (no hyphen) — NOT `z-ai/` (hyphenated). OpenRouter uses `z-ai/` for the same provider.
- GLM-5.1 is NOT documented on this page — only GLM-4.x models.
- `api_base` is NOT documented — the exact Z.AI API base URL is not stated on this page.
- Provider name variations: "Z.AI" (primary), "Zhipu AI" (parenthesized alias).

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]

The repo uses `openrouter/z-ai/glm-5.1` (via OpenRouter). This is the CORRECT path for GLM-5.1 since it's not documented on the native Z.AI provider page. CRITICAL SPELLING DIFFERENCE: LiteLLM native prefix is `zai/` (no hyphen); OpenRouter namespace is `z-ai/` (hyphenated). These are distinct routing paths. Do NOT change `openrouter/z-ai/glm-5.1` to `zai/glm-5.1` — GLM-5.1 is not in the native Z.AI docs and would likely fail routing. If direct Z.AI access is desired, use `zai/glm-4.7` (latest flagship) or `zai/glm-4.5-flash` (free tier). The repo's GLM-5.1 access via OpenRouter is the correct approach since GLM-5.1 is not yet in LiteLLM's native Z.AI docs.

## Confidence / uncertainty

- GLM-5.1 is NOT documented — only GLM-4.x. The model may be newer than the docs or only available via OpenRouter.
- `api_base` is NOT documented on this page — the exact Z.AI API endpoint URL is unknown from this page alone.
- HTTP status inferred from successful full-page content delivery.
