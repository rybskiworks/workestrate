check:
    cargo fmt --manifest-path control/agentctl/Cargo.toml -- --check
    cargo clippy --manifest-path control/agentctl/Cargo.toml -- -D warnings
    cargo check --manifest-path control/agentctl/Cargo.toml

# Full pre-merge validation: format, lint, compile-check, test, and lock-file stability
verify: check test
    git diff --exit-code HEAD -- control/agentctl/Cargo.lock

# Heaviest validation: verify plus Nix build
verify-full: verify
    nix build .#agentctl

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

agentctl *args:
    cargo run --manifest-path control/agentctl/Cargo.toml -- {{args}}

plan:
    cargo run --manifest-path control/agentctl/Cargo.toml -- litellm plan
    cargo run --manifest-path control/agentctl/Cargo.toml -- agent plan pi
    cargo run --manifest-path control/agentctl/Cargo.toml -- agent plan odysseus

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
