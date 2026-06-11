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
- `LITELLM_MASTER_KEY` — proxy auth key
- `OPENAI_API_KEY` — real provider key (host-side only)
- `ANTHROPIC_API_KEY` — real provider key (host-side only)
- `OPENROUTER_API_KEY` — real provider key (host-side only)

## Egress
- Default deny
- Allow TCP/443 to public IPs (model providers)

## Status
Planned / compile-checked / runtime-blocked (no KVM in container)
