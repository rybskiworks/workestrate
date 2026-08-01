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

# Validate LiteLLM config.yaml against the schema indexes.
# Tries direct python3 first (fast path, works inside `nix develop` or any env
# where PyYAML is installed). Falls back to `nix develop -c python3` when bare
# python3 lacks PyYAML (e.g., a minimal container outside the devshell). Errors
# with actionable guidance when neither path is available. Closes the
# "PyYAML is required but not installed" failure mode reported by independent
# verification of `just verify` in a non-devshell container.
litellm-check:
    #!/usr/bin/env bash
    set -euo pipefail
    script=".agents/skills/validation-litellm-config-check/scripts/check_config.py"
    args=(--config config.reference/infra/litellm/config.yaml --schemas-dir docs/litellm/schemas --mode in-memory)
    if python3 -c 'import yaml' >/dev/null 2>&1; then
        python3 "$script" "${args[@]}"
    elif command -v nix >/dev/null 2>&1; then
        # Slow path: borrow PyYAML from the nix devshell.
        nix develop -c python3 "$script" "${args[@]}"
    else
        echo "ERROR: PyYAML is not available via python3 and 'nix' is not on PATH" >&2
        echo "       to fall back to 'nix develop -c python3'." >&2
        echo "Install PyYAML (pip install pyyaml) or run inside 'nix develop'." >&2
        exit 1
    fi

# Parse every fenced toml block in docs/migration/20-target-system-spec.md
# against the ConfigFile schema shape (WP4 / D1 standing guard).
spec-examples:
    cargo test --manifest-path control/agentctl/Cargo.toml --test spec_examples_parse

# Full pre-merge validation: format, lint, compile-check, test, spec-examples,
# config validation, golden-check, schema drift, lock-file stability, AND
# nix-purity lint.
verify: toolchain-check check test spec-examples litellm-check tombi-check golden-check schema-check scaffold-check lint-nix store-audit
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
    @echo "schema written to schemas/workestrate.schema.json"

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
    link="control/agentctl/vendor/microsandbox-filesystem-0.5.6"
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
    link="control/agentctl/vendor/microsandbox-filesystem-0.5.6"
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

# Build pi into agents/pi/build with native npm (hashless local dev loop).
# workestrate falls back to agents/pi/build when WORKESTRATE_PI_BUILD is unset.
# Requires the dev shell's npm/node (run inside `nix develop`).
dev-build-pi:
    rm -rf agents/pi/build
    cp -r agents/pi/repo agents/pi/build
    chmod -R u+w agents/pi/build
    cd agents/pi/build && NODE_ENV=development npm ci --ignore-scripts && npm run build

# Run pi from the local agents/pi/build (overrides the canonical bun path).
# Overrides the dev-shell's exported WORKESTRATE_PI_BUILD for this one command,
# pointing at the local agents/pi/build populated by `just dev-build-pi` — no
# manual export/unset needed.
dev-run-pi *args:
    WORKESTRATE_PI_BUILD=agents/pi/build workestrate pi {{args}}

# Build and load ALL nix-built workload images into microsandbox.
# Driven by the `workload-images` attrset in flake.nix — adding an image
# there = one entry; this recipe picks it up automatically. No per-image recipes.
load-images:
    nix develop -c load-images

