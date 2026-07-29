---
source_url: https://docs.litellm.ai/docs/proxy/virtual_keys
canonical_url: https://docs.litellm.ai/docs/proxy/virtual_keys
title: "Virtual Keys"
sidebar_section_path: proxy > authentication
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
feature_area: auth_access
---
# Virtual Keys

## Headings
- Setup
- Quick Start - Generate a Key
- Spend Tracking
- Model Aliases
- Advanced
  - Pass LiteLLM Key in custom header
  - Enable/Disable Virtual Keys
  - Custom /key/generate
  - Upperbound /key/generate params
  - Default /key/generate params
  - Key Rotations
  - Scheduled Key Rotations
  - Temporary Budget Increase
  - Restricting Key Generation
- Next Steps - Set Budgets, Rate Limits per Virtual Key
- Endpoint Reference (Spec)
  - Keys
  - Users
  - Teams

## Exact config keys found (full path, verbatim spelling)
- `general_settings.master_key` (general_settings) — Proxy Admin key
- `general_settings.database_url` (general_settings) — Postgres connection string
- `general_settings.litellm_key_header_name` (general_settings) — custom header for virtual key
- `general_settings.custom_key_generate` (general_settings) — Custom key-generation callback
- `litellm_settings.drop_params` (litellm_settings)
- `litellm_settings.set_verbose` (litellm_settings)
- `litellm_settings.upperbound_key_generate_params` (litellm_settings) — nested: `max_budget`, `budget_duration`, `duration`, `max_parallel_requests`, `tpm_limit`, `rpm_limit`
- `litellm_settings.default_key_generate_params` (litellm_settings) — nested: `max_budget`, `models`, `duration`, `metadata`, `team_id`
- `litellm_settings.key_generation_settings` (litellm_settings) — nested `team_key_generation` (with `allowed_team_member_roles`, `required_params`) and `personal_key_generation` (with `allowed_user_roles`)
- `model_list[].model_name` (model_list)
- `model_list[].litellm_params.model` (model_list)
- `model_list[].litellm_params.api_key` (model_list)
- `model_list[].litellm_params.api_base` (model_list)

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Quick Start - Generate a Key - Step 1: Save postgres db url
model_list:
  - model_name: gpt-4
    litellm_params:
      model: ollama/llama2
  - model_name: gpt-3.5-turbo
    litellm_params:
      model: ollama/llama2
general_settings:
  master_key: sk-1234
  database_url: "postgresql://<user>:<password>@<host>:<port>/<dbname>" # 👈 KEY CHANGE
```
(source: https://docs.litellm.ai/docs/proxy/virtual_keys)

```yaml
# Advanced - Pass LiteLLM Key in custom header
model_list:
  - model_name: fake-openai-endpoint
    litellm_params:
      model: openai/fake
      api_key: fake-key
      api_base: https://exampleopenaiendpoint-production.up.railway.app/
general_settings:
  master_key: sk-1234
  litellm_key_header_name: "X-Litellm-Key" # 👈 Key Change
```
(source: https://docs.litellm.ai/docs/proxy/virtual_keys)

```yaml
# Advanced - Custom /key/generate
model_list: 
  - model_name: "openai-model"
    litellm_params: 
      model: "gpt-3.5-turbo"
litellm_settings:
  drop_params: True
  set_verbose: True
general_settings:
  custom_key_generate: custom_auth.custom_generate_key_fn
```
(source: https://docs.litellm.ai/docs/proxy/virtual_keys)

```yaml
# Advanced - Upperbound /key/generate params
litellm_settings:
  upperbound_key_generate_params:
    max_budget: 100 # Optional[float], optional): upperbound of $100, for all /key/generate requests
    budget_duration: "10d" # Optional[str], optional): upperbound of 10 days for budget_duration values
    duration: "30d" # Optional[str], optional): upperbound of 30 days for all /key/generate requests
    max_parallel_requests: 1000 # (Optional[int], optional): Max number of requests that can be made in parallel. Defaults to None.
    tpm_limit: 1000 #(Optional[int], optional): Tpm limit. Defaults to None.
    rpm_limit: 1000 #(Optional[int], optional): Rpm limit. Defaults to None.
```
(source: https://docs.litellm.ai/docs/proxy/virtual_keys)

```yaml
# Advanced - Default /key/generate params
litellm_settings:
  default_key_generate_params:
    max_budget: 1.5000
    models: ["azure-gpt-3.5"]
    duration:     # blank means `null`
    metadata: {"setting":"default"}
    team_id: "core-infra"
```
(source: https://docs.litellm.ai/docs/proxy/virtual_keys)

```yaml
# Advanced - Restricting Key Generation
litellm_settings:
  key_generation_settings:
    team_key_generation:
      allowed_team_member_roles: ["admin"]
      required_params: ["tags"] # require team admins to set tags for cost-tracking when generating a team key
    personal_key_generation: # maps to 'Default Team' on UI 
      allowed_user_roles: ["proxy_admin"]
