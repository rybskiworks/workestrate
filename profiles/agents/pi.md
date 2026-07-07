# Pi Sandbox Profile

## Source
- Pinned fork: `github:georgrybski/pi` (single canonical source for `.#pi` and `.#pi-bun`)
- Local override: `agents/pi/` (populated from the flake input by the dev shell)
- Local hacking: `just dev-build-pi` (hashless native npm into `agents/pi/build`)

## Repo Shape
Full TypeScript monorepo (npm workspaces)

## Relevant Package
`packages/coding-agent` — the shippable CLI

## Language / Runtime
TypeScript. Built two ways: `.#pi-bun` (canonical — standalone Bun binary,
Bun runtime embedded, no node/bun needed at runtime) and `.#pi` (npm/node
fallback). Dev shell pins `nodejs_24` (was `nodejs_22`; fixes gondolin
`EBADENGINE`). Upstream Dockerfile uses `node:24-bookworm-slim`.

## Base Image
`node:24-bookworm-slim` with `bash ca-certificates git ripgrep`

## Binary
- Canonical (`.#pi-bun`): `/app/bin/pi` — self-contained Bun-compiled binary
  (Bun runtime embedded), with runtime assets (themes, export-html templates,
  photon wasm) mirrored alongside.
- Fallback (`.#pi`): `node /app/packages/coding-agent/dist/cli.js`.

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
- pi build tree mounted read-only at `/app` — canonical: the `.#pi-bun`
  standalone binary (via `WORKESTRATE_PI_BUILD`); fallback: `agents/pi/build`
  (via `just dev-build-pi`) when `WORKESTRATE_PI_BUILD` is unset.
- `workspaces/pi` mounted read-write at `/workspace`

## Egress
- Default deny
- Allow TCP/4000 to host (LiteLLM)
- Allow DNS (UDP+TCP 53 to host)
- Allow TCP/443 to github.com + api.github.com
- Deny egress to `*.pi.dev` (telemetry domains, including subdomains)

`PI_OFFLINE=1` and `PI_TELEMETRY=0` are set as defense-in-depth; the network-layer deny rule is the authoritative block on Pi telemetry egress.

## Status
- `.#pi` and `.#pi-bun` derivations land in M1 (compile- and plan-verified).
- Bun-binary sandbox exec (`/app/bin/pi`) pending KVM runtime validation;
  `.#pi` (node) is the fallback.
- Fork-carries-compat: nix-build compat lives on the agent fork, not as
  nix-side patches.

## Unknowns
- Exact RPC/headless command handshake
- Exact provider config mechanism (requires `models.json` seeding)
