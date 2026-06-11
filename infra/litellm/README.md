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

- `config.yaml` — proxy configuration
- `.env.example` — environment variable template
- `.env` — local secrets (gitignored, mode 0600)

## Milestone 1 Constraints

- **No Postgres** — runs in-memory. Virtual keys, budgets, and spend tracking are future scope.
- **No dashboards** — no UI, just the proxy.
- **Env vars only** — no real keys committed to the repo.

## Health Endpoints

- `GET /health/liveliness` — liveness probe
- `GET /health/readiness` — readiness (includes DB status)
- `GET /health` — model connectivity (makes actual API calls)

## Adding Models

Add entries to `model_list` in `config.yaml`. Use `os.environ/VAR_NAME` for all secrets.
