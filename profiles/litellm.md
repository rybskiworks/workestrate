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
`infra/litellm/` directory mounted read-only at `/app/config`; proxy started with `--config /app/config/config.yaml` (`config.yaml` `include:`s `models.yaml`)

## Environment
- Default completion model: `coding` (set via `general_settings.completion_model` in `infra/litellm/config.yaml`)
- `PORT=4000` — proxy listen port
- `LITELLM_MASTER_KEY` — proxy auth key (also used by agents in M1)
- `OPENROUTER_API_KEY` (https://openrouter.ai/api/v1)
- `KIMI_CODE_API_KEY` (https://api.kimi.com/coding)
- `NEURALWATT_API_KEY` (https://api.neuralwatt.com/v1)

## Coding-tier model names

Clients ask for a tier; LiteLLM maps it to the configured upstream model.

| Tier | Primary | Fallback chain | Use case |
|---|---|---|---|
| `coding` | Kimi K2.7 (`anthropic/kimi-for-coding`) | neural-kimi-k2.7-code → coding.free | Default coding work |
| `coding.fast` | Qwen 3.6 35B fast (Neuralwatt) | neural-qwen3.6-35b → coding | Quick edits, autocomplete |
| `coding.pro` | GLM-5.2 short (Neuralwatt) | neural-glm-5.2 → neural-kimi-k2.7-code | Complex refactoring, reasoning |
| `coding.free` | Qwen 3 Coder free (OpenRouter) | neural-qwen3.6-35b-fast | Experimentation, no cost |
| `coding.vision` | Kimi K2.6 (Neuralwatt) | neural-qwen3.6-35b → neural-kimi-k2.7-code | Vision + code |
| `orchestrator` | GLM-5.2 (Neuralwatt, 1M ctx) | neural-glm-5.2-short → neural-qwen3.5-397b | Orchestration, long context |
| `lead` | GLM-5.2 short (Neuralwatt) | neural-glm-5.2 → neural-kimi-k2.7-code | Lead agent reasoning |
| `vision` | Kimi K2.6 (Neuralwatt) | neural-qwen3.6-35b | Vision-only tasks |

## Providers

- `anthropic/kimi-for-coding` at `https://api.kimi.com/coding` — Kimi for Coding,
  primary upstream for `coding` (user-provided endpoint; routed through the
  Anthropic provider because the upstream is Anthropic-Messages-API compatible)
- `openai/*` at `https://api.neuralwatt.com/v1` — Neuralwatt (OpenAI-compatible),
  primary upstream for `coding.fast` (qwen3.6-35b-fast), `coding.pro`
  (glm-5.2-short), `coding.vision` (kimi-k2.6), `orchestrator` (glm-5.2),
  `lead` (glm-5.2-short), `vision` (kimi-k2.6), and all `neural-*`
  direct-access aliases
- `openrouter/qwen/qwen3-coder:free` — OpenRouter (free), primary upstream for
  `coding.free`

The `anthropic/` prefix is intentional: the user-provided Kimi endpoint
speaks the Anthropic Messages API, not OpenAI's. Do not change it to a
native LiteLLM provider name without confirming the upstream protocol.

## Proxy behavior
- `general_settings.completion_model: coding` — default tier when a client does not specify one
- `router_settings.fallbacks` — maps each tier to its fallback alias
- `litellm_settings.drop_params: true` — strip provider-unsupported params
  instead of erroring (some upstreams reject fields the Anthropic/OpenAI spec
  does not define)

## Egress
- Default-deny; allow DNS (UDP+TCP/53) to host
- Allow TCP/443 to `openrouter.ai`, `api.kimi.com`, `api.neuralwatt.com`
- All OpenRouter models (Qwen 3 Coder free) route through `openrouter.ai`
- Neuralwatt catalog models (`neural-*`) route through `api.neuralwatt.com`

## Status
Planned / compile-checked / runtime-blocked (no KVM in container)
