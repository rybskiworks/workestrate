#!/usr/bin/env bash
# Source this to point workestrate at the repo-local single tool home (ADR 0023).
# Usage: source scripts/local-xdg.sh
#
# The SOPS age private key is intentionally NOT in the bundle — it stays at
# ~/.config/sops/age/ai-workbench-secrets.txt on the host, outside the repo
# (agent-reachable via ${CWD} mounts). workestrate secret operations therefore
# run on the host and fail closed in the container.
export WORKESTRATE_HOME="$PWD/.workestrate"
echo "workestrate WORKESTRATE_HOME set to $PWD/.workestrate/ (age key stays at host path)"
