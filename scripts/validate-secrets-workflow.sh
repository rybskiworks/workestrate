#!/usr/bin/env bash
# validate-secrets-workflow.sh — validate ai-workbench sops/age secrets workflow end-to-end
#
# Purpose:
#   Test the full secrets lifecycle (init, decrypt, write, workestrate run) in an
#   isolated temp directory without mutating the user's real key or repo state.
#
# Exit codes:
#   0 — all checks passed
#   1 — one or more checks failed, or a wrapper invocation failed
#
# Usage:
#   just validate-secrets   (or: nix develop --override-input devenv-root "file+file://$HOME/.cache/workestrate/devenv-root/<sha256-of-worktree-path-12>" -c scripts/validate-secrets-workflow.sh)
#
# Note: This script must be run via `just validate-secrets` or with the devenv-root override.
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

# Copy nothing from the repo into the workdir: provisioning runs through the
# `workestrate secrets` CLI (sops/age are bundled in its nix wrapper). The
# tool repo root carries NO .sops.yaml/.env.example (those are config-repo
# artifacts the scaffold generates — the old cp of
# $REPO_ROOT/.sops.yaml/.env.example died under set -e), so render
# scaffold-true fixtures here instead.
mkdir -p "$WORKDIR"

# Generate fresh age key for this test
mkdir -p "$WORKDIR/keys"
chmod 700 "$WORKDIR/keys"
age-keygen -o "$WORKDIR/keys/age.txt" 2>/dev/null
chmod 600 "$WORKDIR/keys/age.txt"
TEST_PUBKEY="$(age-keygen -y "$WORKDIR/keys/age.txt")"
log "generated test age key: $TEST_PUBKEY"

# Render .sops.yaml from the scaffold template (single-recipient form —
# exactly what `workestrate config new` writes) with the test recipient.
sed -e "s|{{ config_name }}|validate|g" \
    -e "s|{{ age_recipient }}|$TEST_PUBKEY|g" \
    "$REPO_ROOT/control/agentctl/src/scaffold/template/.sops.yaml.tpl" > "$WORKDIR/.sops.yaml"
log "rendered .sops.yaml from scaffold template with test recipient"

# Fixture .env.example: same KEY= schema shape the scaffold generates,
# limited to the keys exercised below (`workestrate secrets` falls back to
# this file for required keys only if loading the config fails; with the
# fixture workestrate.toml exported below, the config path wins — the fixture
# keeps the workflow hermetic either way).
cat > "$WORKDIR/.env.example" <<'EOF'
# validate-secrets-workflow fixture (not a repo artifact).
GITHUB_TOKEN=
KIMI_CODE_API_KEY=
LITELLM_MASTER_KEY=
MINIMAX_CODING_API_KEY=
NEURALWATT_API_KEY=
ODYSSEUS_ADMIN_PASSWORD=
OPENROUTER_API_KEY=
EOF

# Fixture workestrate.toml (schema_version + secrets catalog — the shape the
# scaffold generates). Exported as WORKESTRATE_CONFIG_DIR below, this gives
# `workestrate secrets-schema` a deterministic key set AND makes Phase 6's
# `workestrate run -- env` decrypt THIS workdir's .env.enc (single dev layer,
# never the user's real home).
cat > "$WORKDIR/workestrate.toml" <<'EOF'
schema_version = 1

[secrets.GITHUB_TOKEN]
env_var = "GITHUB_TOKEN"
required = false

[secrets.KIMI_CODE_API_KEY]
env_var = "KIMI_CODE_API_KEY"
required = false

[secrets.LITELLM_MASTER_KEY]
env_var = "LITELLM_MASTER_KEY"
required = false

[secrets.MINIMAX_CODING_API_KEY]
env_var = "MINIMAX_CODING_API_KEY"
required = false

[secrets.NEURALWATT_API_KEY]
env_var = "NEURALWATT_API_KEY"
required = false

[secrets.ODYSSEUS_ADMIN_PASSWORD]
env_var = "ODYSSEUS_ADMIN_PASSWORD"
required = false

[secrets.OPENROUTER_API_KEY]
env_var = "OPENROUTER_API_KEY"
required = false
EOF

# Export env for init
export SOPS_AGE_KEY_FILE="$WORKDIR/keys/age.txt"
export SOPS_CONFIG="./.sops.yaml"
export LITELLM_MASTER_KEY="$INIT_LITELLM"
export OPENROUTER_API_KEY="$INIT_OPENROUTER"
export KIMI_CODE_API_KEY="$INIT_KIMI"
export NEURALWATT_API_KEY="$INIT_NEURALWATT"
export MINIMAX_CODING_API_KEY="$INIT_MINIMAX"
export GITHUB_TOKEN="$INIT_GITHUB"
export ODYSSEUS_ADMIN_PASSWORD="$INIT_ODYSSEUS"
export WORKESTRATE_CONFIG_DIR="$WORKDIR"
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

