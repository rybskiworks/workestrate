# Workestrate Recommended Patterns

> **RECOMMENDATION DOCUMENT — NOT upstream LiteLLM docs.** This file
> captures how the workestrate project structures its LiteLLM deployment.
> Recommendations are marked **[RECOMMENDATION]**. Where a recommendation
> rests on an upstream fact, the fact is cited from the on-disk corpus
> under `docs/litellm/extracted/` or `docs/litellm/schemas/` and marked
> **[UPSTREAM]**. Items not documented in the fetched source are marked
> **"not documented in fetched source"**; items inferred from context are
> marked **(inferred)**. No keys, prefixes, endpoints, or defaults are
> invented.

## Sources

- Real config: `infra/litellm/config.yaml`, `infra/litellm/README.md`
- `extracted/p0-config_settings.md`, `extracted/p0-deploy.md`,
  `extracted/p0-docker_quick_start.md`, `extracted/p2-deploy-db_info.md`
- `extracted/p1-provider-anthropic.md`, `extracted/p1-provider-openai_compatible.md`,
  `extracted/p1-provider-openrouter.md`, `extracted/p1-provider-minimax.md`,
  `extracted/p1-provider-moonshot.md`, `extracted/p1-provider-zai.md`
- `extracted/p1-routing-fallback_management.md`, `extracted/p1-routing-load_balancing.md`
- `extracted/p1-auth-virtual_keys.md`, `extracted/p1-auth-model_access.md`,
  `extracted/p1-budgets-users.md`, `extracted/p1-budgets-rate_limit_tiers.md`
- `schemas/config-yaml.option-index.json`
- Project context: `docs/litellm/00-index.md`, `docs/litellm/01-mental-model.md`

## What this area controls

How the workestrate should structure `config.yaml`, choose provider
prefixes, enforce security at the proxy + sandbox boundary, and what to
enable when Postgres is added (M4). This is a project-convention doc, not an
upstream reference.

## Verified behavior (upstream facts the recommendations rest on)

- **[UPSTREAM]** Static `router_settings.fallbacks` works without DB/Redis
  (`schemas/config-yaml.option-index.json`: `requires_db=false`,
  `requires_redis=false`).
- **[UPSTREAM]** `master_key` works in-memory; verbatim "must start with
  `sk-`" (`extracted/p1-auth-virtual_keys.md`).
- **[UPSTREAM]** `anthropic/` + custom `api_base` is the documented way to
  reach endpoints that speak the Anthropic Messages API; LiteLLM
  auto-appends `/v1/messages` unless `LITELLM_ANTHROPIC_DISABLE_URL_SUFFIX=true`
  (`extracted/p1-provider-anthropic.md`).
- **[UPSTREAM]** OpenAI-compatible `api_base` must include `/v1` and no
  endpoint suffix (`extracted/p1-provider-openai_compatible.md`).
- **[UPSTREAM]** OpenRouter uses nested `openrouter/<org>/<model>` format
  (`extracted/p1-provider-openrouter.md`).
- **[UPSTREAM]** `minimax/` is the documented prefix for MiniMax
  (`extracted/p1-provider-minimax.md`); `moonshot/` for Moonshot
  OpenAI-compatible (`extracted/p1-provider-moonshot.md`); `zai/` for
  Z.AI direct (`extracted/p1-provider-zai.md`).
- **[UPSTREAM]** `disable_spend_logs: true` — verbatim "turn off writing
  each transaction to the db" (`extracted/p0-config_settings.md`).
- **[UPSTREAM]** Virtual keys / teams / users / budgets / spend tracking
  REQUIRE Postgres (`extracted/p1-auth-virtual_keys.md`,
  `extracted/p2-deploy-db_info.md`).
- **[UPSTREAM]** `general_settings.allowed_ips` — `requires_db=false`
  (`schemas/config-yaml.option-index.json`).
