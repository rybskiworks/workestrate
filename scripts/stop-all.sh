#!/usr/bin/env bash
# Stop all POC processes started by the start-*.sh scripts.
set -euo pipefail
cd "$(dirname "$0")/.."

for name in fake-provider egress-proxy litellm; do
  pidfile="var/run/${name}.pid"
  if [[ -f "$pidfile" ]]; then
    pid="$(cat "$pidfile")"
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" || true
      echo "stopped $name (pid=$pid)"
    else
      echo "$name pid=$pid not running"
    fi
    rm -f "$pidfile"
  fi
done
