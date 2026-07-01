---
source_url: https://docs.litellm.ai/docs/proxy/users
canonical_url: https://docs.litellm.ai/docs/proxy/users
title: "Budgets, Rate Limits"
sidebar_section_path: proxy > budgets_rate_limits
fetched_http_status: 200
priority_tier: P1
extraction_confidence: high
feature_area: budgets
---
# Budgets, Rate Limits

## Headings
- Set Budgets
- Global Proxy
- Team
- Add budgets to teams
- Add budget duration to teams
- Team Members
- Internal User
- Add budgets to users
- Add budget duration to users
- Create new keys for existing user
- Virtual Key
- Add budgets to keys
- Add budget duration to keys
- Set multiple budget windows on a key
- Virtual Key (Model Specific)
- Make a test request
- Agents
- Customers
- Reset Budgets
- Hard budget enforcement (fail closed)
- Set Rate Limits
- TPM Rate Limit Type (Input/Output/Total)
- Set default budget for ALL internal users
- Multi-instance rate limiting
- Grant Access to new model
- Create new keys for existing internal user
- API Specification
- GenericBudgetInfo

## Exact config keys found (full path, verbatim spelling)
- `general_settings.master_key` (general_settings) — proxy admin auth key
- `general_settings.proxy_budget_rescheduler_min_time` (general_settings) — min seconds between budget reset checks
- `general_settings.proxy_budget_rescheduler_max_time` (general_settings) — max seconds between budget reset checks
- `general_settings.fail_closed_budget_enforcement` (general_settings) — force DB-validated budget enforcement
- `general_settings.token_rate_limit_type` (general_settings) — "input"/"output"/"total"
- `litellm_settings.max_budget` (litellm_settings) — global proxy USD budget
- `litellm_settings.budget_duration` (litellm_settings) — global reset window
- `litellm_settings.max_end_user_budget` (litellm_settings) — budget for `user` field on /chat/completions
- `litellm_settings.max_internal_user_budget` (litellm_settings) — default budget for internal users
- `litellm_settings.internal_user_budget_duration` (litellm_settings) — default reset window for internal users
- `model_list[].model_info.access_groups` (model_list) — model access groups
- `key.max_budget` (per-key via `/key/generate`) — USD cap
- `key.budget_duration` (per-key) — reset window
- `key.budget_limits` (per-key) — list of {budget_duration, max_budget}
- `key.model_max_budget` (per-key) — Dict[str, GenericBudgetInfo] (Enterprise only)
- `key.rpm_limit` / `key.tpm_limit` / `key.max_parallel_requests` (per-key)
- `key.model_rpm_limit` / `key.model_tpm_limit` (per-key, dict)
- `team.max_budget` / `team.budget_duration` (per-team)
- `team.rpm_limit` / `team.tpm_limit` / `team.max_parallel_requests` (per-team)
- `team.model_rpm_limit` / `team.model_tpm_limit` (per-team, also via metadata)
- `user.max_budget` / `user.budget_duration` / `user.rpm_limit` / `user.tpm_limit` (per-user)
- `team_member.max_budget_in_team` (per user-in-team via `/team/member_add`)
- `agent.tpm_limit` / `agent.rpm_limit` (per-agent)
- `agent.session_tpm_limit` / `agent.session_rpm_limit` (per-agent)
- `agent.litellm_params.max_iterations` (per-agent)
- `agent.litellm_params.max_budget_per_session` (per-agent)
- `agent.litellm_params.require_trace_id_on_calls_by_agent` (per-agent)

