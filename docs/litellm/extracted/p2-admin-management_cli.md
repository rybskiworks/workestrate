---
source_url: https://docs.litellm.ai/docs/proxy/management_cli
canonical_url: https://docs.litellm.ai/docs/proxy/management_cli
title: "LiteLLM Proxy CLI"
sidebar_section_path: proxy > admin_ui_cli
fetched_http_status: 200
priority_tier: P2
extraction_confidence: high
feature_area: admin
---
# LiteLLM Proxy CLI

## Headings
- Quick Start
- Authentication using CLI
  - Prerequisites
  - Steps
- Main Commands
  - Models Management
  - Credentials Management
  - Keys Management
  - User Management
  - Chat Completions
  - General HTTP Requests
- Environment Variables
- Examples
- Error Handling

## Exact config keys found (full path, verbatim spelling)
- None in YAML — page uses shell-style code blocks only:
  - `LITELLM_PROXY_URL` — Base URL of the proxy server
  - `LITELLM_PROXY_API_KEY` — API key for authentication
  - `EXPERIMENTAL_UI_LOGIN` — must be set to `"True"` on proxy startup to enable CLI SSO Authentication (beta feature)

## Exact YAML examples (verbatim — preserve indentation)
None on this page — page uses shell-style code blocks only.
(source: https://docs.litellm.ai/docs/proxy/management_cli)

## Exact environment variables
- `LITELLM_PROXY_URL` — Base URL of the proxy server
- `LITELLM_PROXY_API_KEY` — API key for authentication
- `EXPERIMENTAL_UI_LOGIN` — must be set to `"True"` on proxy startup to enable CLI SSO Authentication (beta feature)

## Exact endpoint paths / API routes (runtime + management)
- Implicitly the proxy base URL e.g. `http://localhost:4000` (used via env var)
- `/chat/completions` — used in the `lite http request` example
- OpenAPI references:
  - https://litellm-api.up.railway.app/#/model%20management
  - https://litellm-api.up.railway.app/#/credential%20management
  - https://litellm-api.up.railway.app/#/key%20management
  - https://litellm-api.up.railway.app/#/Internal%20User%20management
  - https://litellm-api.up.railway.app/#/chat%2Fcompletions

## Exact CLI commands
- `curl -fsSL https://raw.githubusercontent.com/BerriAI/litellm/main/scripts/install-cli.sh | sh` — one-line installer for the `lite` CLI
- `brew install BerriAI/litellm/lite` — macOS Homebrew install
- `uv tool install 'litellm[cli]'` — install via uv
- `lite` — invoke the CLI (prints help)
- `lite models list` — list models
- `lite login` — SSO login (opens browser)
- `lite models add gpt-4 --param api_key=sk-123 --param max_tokens=2048` — add a model
- `lite models update <model-id> -p temperature=0.7` — update a model
- `lite models delete <model-id>` — delete a model
- `lite credentials list` — list credentials
- `lite credentials create azure-prod --info='{"custom_llm_provider": "azure"}' --values='{"api_key": "sk-123", "api_base": "https://prod.azure.openai.com"}'` — create a credential
- `lite credentials get azure-cred` — get a credential
- `lite credentials delete azure-cred` — delete a credential
- `lite keys list` — list API keys
- `lite keys generate --models=gpt-4 --spend=100 --duration=24h --key-alias=my-key` — generate a key
- `lite keys info --key sk-key1` — key info
- `lite keys delete --keys sk-key1,sk-key2 --key-aliases alias1,alias2` — delete keys
- `lite users list` — list users
- `lite users create --email=user@example.com --role=internal_user --alias="Alice" --team=team1 --max-budget=100.0` — create a user
- `lite users get --id <user-id>` — get user info
- `lite users delete <user-id>` — delete a user
- `lite chat completions gpt-4 -m "user:Hello, how are you?"` — run chat completions
- `lite http request POST /chat/completions --json '{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello"}]}'` — raw HTTP request
- `litellm --config config.yaml` — start the proxy server (used in prerequisites for SSO)

## Requirements
- database: not documented on this page explicitly. However, all CLI operations (models, credentials, keys, users) hit proxy endpoints that, per the db_info page, are stored in Postgres.
- redis: not documented on this page
- enterprise: not documented on this page. The only callout is on `EXPERIMENTAL_UI_LOGIN` which is described as a "Beta Feature" (not enterprise).
- admin_ui: not documented on this page

## Deprecations
- none documented. Note: the `lite` CLI is described as the current tool. The previous `litellm-proxy` CLI is not explicitly marked deprecated on this page.

## Caveats / pitfalls
- "CLI SSO Authentication is currently in beta." (regarding `EXPERIMENTAL_UI_LOGIN`)
- "You must set this environment variable **when starting up your LiteLLM Proxy**"
- "The CLI will display error messages for: Server not accessible; Authentication failures; Invalid parameters or JSON; Nonexistent models/credentials; Any other operation failures"
- "Use the `--debug` flag for detailed debugging output."
- "Any of these gives you the `lite` command; if you already run a proxy server from `litellm[proxy]`, it ships there too."

## Related links
- /docs/proxy/cli_sso — CLI Authentication in-depth guide
- https://github.com/BerriAI/litellm/blob/main/litellm/proxy/client/cli/README.md — CLI README
- https://litellm-api.up.railway.app/ — Swagger
- https://github.com/astral-sh/uv — uv dependency

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- The `lite` CLI commands for models/credentials/keys/users management hit proxy endpoints that require a database (per db_info page). The workestrator runs in-memory (no Postgres), so `lite models add`, `lite credentials create`, `lite keys generate`, `lite users create` will FAIL (DB-backed). However, `lite chat completions` and `lite http request POST /chat/completions` work without DB (inference endpoints). `lite models list` may partially work (lists config.yaml models — inferred). `LITELLM_PROXY_URL` and `LITELLM_PROXY_API_KEY` env vars are relevant for CLI configuration. `EXPERIMENTAL_UI_LOGIN` is moot without DB-backed SSO. The CLI is a thin client — it doesn't require DB itself, but the proxy endpoints it calls do.

## Confidence / uncertainty notes
- high confidence on CLI commands (verbatim). DB requirement is inferred from db_info page (not stated on this page explicitly) — marked as inferred. `lite chat completions` and `lite http request` work without DB (inferred high confidence — they hit inference endpoints). `lite models list` working without DB is inferred (may list config.yaml models).
