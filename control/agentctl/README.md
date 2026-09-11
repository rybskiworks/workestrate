# workestrate

Control plane CLI for the AI workbench.

## Commands

- `workestrate check` — verify required files exist
- `workestrate litellm plan` — print LiteLLM sandbox plan
- `workestrate litellm up` — start the LiteLLM sandbox (detached; M2: requires KVM)
- `workestrate litellm down` — stop and remove the LiteLLM sandbox
- `workestrate litellm logs` — tail the detached LiteLLM service's log
- `workestrate pi plan` — print Pi sandbox plan
- `workestrate pi exec` — attach to the Pi sandbox interactively; execs `/app/bin/pi` (the bun-compile standalone binary built by the config repo flake; npm/node is the fallback). M2: requires KVM.
- `workestrate pi down` — stop and remove the Pi sandbox
- `workestrate odysseus plan` — print Odysseus sandbox plan
- `workestrate odysseus up` — start the Odysseus sandbox (detached; M2: requires KVM)
- `workestrate odysseus down` — stop and remove the Odysseus sandbox
- `workestrate odysseus logs` — tail the detached Odysseus service's log

## Build

Inside the Nix dev shell (run `just shell` from the repo root):
```bash
just shell
cargo build --release
```

Or with just:
```bash
just build
```

Hermetic nix build (no devshell needed to run):

```bash
nix build .#workestrate
./result/bin/workestrate check
```

Workload images (pi bun-compile binary, tempest npm-build tree) are built
by the config repo flake via the tool's exported lib recipes
(`lib.buildImagesFromConfig`), not by this repo; load them into the
microsandbox store from the config repo.

## Development

```bash
just shell -c cargo check
just shell -c cargo clippy -- -D warnings
just shell -c cargo fmt -- --check
```

## Build artifact location (`CARGO_TARGET_DIR`)

Cargo's `target/` directory is **relocated out of the source tree** to keep
the repo small (a clean `cargo build` is ~5–25 GB) and to keep untracked
build artifacts out of the nix store view.

- **Default location:** `${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target`
- Set automatically by:
  - the Nix dev shell (`flake.nix` `devenv.shells.default` `enterShell`), and
  - every cargo recipe in the top-level `justfile`
    (`export CARGO_TARGET_DIR :=` at the top of the file).
- A legacy in-tree `control/agentctl/target/`, if present, is still ignored
  by `control/agentctl/.gitignore` so bare `cargo` invocations outside the
  dev shell do not accidentally commit artifacts.

To override for a one-off build:

```bash
CARGO_TARGET_DIR=/tmp/my-build just build
```
