#!/usr/bin/env bash
# Validate the POC end-to-end.
#
# Steps
# -----
#   1. fake-provider is up and serves /v1/models and /v1/chat/completions
#   2. egress-proxy is up and forwards allowed requests
#   3. egress-proxy denies requests to a non-allowlisted host
#   4. egress-proxy INJECTS the real key when the dummy key is presented
#   5. LiteLLM is up on :4000 and proxies /v1/models and /v1/chat/completions
#   6. The fake-provider's log shows the REAL key, not the dummy
#   7. The LiteLLM process environment does NOT contain the real key
#
# Exits non-zero on the first failure.
set -euo pipefail
cd "$(dirname "$0")/.."

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
blue()  { printf '\033[34m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n' "$*"; }

# Load env so the test script knows the dummy and real keys.
if [[ -f infra/litellm/.env ]]; then
  set -a
  # shellcheck disable=SC1091
  source infra/litellm/.env
  set +a
fi

DUMMY="${FAKE_PROVIDER_DUMMY_KEY:-dummy-key-injected-at-egress}"
REAL="${FAKE_PROVIDER_REAL_KEY:-}"
MASTER="${LITELLM_MASTER_KEY:-}"

if [[ -z "$REAL" || -z "$MASTER" ]]; then
  red "ERROR: infra/litellm/.env is missing keys. Run scripts/setup-env.sh first."
  exit 2
fi

FAILED=0
pass() { green "  PASS: $*"; }
fail() { red   "  FAIL: $*"; FAILED=1; }

# Wait helper: poll an HTTP endpoint until 200 or timeout.
wait_for() {
  local url="$1" timeout="${2:-30}"
  local deadline=$((SECONDS + timeout))
  while (( SECONDS < deadline )); do
    if curl -sf -o /dev/null "$url"; then
      return 0
    fi
    sleep 0.5
  done
  return 1
}

bold "===== 1. fake-provider :8081 ====="
if wait_for "http://127.0.0.1:8081/healthz" 30; then
  pass "fake-provider /healthz OK"
else
  fail "fake-provider /healthz unreachable on :8081"
fi
models=$(curl -sf http://127.0.0.1:8081/v1/models) && pass "fake-provider /v1/models OK" || fail "fake-provider /v1/models failed"
echo "  $models" | head -c 200; echo

bold "===== 2. egress-proxy :8082 (forwarding to 127.0.0.1:8081) ====="
# Use a X-Egress-Target to point the proxy at the allowlisted fake-provider.
resp=$(curl -sf -X POST http://127.0.0.1:8082/v1/chat/completions \
  -H "Authorization: Bearer ${DUMMY}" \
  -H "Content-Type: application/json" \
  -H "X-Egress-Target: 127.0.0.1:8081" \
  -d '{"model":"fake-gpt-4o","messages":[{"role":"user","content":"hello"}]}' 2>&1) || {
  fail "egress-proxy POST /v1/chat/completions failed"
  echo "$resp" | head -c 400; echo
  resp=""
}
if [[ -n "$resp" ]]; then
  if echo "$resp" | grep -qE '"object"[: ]+"chat\.completion"'; then
    pass "egress-proxy forwarded and got a chat.completion"
  else
    fail "egress-proxy returned unexpected payload: $(echo "$resp" | head -c 200)"
  fi
fi

bold "===== 3. egress-proxy :8082 (DENY non-allowlisted host) ====="
deny_resp=$(mktemp)
code=$(curl -s -o "$deny_resp" -w "%{http_code}" -X POST \
  "http://127.0.0.1:8082/v1/chat/completions" \
  -H "Authorization: Bearer ${DUMMY}" \
  -H "Content-Type: application/json" \
  -H "X-Egress-Target: 169.254.169.254:80" \
  -d '{"model":"x","messages":[]}' 2>/dev/null) || true
if [[ "$code" == "403" ]]; then
  pass "egress-proxy DENIED 169.254.169.254 with 403"
  cat "$deny_resp"; echo
else
  fail "egress-proxy did not deny 169.254.169.254 (got HTTP $code)"
  cat "$deny_resp"; echo
fi
# Also try to deny 8.8.8.8 (a public IP) - the proxy has no route but
# the allowlist check should reject before the connect attempt.
code2=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
  "http://127.0.0.1:8082/v1/chat/completions" \
  -H "Authorization: Bearer ${DUMMY}" \
  -H "X-Egress-Target: 8.8.8.8:53" \
  -d '{}' 2>/dev/null) || true
if [[ "$code2" == "403" ]]; then
  pass "egress-proxy DENIED 8.8.8.8:53 with 403"
else
  fail "egress-proxy did not deny 8.8.8.8:53 (got HTTP $code2)"
fi
rm -f "$deny_resp"

