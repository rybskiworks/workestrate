# agentctl

Control plane CLI for the AI workbench.

## Commands

- `agentctl check` — verify required files exist
- `agentctl litellm plan` — print LiteLLM sandbox plan
- `agentctl agent plan pi` — print Pi sandbox plan
- `agentctl agent plan odysseus` — print Odysseus sandbox plan

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
