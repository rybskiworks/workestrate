# LiteLLM

LiteLLM is the central model gateway for the ai-workbench.

## Architecture

```
Pi sandbox ───────┐
Odysseus sandbox ─┼──> LiteLLM sandbox ───> model providers
Future agents ────┘
```

Agents call LiteLLM, not provider APIs directly.
Provider keys are only for LiteLLM.

## Configuration

- `config.yaml` — proxy configuration (parent; `include:`s `models.yaml`)
- `models.yaml` — `model_list` (model definitions, with YAML anchors)
- Secrets are managed via SOPS at the repo root (`.env.enc`) and injected via `run-with-secrets`. The LiteLLM microVM receives its secrets through the sandbox plan's `env()` and `secret_env()` calls.

## Milestone 1 Constraints

- **No Postgres** — runs in-memory. Virtual keys, budgets, and spend tracking are future scope.
- **No dashboards** — no UI, just the proxy.
- **Env vars only** — no real keys committed to the repo.
- **Real provider keys live inside the LiteLLM microVM** — agents authenticate to LiteLLM using `LITELLM_MASTER_KEY` in M1.

## Health Endpoints

- `GET /health/liveliness` — liveness probe
- `GET /health/readiness` — readiness (includes DB status)
- `GET /health` — model connectivity (makes actual API calls)

## Adding Models

Add entries to `model_list` in `models.yaml` (included by `config.yaml`). Use `os.environ/VAR_NAME` for all secrets.
