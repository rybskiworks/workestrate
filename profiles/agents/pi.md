# Pi Sandbox Profile

## Source
- Pinned fork: `github:georgrybski/pi`
- Local override: `agents/pi/`

## Repo Shape
Full TypeScript monorepo (npm workspaces)

## Relevant Package
`packages/coding-agent` — the shippable CLI

## Language / Runtime
TypeScript, Node 22.19+ (upstream Dockerfile uses `node:24-bookworm-slim`)

## Base Image
`node:24-bookworm-slim` with `bash ca-certificates git ripgrep`

## Binary
`pi` → `dist/cli.js`

## Modes
- Interactive TUI (default)
- `pi -p` — print and exit
- `pi --mode json` — JSONL event stream
- `pi --mode rpc` — JSONL over stdio (recommended for headless)

## Port
None — stdio only

## Expected Integration
Pi should call LiteLLM through an OpenAI-compatible endpoint.

**Important:** Pi does NOT honor `OPENAI_BASE_URL`. To use LiteLLM, seed `~/.pi/agent/models.json` with a custom provider pointing at the LiteLLM endpoint.

## Expected Environment
- `PI_OFFLINE=1` — disable telemetry/update checks
- `PI_TELEMETRY=0` — disable telemetry

## Forbidden
- Raw provider API keys in the Pi sandbox
- Direct provider API calls

## Workspace
- `agents/pi` mounted read-only at `/app`
- `workspaces/pi` mounted read-only at `/workspace`

## Egress
- Default deny
- Allow TCP/4000 to host (LiteLLM)
- Block `pi.dev` (telemetry)

## Status
Profile-only in milestone 1

## Unknowns
- Exact RPC/headless command handshake
- Exact provider config mechanism (requires `models.json` seeding)