```
(source: https://docs.litellm.ai/docs/proxy/virtual_keys)

## Exact environment variables
- `DATABASE_URL` — Postgres connection string; required for virtual key feature. Format: `postgresql://<user>:<password>@<host>:<port>/<dbname>`
- `LITELLM_MASTER_KEY` — Sets the Proxy Admin key (alternative to `general_settings.master_key`)
- `LITELLM_KEY_ROTATION_ENABLED` — Enable the rotation worker. Default `false`
- `LITELLM_KEY_ROTATION_CHECK_INTERVAL_SECONDS` — How often to scan for keys to rotate (seconds). Default `86400` (24h)
- `LITELLM_KEY_ROTATION_GRACE_PERIOD` — Duration to keep old key valid after rotation (e.g. `24h`, `2d`). Default `""` (immediate revoke)

## Exact endpoint paths / API routes (runtime + management)
- `POST /key/generate` — Generate a virtual key. Accepts `models`, `user_id`, `team_id`, `metadata`, `duration`, `aliases`, `max_budget`, `auto_rotate`, `rotation_interval`, etc.
- `GET /key/info?key=<user-key>` — Get spend/info for a key
- `POST /key/block` — Disable a key. Body: `{"key": "KEY-TO-BLOCK"}`
- `POST /key/unblock` — Re-enable a key. Body: `{"key": "KEY-TO-UNBLOCK"}`
- `POST /key/update` — Update an existing key (e.g. set `auto_rotate`, `temp_budget_increase`, `temp_budget_expiry`)
- `POST /key/{key}/regenerate` — Rotate an existing API Key (Enterprise)
- `POST /user/new` — Create a user. Body: `{"user_email": "..."}`
- `GET /user/info?user_id=...` — Get spend/info for a user
- `POST /team/new` — Create a team. Body: `{"team_alias": "...", "models": [...]}`
- `GET /team/info?team_id=...` — Get spend/info for a team
- `POST /v1/chat/completions` — Standard chat completions endpoint (used in examples)

## Exact CLI commands
- `litellm --config /path/to/config.yaml` — Start the LiteLLM proxy with a config file
- `export DATABASE_URL=postgresql://<user>:<password>@<host>:<port>/<dbname>` — Set Postgres URL before starting proxy

## Requirements
- database: required — verbatim: "Need a postgres database (e.g. Supabase, Neon, etc)". Also: "Set `DATABASE_URL=postgresql://<user>:<password>@<host>:<port>/<dbname>` in your env". Also: "(the proxy Dockerfile checks if the `DATABASE_URL` is set and then initializes the DB connection)". Also under Scheduled Key Rotations Prerequisites: "**Database connection required** - Key rotation requires a connected database to track rotation schedules"
- redis: not documented on this page
- enterprise: required for Key Rotations — verbatim: under "✨ Key Rotations": "This is an Enterprise feature." Also `POST /key/{key}/regenerate` is Enterprise
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- Master key requirement: "🚨 must start with `sk-`"
- "Spend is automatically tracked for the key in the 'LiteLLM_VerificationTokenTable'. If the key has an attached 'user_id' or 'team_id', the spend for that user is tracked in the 'LiteLLM_UserTable', and team in the 'LiteLLM_TeamTable'."
- "⏳ end-users - via `/end_user/info` - [Comment on this issue for end-user cost tracking]" — implies end-user spend tracking is not yet implemented
- Scheduled Key Rotations valid interval formats: `"30s"`, `"30m"`, `"30h"`, `"30d"`, `"90d"`
- Grace period behavior: "Both old and new keys work until the grace period elapses, enabling seamless cutover without production downtime. Omitted or empty = immediate revoke."

## Related links
- /docs/proxy/deploy#deploy-with-database
- https://github.com/BerriAI/litellm/blob/main/docker/Dockerfile.database
- /docs/proxy/ui
- https://litellm-api.up.railway.app/ (Swagger)
- /docs/proxy/token_auth
- /docs/proxy/access_control (RBAC)
- /docs/proxy/users (budgets, rate limits)

## Workestrate relevance  [PROJECT CONTEXT — NOT upstream docs]
- Virtual keys REQUIRE a Postgres database (`general_settings.database_url` / `DATABASE_URL`). The workestrate runs LiteLLM in-memory (no Postgres), so virtual keys, teams, users, spend tracking, and key rotation are ALL UNAVAILABLE. Only `general_settings.master_key` auth works in-memory (single admin key, no per-key budgets/rate limits/model restrictions). The `litellm_key_header_name` custom header works without a DB. `upperbound_key_generate_params`, `default_key_generate_params`, and `key_generation_settings` are moot without `/key/generate` (which requires DB). `general_settings.disable_spend_logs` (documented on db_info page) is relevant since spend logging is DB-backed.

## Confidence / uncertainty notes
- high confidence on DB requirement (multiple verbatim quotes). Enterprise requirement for key rotations is verbatim. The `custom_key_generate` callback path may work without DB for custom auth flows (inferred) but the standard `/key/generate` endpoint requires DB.
