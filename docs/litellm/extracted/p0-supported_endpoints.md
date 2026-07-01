---
source_url: https://docs.litellm.ai/docs/supported_endpoints
canonical_url: https://docs.litellm.ai/docs/supported_endpoints
title: Supported Endpoints | liteLLM
sidebar_section_path: "Supported Endpoints"
fetched_http_status: 200
priority_tier: P0
extraction_confidence: high
---
# Supported Endpoints | liteLLM

## Headings
- # Supported Endpoints (h1 — only authored heading)
- (47 h2 card titles — these are Docusaurus link cards to child endpoint docs, not authored section headings. Listed below as endpoint paths.)

## Exact config keys found
- not documented on this page (this is an endpoint index page; no config.yaml keys)

## Exact YAML examples (verbatim — preserve indentation exactly)
not documented on this page (zero code blocks on this page)

## Exact environment variables
- not documented on this page

## Exact provider prefixes found
- not documented on this page

## Exact model strings found
- not documented on this page

## Exact endpoint paths found (runtime inference API on this page)
The page lists endpoint paths as URL-path strings only (NO HTTP method verbs). Cross-referenced with openapi_route_inventory.json for METHOD + inference/management classification:

- `/a2a` — A2A Agent Gateway (openapi: no direct match; A2A routes exist under other tags)
- `/assistants` — OpenAI Assistants API (openapi tag: "assistants"; DEPRECATED — shuts down Aug 26, 2026)
- `/audio/transcriptions` — Audio transcriptions (openapi: `POST /v1/audio/transcriptions` = inference)
- `/audio/speech` — Audio speech (openapi: `POST /v1/audio/speech` = inference)
- `/batches` — Batch API (openapi tag: "batch"; `POST /v1/batches` = inference)
- `/containers` — Code interpreter containers (openapi tag: "containers")
- `/containers/files` — Container files (openapi: `/v1/containers/{container_id}/files`)
- `/chat/completions` — Chat completions (openapi tag: "chat/completions"; `POST /v1/chat/completions` = inference) — THE PRIMARY INFERENCE ENDPOINT
- `/completions` — Text completions (openapi tag: "completions"; `POST /v1/completions` = inference)
- `/converse` — Bedrock /converse passthrough
- `/embeddings` — Embeddings (openapi tag: "embeddings"; `POST /v1/embeddings` = inference)
- `/files` — File management (openapi tag: "files")
- `/fine_tuning` — Fine-tuning (openapi tag: "fine_tuning")
- `/evals` — Evaluations API
- `/generateContent` — Google AI generateContent
- `/guardrails/apply_guardrail` — Direct guardrail invocation
- `/invoke` — Bedrock /invoke passthrough
- `/interactions` — Interactions
- `/v1beta/agents` — Gemini Managed Agents
- `/memory` — User/team-scoped memory CRUD
- `/images/edits` — Image editing
- `Image Generations` — Image generation (note: no leading slash in card title)
- `/image/variations` — Image variations [BETA]
- `/videos` — Video generation (openapi tag: "videos")
- `/vector_stores/{vector_store_id}/files` — Vector store files
- `/vector_stores` — Create vector store
- `/vector_stores/search` — Search vector store
- `/mcp` — Model Context Protocol (openapi: `/{mcp_server_name}/mcp` routes)
- `/v1/messages` — Anthropic Messages API (openapi tag: "responses" includes `/v1/messages`-style routes)
- `Token Counting` — Token counting (openapi: `POST /utils/token_counter`)
- `/v1/messages/count_tokens` — Anthropic count_tokens
- `/moderations` — Moderations (openapi tag: "moderations"; `POST /v1/moderations` = inference)
- `/ocr` — OCR (openapi tag: "ocr"; `POST /v1/ocr` = inference)
- `Pass-through Endpoints` — Pass-through endpoints (Anthropic SDK, etc.) (openapi: `/config/pass_through_endpoint` routes)
- `/rag/ingest` — RAG ingestion (openapi tag: "rag"; `POST /v1/rag/ingest` = inference)
- `/rag/query` — RAG query (openapi: `POST /v1/rag/query` = inference)
- `/realtime` — Realtime API (openapi tag: "WebSocket"; `/v1/realtime` = inference)
- `/rerank` — Rerank (openapi tag: "rerank"; `POST /v1/rerank` = inference)
- `/responses` — Responses API (openapi tag: "responses"; `POST /v1/responses` = inference)
- `/responses/compact` — Compact response (openapi: `POST /v1/responses/compact` = inference)
- `/search` — Search
- `/skills` — Anthropic Skills API

## Exact CLI commands found
- not documented on this page

## Exact API routes (management, on this page)
- not documented on this page (management routes are on child pages; this page is an index)

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- "OpenAI has deprecated the Assistants API. It will shut down on August 26, 2026." (in /assistants card description)

## Caveats / pitfalls
- This page is a Docusaurus category index (generatedIndexPage). It contains NO prose body content, NO code blocks, NO CLI commands, NO env vars. It is purely an index of card links to child endpoint docs.
- The page does NOT use "METHOD /path" notation — endpoints are listed as URL-path strings only. HTTP verbs come from child pages or the OpenAPI spec.
- The page does NOT distinguish "inference" vs "management" endpoints. The only visual distinction is card icons: 📄️ (single page) vs 🗃️ (category/folder with children).
- "Image Generations" card title has NO leading slash, unlike every other endpoint card — this is verbatim from the page.
- Three cards (/interactions, /videos, /ocr, /skills) show raw markdown table-header text in their card descriptions (e.g. "| Feature | Supported |") — a markdown-rendering bug on those child docs.

## Related links
- https://docs.litellm.ai/docs/assistants (/assistants)
- https://docs.litellm.ai/docs/chat/completions (/chat/completions — 4 child items)
- https://docs.litellm.ai/docs/completion (/chat/completions landing)
- https://docs.litellm.ai/docs/embeddings (/embeddings)
- https://docs.litellm.ai/docs/mcp (/mcp — 17 items)
- https://docs.litellm.ai/docs/v1/messages (/v1/messages — 3 items)
- https://docs.litellm.ai/docs/responses (/responses)
- https://docs.litellm.ai/docs/batches (/batches — 2 items)
- https://docs.litellm.ai/docs/search (/search — 14 items)
- https://litellm-api.up.railway.app/ (Swagger — all endpoints)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
This page is the canonical endpoint index. For the workestrator config, the relevant inference endpoints are:
- `POST /v1/chat/completions` (and `/chat/completions`) — the primary inference endpoint used by all clients. Confirmed via openapi_route_inventory.json: `chat_completion_v1_chat_completions_post` tagged "inference".
- `GET /v1/models` — model list (inference). Confirmed via openapi: `model_list_v1_models_get` tagged "inference".
- The real config only uses chat/completions inference. Other endpoints (embeddings, moderations, rerank, responses) are available but not currently configured.
- Cross-reference with openapi_route_inventory.json confirms 125 inference operations and 544 management operations across 491 paths / 669 total operations. The `/v1/` prefixed variants are the "inference" routes; the non-`/v1/` variants of the same paths are "management" routes.

## Confidence / uncertainty notes
- HTTP status inferred as 200 from successful full content render.
- The 44 endpoint card titles are verbatim from the rendered HTML (high confidence).
- METHOD verbs and inference/management classification are NOT from this page — they are cross-referenced from openapi_route_inventory.json (the on-disk OpenAPI route inventory). Marked as (cross-ref: openapi).
- The page itself makes no inference/management distinction; groupings in the endpoint list above are inferred from openapi tags.
