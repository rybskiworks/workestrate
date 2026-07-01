# Deployment & Operations

> Synthesized from on-disk corpus under `docs/litellm/extracted/` and
> `docs/litellm/schemas/`. No web fetches were performed. Every key, default,
> and requirement traces to a cited corpus file or is marked **(inferred)** /
> **"not documented in fetched source"**.

## Sources

- https://docs.litellm.ai/docs/proxy/deploy — `extracted/p0-deploy.md`
- https://docs.litellm.ai/docs/proxy/docker_quick_start — `extracted/p0-docker_quick_start.md`
- https://docs.litellm.ai/docs/proxy/db_info — `extracted/p2-deploy-db_info.md`
- https://docs.litellm.ai/docs/proxy/config_settings — `extracted/p0-config_settings.md`
- `schemas/endpoints.index.json`, `schemas/env-vars.index.json`
- Real config: `infra/litellm/config.yaml`, `infra/litellm/README.md`

## What this area controls

How the proxy is packaged (Docker image), started (CLI flags, workers,
config mount), how environment variables are resolved, how it is probed
(health/readiness/liveness/drain), and the production sizing envelope. The
workestrator runs the **main in-memory image** inside a microsandbox
microVM at `:4000` with read-only config and default-deny egress.

## Verified behavior

### Docker images

- **Main (no DB):** `docker.litellm.ai/berriai/litellm:latest`
- **With DB:** `ghcr.io/berriai/litellm-database:latest`

The real config is in-memory (no Postgres) → the **main image suffices**.

### CLI flags (verbatim from `p0-deploy.md`)

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

### Env loading

`os.environ/VAR_NAME` syntax runs `os.getenv` at load time (verbatim from
`p0-docker_quick_start.md`). The env var must be set in the proxy process
environment **before startup**; if unset, the value resolves to an empty
string.

`environment_variables: {}` is a top-level config key for injecting env
vars via config (not used in the real config — secrets come from the
sandbox `env()`/`secret_env()`).

### `NUM_WORKERS` env var

Number of workers. Multi-worker deployments require
`PROMETHEUS_MULTIPROC_DIR` for aggregated Prometheus metric collection
(single-process does not).

### Hot reload

**Not clearly documented in fetched source** — likely needs a process
restart to pick up `config.yaml` changes **(inferred)**. The workestrator
mounts config read-only, so changes require a sandbox restart.

### Health endpoints (from `endpoints.index.json` + `p0-deploy.md`)

| Endpoint | Purpose | DB needed? |
| --- | --- | --- |
| `GET /health` | Model connectivity health check — makes **actual API calls** to upstreams. | no |
| `GET /health/liveliness` | Liveness probe — returns 200 if process alive. | no |
| `GET /health/readiness` | Readiness probe — includes DB connection status. | no (DB status N/A) |

> **Misspelling note:** the OpenAPI contains both `/health/liveness` and
> `/health/liveliness`. The workestrator infra uses the misspelled
> `/health/liveliness`.

### Drain endpoint

`/health/drain` (referenced in `config_settings` routes).

- `general_settings.enable_drain_endpoint` — verbatim: "Off by default; only
  enable it when the health port is reachable solely from inside the cluster."
- `general_settings.drain_endpoint_token` / env `DRAIN_ENDPOINT_TOKEN`.
- `GRACEFUL_SHUTDOWN_TIMEOUT` env var referenced in drain context.

### Production sizing

Verbatim (from `p0-deploy.md`):

> "4 CPU cores and 8 GB RAM"

Redis required at **1000+ RPS or multi-instance**. Not required for
single-instance (workestrator is single-instance).

### Air-gapped / cost map

`LITELLM_LOCAL_MODEL_COST_MAP="True"` disables pulling live model prices
(air-gapped).

### Server env vars

`SERVER_ROOT_PATH`, `KEEPALIVE_TIMEOUT` (default `5`),
`MAX_REQUESTS_BEFORE_RESTART`.

## Config keys / requirements

| Key / env | Purpose | Source |
| --- | --- | --- |
| `--config` | Path to `config.yaml` (mounted at `/app/config.yaml`) | `p0-deploy.md` |
| `--port 4000` | Proxy listen port (default 4000) | `p0-deploy.md` |
| `--num_workers 8` / `NUM_WORKERS` | Worker count | `p0-deploy.md` |
| `--debug` / `--detailed_debug` / `LITELLM_LOG` | Logging verbosity | `p0-deploy.md` |
| `--ssl_keyfile_path` / `--ssl_certfile_path` | TLS cert/key files | `p0-deploy.md` |
| `--keepalive_timeout 75` | Keepalive (env `KEEPALIVE_TIMEOUT`, default 5) | `p0-deploy.md` |
| `--max_requests_before_restart 10000` | Worker recycle threshold (env `MAX_REQUESTS_BEFORE_RESTART`) | `p0-deploy.md` |
| `--run_hypercorn` / `--run_granian` (BETA) | Alternative ASGI servers | `p0-deploy.md` |
| `--iam_token_db_auth` | IAM token DB auth | `p0-deploy.md` |
| `os.environ/VAR` | `os.getenv("VAR")` at load time | `p0-docker_quick_start.md` |
| `environment_variables: {}` | Top-level config key for env injection | `p0-config_settings.md` |
| `LITELLM_LOCAL_MODEL_COST_MAP="True"` | Disable live model price pull (air-gapped) | `p0-deploy.md` |
| `SERVER_ROOT_PATH` | Server root path | `p0-deploy.md` |
| `GRACEFUL_SHUTDOWN_TIMEOUT` | Drain graceful shutdown timeout | `p0-config_settings.md` |
| `DRAIN_ENDPOINT_TOKEN` / `general_settings.drain_endpoint_token` | Drain auth token | `p0-config_settings.md` |
| `general_settings.enable_drain_endpoint` | Enable `/health/drain` (default off) | `p0-config_settings.md` |

