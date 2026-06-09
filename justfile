# `just` recipes for the AI workbench.
#
# Run with the system `just` (if installed), or invoke individual
# recipes via:
#     nix develop -c just <recipe>     # use the Nix dev shell
#     bash scripts/run-tests.sh        # original POC validation
#
# Toolchain selection: each recipe picks one of two backends, in
# order of preference:
#   1. Nix dev shell  — `nix develop -c <cmd> ...`
#      (preferred; provides rustc, cargo, just, gcc, openssl, etc.)
#   2. Local fallback — `CARGO_HOME=.toolchain/cargo` + `.toolchain/cc`
#      (used when Nix is unavailable, e.g. outside this container)
#
# The local fallback is kept for environments without Nix; the Nix
# path is the canonical one for this project.

# Project root for cargo invocations
project_root := justfile_directory()

# Path to the agentctl binary built by the local toolchain. Override
# with `agentctl_bin=...` to point at a Nix-built binary.
agentctl_bin := project_root / "control/agentctl/target/x86_64-unknown-linux-gnu/debug/agentctl"

# Cargo target triple. Override if you build for a different target.
target := "x86_64-unknown-linux-gnu"

# Local-toolchain paths (used only by the toolchain fallback below).
local_cargo_home := project_root / ".toolchain/cargo"
local_rustup_home := project_root / ".toolchain/rustup"
local_cargo_bin := project_root / ".toolchain/cargo/bin"
local_cc_wrapper := project_root / ".toolchain/cc"

# Detect whether the Nix CLI is on PATH. Each recipe re-evaluates
# this at runtime; the value is the literal "yes" or "no" string
# emitted by the shell snippet below.
nix_probe := shell("command -v nix >/dev/null 2>&1 && echo yes || echo no")

# Default recipe: list all available recipes.
default:
    @just --list

# Build agentctl in debug mode (Nix dev shell preferred).
build:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "{{nix_probe}}" = "yes" ]; then
        nix develop -c cargo build --manifest-path control/agentctl/Cargo.toml --target {{target}} --bin agentctl
    else
        cd {{project_root}}/control/agentctl
        CARGO_HOME={{local_cargo_home}} \
        RUSTUP_HOME={{local_rustup_home}} \
        CC={{local_cc_wrapper}} \
        PATH={{local_cargo_bin}}:$PATH \
            cargo build --target {{target}} --bin agentctl
    fi

# Build agentctl in release mode (Nix dev shell preferred).
build-release:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "{{nix_probe}}" = "yes" ]; then
        nix develop -c cargo build --release --manifest-path control/agentctl/Cargo.toml --target {{target}} --bin agentctl
    else
        cd {{project_root}}/control/agentctl
        CARGO_HOME={{local_cargo_home}} \
        RUSTUP_HOME={{local_rustup_home}} \
        CC={{local_cc_wrapper}} \
        PATH={{local_cargo_bin}}:$PATH \
            cargo build --release --target {{target}} --bin agentctl
    fi

# Run the agentctl test suite (Nix dev shell preferred).
test:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "{{nix_probe}}" = "yes" ]; then
        nix develop -c cargo test --manifest-path control/agentctl/Cargo.toml --target {{target}} --bin agentctl
    else
        cd {{project_root}}/control/agentctl
        CARGO_HOME={{local_cargo_home}} \
        RUSTUP_HOME={{local_rustup_home}} \
        CC={{local_cc_wrapper}} \
        PATH={{local_cargo_bin}}:$PATH \
            cargo test --target {{target}} --bin agentctl
    fi

# Reproducible Nix build (alternative to the in-shell `cargo build`).
nix-build:
    nix build .#agentctl

# `agentctl init` — create the on-disk layout. Prefers the Nix-built
# binary when present, otherwise falls back to the local-toolchain
# build output.
init: build
    AGENTCTL_ROOT={{project_root}} {{agentctl_bin}} init

# `agentctl providers check` — validate .env contents.
providers: build
    AGENTCTL_ROOT={{project_root}} {{agentctl_bin}} providers check

# `agentctl litellm print-plan` — describe the LiteLLM sandbox.
litellm-plan: build
    AGENTCTL_ROOT={{project_root}} {{agentctl_bin}} litellm print-plan

# Convenience recipe: run the first three commands the plan calls for.
# Equivalent to: just init && just providers && just litellm-plan
plan: init providers litellm-plan
    @echo "all three milestone commands ran"

# Run the original POC end-to-end validation (the bash tests).
poc-test:
    bash {{project_root}}/scripts/run-tests.sh
