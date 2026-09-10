# Relocate cargo's target dir out of the source tree (closes the
# nix-purity anti-accumulation finding for target/). Evaluated at just-parse
# time so explicit caller paths or the XDG/HOME fallback are validated before
# any recipe runs. Recipes inherit the literal external path automatically.
export CARGO_TARGET_DIR := `bash ./scripts/cargo-target.sh "$(pwd -P)"`

# Enter the devenv shell interactively from a plain host shell. Writes the
# devenv-root override file (this worktree's abs path) and passes
# --override-input devenv-root so PURE eval resolves devenv.root to the
# worktree: dotfile/state land in the gitignored in-tree .devenv/, not a
# read-only store copy (bare `nix develop` cannot see PWD under pure eval).
# Extra args pass through, e.g. `just shell -c <cmd>` for one-shot commands.
[positional-arguments]
shell *args:
    #!/usr/bin/env bash
    set -euo pipefail
    _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
    mkdir -p "$_devenv_root_dir"
    _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
    _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
    printf '%s' "$_repo_root" > "$_devenv_root_file"
    exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" "$@"

# Enter tools without realizing the application or Microsandbox packages.
[positional-arguments]
bootstrap *args:
    #!/usr/bin/env bash
    set -euo pipefail
    exec just shell .#bootstrap "$@"

# Use the shared pinned tracker without entering the runtime shell. Embedded
# Dolt permits one writer; coordinate writes and keep remote synchronization
# explicit. This command does not initialize a tracker or install hooks.
[positional-arguments]
beads *args:
    #!/usr/bin/env bash
    set -euo pipefail
    export BEADS_DIR="${BEADS_DIR:-$PWD/.beads}"
    export DOLT_ROOT_PATH="${DOLT_ROOT_PATH:-$BEADS_DIR/dolt-global}"
    export BD_DISABLE_METRICS=1
    export BD_DISABLE_EVENT_FLUSH=1
    exec nix run --no-update-lock-file .#beads -- --sandbox -C "$PWD" "$@"

# Lock-guard: pre-resolution fork-pin check for control/agentctl/Cargo.lock
# (A1 no-registry-source, A2 ==X pins from Cargo.toml, A3 smoltcp vs fork).
# A broken lock breaks cargo resolution itself, so this bash+python3 script
# runs BEFORE any cargo recipe. Wired FIRST in the `verify` chain.
lock-guard:
    ./scripts/check-cargo-lock.sh

# Verify the host rustc major.minor matches the fenix-pinned toolchain.
# Parses the RUST_TOOLCHAIN_VERSION marker from flake.nix (not hardcoded
# here) so the check stays in sync with the flake input automatically.
# The marker MUST trail the live `rustToolchain = inputs.fenix...` line —
# grepping it there (instead of any standalone comment) keeps the check
# fail-closed: if the marker ever drifts off the evaluated line, this
# recipe errors instead of comparing against a stale comment. The grep anchors
# the marker comment to the live line and fails closed on zero or multiple
# hits (unanchored match or bare -oP extraction would silently accept drift
# or duplicates).
# This is a standalone check for interactive Cargo work. Repository verification
# uses the Fenix packages directly in its sandboxed derivations.
toolchain-check:
    #!/usr/bin/env bash
    set -euo pipefail
    matches=$(grep 'rustToolchain = inputs\.fenix.*# RUST_TOOLCHAIN_VERSION = "' flake.nix || true)
    hits=$(printf '%s\n' "$matches" | grep -c 'RUST_TOOLCHAIN_VERSION' || true)
    if [ "$hits" -ne 1 ]; then
        echo "ERROR: expected exactly 1 RUST_TOOLCHAIN_VERSION marker on the live rustToolchain line (found $hits)" >&2
        exit 1
    fi
    expected=$(printf '%s\n' "$matches" | grep -oP '# RUST_TOOLCHAIN_VERSION = "\K[^"]+')
    if [ -z "$expected" ]; then
        echo "ERROR: could not parse RUST_TOOLCHAIN_VERSION from flake.nix" >&2
        exit 1
    fi
    actual=$(rustc --version 2>/dev/null | grep -oP 'rustc \K[0-9]+\.[0-9]+' || true)
    if [ -z "$actual" ]; then
        echo "ERROR: rustc not found on PATH" >&2
        exit 1
    fi
    echo "expected (flake.nix): rustc $expected.x"
    echo "actual (host):         rustc $actual"
    if [ "$actual" != "$expected" ]; then
        echo "FAIL: rustc major.minor mismatch — expected $expected, got $actual" >&2
        echo "Run 'just shell' to enter the pinned toolchain shell." >&2
        exit 1
    fi
    echo "OK: rustc $actual matches pinned toolchain"

