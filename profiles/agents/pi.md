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

Example `~/.pi/agent/models.json`:
```json
{
  "providers": {
    "litellm": {
      "api": "openai-completions",
      "baseUrl": "http://host.microsandbox.internal:4000/v1",
      "apiKey": "${LITELLM_MASTER_KEY}",
      "models": ["openai-gpt", "anthropic-claude", "openrouter-default"]
    }
  }
}
```

## Expected Environment
- `PI_OFFLINE=1` — defense-in-depth; disables in-process update/telemetry code paths
- `PI_TELEMETRY=0` — defense-in-depth; disables in-process telemetry
- `OPENAI_API_KEY` — set to `LITELLM_MASTER_KEY` in M1 (no virtual keys yet)

The authoritative block on Pi telemetry domains is enforced by the sandbox network policy (deny egress to `*.pi.dev`). The environment variables above provide defense-in-depth in case the upstream agent checks them before making network requests.

## Forbidden
- Raw provider API keys in the Pi sandbox
- Direct provider API calls

## Workspace
- `agents/pi` mounted read-only at `/app`
- `workspaces/pi` mounted read-only at `/workspace`

## Egress
- Default deny
- Allow TCP/4000 to host (LiteLLM)
- Allow DNS (UDP+TCP 53 to host)
- Deny egress to `*.pi.dev` (telemetry domains, including subdomains)

`PI_OFFLINE=1` and `PI_TELEMETRY=0` are set as defense-in-depth; the network-layer deny rule is the authoritative block on Pi telemetry egress.

## Status
Profile-only in milestone 1

## Unknowns
- Exact RPC/headless command handshake
- Exact provider config mechanism (requires `models.json` seeding)
