#!/usr/bin/env bash
# validate-secrets-workflow.sh — validate ai-workbench sops/age secrets workflow end-to-end
#
# Purpose:
#   Test the full secrets lifecycle (init, decrypt, write, run-with-secrets) in an
#   isolated temp directory without mutating the user's real key or repo state.
#
# Exit codes:
#   0 — all checks passed
#   1 — one or more checks failed, or a wrapper invocation failed
#
# Usage:
#   nix develop -c scripts/validate-secrets-workflow.sh
#
# Note: This script must be run from inside the dev shell (or via nix develop -c).
#       All work happens in an isolated temp directory; no repo files are modified.

set -euo pipefail
IFS=$'\n\t'

# Locate repo root (parent of scripts/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Create isolated temp directory
WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/ai-workbench-secrets-validate-XXXXXX")"
trap 'cleanup' EXIT

PASS=0
FAIL=0

cleanup() {
  local rc=$?
  chmod -R u+w "$WORKDIR" 2>/dev/null || true
  rm -rf "$WORKDIR" 2>/dev/null || true
  exit $rc
}

log() { printf "[validate-secrets] %s\n" "$*" >&2; }
pass() { log "[ OK ] $*"; ((PASS++)) || true; }
fail() { log "[FAIL] $*"; ((FAIL++)) || true; }

assert_eq() {
  local actual="$1" expected="$2" label="$3"
  local redacted_actual redacted_expected
  redacted_actual="$(printf '%s' "$actual" | redact)"
  redacted_expected="$(printf '%s' "$expected" | redact)"
  if [ "$actual" = "$expected" ]; then
    pass "$label"
  else
    local trunc="${redacted_actual:0:200}"
    fail "$label (expected: '$redacted_expected', got: '$trunc')"
  fi
}

assert_contains() {
  local haystack="$1" needle="$2" label="$3"
  if [[ "$haystack" == *"$needle"* ]]; then
    pass "$label"
  else
    local redacted_needle
    redacted_needle="$(printf '%s' "$needle" | redact)"
    fail "$label (missing: '$redacted_needle')"
  fi
}

assert_file_exists() {
  local path="$1" label="$2"
  if [ -f "$path" ]; then
    pass "$label"
  else
    fail "$label (file not found: $path)"
  fi
}

assert_perms() {
  local path="$1" expected="$2" label="$3"
  local actual
  actual="$(stat -c '%a' "$path" 2>/dev/null || echo "unknown")"
  if [ "$actual" = "$expected" ]; then
    pass "$label"
  else
    fail "$label (expected perms $expected, got $actual)"
  fi
}

# Test values
INIT_LITELLM="sk-test-initial-master-key-1234567890"
INIT_OPENROUTER="or-test-initial-1234567890"
INIT_KIMI="sk-test-initial-kimi-1234567890"
INIT_NEURALWATT="nw-test-initial-1234567890"
INIT_MINIMAX="mm-test-initial-1234567890"
INIT_GITHUB="ghp_test-initial-1234567890"
INIT_ODYSSEUS="od-test-initial-1234567890"

UPDATE_LITELLM="sk-test-updated-master-key-CHANGED-xyz"

log "working directory: $WORKDIR"
log "repo root: $REPO_ROOT"

# Copy .sops.yaml and .env.example to workdir so setup-secrets.sh can find
# them via its repo-root detection (.sops.yaml marker). Copy the script
# too so its BASH_SOURCE points at a path under $WORKDIR (this is the
# path the script is invoked with in real use, not the nix-store copy).
mkdir -p "$WORKDIR/scripts"
cp "$REPO_ROOT/.sops.yaml" "$WORKDIR/.sops.yaml"
cp "$REPO_ROOT/.env.example" "$WORKDIR/.env.example"
cp "$REPO_ROOT/scripts/setup-secrets.sh" "$WORKDIR/scripts/setup-secrets.sh"
chmod +x "$WORKDIR/scripts/setup-secrets.sh"