# Check SDK version and fork source pins without realizing the runtime packages.
# The fixture suite exercises missing, nonexact, and mismatched declarations.
versions-check:
    ./scripts/check-msb-versions.sh
    python3 ./scripts/test-msb-versions.py

# Verify fallback hook chaining without changing this checkout's Git hooks.
hooks-check:
    python3 ./scripts/test-pre-commit-hook.py

# Verify exact argument forwarding without entering a Nix development shell.
shell-arguments-check:
    python3 ./scripts/test-shell-arguments.py

store-audit-check:
    python3 ./scripts/test-store-audit.py

verification-check:
    python3 ./scripts/test-verification.py

purity-check:
    python3 ./scripts/test-nix-paths.py

check:
    @just _check-inner
[private]
_check-inner:
    nix build --no-link --no-update-lock-file .#checks.x86_64-linux.rust

# Supply-chain gates (cargo-deny, config at repo-root deny.toml): licenses,
# bans (wildcard deps), sources (unknown registries/git). advisories are
# CI-only by design (continue-on-error in ci.yml) — the advisory DB needs
# network + git, so it is deliberately NOT part of `just verify`.
deny-check:
    @just _deny-check-inner
[private]
_deny-check-inner:
    nix build --no-link --no-update-lock-file .#checks.x86_64-linux.deny

# Parse every fenced toml block in docs/migration/20-target-system-spec.md
# against the ConfigFile schema shape (WP4 / D1 standing guard).
spec-examples:
    @just _spec-examples-inner
[private]
_spec-examples-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _spec-examples-inner
    fi
    cargo test --manifest-path control/agentctl/Cargo.toml --test spec_examples_parse

# Repository verification never enters the runtime shell or reads consumer
# homes. Nix checks use the pinned toolchain, offline dependencies and isolated
# test state. Unit checks include spec examples, goldens, schema drift and the
# native scaffold; KVM, Nix-daemon and Copier integration remain separate gates.
verify: lock-guard versions-check hooks-check shell-arguments-check store-audit-check verification-check purity-check lint-nix
    #!/usr/bin/env bash
    set -euo pipefail
    nix build --no-link --no-update-lock-file --keep-going \
      .#checks.x86_64-linux.rust \
      .#checks.x86_64-linux.unit \
      .#checks.x86_64-linux.package \
      .#checks.x86_64-linux.pre-commit \
      .#checks.x86_64-linux.treefmt \
      .#checks.x86_64-linux.tombiCheck \
      .#checks.x86_64-linux.schemaSync \
      .#checks.x86_64-linux.buildRevision \
      .#checks.x86_64-linux.deny
    git diff --exit-code HEAD -- control/agentctl/Cargo.lock
    just store-audit

# Verification already builds the package for its installed CLI smoke check.
verify-full: verify
    nix build --no-link --no-update-lock-file .#workestrate

# Generate golden plan files for all workloads
golden-generate:
    @just _golden-generate-inner
[private]
_golden-generate-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _golden-generate-inner
    fi
    for name in example-service example-agent example-offensive; do \
        WORKESTRATE_CONFIG_DIR=config.reference cargo run --manifest-path control/agentctl/Cargo.toml -- $name plan \
          > control/agentctl/tests/golden/$name.plan.txt; \
    done

