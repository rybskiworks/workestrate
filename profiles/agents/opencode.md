# OpenCode Sandbox Profile

## Source
OpenCode (upstream: `anomalyco/opencode`, npm: `opencode-ai`, binary: `opencode`)

## Language / Runtime
TypeScript, Node 24+

## Base Image
`node:24-bookworm-slim`

## Binary
`opencode` (npm: `opencode-ai`)

## Modes
- Web UI: `opencode serve --hostname 0.0.0.0 --port 3000`
- MCP/ACP server mode (via custom entrypoint script)

## Port
3000 (web UI)

## Expected Integration
OpenCode-ai talks to LiteLLM through the OpenAI-compatible endpoint.
Honors `OPENAI_BASE_URL` and `OPENAI_API_KEY`.

## Config
The tracked file `agents/opencode/config/opencode.jsonc` configures the
OpenAI provider with four LiteLLM model aliases:
- `coding` → Kimi K2.7
- `coding.fast` → Qwen 3.6 35B fast (Neuralwatt)
- `coding.pro` → GLM-5.2 short (Neuralwatt)
- `coding.free` → Qwen 3 Coder (OpenRouter, free)

Default model: `coding`.

## Expected Environment
- `OPENAI_BASE_URL` = `http://host.microsandbox.internal:4000/v1`
- `OPENAI_API_KEY` — set to `LITELLM_MASTER_KEY`, host-bound to the proxy
- `GITHUB_TOKEN` — host-bound to github.com and api.github.com

## Workspace
- `agents/opencode/repo` mounted read-only at `/app`
- `workspaces/opencode` mounted read-write at `/workspace`
- State directory mounted at `/home/node/.local/share/opencode`

## Egress
- Default deny
- Allow TCP/4000 to host (LiteLLM)
- Allow TCP/443 to github.com + api.github.com
- Allow DNS (UDP+TCP 53 to host)

## Status
Compile-checked only in milestone 1

## Unknowns
- Exact opencode-ai config format (opencode.jsonc schema)
- Whether opencode-ai is pre-installed in the base image or needs npm install at runtime
- Whether the `openai` provider in opencode respects OPENAI_BASE_URL env var directly