# Generate fresh age key for this test
mkdir -p "$WORKDIR/keys"
chmod 700 "$WORKDIR/keys"
age-keygen -o "$WORKDIR/keys/age.txt" 2>/dev/null
chmod 600 "$WORKDIR/keys/age.txt"
TEST_PUBKEY="$(age-keygen -y "$WORKDIR/keys/age.txt")"
log "generated test age key: $TEST_PUBKEY"

# Rewrite .sops.yaml to use the test public key
sed -i "s|&repo_secrets_v1 .*|\&repo_secrets_v1 $TEST_PUBKEY|" "$WORKDIR/.sops.yaml"

log "rewrote .sops.yaml with test recipient"

# Export env for init
export SOPS_AGE_KEY_FILE="$WORKDIR/keys/age.txt"
export SOPS_CONFIG="./.sops.yaml"
export SECRET_FILE=".env.enc"
export LITELLM_MASTER_KEY="$INIT_LITELLM"
export OPENROUTER_API_KEY="$INIT_OPENROUTER"
export KIMI_CODE_API_KEY="$INIT_KIMI"
export NEURALWATT_API_KEY="$INIT_NEURALWATT"
export MINIMAX_CODING_API_KEY="$INIT_MINIMAX"
export GITHUB_TOKEN="$INIT_GITHUB"
export ODYSSEUS_ADMIN_PASSWORD="$INIT_ODYSSEUS"
unset HISTFILE

redact() {
  local line
  while IFS= read -r line; do
    for val in "$INIT_LITELLM" "$INIT_OPENROUTER" "$INIT_KIMI" "$INIT_NEURALWATT" "$INIT_MINIMAX" "$INIT_GITHUB" "$INIT_ODYSSEUS" "$UPDATE_LITELLM"; do
      line="${line//"$val"/[REDACTED]}"
    done
    printf '%s\n' "$line"
  done
}

# Helper to run nix develop -c from repo root but execute in workdir
run_in_workdir() {
  local cmd
  printf -v cmd 'cd %q && %s' "$WORKDIR" "$1"
  (cd "$REPO_ROOT" && nix develop -c bash -c "$cmd")
}

# PHASE 1: init
# Invoke the workdir copy of setup-secrets.sh directly (not the nix wrapper)
# so BASH_SOURCE points at $WORKDIR/scripts/setup-secrets.sh and the script's
# repo-root detection (BASH_SOURCE/..) lands in $WORKDIR, where we put
# .sops.yaml and .env.example.
log "=== PHASE 1: setup-secrets init ==="
phase="init"
out="$WORKDIR/phase-${phase}.out"
err="$WORKDIR/phase-${phase}.err"
if run_in_workdir "./scripts/setup-secrets.sh init" >"$out" 2>"$err"; then
  pass "setup-secrets init succeeded"
else
  fail "setup-secrets init failed (phase=$phase)"
  echo "stdout:" >&2
  cat "$out" | redact >&2
  echo "stderr:" >&2
  cat "$err" | redact >&2
fi

assert_file_exists "$WORKDIR/.env.enc" "init: .env.enc created"

# PHASE 2: decrypt after init
log "=== PHASE 2: decrypt-env after init ==="
DECRYPT_INIT="$(run_in_workdir "decrypt-env" 2>&1)" || true

assert_contains "$DECRYPT_INIT" "LITELLM_MASTER_KEY=$INIT_LITELLM" "init decrypt: LITELLM_MASTER_KEY"
assert_contains "$DECRYPT_INIT" "OPENROUTER_API_KEY=$INIT_OPENROUTER" "init decrypt: OPENROUTER_API_KEY"
assert_contains "$DECRYPT_INIT" "KIMI_CODE_API_KEY=$INIT_KIMI" "init decrypt: KIMI_CODE_API_KEY"
assert_contains "$DECRYPT_INIT" "NEURALWATT_API_KEY=$INIT_NEURALWATT" "init decrypt: NEURALWATT_API_KEY"
assert_contains "$DECRYPT_INIT" "MINIMAX_CODING_API_KEY=$INIT_MINIMAX" "init decrypt: MINIMAX_CODING_API_KEY"
assert_contains "$DECRYPT_INIT" "GITHUB_TOKEN=$INIT_GITHUB" "init decrypt: GITHUB_TOKEN"
assert_contains "$DECRYPT_INIT" "ODYSSEUS_ADMIN_PASSWORD=$INIT_ODYSSEUS" "init decrypt: ODYSSEUS_ADMIN_PASSWORD"

