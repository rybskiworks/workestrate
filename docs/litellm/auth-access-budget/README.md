# Auth, Access & Budget

> Synthesized from on-disk corpus under `docs/litellm/extracted/` and
> `docs/litellm/schemas/`. No web fetches were performed. Every key, default,
> and requirement traces to a cited corpus file or is marked **(inferred)** /
> **"not documented in fetched source"**.

## Sources

- https://docs.litellm.ai/docs/proxy/virtual_keys — `extracted/p1-auth-virtual_keys.md`
- https://docs.litellm.ai/docs/proxy/model_access — `extracted/p1-auth-model_access.md`
- https://docs.litellm.ai/docs/proxy/users — `extracted/p1-budgets-users.md`
- https://docs.litellm.ai/docs/proxy/budget_rate_limits — `extracted/p1-budgets-rate_limit_tiers.md`
- https://docs.litellm.ai/docs/proxy/db_info — `extracted/p2-deploy-db_info.md`
- https://docs.litellm.ai/docs/proxy/ui — `extracted/p2-admin-ui.md`
- https://docs.litellm.ai/docs/proxy/manage_ui — `extracted/p2-admin-management_cli.md`
- https://docs.litellm.ai/docs/proxy/config_settings — `extracted/p0-config_settings.md`
- `schemas/config-yaml.option-index.json`, `schemas/openapi-management-endpoints.index.json`
- Real config: `infra/litellm/config.yaml`

## What this area controls

Who can call the proxy (auth), which models each caller may use (access),
how much they may spend and how fast (budgets / rate limits), and how the
admin manages keys/teams/users (management surface). In the workestrator
(in-memory, no Postgres, no Redis) the **only** auth is the `master_key`;
virtual keys, teams, users, budgets, and per-key/per-team model access are
all **unavailable**.

## Verified behavior

### `master_key` works in-memory (the ONLY auth)

Verbatim (from `p1-auth-virtual_keys.md`):

> "must start with `sk-`"

Env var: `LITELLM_MASTER_KEY`. Config path: `general_settings.master_key`.
The real config sets `master_key: os.environ/LITELLM_MASTER_KEY`. Any caller
presenting `Authorization: Bearer $LITELLM_MASTER_KEY` is authenticated as
admin and may access **all** configured `model_name` entries.

### Virtual keys REQUIRE Postgres

Verbatim (from `p1-auth-virtual_keys.md`):

> "Need a postgres database (e.g. Supabase, Neon, etc)"
> "Set DATABASE_URL=postgresql://..."

Config path: `general_settings.database_url`. **UNAVAILABLE in-memory.**

### Model access

- By virtual key (`models` param on `/key/generate`) — REQUIRES DB.
- By team (`models` on `/team/new`) — REQUIRES DB.
- In-memory: model access is controlled **entirely** by which `model_name`
  entries exist in `model_list`. Any caller with `master_key` can access
  all configured models. (inferred from the absence of per-key gating)

### `GET /v1/models` works without DB

Listing configured `model_name` entries works in-memory. The
`include_metadata` and `fallback_type` query params work with static
fallback config.

### Per-deployment rate limits (work without DB — inferred)

`model_list[].litellm_params.rpm` / `tpm` are per-deployment rate limits.
Verbatim (from `extracted/p0-deploy.md`):

> "rpm: 6 # Rate limit for this deployment: in requests per minute (rpm)"

These are treated as working without DB **(inferred — per-deployment,
in-process)** since they cap a single deployment's throughput, not
cross-key spend.

### `allowed_ips` works without DB

`general_settings.allowed_ips` — `requires_db=false` per option-index. IP
allowlist enforced in-process.

### `disable_spend_logs` / `disable_error_logs`

Verbatim (from `p0-config_settings.md`):

> `disable_spend_logs: true` — "turn off writing each transaction to the db"
> `disable_error_logs: true` — disables error log DB writes (independent of
> `disable_spend_logs`).

The real config sets `disable_spend_logs: true` to avoid DB write errors.
`disable_error_logs` is **not** set in the real config (inferred: would also
suppress DB error-log writes if DB were absent).

### DB stores (verbatim from `p2-deploy-db_info.md`)

