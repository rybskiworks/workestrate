#!/usr/bin/env bash
# Start the ai-workestrator POC.
#
# Brings up, in order:
#   1. fake-provider     (port 8081)  — the stand-in "real provider"
#   2. egress-proxy      (port 8082)  — host-side allowlist + secret injector
#   3. LiteLLM proxy     (port 4000)  — the isolated proxy that holds ONLY dummy keys
#
# The real provider key is read from .env (host-side only) and never enters
# the LiteLLM process. The egress proxy substitutes the real key for the
# dummy one at the network boundary.
set -euo pipefail
cd "$(dirname "$0")/.."

# Auto-generate .env from .env.example on first run.
if [[ ! -f infra/litellm/.env ]] && [[ -x scripts/setup-env.sh ]]; then
  echo "First run: generating .env from .env.example"
  bash scripts/setup-env.sh
fi

if [[ ! -f infra/litellm/.env ]]; then
  echo "ERROR: infra/litellm/.env missing and setup-env.sh did not produce one." >&2
  exit 2
fi

# Make sure no stale instance is running.
bash scripts/stop.sh >/dev/null 2>&1 || true

# Start fake-provider first; the others depend on it being up.
bash scripts/start-fake-provider.sh
# Start egress-proxy (it needs FAKE_PROVIDER_REAL_KEY from the host .env).
bash scripts/start-egress-proxy.sh
# Start LiteLLM last. start-litellm.sh deliberately strips FAKE_PROVIDER_REAL_KEY
# from its own environment before exec, then refuses to launch if the leak
# somehow happened anyway.
bash scripts/start-litellm.sh

# Give LiteLLM a moment to bind.
sleep 2

echo
echo "=== ai-workestrator POC started ==="
echo "  fake-provider  -> 127.0.0.1:8081  (logs: var/log/fake-provider.log)"
echo "  egress-proxy   -> 127.0.0.1:8082  (logs: var/log/egress-proxy.log)"
echo "  LiteLLM proxy  -> 127.0.0.1:4000  (logs: var/log/litellm.stdout.log)"
echo
echo "Run: bash scripts/test.sh"
