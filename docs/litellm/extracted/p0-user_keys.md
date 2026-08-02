---
source_url: https://docs.litellm.ai/docs/proxy/user_keys
canonical_url: https://docs.litellm.ai/docs/proxy/user_keys
title: Langchain, OpenAI SDK, LlamaIndex, Instructor, Curl examples | liteLLM
sidebar_section_path: "LiteLLM AI Gateway (Proxy) > Making LLM Requests > Langchain, OpenAI SDK, LlamaIndex, Instructor, Curl examples"
fetched_http_status: 200
priority_tier: P0
extraction_confidence: high
---
# Langchain, OpenAI SDK, LlamaIndex, Instructor, Curl examples | liteLLM

## Headings
- ## /chat/completions
- ## Using Tags for Categorization and Tracking
- ## /embeddings
- ## /moderations
- ## Using with OpenAI compatible projects
- ## Using with Vertex, Boto3, Anthropic SDK (Native format)
- ## Advanced
- ### Request Format (under /chat/completions)
- ### Tag Benefits
- ### Response Format
- ### **Streaming**
- ### Function Calling
- ### Request Format (under /embeddings)
- ### Response Format (under /embeddings)
- ### Request Format (under /moderations)
- ### Response Format (under /moderations)
- ### (BETA) Batch Completions - pass multiple models

## Exact config keys found
- not documented on this page (this is a client-usage page; no config.yaml keys. Config structure is on /docs/proxy/configs.)

## Exact YAML examples (verbatim — preserve indentation exactly)
not documented on this page (no config.yaml examples; all code blocks are client SDK / curl examples)

## Exact environment variables
- not documented on this page (no os.environ env-var definitions; only references to OPENAI_API_KEY in client examples and OPENAI_REVERSE_PROXY / OPENAI_API_KEY in LibreChat .env snippet)

## Exact provider prefixes found
- `openai/` — used in litellm SDK direct example (model="openai/gpt-3.5-turbo")
- `ollama/` — used in guidance example (litellm --model ollama/codellama)
- `groq/` — appears in batch completions response (model='groq/llama3-8b-8192')

## Exact model strings found
- `gpt-3.5-turbo` — OpenAI model used in chat/completions examples
- `gpt-4o` — OpenAI model used in streaming/function-calling examples
- `gpt-4-turbo` — OpenAI model used in streaming/function-calling curl examples
- `gpt-4` — OpenAI model used in Langchain JS example
- `claude-3-opus-20240229` — Anthropic model used in Anthropic SDK example
- `mistral-small-latest` — Mistral model used in Mistral SDK example
- `gemini-pro-flash` — Gemini model used in Instructor example
- `text-embedding-ada-002` — OpenAI embedding model
- `text-moderation-stable` — OpenAI moderation model
- `text-moderation-007` — OpenAI moderation model (in response)
- `gpt-3.5-turbo-0125` — OpenAI model (in batch response)
- `groq/llama3-8b-8192` — Groq model (in batch response)
- `ollama/codellama` — Ollama model (guidance example)

## Exact endpoint paths found (runtime inference API on this page)
- `POST /chat/completions` — primary chat completions endpoint (curl: http://0.0.0.0:4000/chat/completions)
- `POST /v1/chat/completions` — streaming and function-calling curl examples (http://0.0.0.0:4000/v1/chat/completions)
- `POST /embeddings` — embeddings endpoint (curl: http://0.0.0.0:4000/embeddings)
- `POST /moderations` — moderations endpoint (curl: http://0.0.0.0:4000/moderations)
- (prose also mentions /completions, /image/generations, /audio/transcriptions, /audio/speech, /messages as supported)

## Exact CLI commands found
- `litellm --model gpt-3.5-turbo` — start proxy (LibreChat example)
- `litellm --model ollama/codellama --temperature 0.3 --max_tokens 2048 --drop_params` — start proxy with drop_params (guidance example)
- `git clone https://github.com/danny-avila/LibreChat.git` — clone LibreChat
- `docker compose up` — run LibreChat
- `uv add aider` / `aider --openai-api-base http://0.0.0.0:4000 --openai-api-key fake-key` — Aider CLI
- `uv add pyautogen` — AutoGen install

## Exact API routes (management, on this page)
- not documented on this page (this page covers data-plane/LLM request routes only; no /key/*, /team/*, /user/* management routes)

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- not documented on this page (no explicit deprecation labels; "(BETA)" label on Batch Completions is a beta tag, not a deprecation)

## Caveats / pitfalls
- "These are **selected examples**. LiteLLM Proxy is **OpenAI-Compatible**, it works with any project that calls OpenAI. Just change the `base_url`, `api_key` and `model`."
- "Input, Output, Exceptions are mapped to the OpenAI format for all supported models"
- "Set `extra_body={"metadata": { }}` to `metadata` you want to pass" (for logging to langfuse etc.)
- "(BETA) Batch Completions - pass multiple models" — beta feature
- Guidance: "Guidance sends additional params like `stop_sequences` which can cause some models to fail if they don't support it." Fix: "Start your proxy using the `--drop_params` flag"
- Prose uses singular forms `/chat/completion` and `/embedding` in the "This doc covers" line — likely a typo; actual code uses plural `/chat/completions` and `/embeddings`.
- All code fences are bare (no language tags like ```python or ```bash).

## Related links
- https://docs.litellm.ai/docs/completion/input (provider-specific params)
- https://docs.litellm.ai/docs/proxy/configs (config.yaml)
- https://docs.litellm.ai/docs/proxy/clientside_auth (Clientside LLM Credentials — next page)
- https://docs.litellm.ai/docs/proxy/email (Email Notifications — previous page)

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
This page documents how clients call the proxy. Relevant to workestrate:
- The primary inference endpoint is `POST /chat/completions` (and `/v1/chat/completions` for streaming). Clients set `base_url="http://0.0.0.0:4000"` and `api_key` to the proxy master key or a virtual key.
- Auth header pattern: `Authorization: Bearer <key>` (or `Authorization: Bearer $OPTIONAL_YOUR_PROXY_KEY` if master_key not set).
- The `model` field in requests maps to `model_name` in config.yaml (NOT the `litellm_params.model` which is the provider-specific model string).
- `extra_body={"metadata": {...}}` is the mechanism for passing logging metadata (generation_name, trace_id, tags, etc.) — relevant if success_callback (e.g. langfuse) is configured.
- `--drop_params` CLI flag drops unsupported params — corresponds to `litellm_settings.drop_params: True` in config.yaml (used in real config).
- Batch completions (BETA): pass comma-separated model names (e.g. "gpt-3.5-turbo,llama3") to fan out to multiple models.

## Confidence / uncertainty notes
- HTTP status 200, no redirect (confirmed by searcher).
- All 42 code blocks captured verbatim (high confidence). Code fences are bare (no language tags) — reproduced as-is.
- This page is data-plane only (chat/embeddings/moderations requests); no management endpoints documented.
- No environment variables defined on this page (only client-side OPENAI_API_KEY references).
