---
name: litellm-budgets-keys
description: |
  DB-GATED operational reference for LiteLLM virtual keys, model access,
  budgets, and rate-limit tiers. REQUIRES Postgres
  (general_settings.database_url) — UNAVAILABLE in the current in-memory
  workestrate (M1). Load when planning the M4 Postgres cut-over or reviewing
  why key/budget endpoints fail in-memory. Distilled from docs/litellm/;
  in-memory-safe alternatives are listed.
---

# LiteLLM Budgets & Keys (DB-GATED)

> **⚠️ DB-GATED SKILL.** Every feature in this skill REQUIRES Postgres
> (`general_settings.database_url` / `DATABASE_URL`). The current workestrate
> runs in-memory (M1, no Postgres) — **all of these features are UNAVAILABLE**.
> Use the in-memory-safe alternatives at the bottom. Defer this surface to the
> M4 Postgres cut-over.

Distilled operational guidance for LiteLLM virtual keys, per-key/team/user
model access, budgets, and rate-limit tiers. Full detail lives in:

- `docs/litellm/auth-access-budget/README.md` — M1/M4 phasing, DB requirements.
- `docs/litellm/extracted/p1-auth-virtual_keys.md` — virtual keys.
- `docs/litellm/extracted/p1-auth-model_access.md` — model access.
- `docs/litellm/extracted/p1-budgets-users.md` — per-key/team/user budgets.
- `docs/litellm/extracted/p1-budgets-rate_limit_tiers.md` — rate-limit tiers.
- `docs/litellm/schemas/openapi-management-endpoints.index.json` — key mgmt endpoints.

## Triggers

Load this skill when:

- Planning the M4 Postgres cut-over (virtual keys, teams, users, budgets).
- Diagnosing why `/key/generate`, `/team/new`, `/user/new`, `/budget/new` fail.
- Reviewing per-key/team model access (`models` param).
- Configuring per-key/team/user budgets or rate-limit tiers.
- Deciding key rotation strategy (Enterprise).

## What REQUIRES Postgres (all UNAVAILABLE in-memory)

| Feature | Requirement | In-memory? |
|---------|-------------|------------|
| Virtual keys (`/key/*`) | Postgres (`database_url`) | **NO** |
| Teams (`/team/*`), users (`/user/*`) | Postgres | **NO** |
| Per-key / per-team model access (`models` param) | Postgres | **NO** |
| Per-key/team/user budgets | Postgres | **NO** |
| Budget & Rate Limit Tiers (`/budget/new`) | Enterprise + Postgres | **NO** |
| Spend tracking (`/spend/*`) | Postgres | **NO** |
| Admin UI | Postgres | **NO** (set `DISABLE_ADMIN_UI=True`) |
| Dynamic model management (`STORE_MODEL_IN_DB=True`) | Postgres | **NO** (leave False) |
| Multi-instance rate limiting | Redis (cross-pod counters) | **NO** |
| `token_rate_limit_type` | Postgres | **NO** |
| `fail_closed_budget_enforcement` | Postgres | **NO** |
| Key rotation | Enterprise | **NO** |

Verbatim: virtual keys "Need a postgres database (e.g. Supabase, Neon, etc)" /
"Set DATABASE_URL=postgresql://...". Verbatim: "Budget checks read current
spend from a cross-pod counter in Redis" → multi-instance rate limiting
REQUIRES Redis.

## DB-backed management endpoints (all UNAVAILABLE in-memory)

`/key/generate`, `/key/info`, `/key/block`, `/key/unblock`, `/key/update`,
`/user/new`, `/user/info`, `/team/new`, `/team/info`, `/team/update`,
`/team/member_add`, `/budget/new` (Enterprise + DB).

## Per-key/team/user budget keys (ALL REQUIRE DB)

`max_budget`, `budget_duration`, `rpm_limit`, `tpm_limit`,
`max_parallel_requests`, `model_rpm_limit`, `model_tpm_limit`,
`budget_limits`, `model_max_budget` (Enterprise).