- **[UPSTREAM]** `litellm_settings.user_url_validation` /
  `user_url_allowed_hosts` — `requires_db=false`
  (`schemas/config-yaml.option-index.json`).

## Config keys / requirements

### [RECOMMENDATION] Config structure: tier aliases + `-fallback` siblings

The workestrate exposes **tier `model_name` aliases** (not provider model
IDs) and gives each tier a primary deployment plus a `-fallback` sibling.
`router_settings.fallbacks` maps each tier to its fallback.

This mirrors the real config (`infra/litellm/config.yaml`):

```yaml
model_list:
  - model_name: coding
    litellm_params: { model: anthropic/kimi-for-coding, ... }
  - model_name: coding-fallback
    litellm_params: { model: anthropic/MiniMax-M3, ... }
  # ... coding.fast, coding.pro, coding.free, each with -fallback sibling
  - model_name: neural
    litellm_params: { model: openai/neuralwatt, ... }

router_settings:
  fallbacks:
    - coding: [coding-fallback]
    - coding.fast: [coding.fast-fallback]
    - coding.pro: [coding.pro-fallback]
    - coding.free: [coding.free-fallback]
```

Rationale **[UPSTREAM]**: static `router_settings.fallbacks` is the only
fallback path that works without DB/Redis. Tier aliases decouple callers
from provider model IDs, so provider swaps don't change the client contract.

**[RECOMMENDATION]** Give every tier a fallback sibling except where a
single deployment is acceptable (the real config leaves `neural` without a
fallback — acceptable for a single OpenAI-compatible endpoint, but note
the availability gap in [troubleshooting.md](troubleshooting.md)).

### [RECOMMENDATION] In-memory constraints

When running without Postgres/Redis, set:

- `general_settings.disable_spend_logs: true` **[UPSTREAM]** — prevents DB
  spend-log write errors.
- `general_settings.disable_error_logs: true` **[UPSTREAM]** — prevents DB
  error-log write errors (independent of `disable_spend_logs`).
- Do **NOT** set `STORE_MODEL_IN_DB` (leave False) **[UPSTREAM]** — models
  are static in `config.yaml`.
- `DISABLE_ADMIN_UI="True"` **[UPSTREAM]** — Admin UI requires DB.
- No `database_url`, no `token_rate_limit_type`, no
  `fail_closed_budget_enforcement`.

### [RECOMMENDATION] Provider prefix choices

| Tier | Prefix used | Documented pattern | Status |
| --- | --- | --- | --- |
| `coding`, `coding.pro-fallback` | `anthropic/kimi-for-coding` | `anthropic/` + custom `api_base` for Anthropic-Messages endpoints | **[UPSTREAM]** valid |
| `coding-fallback`, `coding.fast` | `anthropic/MiniMax-M3` | documented pattern is `minimax/` prefix | **DEVIATION (flagged)** — works because MiniMax speaks Anthropic Messages, but diverges from `minimax/` |
| `coding.pro`, `coding.fast-fallback`, `coding.free`, `coding.free-fallback` | `openrouter/z-ai/...`, `openrouter/qwen/...`, `openrouter/nex-agi/...` | nested `openrouter/<org>/<model>` | **[UPSTREAM]** valid |
| `neural` | `openai/neuralwatt` | `openai/` + `api_base` with `/v1` | **[UPSTREAM]** valid |

**[RECOMMENDATION]** Keep `anthropic/MiniMax-M3` only if the MiniMax
Anthropic endpoint remains the integration target. If MiniMax's native API
is preferred, migrate to the `minimax/` prefix (documented pattern). Either
way, document the choice in [providers/README.md](providers/README.md) —
the deviation is currently flagged there.

**[RECOMMENDATION]** Do **not** use `moonshot/` for the Kimi coding
endpoint — that prefix targets Moonshot's OpenAI-compatible API, not the
Kimi Anthropic-Messages coding endpoint. The repo's `anthropic/kimi-for-coding`
is the correct integration for the latter.

