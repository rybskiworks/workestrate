---
name: litellm-deployment
description: |
  Operational reference for LiteLLM Proxy deployment & operations: Docker
  images (main vs -database), CLI flags (--config, --debug, --detailed_debug,
  --port, --num_workers), config mounted read-only at /app/config.yaml,
  health endpoints (/health, /health/liveliness, /health/readiness), drain
  endpoint, and the workestrate microsandbox context. Load when starting the
  proxy, choosing an image, wiring health probes, diagnosing startup/env
  resolution, or sizing the deployment. Does NOT cover config anatomy (see
  litellm-config-anatomy), providers (see litellm-providers), or routing
  (see litellm-routing-fallbacks). Full detail lives in
  docs/litellm/deployment-ops/README.md + examples/microvm-safe-proxy.yaml.
---

# LiteLLM Deployment & Operations

Distilled operational reference for how the LiteLLM Proxy is packaged,
started, probed, and sized. Full detail lives in:

- `docs/litellm/deployment-ops/README.md` — deployment synthesis.
- `docs/litellm/examples/microvm-safe-proxy.yaml` — the in-memory shape.
- `docs/litellm/schemas/endpoints.index.json` — endpoint surface.
- `docs/litellm/schemas/env-vars.index.json` — `NUM_WORKERS`, `LITELLM_LOG`,
  `KEEPALIVE_TIMEOUT`, etc.

> **Do not hallucinate.** Every flag/endpoint/default below traces to
> `docs/litellm/deployment-ops/README.md` or `endpoints.index.json`.
> Inferred items are marked **(inferred)**. Project context is marked
> **[PROJECT]**.

## Triggers

Load this skill when:

- Starting the proxy (CLI flags, config mount, port).
- Choosing between the main and `-database` Docker images.
- Wiring k8s/sandbox health probes (liveness vs readiness vs deep).
- Diagnosing startup failures, empty env-var resolution, or port conflicts.
- Sizing the deployment (CPU/RAM, when Redis is needed).
- Enabling/configuring the drain endpoint.

## Docker images

| Image | Use | DB? |
|-------|-----|-----|
| `docker.litellm.ai/berriai/litellm:latest` | Main (in-memory) | no Postgres |
| `ghcr.io/berriai/litellm-database:latest` | With DB | Postgres required |

The workestrate is in-memory (no Postgres) → the **main image suffices**.
Using the `-database` image without a Postgres connection logs DB errors on
every request.

## CLI flags (verbatim)

`--config /path/to/config.yaml`, `--debug`, `--detailed_debug`, `--port 4000`,
`--num_workers 8`, `--ssl_keyfile_path`, `--ssl_certfile_path`,
`--run_hypercorn`, `--run_granian` (BETA), `--keepalive_timeout 75`,
`--max_requests_before_restart 10000`, `--iam_token_db_auth`.

### docker-compose mount (verbatim)

```yaml
volumes:
  - ./litellm-config.yaml:/app/config.yaml
command: ["--config", "/app/config.yaml", "--port", "4000", "--num_workers", "8"]
```

### Debug flags

- `--debug` / `--detailed_debug` / `export LITELLM_LOG="DEBUG"` /
  `export JSON_LOGS="True"`.
- Verbatim: "WARNING: FOR PROD DO NOT USE `--detailed_debug` it slows down
  response times". Use `--debug` or `LITELLM_LOG=INFO` in prod.

## Env loading

`os.environ/VAR_NAME` runs `os.getenv` at load time. The env var must be set
in the proxy process environment **before startup**; if unset, the value
resolves to an empty string (causing downstream auth failures, not config
validation failures).

`environment_variables: {}` is a top-level config key for injecting env vars
via config (workestrate does NOT use this — secrets come from the sandbox
`env()`/`secret_env()`).

## `NUM_WORKERS`

Number of workers. Multi-worker deployments require `PROMETHEUS_MULTIPROC_DIR`
for aggregated Prometheus metric collection (single-process does not). In
workestrate, `NUM_WORKERS` is passed via `--num_workers` CLI flag, not env.

## Health endpoints

| Endpoint | Purpose | DB needed? |
|----------|---------|------------|
| `GET /health` | Model connectivity — makes **actual API calls** to upstreams. | no |
| `GET /health/liveliness` | Liveness — returns 200 if process alive. | no |
| `GET /health/readiness` | Readiness — includes DB connection status. | no (DB status N/A) |

> **Misspelling note:** the OpenAPI contains both `/health/liveness` and
> `/health/liveliness`. The workestrate infra uses the misspelled
> `/health/liveliness`.