## Features that REQUIRE DB/Redis

| Feature | Requirement | Available in-memory? |
| --- | --- | --- |
| `-database` image features (virtual keys, spend, UI) | Postgres | **NO** (main image used) |
| `/health/readiness` DB status field | Postgres | N/A (status reports no DB) |
| Multi-instance coordination | Redis | **NO** (single-instance) |
| 1000+ RPS scaling | Redis | **NO** (single-instance) |

Health endpoints, drain (if enabled), and basic Prometheus metrics all work
without DB/Redis.

## Pitfalls

- **Wrong image.** Using `ghcr.io/berriai/litellm-database:latest` without a
  Postgres connection logs DB errors on every request. Use the main image
  for in-memory.
- **Config mount as directory.** If `./litellm-config.yaml` does not exist
  before `docker compose up`, Docker auto-creates it as an **empty
  directory** and the proxy fails to load config. (Same pitfall documented
  for `prometheus.yml`.)
- **`os.environ/VAR` empty resolution.** If `KIMI_CODE_API_KEY`,
  `MINIMAX_CODING_API_KEY`, `OPENROUTER_API_KEY`, `NEURALWATT_API_KEY`, or
  `LITELLM_MASTER_KEY` are unset, they resolve to empty strings — provider
  calls fail with auth errors. See [troubleshooting.md](../troubleshooting.md).
- **Port 4000 conflicts.** Default proxy port is 4000. Use `--port` to
  change if the sandbox already binds 4000.
- **`--detailed_debug` in prod.** Verbatim: "WARNING: FOR PROD DO NOT USE
  `--detailed_debug` it slows down response times". Use `--debug` or
  `LITELLM_LOG=INFO` instead.
- **Hot reload not documented.** Do not assume config changes apply without
  a restart. The workestrator mounts config read-only and restarts the
  microVM to apply changes **(inferred)**.
- **`/health` makes real API calls.** It is not a cheap liveness probe — it
  calls every configured upstream. Use `/health/liveliness` for k8s
  liveness, `/health/readiness` for readiness, `/health` for deep
  connectivity checks only.
- **Drain endpoint security.** Only enable `enable_drain_endpoint` when the
  health port is reachable solely from inside the cluster (verbatim caveat).
- **Multi-worker Prometheus.** Without `PROMETHEUS_MULTIPROC_DIR`, metrics
  are not aggregated across workers (verbatim caveat — livesum aggregation
  requires it).

## Related schema / workflow

- [`endpoints/README.md`](../endpoints/README.md) — full endpoint surface, health family.
- [`schemas/endpoints.index.json`](../schemas/endpoints.index.json) — inference endpoints with auth/streaming flags.
- [`schemas/env-vars.index.json`](../schemas/env-vars.index.json) — `NUM_WORKERS`, `LITELLM_LOG`, `KEEPALIVE_TIMEOUT`, etc.
- [`schemas/config-yaml.option-index.json`](../schemas/config-yaml.option-index.json) — `general_settings.enable_drain_endpoint`, `drain_endpoint_token`.
- [`config/README.md`](../config/README.md) — top-level config structure.

## Workestrator notes

[PROJECT CONTEXT — NOT upstream docs]

- LiteLLM runs **in-memory inside a microsandbox microVM** at `:4000`.
- Image: **main** (`docker.litellm.ai/berriai/litellm:latest`), no DB.
- `infra/litellm/` is mounted **read-only** at `/app/config`
  (`MountPlan::readonly`), so `config.yaml` and its `include:`d `models.yaml`
  both resolve. Started with `--config /app/config/config.yaml --host 0.0.0.0`.
- Egress is **default-deny** at the microsandbox layer: DNS + tcp/443 only
  to `openrouter.ai`, `api.kimi.com`, `api.minimax.io`, `api.neuralwatt.com`.
  LiteLLM-level `allowed_ips` is **not** set (network policy enforces it).
- Single-instance → no Redis, no `PROMETHEUS_MULTIPROC_DIR` needed for basic
  metrics, no cross-pod counters.
- Health probes used by the sandbox orchestrator:
  `GET /health/liveliness` (liveness), `GET /health/readiness` (readiness),
  `GET /health` (deep connectivity — used sparingly, makes real API calls).
- Drain endpoint is **not enabled** (single-instance, in-cluster only).
- Env vars resolved via `os.environ/`:
  `LITELLM_MASTER_KEY`, `KIMI_CODE_API_KEY`, `MINIMAX_CODING_API_KEY`,
  `OPENROUTER_API_KEY`, `NEURALWATT_API_KEY`. These are **project env vars**,
  not LiteLLM built-ins — they are injected by the sandbox `env()`/`secret_env()`.
- Config changes require a microVM restart (hot reload not documented).