## In-memory-safe alternatives (M1)

These work WITHOUT Postgres/Redis and are the correct M1 substitutes:

| Alternative | Key | Source |
|-------------|-----|--------|
| `master_key`-only auth | `general_settings.master_key` (must start `sk-`) | `p1-auth-virtual_keys.md` |
| Per-deployment rpm/tpm | `model_list[].litellm_params.rpm` / `tpm` | `p0-deploy.md` (inferred in-process) |
| Per-deployment parallelism cap | `max_parallel_requests` / `global_max_parallel_requests` | option-index (inferred) |
| Static model access groups | `model_info.access_groups` | option-index (`requires_db=false`) |
| Model access = which `model_name` entries exist | `model_list` | inferred (no per-key gating) |
| Suppress DB spend-log writes | `general_settings.disable_spend_logs: true` | `p0-config_settings.md` |
| IP allowlist | `general_settings.allowed_ips` | option-index (`requires_db=false`) |

In-memory, every `master_key` caller can access **all** configured `model_name`
entries — do not rely on LiteLLM for tenant isolation until Postgres is added.

## M4 cut-over (when Postgres is added)

Add as a single coordinated change (not piecemeal):

- `general_settings.database_url` / `DATABASE_URL`.
- Virtual keys (`/key/generate` with `models` param).
- Teams/users (`/team/new`, `/user/new`).
- Per-key/team/user budgets (keys above).
- Spend tracking (remove `disable_spend_logs` if `/spend/*` needed).
- Admin UI (remove `DISABLE_ADMIN_UI`).
- `STORE_MODEL_IN_DB=True` for dynamic model management.
- `fail_closed_budget_enforcement`, `token_rate_limit_type`.
- Budget & Rate Limit Tiers (`/budget/new`, Enterprise + DB).
- Set `LITELLM_SALT_KEY` **before** adding any models/credentials (immutable
  afterward).
- Redis (separate later step) for cross-pod rate-limit counters,
  `enable_redis_auth_cache`, `cache_params.type: redis`.

## Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| `/key/generate` 500 / DB error | no `database_url` | Add Postgres (M4) or use `master_key`-only. |
| `lite keys generate` fails | DB-backed CLI command | Use `lite chat completions` / `lite http request` for inference testing. |
| Admin UI blank / errors | requires DB | Set `DISABLE_ADMIN_UI="True"`. |
| `STORE_MODEL_IN_DB=True` breaks model loading | no DB | Leave False; keep models static in `config.yaml`. |
| `token_rate_limit_type` errors | requires DB | Do not set in M1. |
| Spend logs write errors | no DB | `disable_spend_logs: true` (mandatory in-memory). |
| `master_key` rejected | must start `sk-` | Ensure `LITELLM_MASTER_KEY` begins with `sk-`. |
| `master_key` resolves empty | `os.environ/VAR` unset at load | Set env var before proxy startup. |
| Per-key model isolation absent | in-memory = all models to all callers | Add Postgres (M4) for per-key `models`. |

## Related Docs

- `docs/litellm/auth-access-budget/README.md`
- `docs/litellm/extracted/p1-auth-virtual_keys.md`
- `docs/litellm/extracted/p1-auth-model_access.md`
- `docs/litellm/extracted/p1-budgets-users.md`
- `docs/litellm/extracted/p1-budgets-rate_limit_tiers.md`
- `docs/litellm/schemas/openapi-management-endpoints.index.json`
- `docs/litellm/schemas/config-yaml.option-index.json`

> Do not hallucinate budget/key endpoints or `requires_db` flags. Every
> endpoint and key above traces to `auth-access-budget/README.md` and the cited
> extracted corpus. In-memory-safe alternatives are marked "(inferred)" where
> the corpus infers rather than documents. This skill is DB-GATED: do not
> enable any feature here without Postgres.
