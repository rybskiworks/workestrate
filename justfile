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

# Full pre-merge validation: format, lint, compile-check, test, spec-examples, config validation, golden-check, and lock-file stability
verify: check test spec-examples litellm-check golden-check
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

# Set up and verify repo-local XDG state
local-setup:
    @echo "Setting up repo-local XDG state..."
    @source scripts/local-xdg.sh
    @workestrate check

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
