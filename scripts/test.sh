#!/usr/bin/env bash
# End-to-end test of the ai-workestrator POC.
#
# This wraps scripts/run-tests.sh and adds the simple curl probes from
# the task spec (LiteLLM /v1/models and a chat completion through LiteLLM)
# plus a tail of the fake-provider log.
#
# Exits 0 on success, 1 on any failure.
set -euo pipefail
cd "$(dirname "$0")/.."

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
blue()  { printf '\033[34m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n' "$*"; }

# Load .env so we know which keys to expect.
if [[ -f infra/litellm/.env ]]; then
  set -a
  # shellcheck disable=SC1091
  source infra/litellm/.env
  set +a
fi

LITELLM_PORT="${LITELLM_HOST_PORT:-4000}"
MASTER="${LITELLM_MASTER_KEY:-}"
if [[ -z "$MASTER" ]]; then
  red "ERROR: LITELLM_MASTER_KEY not set in .env"
  exit 2
fi

FAILED=0
pass() { green "  PASS: $*"; }
fail() { red   "  FAIL: $*"; FAILED=1; }

bold "===== Test 1: LiteLLM reachable on :$LITELLM_PORT ====="
code=$(curl -s -o /tmp/ll_models.json -w "%{http_code}" \
  "http://127.0.0.1:${LITELLM_PORT}/v1/models" \
  -H "Authorization: Bearer ${MASTER}" || true)
if [[ "$code" == "200" ]] && grep -qE '"object"[: ]+"list"' /tmp/ll_models.json; then
  pass "LiteLLM /v1/models returned 200 with object=list"
else
  fail "LiteLLM /v1/models returned HTTP $code"
fi

bold "===== Test 2: chat completion through LiteLLM ====="
code=$(curl -s -o /tmp/ll_chat.json -w "%{http_code}" -X POST \
  "http://127.0.0.1:${LITELLM_PORT}/v1/chat/completions" \
  -H "Authorization: Bearer ${MASTER}" \
  -H "Content-Type: application/json" \
  -d '{"model":"fake-gpt","messages":[{"role":"user","content":"hello"}]}' || true)
if [[ "$code" == "200" ]] && grep -qE '"object"[: ]+"chat.completion"' /tmp/ll_chat.json; then
  pass "LiteLLM /v1/chat/completions returned a chat.completion"
  bold "  Response excerpt:"
  head -c 280 /tmp/ll_chat.json; echo
else
  fail "LiteLLM /v1/chat/completions returned HTTP $code"
  head -c 300 /tmp/ll_chat.json; echo
fi

bold "===== Test 3: fake-provider log (recent) ====="
if [[ -f var/log/fake-provider.log ]]; then
  tail -n 20 var/log/fake-provider.log
else
  red "  fake-provider log not found"
  FAILED=1
fi

echo
if (( FAILED == 0 )); then
  green "===== ALL BASIC TESTS PASSED ====="
  bold "Run scripts/run-tests.sh for the full validation suite (incl. secret-injection proof)."
  exit 0
else
  red "===== SOME TESTS FAILED ====="
  exit 1
fi
