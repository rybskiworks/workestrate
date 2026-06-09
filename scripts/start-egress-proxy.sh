#!/usr/bin/env bash
# Start the egress proxy on 127.0.0.1:8082.
# The proxy holds the REAL key in its host-side environment only.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p var/log var/run

if [[ -f infra/litellm/.env ]]; then
  set -a
  # shellcheck disable=SC1091
  source infra/litellm/.env
  set +a
else
  echo "WARN: infra/litellm/.env not found. Run scripts/setup-env.sh first." >&2
fi

if [[ -z "${FAKE_PROVIDER_REAL_KEY:-}" ]]; then
  echo "ERROR: FAKE_PROVIDER_REAL_KEY is empty. The proxy has no key to inject." >&2
  exit 2
fi

# The proxy gets the REAL key from the env, NOT from any file the LiteLLM
# process can read. The .env is not mounted into the LiteLLM process.
nohup python3 infra/egress-proxy/server.py \
  > var/log/egress-proxy.stdout.log 2>&1 &
echo $! > var/run/egress-proxy.pid
echo "egress-proxy pid=$(cat var/run/egress-proxy.pid) port=8082 log=var/log/egress-proxy.log"
