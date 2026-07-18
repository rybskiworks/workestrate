#!/usr/bin/env bash
set -euo pipefail

# Migrate workestrate XDG state from container $HOME into repo-local .workestrate/
# Run this from the repo root: bash scripts/migrate-xdg-to-repo.sh

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/.workestrate"

echo "Migrating workestrate XDG state to $TARGET/"

# Create directory structure
mkdir -p "$TARGET/config/workestrate"
mkdir -p "$TARGET/config/sops/age"
mkdir -p "$TARGET/data/workestrate/repos"
mkdir -p "$TARGET/data/workestrate/sources"
mkdir -p "$TARGET/state/workestrate/workspaces"
mkdir -p "$TARGET/state/workestrate/var"

# Migrate personal config repo
OLD_PERSONAL="$HOME/.local/share/workestrate/repos/personal"
NEW_PERSONAL="$TARGET/data/workestrate/repos/personal"
if [ -d "$OLD_PERSONAL" ]; then
    echo "  Moving personal config repo..."
    if [ -d "$NEW_PERSONAL" ]; then
        rm -rf "$NEW_PERSONAL"
    fi
    mv "$OLD_PERSONAL" "$NEW_PERSONAL"
    # Remove leftover test-agent if present
    rm -rf "$NEW_PERSONAL/agents/test-agent"
    # Remove test-agent entry from workestrate.toml if present
    python3 -c "
p = '$NEW_PERSONAL/workestrate.toml'
with open(p, 'r') as f:
    content = f.read()
idx = content.find('\n[workloads.test-agent]')
if idx >= 0:
    content = content[:idx] + '\n'
    with open(p, 'w') as f:
        f.write(content)
    print('  Removed test-agent entry from workestrate.toml')
" 2>/dev/null || true
else
    echo "  No personal config repo found at $OLD_PERSONAL"
fi

# Migrate registry
OLD_REGISTRY="$HOME/.config/workestrate/config.toml"
NEW_REGISTRY="$TARGET/config/workestrate/config.toml"
if [ -f "$OLD_REGISTRY" ]; then
    echo "  Moving registry..."
    mv "$OLD_REGISTRY" "$NEW_REGISTRY"
else
    echo "  No registry found at $OLD_REGISTRY"
fi

# Migrate sops age key
OLD_KEY="$HOME/.config/sops/age/ai-workbench-secrets.txt"
NEW_KEY="$TARGET/config/sops/age/ai-workbench-secrets.txt"
if [ -f "$OLD_KEY" ]; then
    echo "  Moving SOPS age key..."
    mv "$OLD_KEY" "$NEW_KEY"
    chmod 600 "$NEW_KEY"
else
    echo "  No SOPS age key found at $OLD_KEY (secrets may not be decryptable until you create one)"
fi

# Migrate state dir
OLD_STATE="$HOME/.local/state/workestrate"
if [ -d "$OLD_STATE" ] && [ -n "$(ls -A "$OLD_STATE" 2>/dev/null)" ]; then
    echo "  Moving state dir..."
    cp -r "$OLD_STATE"/* "$TARGET/state/workestrate/" 2>/dev/null || true
fi

# Clean up old dirs
echo "  Cleaning up old XDG dirs..."
rm -rf "$HOME/.local/share/workestrate"
rm -rf "$HOME/.config/workestrate"
rm -rf "$HOME/.local/state/workestrate"
rmdir "$HOME/.config/sops/age" 2>/dev/null || true
rmdir "$HOME/.config/sops" 2>/dev/null || true

# Update registry paths to point at new locations
if [ -f "$NEW_REGISTRY" ]; then
    echo "  Updating registry paths..."
    # Get the existing rev
    REV=$(grep '^rev' "$NEW_REGISTRY" | head -1 | sed 's/.*= *"\(.*\)"/\1/' || echo "")
    cat > "$NEW_REGISTRY" << REGEOF
[settings]
default_context = "personal"
store_dir = "$TARGET/data"
state_dir = "$TARGET/state"

[configs.personal]
url = "$NEW_PERSONAL"
ref = "main"
rev = "$REV"

layers = ["personal"]

[[trusted_projects]]
path = "$REPO_ROOT"
REGEOF
    echo "  Registry updated."
fi

echo ""
echo "Migration complete. The .workestrate/ directory is gitignored."
echo "To activate: source scripts/local-xdg.sh"
echo ""
echo "Tree:"
find "$TARGET" -not -path '*/.git/*' -type f | sort | head -20
