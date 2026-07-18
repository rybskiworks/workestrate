#!/usr/bin/env bash
# Source this to set XDG env vars for repo-local workestrate state.
# Usage: source scripts/local-xdg.sh
#
# The .workestrate/ directory contains the age private key — NEVER commit it.
export XDG_CONFIG_HOME="$PWD/.workestrate/config"
export XDG_DATA_HOME="$PWD/.workestrate/data"
export XDG_STATE_HOME="$PWD/.workestrate/state"
export SOPS_AGE_KEY_FILE="$PWD/.workestrate/config/sops/age/ai-workbench-secrets.txt"
echo "workestrate XDG env set to $PWD/.workestrate/"
