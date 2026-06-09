#!/usr/bin/env bash
# Print the current status of the ai-workestrator POC.
set -euo pipefail
cd "$(dirname "$0")/.."

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
blue()  { printf '\033[34m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n' "$*"; }

check_endpoint() {
  local name="$1" url="$2" want_auth_header="${3:-}"
  local code
  if [[ -n "$want_auth_header" ]]; then
    code=$(curl -s -o /dev/null -w "%{http_code}" -H "$want_auth_header" "$url" || echo "000")
  else
    code=$(curl -s -o /dev/null -w "%{http_code}" "$url" || echo "000")
  fi
  if [[ "$code" == "200" ]]; then
    green "  $name  UP  ($url -> $code)"
  else
    red   "  $name  DOWN ($url -> $code)"
  fi
}

# Load .env for the master key.
if [[ -f infra/litellm/.env ]]; then
  set -a
  # shellcheck disable=SC1091
  source infra/litellm/.env
  set +a
fi

LITELLM_PORT="${LITELLM_HOST_PORT:-4000}"
MASTER="${LITELLM_MASTER_KEY:-}"

bold "===== Process status ====="
for name in fake-provider egress-proxy litellm; do
  pidfile="var/run/${name}.pid"
  if [[ -f "$pidfile" ]]; then
    pid="$(cat "$pidfile")"
    if kill -0 "$pid" 2>/dev/null; then
      green "  $name  pid=$pid  RUNNING"
    else
      red   "  $name  pid=$pid  STALE (not running)"
    fi
  else
    blue  "  $name  no pidfile"
  fi
done

bold "===== Endpoint status ====="
check_endpoint "fake-provider :8081"  "http://127.0.0.1:8081/healthz"
# Egress-proxy only handles POST with X-Egress-Target. Use POST.
code=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
  "http://127.0.0.1:8082/v1/chat/completions" \
  -H "Authorization: Bearer dummy-key-injected-at-egress" \
  -H "Content-Type: application/json" \
  -H "X-Egress-Target: 127.0.0.1:8081" \
  -d '{"model":"x","messages":[]}' || echo "000")
if [[ "$code" == "200" ]]; then
  green "  egress-proxy  :8082  UP  (POST -> $code)"
else
  red   "  egress-proxy  :8082  DOWN (POST -> $code)"
fi
if [[ -n "$MASTER" ]]; then
  check_endpoint "LiteLLM       :$LITELLM_PORT" \
    "http://127.0.0.1:${LITELLM_PORT}/v1/models" \
    "Authorization: Bearer ${MASTER}"
else
  red "  LiteLLM :$LITELLM_PORT  unknown (LITELLM_MASTER_KEY not set)"
fi

bold "===== Most recent Authorization header (fake-provider log) ====="
if [[ -f var/log/fake-provider.log ]]; then
  last_auth=$(grep -E '^\[.*Authorization=' var/log/fake-provider.log | tail -1 || true)
  if [[ -n "$last_auth" ]]; then
    echo "  $last_auth"
  else
    blue "  (no requests with Authorization header logged yet)"
  fi
else
  red "  no fake-provider log"
fi

bold "===== Isolation check: real key in litellm process env ====="
if [[ -f var/run/litellm.pid ]]; then
  lp=$(cat var/run/litellm.pid)
  if kill -0 "$lp" 2>/dev/null; then
    if grep -az "FAKE_PROVIDER_REAL_KEY" "/proc/$lp/environ" 2>/dev/null >/dev/null; then
      red "  FAKE_PROVIDER_REAL_KEY FOUND in litellm environ (LEAK)"
    else
      green "  FAKE_PROVIDER_REAL_KEY NOT in litellm environ (isolation holds)"
    fi
  else
    blue "  litellm not running; cannot inspect /proc/$lp/environ"
  fi
else
  blue "  no litellm pidfile; cannot inspect environ"
fi