# PHASE 3: update with LITELLM_MASTER_KEY env var path
log "=== PHASE 3: setup-secrets update (LITELLM_MASTER_KEY env var) ==="
# Set LITELLM_MASTER_KEY to the new value. setup-secrets.sh detects this
# and replaces just that key, keeping all others unchanged.
export LITELLM_MASTER_KEY="$UPDATE_LITELLM"
phase="update"
out="$WORKDIR/phase-${phase}.out"
err="$WORKDIR/phase-${phase}.err"
if run_in_workdir "./scripts/setup-secrets.sh update" >"$out" 2>"$err"; then
  pass "setup-secrets update succeeded"
else
  fail "setup-secrets update failed (phase=$phase)"
  echo "stdout:" >&2
  cat "$out" | redact >&2
  echo "stderr:" >&2
  cat "$err" | redact >&2
fi

# PHASE 4: decrypt after update
log "=== PHASE 4: decrypt-env after update ==="
DECRYPT_UPDATE="$(run_in_workdir "decrypt-env" 2>&1)" || true

assert_contains "$DECRYPT_UPDATE" "LITELLM_MASTER_KEY=$UPDATE_LITELLM" "update decrypt: LITELLM_MASTER_KEY changed"
assert_contains "$DECRYPT_UPDATE" "OPENROUTER_API_KEY=$INIT_OPENROUTER" "update decrypt: OPENROUTER_API_KEY unchanged"
assert_contains "$DECRYPT_UPDATE" "KIMI_CODE_API_KEY=$INIT_KIMI" "update decrypt: KIMI_CODE_API_KEY unchanged"
assert_contains "$DECRYPT_UPDATE" "NEURALWATT_API_KEY=$INIT_NEURALWATT" "update decrypt: NEURALWATT_API_KEY unchanged"
assert_contains "$DECRYPT_UPDATE" "MINIMAX_CODING_API_KEY=$INIT_MINIMAX" "update decrypt: MINIMAX_CODING_API_KEY unchanged"
assert_contains "$DECRYPT_UPDATE" "GITHUB_TOKEN=$INIT_GITHUB" "update decrypt: GITHUB_TOKEN unchanged"
assert_contains "$DECRYPT_UPDATE" "ODYSSEUS_ADMIN_PASSWORD=$INIT_ODYSSEUS" "update decrypt: ODYSSEUS_ADMIN_PASSWORD unchanged"

# PHASE 5: write-env
log "=== PHASE 5: write-env ==="
phase="write-env"
out="$WORKDIR/phase-${phase}.out"
err="$WORKDIR/phase-${phase}.err"
if run_in_workdir "write-env" >"$out" 2>"$err"; then
  pass "write-env succeeded"
else
  fail "write-env failed (phase=$phase)"
  echo "stdout:" >&2
  cat "$out" | redact >&2
  echo "stderr:" >&2
  cat "$err" | redact >&2
fi

assert_file_exists "$WORKDIR/.env" "write-env: .env created"
assert_perms "$WORKDIR/.env" "600" "write-env: .env has correct permissions"

ENV_CONTENT="$(cat "$WORKDIR/.env")"
assert_contains "$ENV_CONTENT" "LITELLM_MASTER_KEY=$UPDATE_LITELLM" "write-env content: LITELLM_MASTER_KEY"
assert_contains "$ENV_CONTENT" "OPENROUTER_API_KEY=$INIT_OPENROUTER" "write-env content: OPENROUTER_API_KEY"
assert_contains "$ENV_CONTENT" "KIMI_CODE_API_KEY=$INIT_KIMI" "write-env content: KIMI_CODE_API_KEY"
assert_contains "$ENV_CONTENT" "NEURALWATT_API_KEY=$INIT_NEURALWATT" "write-env content: NEURALWATT_API_KEY"
assert_contains "$ENV_CONTENT" "MINIMAX_CODING_API_KEY=$INIT_MINIMAX" "write-env content: MINIMAX_CODING_API_KEY"
assert_contains "$ENV_CONTENT" "GITHUB_TOKEN=$INIT_GITHUB" "write-env content: GITHUB_TOKEN"
assert_contains "$ENV_CONTENT" "ODYSSEUS_ADMIN_PASSWORD=$INIT_ODYSSEUS" "write-env content: ODYSSEUS_ADMIN_PASSWORD"