# Check golden plan parity
golden-check:
    @just _golden-check-inner
[private]
_golden-check-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _golden-check-inner
    fi
    for name in example-service example-agent example-offensive; do \
        WORKESTRATE_CONFIG_DIR=config.reference cargo run --manifest-path control/agentctl/Cargo.toml -- $name plan \
          | diff - control/agentctl/tests/golden/$name.plan.txt \
          || (echo "golden mismatch for $name; run 'just golden-generate' to update" && exit 1); \
    done

# Regenerate the canonical JSON Schemas for workestrate.toml from the
# schemars-derived ConfigFile, plus the bare-workload subschema derived from
# WorkloadConfig, plus the tool-home registry schema derived from Registry.
# Writes to schemas/workestrate.schema.json,
# schemas/workestrate-workload.schema.json, and schemas/registry.schema.json.
# Run on a nix-capable host (the
# dev shell's RUSTFLAGS → libcap-ng OUT lib dir is required to build the
# aws-lc-rs / parking_lot_core native crates).
# After regenerating, run `just schema-sync-check` (CI gate; exits 1 when a
# consumer copy is stale) and `workestrate schemas update` to distribute all
# three artifacts (full + workload subschema + registry schema) to the copier template
# (templates/workestrate-config/schemas/), tool home (schemas/), and each
# registered config repo that carries a schemas/ dir.
generate-schema:
    #!/usr/bin/env bash
    set -euo pipefail
    _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
    mkdir -p "$_devenv_root_dir"
    _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
    _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
    printf '%s' "$_repo_root" > "$_devenv_root_file"
    nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c cargo run --manifest-path control/agentctl/Cargo.toml --quiet -- generate-schema --output schemas/workestrate.schema.json --output-workload schemas/workestrate-workload.schema.json --output-registry schemas/registry.schema.json
    echo "schema written to schemas/workestrate.schema.json"
    echo "workload schema written to schemas/workestrate-workload.schema.json"
    echo "registry schema written to schemas/registry.schema.json"

# CI drift guard for schemas/workestrate.schema.json,
# schemas/workestrate-workload.schema.json, and schemas/registry.schema.json
# (ADR 0021 §8). Invokes control/agentctl/tests/schema_drift.rs (all three
# artifacts) plus the workload-only control/agentctl/tests/schema_subschema_drift.rs.
schema-check:
    @just _schema-check-inner
[private]
_schema-check-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _schema-check-inner
    fi
    cargo test --manifest-path control/agentctl/Cargo.toml --test schema_drift
    cargo test --manifest-path control/agentctl/Cargo.toml --test schema_subschema_drift

# CI drift guard for the consumer schema copies (schemas/ at templates/,
# tool home, and registered config repos). Exits 1 when any consumer copy
# is stale or missing; run `workestrate schemas update` to refresh.
schema-sync-check:
    @just _schema-sync-check-inner
[private]
_schema-sync-check-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _schema-sync-check-inner
    fi
    cargo run --manifest-path control/agentctl/Cargo.toml --quiet -- schemas update --check

# CI drift guard for the `workestrate config new` scaffold. Invokes
# control/agentctl/tests/scaffold_template.rs, which renders the embedded
# scaffold templates via the real binary, runs `validate-config` on the
# output, and (when `copier` is available) enforces byte-parity with the
# copier template's minimal-personal render + copier-update interop.
scaffold-check:
    @just _scaffold-check-inner
[private]
_scaffold-check-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _scaffold-check-inner
    fi
    cargo test --manifest-path control/agentctl/Cargo.toml --test scaffold_template

build:
    @just _build-inner
[private]
_build-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _build-inner
    fi
    cargo build --release --manifest-path control/agentctl/Cargo.toml

fmt:
    @just _fmt-inner
[private]
_fmt-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _fmt-inner
    fi
    cargo fmt --manifest-path control/agentctl/Cargo.toml

# Check formatting without modifying files
fmt-check:
    @just _fmt-check-inner