# Helper to run nix develop -c from repo root but execute in workdir. The
# devenv-root override (worktree abs path in a root file) is required: bare
# `nix develop` cannot resolve devenv.root under pure eval.
run_in_workdir() {
  local cmd
  printf -v cmd 'cd %q && %s' "$WORKDIR" "$1"
  local root_dir="$HOME/.cache/workestrate/devenv-root"
  mkdir -p "$root_dir"
  local root_file="$root_dir/$(printf '%s' "$REPO_ROOT" | sha256sum | cut -c1-12)"
  printf '%s' "$REPO_ROOT" > "$root_file"
  (cd "$REPO_ROOT" && nix develop --override-input devenv-root "file+file://$root_file" -c bash -c "$cmd")
}

# PHASE 1: init
# `workestrate secrets init` targets WORKESTRATE_CONFIG_DIR ($WORKDIR),
# where the fixture .sops.yaml/.env.example/workestrate.toml live.
log "=== PHASE 1: workestrate secrets init ==="
phase="init"
out="$WORKDIR/phase-${phase}.out"
err="$WORKDIR/phase-${phase}.err"
if run_in_workdir "workestrate secrets init" >"$out" 2>"$err"; then
  pass "workestrate secrets init succeeded"
else
  fail "workestrate secrets init failed (phase=$phase)"
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
log "=== PHASE 3: workestrate secrets update (LITELLM_MASTER_KEY env var) ==="
# Set LITELLM_MASTER_KEY to the new value. `workestrate secrets update`
# detects schema keys set non-empty in the process env and replaces exactly
# those keys, keeping all others unchanged.
export LITELLM_MASTER_KEY="$UPDATE_LITELLM"
phase="update"
out="$WORKDIR/phase-${phase}.out"
err="$WORKDIR/phase-${phase}.err"
if run_in_workdir "workestrate secrets update" >"$out" 2>"$err"; then
  pass "workestrate secrets update succeeded"
else
  fail "workestrate secrets update failed (phase=$phase)"
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

# PHASE 6: workestrate run -- env
# workestrate run decrypts .env.enc internally and loads it into the child
# process environment (decrypt+source workaround, since
# `sops exec-env` has no --input-type flag and mis-detects dotenv as JSON).
log "=== PHASE 6: workestrate run -- env ==="
phase="workestrate-run-env"
out="$WORKDIR/phase-${phase}.out"
err="$WORKDIR/phase-${phase}.err"
if run_in_workdir "workestrate run -- env" >"$out" 2>"$err"; then
  pass "workestrate run -- env succeeded"
else
  fail "workestrate run -- env failed (phase=$phase)"
  echo "stdout:" >&2
  cat "$out" | redact >&2
  echo "stderr:" >&2
  cat "$err" | redact >&2
fi
assert_contains "$(cat "$out")" "LITELLM_MASTER_KEY=$UPDATE_LITELLM" "workestrate run: LITELLM_MASTER_KEY injected"
assert_contains "$(cat "$out")" "OPENROUTER_API_KEY=$INIT_OPENROUTER" "workestrate run: OPENROUTER_API_KEY injected"
assert_contains "$(cat "$out")" "KIMI_CODE_API_KEY=$INIT_KIMI" "workestrate run: KIMI_CODE_API_KEY injected"
assert_contains "$(cat "$out")" "NEURALWATT_API_KEY=$INIT_NEURALWATT" "workestrate run: NEURALWATT_API_KEY injected"
assert_contains "$(cat "$out")" "MINIMAX_CODING_API_KEY=$INIT_MINIMAX" "workestrate run: MINIMAX_CODING_API_KEY injected"
assert_contains "$(cat "$out")" "GITHUB_TOKEN=$INIT_GITHUB" "workestrate run: GITHUB_TOKEN injected"
assert_contains "$(cat "$out")" "ODYSSEUS_ADMIN_PASSWORD=$INIT_ODYSSEUS" "workestrate run: ODYSSEUS_ADMIN_PASSWORD injected"

log "=== PHASE 7: workestrate --help (proves CLI loads) ==="
# agentctl rejects a `--` separator and has no `env` subcommand, so we use
# `--help` (a real, harmless agentctl flag) to prove the CLI loads.
# env-injection is already covered by Phase 6 (workestrate run -- env), which uses
# the same decrypt+source code path.
phase="workestrate-help"
out="$WORKDIR/phase-${phase}.out"
err="$WORKDIR/phase-${phase}.err"
if run_in_workdir "workestrate --help" >"$out" 2>"$err"; then
  pass "workestrate --help succeeded"
else
  fail "workestrate --help failed (phase=$phase)"
  echo "stdout:" >&2
  cat "$out" | redact >&2
  echo "stderr:" >&2
  cat "$err" | redact >&2
fi
# Sanity: the workestrate help text appears, proving the CLI was invoked.
assert_contains "$(cat "$out")" "workestrate" "workestrate: CLI help text present"

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
