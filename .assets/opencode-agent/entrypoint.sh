#!/bin/bash
# =============================================================================
# OpenCode Fractal Squad - Entrypoint Script (ACP/MCP Server Edition)
# =============================================================================

set -e

echo "--- FRACTAL SQUAD BOOT SEQUENCE ---"

# 1. Source Nix environment (if installed)
if [ -f "$HOME/.nix-profile/etc/profile.d/nix.sh" ]; then
    echo "[entrypoint] Sourcing Nix environment..."
    . "$HOME/.nix-profile/etc/profile.d/nix.sh"
fi

# 2. Point the app to your config via Environment Variable
export OPENCODE_CONFIG="/home/node/.config/opencode/opencode.jsonc"

# 3. Fix permissions on internal volumes
    echo "[entrypoint] Checking volume permissions..."
    for dir in "$HOME/.config/opencode/memory" "$HOME/outputs"; do
        if [ -d "$dir" ]; then
            chmod -R u+rwX "$dir" 2>/dev/null || true
        fi
    done
    
    # 4. Determine server mode
SERVER_MODE="${SERVER_MODE:-mcp}"  # Options: mcp, webui

echo "[entrypoint] Server Mode: $SERVER_MODE"
echo "    Config Path: $OPENCODE_CONFIG"
echo ""

if [ "$SERVER_MODE" = "mcp" ] || [ "$SERVER_MODE" = "acp" ]; then
    # Start MCP/ACP Server
    echo "[entrypoint] Starting OpenCode MCP/ACP Server..."
    echo "    Host: ${ACP_HOST:-0.0.0.0}"
    echo "    Port: ${ACP_PORT:-3000}"
    echo "    Transport: ${ACP_TRANSPORT:-http}"
    echo ""
    exec node /home/node/.config/opencode/mcp-server.js
else
    # Start Web UI (original behavior)
    echo "[entrypoint] Starting OpenCode Web UI..."
    echo ""
    exec opencode serve --hostname 0.0.0.0 --port 3000
fi