[private]
_fmt-check-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _fmt-check-inner
    fi
    cargo fmt --manifest-path control/agentctl/Cargo.toml -- --check

# Run Clippy with -D warnings (standalone)
clippy:
    @just _clippy-inner
[private]
_clippy-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _clippy-inner
    fi
    cargo clippy --manifest-path control/agentctl/Cargo.toml --all-targets -- -D warnings

# Run unit tests
test *args:
    @just _test-inner {{args}}
[private]
_test-inner *args:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _test-inner {{args}}
    fi
    cargo test --manifest-path control/agentctl/Cargo.toml {{args}}

# Run the three ignored KVM tests (lifecycle_detached, flake_root_gate,
# ensure_images_e2e) via the encapsulated entry point. HOST-KVM only:
# preflights /dev/kvm + the runtime-store image, runs each test serially
# in its own devshell, and prints a PASS/FAIL summary. Any shell with nix.
kvm-tests:
    ./scripts/kvm-tests.sh

workestrate *args:
    @just _workestrate-inner {{args}}
[private]
_workestrate-inner *args:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _workestrate-inner {{args}}
    fi
    cargo run --manifest-path control/agentctl/Cargo.toml -- {{args}}

plan:
    @just _plan-inner
[private]
_plan-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _plan-inner
    fi
    cargo run --manifest-path control/agentctl/Cargo.toml -- example-service plan
    cargo run --manifest-path control/agentctl/Cargo.toml -- example-agent plan
    cargo run --manifest-path control/agentctl/Cargo.toml -- example-offensive plan

# Check that the Debian/Linux host is ready to run the workbench
host-check:
    ./scripts/host-check.sh

# Provision/sync the host's nix-installed workestrate binary to the current
# tree, then run host-check + `workestrate doctor` and print a readiness
# verdict. Idempotent; only mutation is a nix profile install when stale.
host-provision:
    ./scripts/host-provision.sh

# Read-only provisioning check: asserts profile singularity, version
# identity, and msb/agentd liveness. Never installs anything.
provision-check:
    ./scripts/host-provision.sh --check-only

# Bootstrap or update encrypted secrets
setup-secrets *args:
    #!/usr/bin/env bash
    set -euo pipefail
    _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
    mkdir -p "$_devenv_root_dir"
    _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
    _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
    printf '%s' "$_repo_root" > "$_devenv_root_file"
    exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c setup-secrets {{args}}

# Validate the full secrets workflow (non-interactive, uses test values)
validate-secrets:
    #!/usr/bin/env bash
    set -euo pipefail
    _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
    mkdir -p "$_devenv_root_dir"
    _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
    _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
    printf '%s' "$_repo_root" > "$_devenv_root_file"
    exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c scripts/validate-secrets-workflow.sh

# Replace the Nix-managed vendor symlink with a writable copy for local editing
vendor-unlock:
    #!/usr/bin/env bash
    set -euo pipefail
    link="control/agentctl/vendor/microsandbox-fork"
    if [ ! -L "$link" ]; then
        echo "error: $link is not a symlink (already unlocked or blocked)" >&2
        exit 1
    fi
    target=$(readlink -f "$link")
    rm "$link"
    cp -rL "$target" "$link"
    chmod -R u+w "$link"
    echo "Unlocked $link for editing"

# Remove local vendor edits; the Nix-managed symlink will be recreated by just shell
vendor-lock:
    #!/usr/bin/env bash
    set -euo pipefail
    link="control/agentctl/vendor/microsandbox-fork"
    if [ -L "$link" ]; then
        echo "$link is already a symlink" >&2
        exit 0
    fi
    if [ ! -d "$link" ]; then
        echo "error: $link does not exist" >&2
        exit 1
    fi
    rm -rf "$link"
    echo "Removed $link. Run 'just shell' to recreate the Nix-managed symlink."