# Clean up .env after verification
rm -f "$WORKDIR/.env"
pass "write-env: .env cleaned up"

# PHASE 6: with-secrets / run-with-secrets
# These wrappers source the decrypted dotenv (decrypt+source workaround, since
# `sops exec-env` has no --input-type flag and mis-detects dotenv as JSON).
log "=== PHASE 6: with-secrets -- env ==="
phase="with-secrets-env"
out="$WORKDIR/phase-${phase}.out"
err="$WORKDIR/phase-${phase}.err"
if run_in_workdir "with-secrets env" >"$out" 2>"$err"; then
  pass "with-secrets env succeeded"
else
  fail "with-secrets env failed (phase=$phase)"
  echo "stdout:" >&2
  cat "$out" | redact >&2
  echo "stderr:" >&2
  cat "$err" | redact >&2
fi
assert_contains "$(cat "$out")" "LITELLM_MASTER_KEY=$UPDATE_LITELLM" "with-secrets: LITELLM_MASTER_KEY injected"
assert_contains "$(cat "$out")" "OPENROUTER_API_KEY=$INIT_OPENROUTER" "with-secrets: OPENROUTER_API_KEY injected"
assert_contains "$(cat "$out")" "KIMI_CODE_API_KEY=$INIT_KIMI" "with-secrets: KIMI_CODE_API_KEY injected"
assert_contains "$(cat "$out")" "NEURALWATT_API_KEY=$INIT_NEURALWATT" "with-secrets: NEURALWATT_API_KEY injected"
assert_contains "$(cat "$out")" "MINIMAX_CODING_API_KEY=$INIT_MINIMAX" "with-secrets: MINIMAX_CODING_API_KEY injected"
assert_contains "$(cat "$out")" "GITHUB_TOKEN=$INIT_GITHUB" "with-secrets: GITHUB_TOKEN injected"
assert_contains "$(cat "$out")" "ODYSSEUS_ADMIN_PASSWORD=$INIT_ODYSSEUS" "with-secrets: ODYSSEUS_ADMIN_PASSWORD injected"

log "=== PHASE 7: run-with-secrets --help (proves decrypt does not fail) ==="
# agentctl rejects a `--` separator and has no `env` subcommand, so we use
# `--help` (a real, harmless agentctl flag) to prove the decrypt step works.
# env-injection is already covered by Phase 6 (with-secrets env), which uses
# the same decrypt+source code path.
phase="run-with-secrets-help"
out="$WORKDIR/phase-${phase}.out"
err="$WORKDIR/phase-${phase}.err"
if run_in_workdir "run-with-secrets --help" >"$out" 2>"$err"; then
  pass "run-with-secrets --help succeeded (decrypt + agentctl exec)"
else
  fail "run-with-secrets --help failed (phase=$phase)"
  echo "stdout:" >&2
  cat "$out" | redact >&2
  echo "stderr:" >&2
  cat "$err" | redact >&2
fi
# Sanity: the agentctl help text appears, proving agentctl was actually exec'd.
assert_contains "$(cat "$out")" "agentctl <COMMAND>" "run-with-secrets: agentctl was exec'd (help text present)"

# SUMMARY
log "=== SUMMARY ==="
TOTAL=$((PASS + FAIL))
if [ "$FAIL" -eq 0 ]; then
  log "passed $TOTAL/$TOTAL checks"
  exit 0
else
  log "FAILED: $FAIL/$TOTAL checks"
  exit 1
fi
