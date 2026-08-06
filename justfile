# Relocate cargo's target dir out of the source tree (closes the
# nix-purity anti-accumulation finding for target/). Evaluated at just-parse
# time so the running user's $HOME / $XDG_CACHE_HOME are resolved. Recipes
# that invoke cargo inherit this env automatically.
export CARGO_TARGET_DIR := `echo "${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target"`

# Verify the host rustc major.minor matches the fenix-pinned toolchain.
# Parses the RUST_TOOLCHAIN_VERSION marker from flake.nix (not hardcoded
# here) so the check stays in sync with the flake input automatically.
# Wired into `verify` as the FIRST gate — a toolchain mismatch invalidates
# all downstream cargo results.
toolchain-check:
    #!/usr/bin/env bash
    set -euo pipefail
    expected=$(grep -oP 'RUST_TOOLCHAIN_VERSION = "\K[^"]+' flake.nix)
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
verify: toolchain-check check test spec-examples tombi-check golden-check schema-check scaffold-check lint-nix store-audit
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

# Regenerate the canonical JSON Schema for workestrate.toml from the
# schemars-derived ConfigFile. Writes to schemas/workestrate.schema.json.
# Run on a nix-capable host (the dev shell's RUSTFLAGS → libcap-ng OUT lib
# dir is required to build the aws-lc-rs / parking_lot_core native crates).
generate-schema:
    #!/usr/bin/env bash
    set -euo pipefail
    nix develop -c cargo run --manifest-path control/agentctl/Cargo.toml --quiet -- generate-schema --output schemas/workestrate.schema.json
    echo "schema written to schemas/workestrate.schema.json"

# CI drift guard for schemas/workestrate.schema.json (ADR 0021 §8).
# Invokes control/agentctl/tests/schema_drift.rs, which runs the built
# `workestrate generate-schema` and diffs against the committed file.
schema-check:
    cargo test --manifest-path control/agentctl/Cargo.toml --test schema_drift

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