# Collect nix store garbage and optimise (dedupe) the store. Run periodically
# to reclaim disk from old generations / orphaned paths. Anti-accumulation
# maintenance recipe — pairs with the CARGO_TARGET_DIR relocation and the
# build-output deletions to keep the workbench footprint bounded.
gc:
    nix-collect-garbage --delete-old
    nix store optimise

# Store audit: top-20 report + INFORMATIONAL local-copy scan (always exit 0).
# Reports the top-20 store paths by closure size, then warns (stderr, exit 0)
# when any attributable local path-style input copy
# (<hash>-{workestrate,personal,duelbits,nix-tooling}[-source]) exceeds
# 50 MB closure size — scripts/store-audit.py --warn-if-source-over. The old
# blocking *ai-workbench*-source gate is retired: the naming era is obsolete
# and git+file *-source copies are name-indistinguishable from legitimate
# github ones (nixpkgs) — see the store-audit.py docstring; the real defense
# is the github-input swap (host-pending) + the active `lint-nix`. Wired
# into `verify` as the FINAL step.
# Skips (exit 0) when nix or python3 is unavailable, or when `nix path-info`
# itself fails (daemon/DB errors degrade to a note) — the passive complement
# to the active `lint-nix`.
store-audit:
    #!/usr/bin/env bash
    # NOTE: deliberately NO `set -e` — daemon/DB/parse failures MUST NOT
    # fail the verify gate. `set -uo pipefail` catches unset-variable bugs
    # + surfaces pipe failures via `$?` without aborting. The python scan is
    # informational (always exits 0); only the SKIP branches above exit early.
    set -uo pipefail
    if ! command -v nix >/dev/null 2>&1; then
        echo "store-audit: SKIP (nix not on PATH — run on a nix-capable host for the audit)"
        exit 0
    fi
    if ! command -v python3 >/dev/null 2>&1; then
        echo "store-audit: SKIP (python3 not on PATH — non-blocking)"
        exit 0
    fi
    echo "=== store-audit: top-20 store paths by closure size ==="
    # Capture once; a daemon / DB failure degrades to an informational note
    # rather than aborting the recipe. The report + source-path scan logic
    # lives in scripts/store-audit.py (stdlib-only) — kept out of the
    # justfile because just's parser choked on the inline python.
    path_info=$(nix path-info --all --json --json-format 1 --closure-size 2>/dev/null || true)
    if [ -z "$path_info" ]; then
        echo "(nix path-info failed unexpectedly — non-blocking)"
        exit 0
    fi
    echo "$path_info" | python3 scripts/store-audit.py --warn-if-source-over 50

# Lint nix code for purity violations: --impure flags, builtins.getFlake
# with toString, bare builtins.path (no filter), cleanSourceWith without
# an exclusion list, repo-root path literals in nix code. Active
# enforcement of the anti-accumulation invariants — wired into `verify`.
lint-nix:
    ./scripts/check-nix-paths.sh

# tombi TOML gates: format + lint (+ schema validation for
# config.reference/workestrate.toml) via scripts/check-toml.sh.
# Wired into `verify`.
tombi-check:
    @just _tombi-check-inner
[private]
_tombi-check-inner:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "${WORKESTRATE_DEVSHELL:-}" ]; then
        if [ -n "${_WS_REENTERED:-}" ]; then echo "FATAL: devshell did not export WORKESTRATE_DEVSHELL; refusing re-exec loop" >&2; exit 1; fi
        export _WS_REENTERED=1
        # Pure-eval devenv root: override the flake's devenv-root placeholder
        # input with a file holding this worktree's abs path (see `shell`).
        _devenv_root_dir="$HOME/.cache/workestrate/devenv-root"
        mkdir -p "$_devenv_root_dir"
        _repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
        _devenv_root_file="$_devenv_root_dir/$(printf '%s' "$_repo_root" | sha256sum | cut -c1-12)"
        printf '%s' "$_repo_root" > "$_devenv_root_file"
        exec nix develop --override-input devenv-root "file+file://$_devenv_root_file" -c just _tombi-check-inner
    fi
    ./scripts/check-toml.sh
