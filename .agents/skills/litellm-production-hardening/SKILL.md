---
name: litellm-production-hardening
description: |
  RECOMMENDATION-flavored operational reference for hardening a LiteLLM proxy
  deployment: timeouts, retry/fallback, spend/error log suppression in-memory,
  master_key handling, allowed_ips, SSRF/user_url_validation, force_ipv4,
  health checks, store_model_in_db, Docker image choice, and microsandbox
  default-deny egress. Load when hardening or reviewing a production LiteLLM
  config. Distilled from docs/litellm/; recommendations are clearly marked.
---

# LiteLLM Production Hardening

> **RECOMMENDATION-FLAVORED.** This skill distills hardening guidance. Items
> marked **[RECOMMENDATION]** are project convention; items marked
> **[UPSTREAM]** trace to a cited corpus fact. No keys/defaults are invented.

Distilled operational guidance for hardening a LiteLLM proxy deployment
(especially the in-memory, microsandbox-hosted workestrate shape). Full
detail lives in:

- `docs/litellm/deployment-ops/README.md` — image choice, health probes, env.
- `docs/litellm/workestrate-recommended-patterns.md` — security layers, M1/M4.
- `docs/litellm/examples/microvm-safe-proxy.yaml` — the in-memory shape used.
- `docs/litellm/auth-access-budget/README.md` — `master_key`, `disable_spend_logs`.

## Triggers

Load this skill when:

- Hardening or reviewing a production `config.yaml`.
- Setting timeouts, retry, fallback, cooldown.
- Securing `master_key` (env indirection, `disable_master_key_return`).
- Configuring `allowed_ips`, `user_url_validation`, `force_ipv4`.
- Choosing the Docker image (main vs `-database`).
- Wiring health probes (`/health`, `/health/liveliness`, `/health/readiness`).
- Enforcing microsandbox default-deny egress.

## Timeouts

| Key | Default | Recommendation |
|-----|---------|----------------|
| `litellm_settings.request_timeout` | `6000` (verbatim) | **[RECOMMENDATION]** ~300 — the 6000s default is dangerously long. |
| `router_settings.timeout` | 10 minutes (verbatim) | **[RECOMMENDATION]** 300. |
| `router_settings.stream_timeout` | (verbatim key) | **[RECOMMENDATION]** 300. |
| `--keepalive_timeout` / `KEEPALIVE_TIMEOUT` | `5` (verbatim) | Keep default unless tuning. |

## Retry / fallback

| Key | Default | Recommendation |
|-----|---------|----------------|
| `router_settings.num_retries` | `3` (verbatim) | **[RECOMMENDATION]** 2. |
| `router_settings.allowed_fails` | (verbatim key) | **[RECOMMENDATION]** 3. |
| `router_settings.cooldown_time` | `30` (verbatim) | **[RECOMMENDATION]** 60. |
| `router_settings.retry_policy` | (verbatim, e.g. `{TimeoutErrorRetries: 3}`) | **[RECOMMENDATION]** set per-error-type retries. |
| `router_settings.fallbacks` | (verbatim, `requires_db=false`) | **[RECOMMENDATION]** give every tier a `-fallback` sibling. |

Static `router_settings.fallbacks` is the only fallback path that works
without DB/Redis **[UPSTREAM]**.

## In-memory log suppression

| Key | Effect | Source |
|-----|--------|--------|
| `general_settings.disable_spend_logs: true` | "turn off writing each transaction to the db" | **[UPSTREAM]** `p0-config_settings.md` |
| `general_settings.disable_error_logs: true` | disables error-log DB writes (independent) | **[UPSTREAM]** `p0-config_settings.md` |

**[RECOMMENDATION]** Set both `true` in-memory (no DB) to suppress DB write
errors. The real config sets `disable_spend_logs: true`; `disable_error_logs`
is not set (inferred: would also suppress if DB absent).

## master_key handling

| Concern | Guidance |
|---------|----------|
| Prefix | **[UPSTREAM]** must start with `sk-`. |
| Env indirection | **[RECOMMENDATION]** `master_key: os.environ/LITELLM_MASTER_KEY`; never commit. |
| `disable_master_key_return` | **[RECOMMENDATION]** enable to avoid returning the master key in responses. |
| Empty resolution | **[UPSTREAM]** `os.environ/VAR` unset → empty string → auth breaks. Set env before startup. |

## Network / SSRF