# Refresh all fixed-output derivation (FOD) dependency hashes for the agent
# recipes. Run this whenever the agent source inputs change (flake.lock bumps
# to tempest/opencode/odysseus) or after editing per-recipe lock/requirements
# files.
#
# Each agent recipe uses lib.fakeHash as a placeholder until the real hash is
# computed on a nix-capable host (the sandbox cannot reach the network for hash
# computation; see HOST-GATE comments in nix/packages/{tempest,opencode,odysseus}.nix).
#
# Workflow:
#   1. Run this recipe (it issues three nix commands and prints results).
#   2. For each hash output, inline the sha256-... value into the matching file:
#        tempest:   nix/packages/tempest.nix   (npmDepsHash)
#        opencode:  nix/packages/opencode.nix  (bunDeps.outputHash)
#        odysseus:  nix/packages/odysseus.nix  (pipDeps.outputHash)
#   3. Re-run `nix build .#tempest .#opencode-built .#odysseus-built` to confirm.
update-hashes:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== tempest: prefetch-npm-deps (buildNpmPackage internal FOD) ==="
    nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json
    echo ""
    echo "=== opencode-built: bunDeps FOD (first build fails with lib.fakeHash) ==="
    echo "(copy the 'got:' sha256-... value into nix/packages/opencode.nix bunDeps.outputHash)"
    nix build .#opencode-built --no-link 2>&1 | grep -E 'got:|specified:|error: hash' || true
    echo ""
    echo "=== odysseus-built: pipDeps FOD (first build fails with lib.fakeHash) ==="
    echo "(copy the 'got:' sha256-... value into nix/packages/odysseus.nix pipDeps.outputHash)"
    nix build .#odysseus-built --no-link 2>&1 | grep -E 'got:|specified:|error: hash' || true
    echo ""
    echo "Done. Inline each 'got:' sha256-... value into the matching nix/packages/*.nix"
    echo "file (see HOST-GATE comments), then run:"
    echo "  nix build .#tempest .#opencode-built .#odysseus-built"


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

# Periodic host/CI check: measures /nix/store growth from one pure eval
# (nix eval .#packages.x86_64-linux.pi-image.drvPath). Asserts <50M new
# source paths (exit 1 when the byte delta exceeds 50_000_000). Non-blocking
# style consistent with store-audit: skips with exit 0 when nix is absent.
# Run periodically on a nix-capable host or in CI to catch source-closure
# regressions early. NOT wired into `verify` (it requires nix + is slow).
store-delta-check:
    #!/usr/bin/env bash
    # NOTE: deliberately NO `set -e` — non-blocking style: skips with exit 0
    # when nix is absent; exits 1 only when nix is present AND the measured
    # store delta exceeds 50 MiB. `set -uo pipefail` catches unset-variable
    # bugs + surfaces pipe failures via `$?` without aborting.
    set -uo pipefail
    if ! command -v nix >/dev/null 2>&1; then
        echo "store-delta-check: SKIP (nix not on PATH — run on a nix-capable host)"
        exit 0
    fi
    before=$(du -sb /nix/store 2>/dev/null | awk '{print $1}')
    if [ -z "${before:-}" ]; then
        echo "store-delta-check: SKIP (could not measure /nix/store before eval)"
        exit 0
    fi
    echo "store-delta-check: /nix/store before = ${before} bytes"
    # Pure eval of the pi-image drvPath (git-filtered .# ref — must not copy
    # the raw working tree). Failure to eval is non-blocking here; the byte
    # delta is the assertion.
    if ! nix eval .#packages.x86_64-linux.pi-image.drvPath >/dev/null 2>&1; then
        echo "store-delta-check: SKIP (nix eval of pi-image drvPath failed — run on a nix-capable host)"
        exit 0
    fi
    after=$(du -sb /nix/store 2>/dev/null | awk '{print $1}')
    if [ -z "${after:-}" ]; then
        echo "store-delta-check: SKIP (could not measure /nix/store after eval)"
        exit 0
    fi
    echo "store-delta-check: /nix/store after  = ${after} bytes"
    delta=$((after - before))
    echo "store-delta-check: delta = ${delta} bytes"
    threshold=50000000
    if [ "$delta" -gt "$threshold" ]; then
        echo "FAIL: store-delta-check: /nix/store grew by ${delta} bytes (> ${threshold}) from one pure eval — unbounded source copy regression" >&2
        exit 1
    fi
    echo "OK: store-delta-check: delta ${delta} bytes within ${threshold}-byte budget"


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