- `/health` is NOT a cheap liveness probe — it calls every configured upstream.
  Use `/health/liveliness` for k8s liveness, `/health/readiness` for
  readiness, `/health` for deep connectivity checks only.

## Drain endpoint

`/health/drain` (referenced in `config_settings` routes).

- `general_settings.enable_drain_endpoint` — verbatim: "Off by default; only
  enable it when the health port is reachable solely from inside the cluster."
- `general_settings.drain_endpoint_token` / env `DRAIN_ENDPOINT_TOKEN`.
- `GRACEFUL_SHUTDOWN_TIMEOUT` env var referenced in drain context.

## Production sizing

Verbatim: "4 CPU cores and 8 GB RAM".

Redis required at **1000+ RPS or multi-instance**. Not required for
single-instance (workestrate is single-instance).

## Air-gapped / cost map

`LITELLM_LOCAL_MODEL_COST_MAP="True"` disables pulling live model prices
(air-gapped/offline).

## Server env vars

`SERVER_ROOT_PATH`, `KEEPALIVE_TIMEOUT` (default 5),
`MAX_REQUESTS_BEFORE_RESTART`.

## Hot reload

**Not clearly documented in fetched source** — likely needs a process restart
to pick up `config.yaml` changes **(inferred)**. The workestrate mounts config
read-only, so changes require a sandbox restart.

## Common Mistakes

| Mistake | Cause | Fix |
|---------|-------|-----|
| `-database` image without Postgres | Wrong image for in-memory | Use main image `docker.litellm.ai/berriai/litellm:latest`. |
| Config mount auto-created as empty dir | `./litellm-config.yaml` didn't exist before `docker compose up` | Create the file first; mount read-only. |
| `os.environ/VAR` empty resolution | VAR unset at startup | Set in proxy env before startup; failure is auth, not config. |
| Port 4000 conflict | Default proxy port | Use `--port` to change. |
| `--detailed_debug` in prod | Slows response times | Use `--debug` or `LITELLM_LOG=INFO`. |
| `/health` used as liveness probe | Makes real API calls to all upstreams | Use `/health/liveliness` for liveness. |
| `/health/liveliness` misspelling assumed typo | OpenAPI has both spellings | Use the misspelled form the infra expects. |
| Drain endpoint exposed externally | Security caveat | Only enable when health port is in-cluster only. |
| Multi-worker metrics not aggregated | Missing `PROMETHEUS_MULTIPROC_DIR` | Set env var for multi-worker. |
| Config changes assumed hot-reloaded | Not documented | Restart the proxy/microVM to apply. |

## Project context [PROJECT — not upstream docs]

The workestrate runs LiteLLM **in-memory inside a microsandbox microVM** at
`:4000`:

- Image: **main** (`docker.litellm.ai/berriai/litellm:latest`), no DB.
- `infra/litellm/config.yaml` mounted **read-only** at `/app/config.yaml`
  (`MountPlan::readonly`), started with
  `--config /app/config.yaml --host 0.0.0.0`.
- Egress is **default-deny** at the microsandbox layer: DNS + tcp/443 only to
  `openrouter.ai`, `api.kimi.com`, `api.minimax.io`, `api.neuralwatt.com`.
  LiteLLM-level `allowed_ips` is NOT set (network policy enforces it).
- Single-instance → no Redis, no `PROMETHEUS_MULTIPROC_DIR` needed for basic
  metrics, no cross-pod counters.
- Health probes used by the sandbox orchestrator:
  `GET /health/liveliness` (liveness), `GET /health/readiness` (readiness),
  `GET /health` (deep connectivity — used sparingly).
- Drain endpoint is **not enabled** (single-instance, in-cluster only).
- Env vars resolved via `os.environ/`: `LITELLM_MASTER_KEY`,
  `KIMI_CODE_API_KEY`, `MINIMAX_CODING_API_KEY`, `OPENROUTER_API_KEY`,
  `NEURALWATT_API_KEY`. These are **project env vars** (not LiteLLM
  built-ins), injected by the sandbox `env()`/`secret_env()`.
- Config changes require a microVM restart (hot reload not documented).

## Related Docs

- `docs/litellm/deployment-ops/README.md`
- `docs/litellm/examples/microvm-safe-proxy.yaml`
- `docs/litellm/config/config-validation.md`
- `docs/litellm/schemas/endpoints.index.json`
- `docs/litellm/schemas/env-vars.index.json`
- `docs/litellm/troubleshooting.md`
