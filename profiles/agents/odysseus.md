# Odysseus Sandbox Profile

## Source
- Pinned fork: `github:georgrybski/odysseus`
- Local override: `agents/odysseus/repo`

## Repo Shape
Flat Python application (FastAPI + ChromaDB + RAG)

## Language / Runtime
Python 3.12

## Base Image
`python:3.12-slim` with `build-essential cmake curl git nodejs npm tmux openssh-client gosu`

## Entrypoint
`python -m uvicorn app:app --host 0.0.0.0 --port 7000` (with `PYTHONPATH=/app/.deps` so the slim image imports vendored deps from `agents/odysseus/build/.deps`)

## Port
7000:7000 (host:guest). With the `allow_local` ingress policy the service is reachable at `http://localhost:7000`.

## Starting
From the repo root:
```bash
# Start the LiteLLM proxy first
nix develop -c run-with-secrets litellm up

# Start Odysseus in the foreground
nix develop -c run-with-secrets odysseus up
```

## Background mode
Both LiteLLM and Odysseus support a `-b` / `--background` flag that spawns a detached child process and exits immediately. The detached child keeps the sandbox alive and writes logs to `~/.microsandbox/sandboxes/<name>/agentctl.log`.

```bash
nix develop -c run-with-secrets odysseus up -b
nix develop -c run-with-secrets litellm up -b
```

**Important:** When started through `run-with-secrets`, the detached child is `agentctl` itself, so it does not inherit decrypted secrets. For M1 this is an accepted limitation. If you need secrets in a background sandbox, use the classic foreground-in-background approach:

```bash
nohup nix develop -c run-with-secrets odysseus up > odysseus.log 2>&1 &
```

## Expected Integration
Odysseus should call LiteLLM if it acts as an agent/client.

**Important:** Odysseus does NOT honor `OPENAI_BASE_URL`. It uses `LLM_HOST` for local model discovery or `data/settings.json` for per-provider configuration.

For LiteLLM, use `data/settings.json` (not `LLM_HOST`). Example snippet:
```json
{
  "providers": {
    "litellm": {
      "base_url": "http://host.microsandbox.internal:4000/v1",
      "api_key": "${LITELLM_MASTER_KEY}",
      "model": "coding"
    }
  }
}
```

The tracked file `agents/odysseus/config/settings.json` is the source of truth
for provider config and is mounted read-only at `/app/data/settings.json`.
Secrets are now managed via centralized secret definitions, which agentctl
injects at runtime — no literal `${LITELLM_MASTER_KEY}` substitution is needed
in the tracked file.

`LLM_HOST` is only for local model servers (Ollama, LM Studio) and is not used in the LiteLLM routing path.

## Expected Environment
- `APP_PORT=7000`
- `AUTH_ENABLED=true`
- `ODYSSEUS_ADMIN_PASSWORD` — admin login password (secret; sourced from `.env.enc` via `setup-secrets`). Required because `AUTH_ENABLED=true`; without it Odysseus auto-generates a random password printed to the sandbox logs.
- `LOCALHOST_BYPASS=false` — do NOT enable in sandbox
- `DATABASE_URL=sqlite:///app/data/app.db`
- `OPENAI_BASE_URL=http://host.microsandbox.internal:4000/v1` (defense-in-depth — Odysseus uses settings.json for provider config, not this env var)
- `OPENAI_MODEL=coding`
- `OPENAI_API_KEY` — set to `LITELLM_MASTER_KEY` in M1 (no virtual keys yet)

## Forbidden
- Raw provider API keys in the Odysseus sandbox
- Direct provider API calls

## Workspace
- `agents/odysseus/repo` mounted read-only at `/app`
- `workspaces/odysseus-data` mounted read-write at `/app/data` (persistent state)
- `${MSB_HOME}/sandboxes/odysseus/data` mounted read-write at `/data` (persistent state outside the app tree)
- `agents/odysseus/config/settings.json` mounted read-only at `/app/data/settings.json`

## Egress
- Default deny
- Allow DNS (TCP+UDP 53 to host)
- Allow TCP/4000 to host (LiteLLM)
- Allow TCP/443 to github.com + api.github.com
- Allow TCP/443 to huggingface.co + cdn-lfs.huggingface.co + cdn-lfs-us-1.huggingface.co (fastembed ONNX model fetch)

## Status
Profile-only in milestone 1

## Unknowns
- Exact entrypoint customization for LiteLLM proxy mode
- Exact provider config mechanism (requires `data/settings.json` setup)
- Companion process requirements
