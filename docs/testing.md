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
cargo run --manifest-path control/agentctl/Cargo.toml -- pi plan
cargo run --manifest-path control/agentctl/Cargo.toml -- odysseus plan
```

## What should pass

- `just check` exits 0
- All four `workestrate` commands exit 0
- `workestrate check` reports `[OK]` for most entries; `[MISSING] agents/pi/repo (or flake input)`
  is expected because `agents/pi/repo/` is an optional local override. Clone your Pi fork into
  `agents/pi/repo` if you want the check to pass locally.

## What is NOT expected to work in M1

- Actual sandbox runtime (`workestrate litellm up`, etc.) is implemented in code but can only be tested on a host with KVM.
- Pi/Odysseus doing real model calls through LiteLLM without seeding their native config files (`models.json` / `data/settings.json`).

## Optional: Nix package build

```bash
nix build .#workestrate
./result/bin/workestrate check
```

## Optional: flake lock update

If the GitHub fork URLs ever change:

```bash
nix flake lock --update-input pi
nix flake lock --update-input odysseus
```

## Troubleshooting

### `cargo: command not found`

You are outside the Nix dev shell. Either run `nix develop` first, or prefix commands with `nix develop -c`:

```bash
nix develop -c cargo check
nix develop -c just check
```

### `nix build .#workestrate` fails

The `microsandbox` crate writes to `$HOME` during its build. The Nix derivation sets `HOME=$TMPDIR` to handle this. If you still see a permission error, ensure your Nix sandbox is enabled.
