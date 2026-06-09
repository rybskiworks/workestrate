# `agentctl` — control plane CLI for the AI workestrator

The Rust control plane for the workestrator. See `../../README.md` for
the project description.

## Status

First-milestone commands implemented and validated:

| command                              | status |
| ------------------------------------ | ------ |
| `agentctl init`                      | works  |
| `agentctl providers check`           | works  |
| `agentctl litellm print-plan`        | works  |
| `agentctl postgres status`           | stub ("deferred") |
| `agentctl sandbox plan`              | stub ("deferred") |
| `agentctl agents list`               | stub ("deferred") |

The stubs print a "deferred" line and exit 0 so the CLI surface
matches the plan without claiming behavior that isn't implemented.

## Build

This crate is built with the project-local toolchain in
`../../.toolchain/` (rustup + zig cc), because the Nix CLI is not
installed in this environment and there is no system `cc`.

```bash
cd <project-root>
just build                # debug build of agentctl
just test                 # cargo test --bin agentctl
just build-release        # release build
```

The `justfile` (one level up) wraps the env-var and target-triple
ceremony. With `just` not installed, the same invocations work directly:

```bash
cd control/agentctl
CARGO_HOME=$(pwd)/../../.toolchain/cargo \
RUSTUP_HOME=$(pwd)/../../.toolchain/rustup \
CC=$(pwd)/../../.toolchain/zig-x86_64-linux-0.16.0/zig cc \
cargo build --target x86_64-unknown-linux-gnu --bin agentctl
```

## Run the three commands

```bash
just init                 # creates agents/, workspaces/, var/, tmp/
just providers            # validates .env against the expected shape
just litellm-plan         # describes the LiteLLM sandbox
just plan                 # all three in sequence
just poc-test             # runs the original POC's run-tests.sh
```

The `init` and `plan` recipes do not touch the existing `var/` or
`infra/`; they only create what's missing and skip what's already
there.

## Layout

```
control/agentctl/
├── Cargo.toml           crate manifest (clap is the only dep)
├── README.md            this file
└── src/
    ├── main.rs          clap dispatcher
    ├── config.rs        path resolution (project_root, env_file, etc.)
    ├── envfile.rs       minimal .env parser
    ├── init.rs          `agentctl init`
    ├── providers.rs     `agentctl providers check`
    ├── litellm.rs       `agentctl litellm print-plan` (with hand-rolled
    │                    YAML reader for the model_list + general_settings
    │                    subset we need)
    ├── postgres.rs      `agentctl postgres status` (deferred stub)
    ├── sandbox.rs       `agentctl sandbox plan` (deferred stub)
    └── agents.rs        `agentctl agents list` (deferred stub)
```

## Why no external dependencies beyond `clap`?

The plan's design uses `serde_yaml` / `serde` / `tokio` / `sqlx` etc.
Those crates are fine on a system with a working C compiler. Here the
only available C toolchain is `zig cc` (used as the linker and as the
`cc` for build scripts), and pulling in serde-yaml's transitive deps
adds build time and risk. The hand-rolled `.env` and YAML readers are
60–80 lines each and handle exactly the subset of formats this
project produces.

If/when the system C toolchain is available, the YAML reader can be
replaced with `serde_yaml` without changing any call site — the public
API of `litellm::Plan` is stable.
