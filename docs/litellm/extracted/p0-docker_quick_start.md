---
source_url: https://docs.litellm.ai/docs/proxy/docker_quick_start
canonical_url: https://docs.litellm.ai/docs/proxy/docker_quick_start
title: Getting Started Tutorial | liteLLM
sidebar_section_path: "LiteLLM AI Gateway (Proxy) > Getting Started Tutorial"
fetched_http_status: 200
priority_tier: P0
extraction_confidence: high
---
# Getting Started Tutorial | liteLLM

## Headings
- ## Quick Install (Recommended for local / beginners)
- ## Pre-Requisites
- ## Step 1 — Pull the LiteLLM database image
- ## Step 2 — Set up a database
- ## Step 3 — Start the proxy server and test it
- ## Optional — Navigate to the LiteLLM UI and generate a virtual key
- ## Step 1 — Add a model
- ## 2. Make a successful /chat/completion call
- ## Optional: Generate a virtual key
- ## Key Concepts
- ## Troubleshooting
- ## Support & Talk with founders
- ### 1. Install
- ### 2. Follow the wizard
- ### 3. Make a call
- ### 2.1 — Get `docker-compose.yml` and create `.env`
- ### 2.2 — Create `config.yaml`
- ### 2.3 — Create `prometheus.yml`
- ### Model List Specification
- ### 2.1 Start Proxy
- ### 2.2 Make Call
- ### Prerequisite — Set up a database
- ### Start Proxy
- ### Create Key w/ RPM Limit
- ### Test it!
- ### Understanding Model Configuration
- ### `prometheus.yml` mount error — "not a directory"
- ### Non-root docker image?
- ### SSL Verification Issue / Connection Error.
- ### (DB) All connection attempts failed

## Exact config keys found
- `model_list` (section: top-level)
- `model_list[].model_name` (section: model_list)
- `model_list[].litellm_params` (section: model_list)
- `model_list[].litellm_params.model` (section: model_list)
- `model_list[].litellm_params.api_base` (section: model_list)
- `model_list[].litellm_params.api_key` (section: model_list)
- `model_list[].litellm_params.api_version` (section: model_list)
- `general_settings` (section: top-level)
- `general_settings.master_key` (section: general_settings)
- `general_settings.database_url` (section: general_settings)
- `litellm_settings` (section: top-level)
- `litellm_settings.ssl_verify` (section: litellm_settings)