> Virtual Keys, Organizations, Teams, Users, Budgets, Per-request Usage
> Tracking, Model Management (when STORE_MODEL_IN_DB=True), SpendLogs.

ALL unavailable without DB.

### Admin UI REQUIRES DB

Verbatim (from `p2-admin-ui.md`):

> "Requires db connected"

`DISABLE_ADMIN_UI="True"` should be set in-memory.

### `lite` CLI

- DB-backed (FAIL in-memory): `lite models add`, `lite credentials create`,
  `lite keys generate`, `lite users create`.
- Work in-memory: `lite chat completions`, `lite http request` (inference
  endpoints).

## Config keys / requirements

### `general_settings` auth/access/budget keys

| Key | `requires_db` | Available in-memory? | Source |
| --- | --- | --- | --- |
| `master_key` | false | **YES** (only auth) | option-index |
| `database_url` | true | NO (not set) | option-index |
| `completion_model` | false | YES (set to `coding`) | option-index |
| `disable_spend_logs` | false | YES (set `true`) | option-index |
| `disable_error_logs` | false | YES (not set in real config) | option-index |
| `allowed_ips` | false | YES | option-index |
| `token_rate_limit_type` (`input`\|`output`\|`total`) | true | NO | option-index |
| `fail_closed_budget_enforcement` | true (default `false`) | NO | option-index |
| `max_parallel_requests` / `global_max_parallel_requests` | (inferred: works without DB for per-deployment/proxy caps) | (inferred) | option-index |

### `model_list[].litellm_params` access/budget keys

| Key | `requires_db` | Available in-memory? | Source |
| --- | --- | --- | --- |
| `rpm` / `tpm` | (inferred: per-deployment, in-process) | (inferred) YES | `p0-deploy.md` |
| `model_info.access_groups` | false (static config) | YES | option-index |

### DB-backed management endpoints (all UNAVAILABLE in-memory)

`/key/generate`, `/key/info`, `/key/block`, `/key/unblock`, `/key/update`,
`/user/new`, `/user/info`, `/team/new`, `/team/info`, `/team/update`,
`/team/member_add`, `/budget/new` (Enterprise + DB).

### Per-key/team/user budget keys (ALL REQUIRE DB)

`max_budget`, `budget_duration`, `rpm_limit`, `tpm_limit`,
`max_parallel_requests`, `model_rpm_limit`, `model_tpm_limit`,
`budget_limits`, `model_max_budget` (Enterprise).

### Global proxy budget

`litellm_settings.max_budget` / `budget_duration` — spend tracked in DB;
enforcement limited without DB **(inferred)**.

### Credentials encryption

`LITELLM_SALT_KEY` encrypts LLM API key credentials; cannot be changed after
adding a model. **Not used** in real config (no DB, no credential store).

## Features that REQUIRE DB/Redis

| Feature | Requirement | Available in-memory? |
| --- | --- | --- |
| Virtual keys (`/key/*`) | Postgres (`database_url`) | **NO** |
| Teams (`/team/*`), users (`/user/*`) | Postgres | **NO** |
| Per-key / per-team / per-user budgets | Postgres | **NO** |
| Per-key / per-team model access (`models` param) | Postgres | **NO** |
| Budget & Rate Limit Tiers (`/budget/new`) | Enterprise + Postgres | **NO** |
| Spend tracking (`/spend/*`) | Postgres | **NO** |
| Admin UI | Postgres | **NO** (set `DISABLE_ADMIN_UI=True`) |
| Dynamic model management (`STORE_MODEL_IN_DB=True`) | Postgres | **NO** (leave False) |
| Multi-instance rate limiting | Redis (cross-pod counters) | **NO** |
| `token_rate_limit_type` | Postgres | **NO** |
| `fail_closed_budget_enforcement` | Postgres | **NO** |

Verbatim (from `p1-budgets-rate_limit_tiers.md` / budgets corpus):

> "Budget checks read current spend from a cross-pod counter in Redis"

→ Multi-instance rate limiting REQUIRES Redis. **UNAVAILABLE.**

## Pitfalls

- **`master_key` must start with `sk-`.** A key without the `sk-` prefix is
  rejected. Ensure `LITELLM_MASTER_KEY` begins with `sk-`.