## Exact YAML examples (verbatim — preserve indentation)
```yaml
# Global Proxy
general_settings:
  master_key: sk-1234
litellm_settings:
  # other litellm settings
  max_budget: 0 # (float) sets max budget as $0 USD
  budget_duration: 30d # (str) frequency of reset - You can set duration as seconds ("30s"), minutes ("30m"), hours ("30h"), days ("30d").
```
(source: https://docs.litellm.ai/docs/proxy/users)

```yaml
# Customers
general_settings:
  master_key: sk-1234
litellm_settings:
  max_end_user_budget: 0.0001 # budget for 'user' passed to /chat/completions
```
(source: https://docs.litellm.ai/docs/proxy/users)

```yaml
# Hard budget enforcement (fail closed)
general_settings:
  fail_closed_budget_enforcement: true
```
(source: https://docs.litellm.ai/docs/proxy/users)

```yaml
# TPM Rate Limit Type
general_settings:
  master_key: sk-1234
  token_rate_limit_type: "output"  # Options: "input", "output", "total" (default)
```
(source: https://docs.litellm.ai/docs/proxy/users)

```yaml
# Reset Budgets - rescheduler timing
general_settings:
  proxy_budget_rescheduler_min_time: 1
  proxy_budget_rescheduler_max_time: 1
```
(source: https://docs.litellm.ai/docs/proxy/users)

```yaml
# Set default budget for ALL internal users
model_list:
  - model_name: "gpt-3.5-turbo"
    litellm_params:
      model: gpt-3.5-turbo
      api_key: os.environ/OPENAI_API_KEY
litellm_settings:
  max_internal_user_budget: 0 # amount in USD
  internal_user_budget_duration: "1mo" # reset every month
```
(source: https://docs.litellm.ai/docs/proxy/users)

```yaml
# Grant Access to new model (access_groups)
model_list:
  - model_name: text-embedding-ada-002
    litellm_params:
      model: azure/azure-embedding-model
      api_base: "os.environ/AZURE_API_BASE"
      api_key: "os.environ/AZURE_API_KEY"
      api_version: "2023-07-01-preview"
    model_info:
      access_groups: ["beta-models"] # 👈 Model Access Group
```
(source: https://docs.litellm.ai/docs/proxy/users)

## Exact environment variables
- none documented on this page

## Exact endpoint paths / API routes (runtime + management)
- `POST /team/new` — create a team; accepts `team_alias`, `members_with_roles`, `rpm_limit`, `tpm_limit`, `max_budget`, `budget_duration`, `max_parallel_requests`, `model_rpm_limit`, `model_tpm_limit`, `metadata`
- `POST /team/update` — update existing team
- `POST /team/member_add` — add a user to a team; accepts `team_id`, `max_budget_in_team`, `member`
- `POST /user/new` — create internal user; accepts `user_id`, `models`, `max_budget`, `budget_duration`, `tpm_limit`, `rpm_limit`, `max_parallel_requests`, `team_id`
- `POST /user/update` — update internal user
- `POST /key/generate` — generate virtual key; accepts `user_id`, `team_id`, `models`, `max_budget`, `budget_duration`, `budget_limits`, `model_max_budget`, `rpm_limit`, `tpm_limit`, `max_parallel_requests`, `model_rpm_limit`, `model_tpm_limit`
- `POST /chat/completions` — main proxy endpoint; supports `user` field for end-user budgeting
- `POST /budget/new` — create a budget; accepts `budget_id`, `tpm_limit`, `rpm_limit`
- `POST /customer/new` — create end-user customer; accepts `user_id`, `budget_id`
- `POST /v1/agents` — register agent; accepts `agent_name`, `agent_card_params`, `tpm_limit`, `rpm_limit`, `session_tpm_limit`, `session_rpm_limit`, `litellm_params`
- `PATCH /v1/agents/{agent_id}` — update agent rate limits

## Exact CLI commands
- `litellm /path/to/config.yaml` — start proxy (mentioned only)

## Requirements
- database: required — verbatim: "Requirements: - Need to a postgres database (e.g. Supabase, Neon, etc) See Setup". Also: "Costs Per key get auto-populated in `LiteLLM_VerificationToken` Table"
- redis: required for multi-instance rate limiting — verbatim: "Budget checks read current spend from a cross-pod counter in Redis, which keeps enforcement fast and consistent across workers and replicas. The counter is the source of truth on the hot path, and the database is reconciled in the background." Also: "This moves to using async_increment instead of async_set_cache when updating current requests/tokens. The in-memory cache is synced with redis every 0.01s, to avoid calling redis for every request."
- enterprise: required for `model_max_budget` — verbatim: "✨ This is an Enterprise only feature [Get Started with Enterprise here]" (referring to `model_max_budget` for per-model Virtual Key budgets)
- admin_ui: not documented on this page

## Deprecations
- none documented

## Caveats / pitfalls
- "**Important Notes:** - **Rate limits do not apply to proxy admin users.** - When testing rate limits, use internal user roles (non-admin) to ensure limits are enforced as expected."
- "By default, the server checks for resets every 10 minutes, to minimize DB calls."
- "This will NOT apply if a key has a team_id (team budgets will apply then)."
- "***If a key belongs to a team, the team budget is applied, not the user's personal budget.***"
- "By default the `max_budget` is set to `null` and is not checked for keys"
- "Note: By default, the server checks for resets every 10 minutes, to minimize DB calls. To change this, set `proxy_budget_rescheduler_min_time` and `proxy_budget_rescheduler_max_time`"
- Redis restart caveat: "If Redis restarts and reloads an older snapshot, the counter can come back lower than the spend already recorded in the database; on the hot path that stale value is trusted, which can let a key keep spending past its `max_budget` until the counter is corrected."

## Related links
- /docs/proxy/virtual_keys#setup
- /docs/proxy/team_budgets
- /docs/proxy/project_management
- /docs/proxy/ui_team_soft_budget_alerts
- /docs/proxy/tag_budgets
- /docs/proxy/customers
- /docs/proxy/dynamic_rate_limit
- /docs/proxy/rate_limit_tiers
- /docs/proxy/temporary_budget_increase
- /docs/proxy/budget_reset_and_tz
- /docs/proxy/configs
- /docs/a2a
- /docs/a2a_iteration_budgets

## Workestrator relevance  [PROJECT CONTEXT — NOT upstream docs]
- Per-key, per-team, per-user budgets and rate limits ALL REQUIRE a Postgres database (virtual keys/teams/users). The workestrator runs in-memory (no Postgres, no Redis), so these are UNAVAILABLE. Only `litellm_settings.max_budget` (global proxy budget) and `litellm_settings.budget_duration` (global reset) work without a DB — but even these track spend in the DB, so enforcement is limited. `general_settings.token_rate_limit_type` is moot without per-key rate limits. `model_list[].model_info.access_groups` works without a DB (static config). Multi-instance rate limiting requires Redis (unavailable). `general_settings.fail_closed_budget_enforcement` requires DB (unavailable). `general_settings.disable_spend_logs` (from db_info page) is the relevant setting to avoid DB spend log writes.

## Confidence / uncertainty notes
- high confidence on DB + Redis requirements (verbatim quotes). The global `litellm_settings.max_budget` may partially function without DB but spend tracking is DB-backed (inferred). `access_groups` in model_info is static config (high confidence it works without DB).
