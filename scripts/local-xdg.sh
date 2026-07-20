#!/usr/bin/env bash
# Source this to set XDG env vars for repo-local workestrate state.
# Usage: source scripts/local-xdg.sh
#
# The SOPS age private key is intentionally NOT in the bundle — it stays at
# ~/.config/sops/age/ai-workbench-secrets.txt on the host, outside the repo
# (agent-reachable via ${CWD} mounts). workestrate secret operations therefore
# run on the host and fail closed in the container.
export XDG_CONFIG_HOME="$PWD/.workestrate/config"
export XDG_DATA_HOME="$PWD/.workestrate/data"
export XDG_STATE_HOME="$PWD/.workestrate/state"
echo "workestrate XDG env set to $PWD/.workestrate/ (age key stays at host path)"