bold "===== 4. secret injection proof ====="
# Trigger a chat through the proxy using the dummy key, then check
# the fake-provider's log: the Authorization header it received
# must contain the REAL key, not the dummy.
curl -sf -X POST http://127.0.0.1:8082/v1/chat/completions \
  -H "Authorization: Bearer ${DUMMY}" \
  -H "Content-Type: application/json" \
  -H "X-Egress-Target: 127.0.0.1:8081" \
  -d '{"model":"fake-gpt-4o","messages":[{"role":"user","content":"secret-injection-marker"}]}' \
  >/dev/null
sleep 0.3
fplog="var/log/fake-provider.log"
if [[ ! -f "$fplog" ]]; then
  fail "fake-provider log not found at $fplog"
else
  # The log has two lines per request:
  #   line 1: [timestamp] METHOD path Authorization='Bearer ...'
  #   line 2:   body: {...}
  # The marker text is in line 2 (body), but we need to inspect line 1
  # (the Authorization header). Walk the log and pair them.
  if [[ -f "$fplog" ]] && grep -qF "secret-injection-marker" "$fplog"; then
    # Use python to pair body-lines with their preceding request-lines.
    evidence=$(python3 scripts/_find_evidence.py "$fplog" 2>/dev/null || true)
    if [[ -n "$evidence" ]]; then
      if echo "$evidence" | grep -F "Bearer ${REAL}" >/dev/null; then
        pass "fake-provider received REAL key in Authorization header"
        bold "  Evidence (fake-provider.log):"
        echo "    $evidence"
      elif echo "$evidence" | grep -F "Bearer ${DUMMY}" >/dev/null; then
        fail "fake-provider received DUMMY key (secret injection FAILED): $evidence"
      else
        fail "Authorization header in fake-provider log is unexpected: $evidence"
      fi
    else
      fail "no Authorization line paired with secret-injection-marker body"
    fi
  else
    fail "no 'secret-injection-marker' entry in fake-provider log"
  fi
fi

bold "===== 5. LiteLLM :4000 ====="
if wait_for "http://127.0.0.1:4000/health/liveliness" 60; then
  pass "litellm /health/liveliness OK"
else
  fail "litellm /health/liveliness unreachable on :4000"
fi
if curl -sf -H "Authorization: Bearer ${MASTER}" \
     "http://127.0.0.1:4000/v1/models" >/dev/null; then
  pass "litellm /v1/models with master key OK"
else
  fail "litellm /v1/models with master key FAILED"
fi
# Reject without auth.
noauth_code=$(curl -s -o /dev/null -w "%{http_code}" \
  "http://127.0.0.1:4000/v1/models" 2>/dev/null || true)
if [[ "$noauth_code" == "401" || "$noauth_code" == "403" ]]; then
  pass "litellm /v1/models rejects unauthenticated requests ($noauth_code)"
else
  fail "litellm /v1/models did not reject unauthenticated (got $noauth_code)"
fi

bold "===== 6. end-to-end chat through LiteLLM ====="
chat_resp=$(curl -sf -X POST "http://127.0.0.1:4000/v1/chat/completions" \
  -H "Authorization: Bearer ${MASTER}" \
  -H "Content-Type: application/json" \
  -d '{"model":"fake-gpt","messages":[{"role":"user","content":"end-to-end-test"}]}') || {
  fail "litellm /v1/chat/completions failed"; chat_resp=""; }
if [[ -n "$chat_resp" ]] && echo "$chat_resp" | grep -qE '"object"[: ]+"chat\.completion"'; then
  pass "litellm returned a chat.completion"
  bold "  Response excerpt:"
  echo "$chat_resp" | head -c 300; echo
else
  fail "litellm chat response unexpected: ${chat_resp:0:200}"
fi

bold "===== 7. isolation proof: real key NOT in litellm process env ====="
lp=$(cat var/run/litellm.pid 2>/dev/null || echo "")
if [[ -n "$lp" ]] && kill -0 "$lp" 2>/dev/null; then
  if grep -az "FAKE_PROVIDER_REAL_KEY" "/proc/$lp/environ" 2>/dev/null; then
    fail "litellm process environment contains FAKE_PROVIDER_REAL_KEY"
  else
    pass "litellm process environment does NOT contain FAKE_PROVIDER_REAL_KEY"
  fi
  # Also check for the actual real key value.
  if [[ -n "$REAL" ]] && grep -azF "$REAL" "/proc/$lp/environ" 2>/dev/null; then
    fail "litellm process environment contains the real key value"
  else
    pass "litellm process environment does NOT contain the real key value"
  fi
else
  fail "litellm pid not running; cannot inspect /proc"
fi

echo
if (( FAILED == 0 )); then
  green "===== ALL CHECKS PASSED ====="
  exit 0
else
  red "===== SOME CHECKS FAILED ====="
  exit 1
fi
