# Relocate cargo's target dir out of the source tree (closes the
# nix-purity anti-accumulation finding for target/). Evaluated at just-parse
# time so the running user's $HOME / $XDG_CACHE_HOME are resolved. Recipes
# that invoke cargo inherit this env automatically.
export CARGO_TARGET_DIR := `echo "${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target"`

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
# Wired into `verify` as the SECOND gate (after lock-guard) — a toolchain
# mismatch invalidates all downstream cargo results.
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
        echo "Run 'nix develop' to enter the pinned toolchain shell." >&2
        exit 1
    fi
    echo "OK: rustc $actual matches pinned toolchain"

# Phase-0 observability pin check: the hand-maintained msb version pin
# ("0.6.16") and fork rev pin must agree across the nix packages,
# control/agentctl/Cargo.toml, control/agentctl/src/commands/versions.rs,
# flake.nix, and flake.lock. Bash+python3 only (no cargo/nix), runs in <1s.
# Wired into `verify` after toolchain-check.
versions-check:
    ./scripts/check-msb-versions.sh

check:
    cargo fmt --manifest-path control/agentctl/Cargo.toml -- --check
    cargo clippy --manifest-path control/agentctl/Cargo.toml --all-targets -- -D warnings
    cargo check --manifest-path control/agentctl/Cargo.toml

# Parse every fenced toml block in docs/migration/20-target-system-spec.md
# against the ConfigFile schema shape (WP4 / D1 standing guard).
spec-examples:
    cargo test --manifest-path control/agentctl/Cargo.toml --test spec_examples_parse

# Full pre-merge validation: format, lint, compile-check, test, spec-examples,
# golden-check, schema drift, lock-file stability, AND nix-purity lint.
verify: lock-guard toolchain-check versions-check check test spec-examples tombi-check golden-check schema-check schema-sync-check scaffold-check lint-nix store-audit
    git diff --exit-code HEAD -- control/agentctl/Cargo.lock

# Heaviest validation: verify plus Nix build
verify-full: verify
    nix build .#workestrate

# Generate golden plan files for all workloads
golden-generate:
    @for name in example-service example-agent example-offensive; do \
        WORKESTRATE_CONFIG_DIR=config.reference cargo run --manifest-path control/agentctl/Cargo.toml -- $name plan \
          > control/agentctl/tests/golden/$name.plan.txt; \
    done

# Check golden plan parity
golden-check:
    @for name in example-service example-agent example-offensive; do \
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
    nix develop -c cargo run --manifest-path control/agentctl/Cargo.toml --quiet -- generate-schema --output schemas/workestrate.schema.json --output-workload schemas/workestrate-workload.schema.json --output-registry schemas/registry.schema.json
    echo "schema written to schemas/workestrate.schema.json"
    echo "workload schema written to schemas/workestrate-workload.schema.json"
    echo "registry schema written to schemas/registry.schema.json"

# CI drift guard for schemas/workestrate.schema.json,
# schemas/workestrate-workload.schema.json, and schemas/registry.schema.json
# (ADR 0021 §8). Invokes control/agentctl/tests/schema_drift.rs (all three
# artifacts) plus the workload-only control/agentctl/tests/schema_subschema_drift.rs.
schema-check:
    cargo test --manifest-path control/agentctl/Cargo.toml --test schema_drift
    cargo test --manifest-path control/agentctl/Cargo.toml --test schema_subschema_drift

# CI drift guard for the consumer schema copies (schemas/ at templates/,
# tool home, and registered config repos). Exits 1 when any consumer copy
# is stale or missing; run `workestrate schemas update` to refresh.
schema-sync-check:
    cargo run --manifest-path control/agentctl/Cargo.toml --quiet -- schemas update --check

# CI drift guard for the `workestrate config new` scaffold. Invokes
# control/agentctl/tests/scaffold_template.rs, which renders the embedded
# scaffold templates via the real binary, runs `validate-config` on the
# output, and (when `copier` is available) enforces byte-parity with the
# copier template's minimal-personal render + copier-update interop.
scaffold-check:
    cargo test --manifest-path control/agentctl/Cargo.toml --test scaffold_template

build:
    cargo build --release --manifest-path control/agentctl/Cargo.toml

fmt:
    cargo fmt --manifest-path control/agentctl/Cargo.toml

# Check formatting without modifying files
fmt-check:
    cargo fmt --manifest-path control/agentctl/Cargo.toml -- --check

# Run Clippy with -D warnings (standalone)
clippy:
    cargo clippy --manifest-path control/agentctl/Cargo.toml --all-targets -- -D warnings

# Run unit tests
test:
    cargo test --manifest-path control/agentctl/Cargo.toml

# Run the three ignored KVM tests (lifecycle_detached, flake_root_gate,
# ensure_images_e2e) via the encapsulated entry point. HOST-KVM only:
# preflights /dev/kvm + the runtime-store image, runs each test serially
# in its own devshell, and prints a PASS/FAIL summary. Any shell with nix.
kvm-tests:
    ./scripts/kvm-tests.sh

workestrate *args:
    cargo run --manifest-path control/agentctl/Cargo.toml -- {{args}}

plan:
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
    nix develop -c setup-secrets {{args}}

# Validate the full secrets workflow (non-interactive, uses test values)
validate-secrets:
    nix develop -c scripts/validate-secrets-workflow.sh

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

# Remove local vendor edits; the Nix-managed symlink will be recreated by nix develop
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
    echo "Removed $link. Run 'nix develop' to recreate the Nix-managed symlink."

# Collect nix store garbage and optimise (dedupe) the store. Run periodically
# to reclaim disk from old generations / orphaned paths. Anti-accumulation
# maintenance recipe — pairs with the CARGO_TARGET_DIR relocation and the
# build-output deletions to keep the workbench footprint bounded.
gc:
    nix-collect-garbage --delete-old
    nix store optimise

# Store audit: top-20 report (informational) + BLOCKING source-path gate.
# Reports the top-20 store paths by closure size, then fails (exit 1) if any
# *ai-workbench*-source path exceeds 50 MB closure size — the impure
# path-style copy probe, now enforced by scripts/store-audit.py
# --fail-if-source-over. Wired into `verify` as the FINAL step.
# Non-blocking only when nix or python3 is unavailable, or when
# `nix path-info` itself fails (daemon/DB errors degrade to a note, exit 0) —
# the gate fails ONLY on actual oversized source paths. This is the passive
# complement to the active `lint-nix`.
store-audit:
    #!/usr/bin/env bash
    # NOTE: deliberately NO `set -e` — daemon/DB/parse failures MUST NOT
    # fail the verify gate. `set -uo pipefail` catches unset-variable bugs
    # + surfaces pipe failures via `$?` without aborting. The python exit
    # code (1 on oversized source paths) is what propagates out of the recipe.
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
    # rather than aborting the recipe. The report + source-path gate logic
    # lives in scripts/store-audit.py (stdlib-only) — kept out of the
    # justfile because just's parser choked on the inline python.
    path_info=$(nix path-info --all --json 2>/dev/null || true)
    if [ -z "$path_info" ]; then
        echo "(nix path-info failed unexpectedly — non-blocking)"
        exit 0
    fi
    echo "$path_info" | python3 scripts/store-audit.py --fail-if-source-over 50

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
    ./scripts/check-toml.sh
