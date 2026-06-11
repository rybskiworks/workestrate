# Odysseus Sandbox Profile

## Source
- Pinned fork: `github:georgrybski/odysseus`
- Local override: `agents/odysseus/`

## Repo Shape
Flat Python application (FastAPI + ChromaDB + RAG)

## Language / Runtime
Python 3.12

## Base Image
`python:3.12-slim` with `build-essential cmake curl git nodejs npm tmux openssh-client gosu`

## Entrypoint
`uvicorn app:app --host 0.0.0.0 --port 7000`

## Port
7000:7000 (host:guest)

## Expected Integration
Odysseus should call LiteLLM if it acts as an agent/client.

**Important:** Odysseus does NOT honor `OPENAI_BASE_URL`. It uses `LLM_HOST` for local model discovery or `data/settings.json` for per-provider configuration.

## Expected Environment
- `APP_PORT=7000`
- `AUTH_ENABLED=true`
- `LOCALHOST_BYPASS=false` — do NOT enable in sandbox
- `OPENAI_API_KEY` — LiteLLM-facing dummy key

## Forbidden
- Raw provider API keys in the Odysseus sandbox
- Direct provider API calls

## Workspace
- `agents/odysseus` mounted read-only at `/app`
- `workspaces/odysseus-data` mounted read-write at `/app/data` (persistent state)

## Egress
- Default deny
- Allow TCP/4000 to host (LiteLLM)

## Status
Profile-only in milestone 1

## Unknowns
- Exact entrypoint customization for LiteLLM proxy mode
- Exact provider config mechanism (requires `data/settings.json` setup)
- Companion process requirements
