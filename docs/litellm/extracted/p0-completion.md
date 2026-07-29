---
source_url: https://docs.litellm.ai/docs/completion
canonical_url: https://docs.litellm.ai/docs/completion
title: Chat Completions | liteLLM
sidebar_section_path: "Supported Endpoints > /chat/completions"
fetched_http_status: 200
priority_tier: P0
extraction_confidence: high
---
# Chat Completions | liteLLM

## Headings
- # Chat Completions (h1 — only heading; page is a card-style index landing page)
- (No h2/h3 section headings in the document body. The four "card titles" are navigation links, not section headings.)

## Exact config keys found
- not documented on this page (this is a category index page; config keys live on child pages)

## Exact YAML examples (verbatim — preserve indentation exactly)
not documented on this page (zero fenced code blocks in the body)

## Exact environment variables
- not documented on this page

## Exact provider prefixes found
- not documented on this page

## Exact model strings found
- not documented on this page

## Exact endpoint paths found (runtime inference API on this page)
- not documented on this page (the page is an index; the actual /chat/completions API reference is on child pages)
- (cross-ref: openapi_route_inventory.json confirms `POST /v1/chat/completions` = inference, `POST /chat/completions` = management)

## Exact CLI commands found
- not documented on this page

## Exact API routes (management, on this page)
- not documented on this page

## Requirements
- database: not documented on this page
- redis: not documented on this page
- enterprise: not documented on this page
- admin_ui: not documented on this page

## Deprecations
- not documented on this page

## Caveats / pitfalls
- This page is a Docusaurus generatedIndexPage (category landing). It links to four child pages and contains NO inline code blocks, NO h2/h3 headings, NO model strings, NO env vars, NO CLI commands, NO requirements, NO deprecations. All detailed content lives on the four linked child pages.
- Meta description: "Details on the completion() function"
- For the actual /chat/completions API reference (params, model strings, code blocks), fetch the child pages listed in Related links.

## Related links
- https://docs.litellm.ai/docs/completion/input (Input Params — "Common Params")
- https://docs.litellm.ai/docs/completion/output (Output — "Format")
- https://docs.litellm.ai/docs/completion/usage (Usage — "LiteLLM returns the OpenAI compatible usage object across all providers.")
- https://docs.litellm.ai/docs/completion/http_handler_config (Custom HTTP Handler — "Configure custom aiohttp sessions for better performance and control in LiteLLM completions.")
- https://docs.litellm.ai/docs/supported_endpoints (parent index)

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
This is the /chat/completions endpoint landing page. It has no direct config.yaml relevance. The actual request/response format for the primary inference endpoint (`POST /v1/chat/completions`) is documented on the child pages (/completion/input, /completion/output, /completion/usage). For the workestrate config, the chat/completions endpoint is the primary inference path used by all clients (OpenAI SDK, curl, Langchain). The request format (model + messages + optional metadata) is documented on /docs/proxy/user_keys (extracted separately as p0-user_keys.md), not on this index page.

## Confidence / uncertainty notes
- HTTP status inferred as 200 from successful full content render (canonical URL present, no error page).
- Page genuinely has no in-body code blocks, headings, model strings, env vars, CLI commands, requirements, or deprecations — confirmed by examining both HTML and markdown. This is by design (Docusaurus generatedIndexPage).
- The four child page URLs are the actual content sources for /chat/completions API reference. These are P3 pages (not in P0 extraction scope) — marked as TODO if detailed param schema is needed.