**[RECOMMENDATION]** For Z.AI/GLM, prefer `openrouter/z-ai/glm-5.1`
(OpenRouter routing) over `zai/` direct unless you have a direct Z.AI
account and want to bypass OpenRouter billing/rate limits.

### [RECOMMENDATION] Security

- **`master_key` (sk- prefix)** — the only auth in-memory **[UPSTREAM]**.
  Inject via sandbox `secret_env()`, never commit to config.
- **Microsandbox default-deny egress** — DNS + tcp/443 only to
  `openrouter.ai`, `api.kimi.com`, `api.minimax.io`, `api.neuralwatt.com`.
  This is the primary network isolation layer; LiteLLM-level `allowed_ips`
  is **not** set in the real config.
- **`general_settings.allowed_ips`** **[UPSTREAM]** (`requires_db=false`) —
  optional additional IP allowlist at the proxy layer. **[RECOMMENDATION]**
  enable only if the proxy is reachable beyond the microsandbox network.
- **Allowed host binding** — `litellm_settings.user_url_validation` /
  `user_url_allowed_hosts` **[UPSTREAM]** (`requires_db=false`).
  **[RECOMMENDATION]** set `user_url_allowed_hosts` to the proxy's expected
  Host header(s) to prevent host-header spoofing.
- **`require_auth_for_metrics_endpoint: true`** **[UPSTREAM]** — gate
  `GET /metrics` behind `master_key` if reachable outside the sandbox.

## Features that REQUIRE DB/Redis

See [auth-access-budget/README.md](auth-access-budget/README.md) and
[routing/README.md](routing/README.md) for the authoritative lists. In
summary, in-memory the following are **unavailable**:

- Virtual keys, teams, users, budgets, spend tracking.
- Per-key / per-team / per-user model access and rate limits.
- Dynamic fallback management endpoints.
- Budget/spend alerts, `/daily_metrics`, budget/rate-limit Prometheus
  metrics.
- Admin UI, `lite` management CLI commands (keys/models/credentials/users).
- Multi-instance rate limiting (Redis).

## Pitfalls

- **[RECOMMENDATION]** Do not enable DB-gated features piecemeal without
  Postgres — partial enablement (e.g. `token_rate_limit_type` without
  `database_url`) yields confusing errors. Add Postgres as a single M4
  cut-over.
- **[RECOMMENDATION]** When adding Postgres, set `LITELLM_SALT_KEY` **before**
  adding any models/credentials — it cannot be changed afterward
  **[UPSTREAM]**.
- **[RECOMMENDATION]** Keep tier aliases stable across M1→M4 — callers
  (Pi, Odysseus sandboxes) depend on `coding`/`coding.fast`/`coding.pro`/
  `coding.free`/`neural` names. Provider swaps behind an alias are safe;
  alias renames break clients.
- **[RECOMMENDATION]** Document every provider-prefix deviation in
  [providers/README.md](providers/README.md) — the `anthropic/MiniMax-M3`
  deviation is already flagged there.

## What to enable when Postgres is added (M4)

**[RECOMMENDATION]** M4 cut-over additions (all DB-gated features below are
**[UPSTREAM]** DB-required unless noted):

