---
source_url: https://docs.litellm.ai/docs/proxy/ui
canonical_url: https://docs.litellm.ai/docs/proxy/ui
title: "Quick Start (Admin UI)"
sidebar_section_path: proxy > admin_ui
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: admin
---
# Quick Start (Admin UI)

## Headings
- Quick Start
  - 1. Start the proxy
  - 2. Go to UI
  - 3. Get Admin UI Link on Swagger
  - 4. Change default username + password
  - 5. Configure Root Redirect URL
- Invite-other users
- Model Management
- Disable Admin UI

## Exact config keys found (full path, verbatim spelling)
- None in YAML — all configuration is shown as shell-style `.env` code blocks:
  - `LITELLM_MASTER_KEY="sk-1234"` — master key for the proxy server
  - `UI_USERNAME=ishaan-litellm` — username to sign in on UI
  - `UI_PASSWORD=langchain` — password to sign in on UI
  - `DOCS_URL="/docs"` — Set docs to a different path
  - `ROOT_REDIRECT_URL="/ui"` — Redirect root path (/) to /ui
  - `DISABLE_ADMIN_UI="True"` — disable the Admin UI

## Exact YAML examples (verbatim — preserve indentation)
None on this page — page contains no YAML blocks. All configuration is shown as shell-style `.env` code blocks.
(source: https://docs.litellm.ai/docs/proxy/ui)

## Exact environment variables
- `LITELLM_MASTER_KEY` — master key for the proxy server (also used as the Admin UI credential)
- `UI_USERNAME` — username to sign in on the Admin UI
- `UI_PASSWORD` — password to sign in on the Admin UI
- `DOCS_URL` — sets docs to a different path (default `"/"`)
- `ROOT_REDIRECT_URL` — redirects root path `/` to this URL when `DOCS_URL` is changed
- `DISABLE_ADMIN_UI` — set to `"True"` to disable the Admin UI

## Exact endpoint paths / API routes (runtime + management)
- `http://0.0.0.0:4000/ui` (i.e. `<proxy_base_url>/ui`) — the Admin UI URL
- `http://localhost:4000/` — root of the proxy, where Swagger is served
- Swagger reference: `https://litellm-api.up.railway.app/`

## Exact CLI commands
- `litellm --config /path/to/config.yaml` — start the proxy server with a config file (output: "INFO: Proxy running on http://0.0.0.0:4000")

## Requirements
- database: required — verbatim: "- Requires db connected" (bullet under Quick Start). Also: "- Requires proxy master key to be set". Reference link: "Follow [setup](/docs/proxy/virtual_keys#setup)"
- redis: not documented on this page
- enterprise: not documented on this page (the "LiteLLM Enterprise" footer block is marketing, not a requirement statement)
- admin_ui: this IS the admin UI page

## Deprecations
- none documented

## Caveats / pitfalls
- "Requires proxy master key to be set"
- "Requires db connected"
- "Useful, if your security team has additional restrictions on UI usage." (regarding `DISABLE_ADMIN_UI`)
- "By default, `DOCS_URL` is `"/"`, so this setting is only needed when you've changed `DOCS_URL` to a different path."

## Related links
- /docs/proxy/virtual_keys#setup
- /docs/proxy/self_serve
- /docs/proxy/model_management
- /docs/proxy/ai_hub
- /docs/proxy/sync_models_github
- /docs/proxy/admin_ui_sso
- /docs/proxy/ui_credentials
- /docs/proxy/access_control
- /docs/proxy/customer_usage
- /docs/proxy/ui_logs

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- CRITICAL: The Admin UI REQUIRES a database — verbatim: "Requires db connected". The workestrator runs in-memory (no Postgres), so the Admin UI is UNAVAILABLE. `DISABLE_ADMIN_UI="True"` should be set to disable the UI and avoid DB-dependent errors. `LITELLM_MASTER_KEY` works without DB (master_key auth). `UI_USERNAME` / `UI_PASSWORD` are moot without DB. `DOCS_URL` and `ROOT_REDIRECT_URL` work without DB (static routing). The Swagger at `/` may partially work without DB (endpoint listing) but DB-backed management endpoints will fail.

## Confidence / uncertainty notes
- high confidence on DB requirement (verbatim "Requires db connected"). The `DISABLE_ADMIN_UI` env var is the relevant setting for the in-memory deployment (high confidence). `DOCS_URL` / `ROOT_REDIRECT_URL` work without DB (inferred high confidence — static routing).
