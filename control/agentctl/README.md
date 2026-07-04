# workestrate

Control plane CLI for the AI workbench.

## Commands

- `workestrate check` — verify required files exist
- `workestrate litellm plan` — print LiteLLM sandbox plan
- `workestrate litellm up` — start the LiteLLM sandbox (detached; M2: requires KVM)
- `workestrate litellm down` — stop and remove the LiteLLM sandbox
- `workestrate litellm logs` — tail the detached LiteLLM service's log
- `workestrate pi plan` — print Pi sandbox plan
- `workestrate pi exec` — attach to the Pi sandbox interactively (M2: requires KVM)
- `workestrate pi down` — stop and remove the Pi sandbox
- `workestrate odysseus plan` — print Odysseus sandbox plan
- `workestrate odysseus up` — start the Odysseus sandbox (detached; M2: requires KVM)
- `workestrate odysseus down` — stop and remove the Odysseus sandbox
- `workestrate odysseus logs` — tail the detached Odysseus service's log

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
