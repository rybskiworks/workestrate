#!/usr/bin/env bash
# Stop all ai-workestrator POC processes.
set -euo pipefail
cd "$(dirname "$0")/.."

stopped=0
for name in fake-provider egress-proxy litellm; do
  pidfile="var/run/${name}.pid"
  if [[ -f "$pidfile" ]]; then
    pid="$(cat "$pidfile")"
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" || true
      echo "stopped $name (pid=$pid)"
      stopped=$((stopped+1))
    fi
    rm -f "$pidfile"
  fi
done

if [[ $stopped -eq 0 ]]; then
  echo "no ai-workestrator processes were running"
fi