| Add | Key / endpoint | Source |
| --- | --- | --- |
| Postgres connection | `general_settings.database_url` / `DATABASE_URL` | `p1-auth-virtual_keys.md` |
| Virtual keys | `/key/generate` (with `models` param for per-key access) | `p1-auth-virtual_keys.md`, `p1-auth-model_access.md` |
| Teams / users | `/team/new`, `/user/new` (with `models` on `/team/new`) | `p1-auth-model_access.md`, `p1-budgets-users.md` |
| Per-key/team/user budgets | `max_budget`, `budget_duration`, `rpm_limit`, `tpm_limit`, `max_parallel_requests`, `model_rpm_limit`, `model_tpm_limit` | `p1-budgets-users.md` |
| Spend tracking | remove `disable_spend_logs` (or keep) — `/spend/*` endpoints | `p2-deploy-db_info.md` |
| Admin UI | remove `DISABLE_ADMIN_UI` | `p2-admin-ui.md` |
| Dynamic model management | `STORE_MODEL_IN_DB=True` | `p2-deploy-db_info.md` |
| Budget enforcement | `fail_closed_budget_enforcement` (default false) | `p0-config_settings.md` |
| Token rate limit type | `token_rate_limit_type` (`input`\|`output`\|`total`) | `p0-config_settings.md` |
| Budget & Rate Limit Tiers | `/budget/new` (Enterprise + DB) | `p1-budgets-rate_limit_tiers.md` |
| Skills gateway persistence | DB-backed skill/agent/prompt CRUD (currently static config only) | `p2-deploy-db_info.md` (DB stores list) |

**[RECOMMENDATION]** When multi-instance is needed, add Redis for:

- Cross-pod rate-limit counters **[UPSTREAM]** — verbatim "Budget checks
  read current spend from a cross-pod counter in Redis".
- `enable_redis_auth_cache` (virtual-key auth caching across workers).
- `cache_params.type: redis` or `redis-semantic` (if Redis cache is desired
  over `local`/`disk`).
- `PROMETHEUS_MULTIPROC_DIR` for aggregated multi-worker metrics
  **[UPSTREAM]**.

**[RECOMMENDATION]** Keep `disable_spend_logs: true` only if you do not
need spend tracking; otherwise remove it when Postgres is live so
`/spend/*` and budget alerts populate.

## Related schema / workflow

- [`schemas/config-yaml.option-index.json`](schemas/config-yaml.option-index.json) — per-key `requires_db` / `requires_redis` / `workestrate_recommendation` flags.
- [`schemas/config-yaml.normalized.schema.md`](schemas/config-yaml.normalized.schema.md) — "Workestrate in-memory deployment mapping".
- [`routing/README.md`](routing/README.md) — static fallbacks (M1) → dynamic fallbacks (M4).
- [`auth-access-budget/README.md`](auth-access-budget/README.md) — M1/M4 phasing.
- [`deployment-ops/README.md`](deployment-ops/README.md) — image choice, health probes, egress.
- [`observability-cache-guardrails/README.md`](observability-cache-guardrails/README.md) — DB-free observability additions.
- [`providers/README.md`](providers/README.md) — provider prefix matrix + deviations.
- [`troubleshooting.md`](troubleshooting.md) — common failure modes.

## Workestrate notes

[PROJECT CONTEXT — NOT upstream docs]

- Current phase: **M1** — `master_key`-only, in-memory, no Postgres, no
  Redis, no UI. Config at `infra/litellm/config.yaml` mounted read-only.
- Tier lineup (verbatim from config header comment):
  - `coding` → Kimi K2.7 (specialist) → MiniMax M3
  - `coding.fast` → MiniMax M3 (cheap) → Qwen 3.7 Plus
  - `coding.pro` → GLM-5.1 (strong reasoning) → Kimi K2.7
  - `coding.free` → Qwen 3 Coder (free) → Nex N2 Pro (free)
  - `neural` → Neuralwatt (no fallback)
- Provider prefix deviations are documented in
  [providers/README.md](providers/README.md); the only flagged deviation is
  `anthropic/MiniMax-M3` (documented pattern: `minimax/`).
- Security layers, in order: microsandbox default-deny egress →
  `master_key` auth → (optional) `allowed_ips` → (optional)
  `user_url_allowed_hosts` → (optional) `require_auth_for_metrics_endpoint`.
- M4 cut-over is a single coordinated change (Postgres + virtual keys +
  budgets + spend + UI + `STORE_MODEL_IN_DB`), not a piecemeal rollout.
  Redis is a separate later step only when multi-instance is required.