- **`os.environ/VAR` resolves at load time.** If `LITELLM_MASTER_KEY` is
  unset in the proxy process environment, `master_key` resolves to an empty
  string — auth silently breaks. See [troubleshooting.md](../troubleshooting.md).
- **No per-key model isolation.** In-memory, every `master_key` caller can
  hit every `model_name`. Do not rely on LiteLLM for tenant isolation until
  Postgres is added.
- **`disable_spend_logs: true` is mandatory in-memory.** Without it, every
  request triggers a DB spend-log write that fails. The real config sets it.
- **`STORE_MODEL_IN_DB` must NOT be set.** Leaving it False keeps models
  defined statically in `config.yaml`. Setting it True without a DB breaks
  model loading.
- **Admin UI must be disabled.** Set `DISABLE_ADMIN_UI="True"` to avoid
  serving a UI that cannot function without DB.
- **`lite` management commands fail.** `lite keys generate`, `lite models add`
  hit DB-backed endpoints. Use `lite chat completions` / `lite http request`
  for inference testing only.
- **`LITELLM_SALT_KEY` is immutable after first model add.** Not relevant
  in-memory (no credential store), but plan it before adding Postgres.

## M1 / M4 phasing

[PROJECT CONTEXT — NOT upstream docs]

- **M1 (current):** `master_key`-only auth. No virtual keys, no budgets, no
  spend tracking. Model access = which `model_name` entries exist in
  `config.yaml`. `disable_spend_logs: true` compensates for missing DB.
- **M4 (planned, when Postgres is added):** `general_settings.database_url`,
  virtual keys (`/key/generate`), teams/users, per-key/team/user budgets,
  spend tracking (remove or keep `disable_spend_logs`), Admin UI (remove
  `DISABLE_ADMIN_UI`), `STORE_MODEL_IN_DB=True` for dynamic model management,
  `fail_closed_budget_enforcement`, `token_rate_limit_type`. Redis for
  multi-instance rate limiting + `enable_redis_auth_cache` +
  `cache_params.type: redis`.

## Related schema / workflow

- [`schemas/openapi-management-endpoints.index.json`](../schemas/openapi-management-endpoints.index.json) — management endpoint families with `requires_db` flags.
- [`schemas/config-yaml.option-index.json`](../schemas/config-yaml.option-index.json) — per-key `requires_db` / `requires_redis` flags.
- [`schemas/env-vars.index.json`](../schemas/env-vars.index.json) — `LITELLM_MASTER_KEY`, `LITELLM_SALT_KEY`, `DATABASE_URL`.
- `litellm-budgets-keys` (`.agents/skills/litellm-budgets-keys/`) — operational skill (virtual keys, budgets, rate limits; DB-gated; M4).
- [`endpoints/README.md`](../endpoints/README.md) — management endpoint families by DB requirement.

## Workestrator notes

[PROJECT CONTEXT — NOT upstream docs]

- Auth is **`master_key` only**. Agent sandboxes (Pi, Odysseus) call LiteLLM
  with `Authorization: Bearer $LITELLM_MASTER_KEY`.
- `general_settings.master_key: os.environ/LITELLM_MASTER_KEY`,
  `completion_model: coding`, `disable_spend_logs: true` are the only
  `general_settings` keys set in the real config.
- No `database_url`, no `token_rate_limit_type`, no
  `fail_closed_budget_enforcement`, no `allowed_ips` (egress is instead
  enforced by microsandbox default-deny — see
  [deployment-ops/README.md](../deployment-ops/README.md)).
- Model access = the 9 `model_name` aliases in `model_list` (`coding`,
  `coding-fallback`, `coding.fast`, `coding.fast-fallback`, `coding.pro`,
  `coding.pro-fallback`, `coding.free`, `coding.free-fallback`, `neural`).
  All are reachable by any `master_key` caller.
- Per-deployment `rpm`/`tpm` are **not set** in the real config — deployments
  are unthrottled at the LiteLLM layer (egress/network caps apply upstream).
- M1 = master_key only (current); virtual keys / budgets / spend defer to
  Postgres / M4.