## Exact YAML examples (verbatim — preserve indentation exactly)
```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: azure/my_azure_deployment
      api_base: os.environ/AZURE_API_BASE
      api_key: os.environ/AZURE_API_KEY
      api_version: "2025-01-01-preview"
general_settings:
  master_key: sk-1234 # 🔑 your proxy admin key (must start with sk-)
  database_url: "postgresql://llmproxy:dbpassword9090@db:5432/litellm"
```
(source: https://docs.litellm.ai/docs/proxy/docker_quick_start)

Add a model config (verbatim):
```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: azure/my_azure_deployment
      api_base: os.environ/AZURE_API_BASE
      api_key: "os.environ/AZURE_API_KEY"
      api_version: "2025-01-01-preview" # [OPTIONAL] litellm uses the latest azure api_version by default
```
(source: https://docs.litellm.ai/docs/proxy/docker_quick_start)

Prerequisite config with database (verbatim):
```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: azure/my_azure_deployment
      api_base: os.environ/AZURE_API_BASE
      api_key: "os.environ/AZURE_API_KEY"
      api_version: "2025-01-01-preview" # [OPTIONAL] litellm uses the latest azure api_version by default
general_settings: 
  master_key: sk-1234 
  database_url: "postgresql://<user>:<password>@<host>:<port>/<dbname>" # 👈 KEY CHANGE
```
(source: https://docs.litellm.ai/docs/proxy/docker_quick_start)

SSL fix config (verbatim):
```yaml
model_list:
  - model_name: gpt-4o
    litellm_params:
      model: azure/my_azure_deployment
      api_base: os.environ/AZURE_API_BASE
      api_key: "os.environ/AZURE_API_KEY"
      api_version: "2025-01-01-preview"
litellm_settings:
    ssl_verify: false # 👈 KEY CHANGE
```
(source: https://docs.litellm.ai/docs/proxy/docker_quick_start)

prometheus.yml (verbatim):
```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
scrape_configs:
  - job_name: "litellm"
    static_configs:
      - targets: ["litellm:4000"]
```
(source: https://docs.litellm.ai/docs/proxy/docker_quick_start)

docker-compose volume check (verbatim):
```yaml
services:
  litellm:
    volumes:
      - ./config.yaml:/app/config.yaml # ✅ must be uncommented
    command:
      - "--config=/app/config.yaml" # ✅ must be uncommented
```
(source: https://docs.litellm.ai/docs/proxy/docker_quick_start)

## Exact environment variables
- `LITELLM_MASTER_KEY` — set in .env; proxy admin key (must start with sk-)
- `LITELLM_SALT_KEY` — set in .env; encrypts LLM API key credentials (cannot be changed after adding a model)
- `AZURE_API_BASE` — set in .env; referenced via os.environ/AZURE_API_BASE
- `AZURE_API_KEY` — set in .env; referenced via os.environ/AZURE_API_KEY
- `DATABASE_URL` — postgres connection string (postgresql://user:password@host:port/dbname)
- `OPENAI_API_KEY` — referenced in Common Patterns example (os.environ/OPENAI_API_KEY)

## Exact provider prefixes found
- `azure/` — Azure OpenAI (e.g. azure/my_azure_deployment)
- `openai/` — OpenAI / OpenAI-compatible (e.g. openai/nvidia/llama-3.2-nv-embedqa-1b-v2, openai/meta/llama-3-8b, openai/gpt-4)
- `bedrock/` — AWS Bedrock (e.g. bedrock/anthropic.claude-3-sonnet-20240229-v1:0)

## Exact model strings found
- `gpt-4o` — model_name alias
- `azure/my_azure_deployment` — Azure deployment
- `gpt-4` — model_name alias
- `azure/gpt-4-deployment` — Azure deployment
- `openai/gpt-4` — OpenAI model
- `openai/nvidia/llama-3.2-nv-embedqa-1b-v2` — OpenAI-compatible (nvidia)
- `openai/meta/llama-3-8b` — OpenAI-compatible (meta)
- `bedrock/anthropic.claude-3-sonnet-20240229-v1:0` — Bedrock Claude
- `my-custom-model` — model_name alias (custom OpenAI-compatible)
- `my-llama-model` — model_name alias (vLLM)

## Exact endpoint paths found (runtime inference API on this page)
- `POST /chat/completions` — primary chat endpoint (curl examples to http://0.0.0.0:4000/chat/completions)
- `POST /key/generate` — virtual key generation (curl to http://0.0.0.0:4000/key/generate)
- `GET /ui` — Admin UI (http://localhost:4000/ui)

## Exact CLI commands found
- `curl -fsSL https://raw.githubusercontent.com/BerriAI/litellm/main/scripts/install.sh | sh` — install script
- `litellm --setup` — setup wizard
- `docker pull docker.litellm.ai/berriai/litellm:latest` — pull main image
- `docker pull ghcr.io/berriai/litellm-database:latest` — pull database image
- `uv tool install 'litellm[proxy]'` — install LiteLLM CLI
- `docker run -v $(pwd)/litellm_config.yaml:/app/config.yaml -e AZURE_API_KEY=... -e AZURE_API_BASE=... -p 4000:4000 docker.litellm.ai/berriai/litellm:latest --config /app/config.yaml --detailed_debug` — run with config
- `docker run -v $(pwd)/litellm_config.yaml:/app/config.yaml -e AZURE_API_KEY=... -e AZURE_API_BASE=... -p 4000:4000 ghcr.io/berriai/litellm-database:latest --config /app/config.yaml --detailed_debug` — run database image
- `docker compose up` — start with docker compose
- `litellm --config /app/config.yaml --detailed_debug` — run with CLI
- `rm -rf prometheus.yml` — fix prometheus mount error
- `psql -U postgres` then `CREATE DATABASE litellm;` — DB setup
- `GRANT ALL PRIVILEGES ON DATABASE litellm TO your_username;` — CloudSQL grant

## Exact API routes (management, on this page)
- `POST /key/generate` — virtual key generation (with rpm_limit)

## Requirements
- database: PostgreSQL required for the database image path. "LiteLLM provides a dedicated `litellm-database` image for proxy deployments that connect to Postgres." Docker Compose bundles LiteLLM with a Postgres container at db:5432. "database_url enables virtual keys, spend tracking, and the UI." Connection format: postgresql://user:password@host:port/dbname.
- redis: not documented on this page
- enterprise: only mentioned in footer promo card (not a hard requirement)
- admin_ui: "Optional — Navigate to the LiteLLM UI and generate a virtual key" — open http://localhost:4000/ui and log in with master key. Virtual keys let you track spend, set rate limits, and control model access.

## Deprecations
- none documented on this page

## Caveats / pitfalls
- "⚠️ cannot be changed after adding a model" (re: LITELLM_SALT_KEY)
- "🔑 your proxy admin key (must start with `sk-`)" (re: master_key)
- "🚨 must start with `sk-`" (re: master key requirement)
- "`database_url` enables virtual keys, spend tracking, and the UI. Replace it with your Supabase or Neon connection string if you prefer a managed database."
- "All three files (`.env`, `config.yaml`, `prometheus.yml`) must be present before running `docker compose up`."
- "This file **must exist as a file** before `docker compose up`. If it is missing, Docker auto-creates it as an empty directory and the Prometheus container fails to start." (re: prometheus.yml)
- "the `config.yaml` volume mount and `--config` flag are **not commented out** in `docker-compose.yml`"
- "[OPTIONAL] litellm uses the latest azure api_version by default" (re: api_version)
- SSL fix: `litellm_settings.ssl_verify: false` for self-signed certificates
- DB permission error: "STATEMENT: CREATE DATABASE "litellm"" → "ERROR: permission denied to create" indicates permission issue.
- Model resolution: client sends `model: "gpt-4o"` → proxy looks up `model_name: gpt-4o` in config.yaml → extracts `litellm_params` → routes to provider. The `model` field in litellm_params is `<provider>/<model-name>`.
- "the `openai/` prefix tells litellm which provider to use" — for OpenAI-compatible endpoints, the full string after `openai/` is sent to the provider API.

## Related links
- https://docs.litellm.ai/docs/proxy/configs (Config.yaml Overview)
- https://docs.litellm.ai/docs/proxy/quick_start (CLI Quick Start)
- https://docs.litellm.ai/docs/proxy/deploy (Docker, Helm, Terraform)
- https://docs.litellm.ai/docs/proxy/ui (Admin UI)
- https://docs.litellm.ai/docs/proxy/virtual_keys (Authentication)
- https://litellm-api.up.railway.app/ (Swagger)
- https://github.com/BerriAI/litellm/pkgs/container/litellm-database (database image)
- https://github.com/orgs/BerriAI/packages (all images)

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
This is the primary getting-started tutorial. Relevant to workestrator:
- The config.yaml pattern shown here (model_list with model_name + litellm_params{model, api_base, api_key: os.environ/X} + general_settings{master_key, database_url}) is the canonical structure. The real config uses the same model_list pattern but omits database_url (in-memory).
- `os.environ/AZURE_API_KEY` and `os.environ/AZURE_API_BASE` syntax confirmed here — runs os.getenv() at load time. Used in real config for api_key resolution.
- `general_settings.master_key: sk-1234` — must start with `sk-`. Real config uses `os.environ/...` form for master_key.
- `general_settings.database_url` — "enables virtual keys, spend tracking, and the UI." NOT set in real config (in-memory, no Postgres). `disable_spend_logs: true` compensates.
- `litellm_settings.ssl_verify: false` — available for self-signed cert workarounds. Not used in real config.
- Docker images: `docker.litellm.ai/berriai/litellm:latest` (main, no DB) vs `ghcr.io/berriai/litellm-database:latest` (with DB). Real config is in-memory → main image suffices.
- `LITELLM_SALT_KEY` — encrypts LLM API key credentials, cannot be changed after adding a model. Not used in real config (no DB to encrypt credentials for).
- Model resolution flow: client `model` field → `model_name` in config.yaml → `litellm_params.model` (provider/model) → provider API. This is the core routing mechanism.
- `openai/` prefix = OpenAI-compatible API; the string after `openai/` is the full model identifier sent to the provider. Used in real config for openrouter models (e.g. `openai/...` with api_base pointing to openrouter).
- `POST /key/generate` with `rpm_limit` — virtual key creation with rate limits. Requires database_url (not available in real in-memory config).

## Confidence / uncertainty notes
- HTTP status inferred as 200 from successful full content render (webfetch tool does not expose raw status code).
- All 44 code blocks captured verbatim (high confidence). Some blocks have inline comments collapsed without newlines in the markdown source — reproduced as fetched.
- The page covers both "Quick Install" (litellm --setup wizard) and "Docker" paths. The Docker path is the primary deployment method.
- Config.yaml structure on this page includes general_settings (master_key, database_url) and litellm_settings (ssl_verify) — more complete than quick_start but less complete than config_settings.