| Key | Effect | Source |
|-----|--------|--------|
| `general_settings.allowed_ips` | IP allowlist, `requires_db=false` | **[UPSTREAM]** option-index |
| `litellm_settings.user_url_validation` | SSRF protection, default `true` | **[UPSTREAM]** option-index |
| `litellm_settings.user_url_allowed_hosts` | allowed Host headers, `requires_db=false` | **[UPSTREAM]** option-index |
| `litellm_settings.force_ipv4` | workaround httpx ConnectionError ipv6+Anthropic | **[UPSTREAM]** option-index common_misconfiguration note |

**[RECOMMENDATION]** Enable `allowed_ips` only if proxy reachable beyond the
microsandbox network. Set `user_url_allowed_hosts` to expected Host header(s).
Keep `force_ipv4: true` for Anthropic upstreams.

## Health checks

| Endpoint | Purpose | DB? |
|----------|---------|-----|
| `GET /health` | deep connectivity — makes **real API calls** to upstreams | no |
| `GET /health/liveliness` | liveness (process alive → 200) | no |
| `GET /health/readiness` | readiness (includes DB status) | no (DB status N/A) |
| `GET /health/drain` | drain (enable via `enable_drain_endpoint`, default off) | no |

**[RECOMMENDATION]** Use `/health/liveliness` for k8s liveness,
`/health/readiness` for readiness, `/health` sparingly (it calls upstreams).
Note: OpenAPI has both `/health/liveness` and `/health/liveliness` (misspelled);
workestrate uses the misspelled form.

## store_model_in_db

**[UPSTREAM]** `STORE_MODEL_IN_DB=True` requires Postgres. **[RECOMMENDATION]**
leave `false` in-memory (M1); set `true` only at M4 cut-over with Postgres.

## Docker image

| Image | Use |
|-------|-----|
| `docker.litellm.ai/berriai/litellm:latest` | **main (no DB)** — use for in-memory **[RECOMMENDATION]** |
| `ghcr.io/berriai/litellm-database:latest` | with DB — do NOT use without Postgres |

**[RECOMMENDATION]** Use the main image for the in-memory workestrate. The
`-database` image logs DB errors on every request without a Postgres
connection.

## Microsandbox default-deny egress

**[RECOMMENDATION]** Enforce egress at the microsandbox runtime layer (not
LiteLLM config): DNS + tcp/443 only to the upstream hosts (e.g.
`openrouter.ai`, `api.kimi.com`, `api.minimax.io`, `api.neuralwatt.com`).
This is the primary network isolation layer; LiteLLM-level `allowed_ips` is
secondary. The real config does NOT set `allowed_ips` (network policy
enforces it).

## Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| Requests hang 6000s then fail | `request_timeout` default | Set `request_timeout: 300` + `router_settings.timeout: 300`. |
| DB spend-log write errors every request | no DB, `disable_spend_logs` unset | Set `disable_spend_logs: true` (+ `disable_error_logs: true`). |
| `master_key` auth breaks | `os.environ/VAR` unset | Set `LITELLM_MASTER_KEY` (with `sk-` prefix) before startup. |
| Wrong image logs DB errors | `-database` image without Postgres | Use main image `docker.litellm.ai/berriai/litellm:latest`. |
| `/health` exhausts upstream quota | deep probe calls every upstream | Use `/health/liveliness` for liveness; `/health` sparingly. |
| httpx ConnectionError (ipv6 + Anthropic) | ipv6 routing | `force_ipv4: true`. |
| `STORE_MODEL_IN_DB=True` breaks loading | no DB | Leave `false` (M1). |
| Drain endpoint abused | reachable outside cluster | Only enable `enable_drain_endpoint` in-cluster (verbatim caveat). |
| `--detailed_debug` slows prod | verbatim warning | Use `--debug` / `LITELLM_LOG=INFO` instead. |
| Config change not applied | hot reload not documented | Restart microVM (config mounted read-only). |

## Related Docs

- `docs/litellm/deployment-ops/README.md`
- `docs/litellm/workestrate-recommended-patterns.md`
- `docs/litellm/examples/microvm-safe-proxy.yaml`
- `docs/litellm/auth-access-budget/README.md`
- `docs/litellm/observability-cache-guardrails/README.md`
- `docs/litellm/schemas/config-yaml.option-index.json`

> Do not hallucinate defaults, keys, or image tags. Every default/key above
> traces to `deployment-ops/README.md`, `workestrate-recommended-patterns.md`,
> `microvm-safe-proxy.yaml`, or `config-yaml.option-index.json`.
> Recommendations are marked **[RECOMMENDATION]**; upstream facts are marked
> **[UPSTREAM]**. The 6000s `request_timeout` default is verbatim; the ~300
> recommendation is project convention.
