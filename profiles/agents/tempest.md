# T3MP3ST Sandbox Profile

## Source
- Pinned fork: `github:georgrybski/T3MP3ST` (single canonical source for `.#tempest-built`)
- Local override: `agents/tempest/` (populated from the flake input by the dev shell)
- Local hacking: `npm install && npm run build` in `agents/tempest/repo`

## Repo Shape
Single-package TypeScript app (no npm workspaces)

## Relevant Package
Root package — `dist/cli.js` is the shippable CLI

## Language / Runtime
TypeScript, compiled with `tsc` to `dist/`. Runs on Node.js 24
(`nodejs_24`). Upstream `engines` requires `>=18.0.0`.

## Base Image
Nix-built `tempest:latest` (dockerTools.buildLayeredImage).
Provides nodejs_24 + nmap + bind.dnsutils + cacert + busybox + fakeNss +
the compiled T3MP3ST tree.

## Binary
`node dist/cli.js` — the interactive CLI TUI (Agent workload, like Pi).
The compiled tree (dist/ + node_modules/ + package.json) is baked into the
image.

## Modes
- CLI interactive (default) — `workestrate tempest exec`
- Server mode (future) — `node dist/server.js` (Express API, loopback only
  via `T3MP3ST_HOST=127.0.0.1`)

## Port
None — CLI is stdio/TUI only. The Express API server (if started) binds to
127.0.0.1 inside the microVM and is never exposed to the host.

## Expected Integration
T3MP3ST uses the `local` LLM provider, which reads ALL config from env vars
(no conf-store dependency for secrets). The only conf-store dependency is
`defaultProvider: "local"`, which is baked into the image config
(`root/.config/t3mp3st/config.json`).

T3MP3ST calls the LiteLLM proxy through an OpenAI-compatible endpoint.

## Expected Environment
- `TEMPEST_LOCAL_BASE_URL` — `http://host.microsandbox.internal:4000/v1`
  (LiteLLM proxy)
- `TEMPEST_LOCAL_MODEL` — `coding` (LiteLLM model alias)
- `TEMPEST_LOCAL_API_KEY` — remapped from `LITELLM_MASTER_KEY` (the local
  provider reads the API key from this env var; see
  `agents/tempest/repo/src/config/index.ts` `getApiKey('local')`)
- `T3MP3ST_HOST` — `127.0.0.1` (loopback only; Express API stays inside the
  microVM)

## Provider Config
`defaultProvider: "local"` is baked into the image
(`root/.config/t3mp3st/config.json`). No secrets in this file — just
provider selection. All secrets come from env vars.

## Egress
- `default_deny: false` — T3MP3ST is an offensive-security tool that needs
  broad egress for scanning arbitrary targets (nmap, dig, etc.). The microVM
  boundary itself is the containment.
- Future: per-mission scope configuration to tighten egress to specific
  target ranges.

## Security
- microVM isolation (the sandbox boundary)
- No `HERMES_YOLO`, no `TEMPEST_MODEL_FALLBACK`, no `FULL_ARSENAL` by default
  (these are opt-in flags that T3MP3ST reads from env; they are NOT set by
  the workload plan)
- The LiteLLM proxy is the only LLM egress path (via
  `TEMPEST_LOCAL_BASE_URL`); direct provider API keys are not in the sandbox

## Forbidden
- Raw provider API keys in the T3MP3ST sandbox (all LLM traffic goes through
  LiteLLM)
- Direct provider API calls (bypassing LiteLLM)

## Workspace
- `workspaces/tempest-state` mounted read-write at `/data` (persistent state)
- Host cwd mounted read-write at `/work` (user project)

## Status
- `.#tempest-built` and `.#tempest-image` derivations land in M1
  (compile- and plan-verified).
- Runtime exec (`node dist/cli.js`) pending KVM runtime validation.
- npmDepsHash is a placeholder until computed in a nix-capable environment.
