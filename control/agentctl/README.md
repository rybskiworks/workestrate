# agentctl

Control plane CLI for the AI workbench.

## Commands

- `agentctl check` — verify required files exist
- `agentctl litellm plan` — print LiteLLM sandbox plan
- `agentctl litellm up` — start the LiteLLM sandbox (M2: requires KVM)
- `agentctl litellm down` — stop and remove the LiteLLM sandbox
- `agentctl agent plan pi` — print Pi sandbox plan
- `agentctl agent plan odysseus` — print Odysseus sandbox plan
- `agentctl agent up pi` — start the Pi sandbox (M2: requires KVM)
- `agentctl agent down pi` — stop and remove the Pi sandbox
- `agentctl agent up odysseus` — start the Odysseus sandbox (M2: requires KVM)
- `agentctl agent down odysseus` — stop and remove the Odysseus sandbox

## Build

Inside the Nix dev shell:
```bash
nix develop
cargo build --release
```

Or with just:
```bash
just build
```

## Development

```bash
nix develop -c cargo check
nix develop -c cargo clippy -- -D warnings
nix develop -c cargo fmt -- --check
```
