# Testing Milestone 1

## Environment requirements

- Linux host with KVM available (`/dev/kvm` exists)
- Nix with flakes enabled
- Network access to GitHub and crates.io

## Quick validation

```bash
# Enter the dev shell
nix develop

# Run all code quality checks
just check

# Run the four milestone-1 commands
cargo run --manifest-path control/agentctl/Cargo.toml -- check
cargo run --manifest-path control/agentctl/Cargo.toml -- litellm plan
cargo run --manifest-path control/agentctl/Cargo.toml -- agent plan pi
cargo run --manifest-path control/agentctl/Cargo.toml -- agent plan odysseus
```

## What should pass

- `just check` exits 0
- All four `agentctl` commands exit 0
- `agentctl check` reports `[OK]` for most entries; `[MISSING] agents/pi` is expected unless you cloned it locally

## What is NOT expected to work

- Actual sandbox runtime (`agentctl litellm up`, etc.) — those are milestone 2
- Pi/Odysseus doing real model calls through LiteLLM without manual provider config seeding

## Optional: Nix package build

```bash
nix build .#agentctl
./result/bin/agentctl check
```

## Optional: flake lock update

If the GitHub fork URLs ever change:

```bash
nix flake lock --update-input pi
nix flake lock --update-input odysseus
```
