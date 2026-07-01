# agentctl

Control plane CLI for the AI workbench.

## Commands

- `agentctl check` — verify required files exist
- `agentctl litellm plan` — print LiteLLM sandbox plan
- `agentctl litellm up` — start the LiteLLM sandbox (M2: requires KVM)
- `agentctl litellm down` — stop and remove the LiteLLM sandbox
- `agentctl pi plan` — print Pi sandbox plan
- `agentctl odysseus plan` — print Odysseus sandbox plan
- `agentctl pi up` — start the Pi sandbox (M2: requires KVM)
- `agentctl pi down` — stop and remove the Pi sandbox
- `agentctl odysseus up` — start the Odysseus sandbox (M2: requires KVM)
- `agentctl odysseus down` — stop and remove the Odysseus sandbox

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
