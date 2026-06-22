# LiteLLM Sandbox Profile

## Name
litellm

## Role
Central model gateway

## Image
`ghcr.io/berriai/litellm:main-stable`

## Port
4000:4000 (host:guest)

## Config
`infra/litellm/config.yaml` mounted read-only at `/app/config.yaml`

## Environment
- Default completion model: `chat` (set via `general_settings.completion_model` in `infra/litellm/config.yaml`)
- `PORT=4000` — proxy listen port
- `LITELLM_MASTER_KEY` — proxy auth key (also used by agents in M1)
- `OPENROUTER_API_KEY` (https://openrouter.ai/api/v1)
- `KIMI_CODE_API_KEY` (https://api.kimi.com/coding)
- `MINIMAX_CODING_API_KEY` (https://api.minimax.io/anthropic)
- `INCEPTION_API_KEY` (https://api.inceptionlabs.ai/v1)

## Role-based model names

Clients ask for a role; LiteLLM maps it to the configured upstream model.

| Role | Primary upstream | Fallback |
|---|---|---|
| `chat` | `openrouter/openai/gpt-4o` | `chat-fallback` |
| `coding` | `anthropic/kimi-for-coding` at `https://api.kimi.com/coding` | `coding-fallback` |
| `reasoning` | `inception/mercury-2` at `https://api.inceptionlabs.ai/v1` | `reasoning-fallback` |

Fallback targets are internal aliases and are not exposed to clients:

| Fallback alias | Upstream |
|---|---|
| `chat-fallback` | `anthropic/MiniMax-M3` at `https://api.minimax.io/anthropic` |
| `coding-fallback` | `anthropic/MiniMax-M3` at `https://api.minimax.io/anthropic` |
| `reasoning-fallback` | `openrouter/openai/gpt-4o` |

## Providers
- `openrouter/openai/gpt-4o` — OpenRouter
- `anthropic/kimi-for-coding` at `https://api.kimi.com/coding` — Kimi for Coding
  (user-provided endpoint; routed through the Anthropic provider because the
  upstream is Anthropic-Messages-API compatible)
- `anthropic/MiniMax-M3` at `https://api.minimax.io/anthropic` — MiniMax Coding
  (user-provided endpoint; routed through the Anthropic provider because the
  upstream is Anthropic-Messages-API compatible. LiteLLM's native `minimax/`
  prefix is OpenAI-compatible and would not match)

  The `anthropic/` prefix is intentional: the user-provided Kimi and MiniMax
  endpoints speak the Anthropic Messages API, not OpenAI's. Do not change
  them to native LiteLLM provider names without confirming the upstream
  protocol.
- `inception/mercury-2` at `https://api.inceptionlabs.ai/v1` — Inception Labs

## Proxy behavior
- `general_settings.completion_model: chat` — default role when a client does not specify one
- `router_settings.fallbacks` — maps each role to its fallback alias
- `litellm_settings.drop_params: true` — strip provider-unsupported params
  instead of erroring (some upstreams reject fields the Anthropic/OpenAI spec
  does not define)

## Egress
- Default-deny; allow DNS (UDP+TCP/53) to host
- Allow TCP/443 to `openrouter.ai`, `api.kimi.com`, `api.minimax.io`, `api.inceptionlabs.ai`

## Status
Planned / compile-checked / runtime-blocked (no KVM in container)
