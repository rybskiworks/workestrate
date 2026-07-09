check:
    cargo fmt --manifest-path control/agentctl/Cargo.toml -- --check
    cargo clippy --manifest-path control/agentctl/Cargo.toml -- -D warnings
    cargo check --manifest-path control/agentctl/Cargo.toml

# Validate LiteLLM config.yaml against the schema indexes
litellm-check:
    python3 .agents/skills/validation-litellm-config-check/scripts/check_config.py \
      --config infra/litellm/config.yaml --schemas-dir docs/litellm/schemas --mode in-memory

# Full pre-merge validation: format, lint, compile-check, test, config validation, and lock-file stability
verify: check test litellm-check
    git diff --exit-code HEAD -- control/agentctl/Cargo.lock

# Heaviest validation: verify plus Nix build
verify-full: verify
    nix build .#workestrate

build:
    cargo build --release --manifest-path control/agentctl/Cargo.toml

fmt:
    cargo fmt --manifest-path control/agentctl/Cargo.toml

# Check formatting without modifying files
fmt-check:
    cargo fmt --manifest-path control/agentctl/Cargo.toml -- --check

# Run Clippy with -D warnings (standalone)
clippy:
    cargo clippy --manifest-path control/agentctl/Cargo.toml -- -D warnings

# Run unit tests
test:
    cargo test --manifest-path control/agentctl/Cargo.toml

workestrate *args:
    cargo run --manifest-path control/agentctl/Cargo.toml -- {{args}}

plan:
    cargo run --manifest-path control/agentctl/Cargo.toml -- litellm plan
    cargo run --manifest-path control/agentctl/Cargo.toml -- pi plan
    cargo run --manifest-path control/agentctl/Cargo.toml -- odysseus plan
    cargo run --manifest-path control/agentctl/Cargo.toml -- opencode plan
    cargo run --manifest-path control/agentctl/Cargo.toml -- tempest plan

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
