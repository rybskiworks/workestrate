#!/usr/bin/env bash
# Start LiteLLM on 127.0.0.1:4000.
#
# In the production design, this would be `msb run ...` against a
# LiteLLM image with the config mounted read-only and a strict
# network policy. In this POC fallback, we run it as a regular
# process, but we deliberately pass ONLY the dummy key and the
# master key. The real key never appears in this process's env.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p var/log var/run

if [[ -f infra/litellm/.env ]]; then
  # Source .env into a clean shell with REAL_KEY filtered out.
  # We do this by reading the .env file and exporting only the
  # safe keys. This is the architectural assertion of the POC:
  # the LiteLLM process NEVER sees FAKE_PROVIDER_REAL_KEY.
  while IFS='=' read -r k v; do
    case "$k" in
      ''|\#*) continue ;;
    esac
    if [[ "$k" == "FAKE_PROVIDER_REAL_KEY" ]]; then
      # Deliberately do not export the real key.
      continue
    fi
    # Strip optional surrounding quotes.
    v="${v%\"}"; v="${v#\"}"
    v="${v%\'}"; v="${v#\'}"
    export "$k=$v"
  done < infra/litellm/.env
else
  echo "ERROR: infra/litellm/.env not found. Run scripts/setup-env.sh first." >&2
  exit 2
fi

# Sanity: FAKE_PROVIDER_REAL_KEY must NOT be in our environment here.
if env | grep -q "^FAKE_PROVIDER_REAL_KEY="; then
  echo "ERROR: FAKE_PROVIDER_REAL_KEY leaked into the litellm environment." >&2
  exit 3
fi

# Confirm the api_base points at the egress proxy, not the provider directly.
case "${FAKE_PROVIDER_API_BASE:-}" in
  *:8082/*) ;;  # OK, egress proxy
  *) echo "ERROR: FAKE_PROVIDER_API_BASE should target the egress proxy (:8082)." >&2
     echo "       Got: ${FAKE_PROVIDER_API_BASE:-<unset>}" >&2
     exit 3 ;;
esac

nohup /home/node/.local/bin/litellm \
  --config infra/litellm/config.yaml \
  --host 127.0.0.1 --port 4000 \
  > var/log/litellm.stdout.log 2>&1 &
echo $! > var/run/litellm.pid
echo "litellm pid=$(cat var/run/litellm.pid) port=4000 log=var/log/litellm.stdout.log"
