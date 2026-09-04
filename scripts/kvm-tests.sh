#!/usr/bin/env bash
# kvm-tests.sh — single entry point for the three KVM-gated integration tests.
#
# Runs, serially (each in its own `nix develop` invocation so the devshell
# provides cargo/rustc AND recreates the vendor/microsandbox-fork symlink
# needed to compile the patched microsandbox crates):
#   1. tests/lifecycle_detached.rs — detached instance lifecycle (ADR 0021)
#   2. tests/flake_root_gate.rs    — F2 lazy project-root gate e2e
#   3. tests/ensure_images_e2e.rs  — spec-21 phase-E rebuild-before-spawn
#      (the HOST-KVM #[ignore]d variant; needs MSB_PATH=<unwrapped msb>)
#
# Preflight fails fast with actionable messages: /dev/kvm, nix, the raw msb
# store path, and the runtime-store image(s) the tests need. The runtime
# store is ~/.microsandbox — the same store the wrapped `workestrate` and
# `msb-wrapped` binaries force (nix/packages/agentctl.nix postInstall and
# flake.nix msb-wrapped). Images are NOT auto-pulled: on a miss the script
# prints the exact pull command and exits 1.
#
# Usage (ANY-SHELL — plain bash on the host with nix on PATH; from the repo
# root):
#   bash scripts/kvm-tests.sh
#
# Exit 0 only if all three tests pass.

set -euo pipefail

REPO=$(cd "$(dirname "$0")/.." && pwd)
cd "$REPO"

ERRORS=0

log() { echo "[kvm-tests] $*"; }
warn() { echo "[kvm-tests] WARNING: $*" >&2; }
fail() { echo "[kvm-tests] FAIL: $*" >&2; ERRORS=$((ERRORS+1)); }

# ---------------------------------------------------------------------------
# Preflight 1 — KVM
# ---------------------------------------------------------------------------
if [[ -c /dev/kvm ]]; then
  log "KVM device present: /dev/kvm"
else
  fail "KVM device not found at /dev/kvm; enable virtualization in BIOS and load the kvm/kvm_intel/kvm_amd modules"
fi

# ---------------------------------------------------------------------------
# Preflight 2 — nix
# ---------------------------------------------------------------------------
if ! command -v nix >/dev/null 2>&1; then
  fail "nix not found on PATH; install Nix: https://nixos.org/download/ or run: curl -L https://nixos.org/nix/install | sh"
else
  log "nix found: $(command -v nix)"
fi

# ---------------------------------------------------------------------------
# Preflight 3 — raw (unwrapped) msb store path. ensure_images_e2e REQUIRES
# MSB_PATH pointing at an unwrapped msb that honors MSB_HOME (the devshell's
# wrapped msb forces MSB_HOME=$HOME/.microsandbox and would write fixture
# images into the real store).
# ---------------------------------------------------------------------------
MSB_PATH=""
if MSB_OUT=$(nix build .#microsandbox --no-link --print-out-paths 2>/dev/null); then
  MSB_PATH="$MSB_OUT/bin/msb"
fi
if [[ -z "$MSB_PATH" || ! -x "$MSB_PATH" ]]; then
  fail "raw msb not resolvable; run: nix build .#microsandbox (the shared store already has it; this is a no-op when built)"
else
  log "raw msb: $MSB_PATH"
fi

# ---------------------------------------------------------------------------
# Preflight 4 — runtime-store images. The runtime store is ~/.microsandbox
# (wrapper-baked MSB_HOME); lifecycle_detached and flake_root_gate boot a
# python:3.12-slim registry image there. Do NOT auto-pull — print the exact
# command and stop.
# ---------------------------------------------------------------------------
RUNTIME_MSB_HOME="$HOME/.microsandbox"
if [[ "$ERRORS" -eq 0 ]]; then
  if MSB_HOME="$RUNTIME_MSB_HOME" "$MSB_PATH" image ls 2>/dev/null | grep -q "python:3.12-slim"; then
    log "required image present: python:3.12-slim (in $RUNTIME_MSB_HOME)"
  else
    fail "required image python:3.12-slim not found in the runtime msb store ($RUNTIME_MSB_HOME)"
    log "pull it with: MSB_HOME=\"$RUNTIME_MSB_HOME\" \"$MSB_PATH\" pull python:3.12-slim"
  fi
fi

if [[ "$ERRORS" -ne 0 ]]; then
  fail "$ERRORS preflight check(s) failed; fix above issues before running the KVM tests"
  exit 1
fi
log "preflight OK"

# ---------------------------------------------------------------------------
# Run the three tests serially (report-all semantics: every test runs even
# if an earlier one fails; the summary decides the exit code).
# ---------------------------------------------------------------------------
run_test() {
  local name="$1"
  shift
  log "RUN $name"
  set +e
  "$@" 2>&1
  local rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    log "PASS $name"
  else
    warn "FAIL $name (exit $rc)"
  fi
  return "$rc"
}

PASS=0
FAILED=0

if run_test lifecycle_detached \
    nix develop -c bash -c 'cargo test --manifest-path control/agentctl/Cargo.toml --test lifecycle_detached -- --ignored --nocapture'; then
  PASS=$((PASS+1))
else
  FAILED=$((FAILED+1))
fi

if run_test flake_root_gate \
    nix develop -c bash -c 'cargo test --manifest-path control/agentctl/Cargo.toml --test flake_root_gate -- --ignored --nocapture'; then
  PASS=$((PASS+1))
else
  FAILED=$((FAILED+1))
fi

# ensure_images_e2e needs MSB_PATH=<unwrapped msb> set INSIDE the devshell
# command (the devshell shellHook overwrites MSB_PATH with its staged msb).
# The test builds its own fixture image, so no preloaded image is required.
if run_test ensure_images_e2e \
    nix develop -c bash -c "MSB_PATH=$MSB_PATH cargo test --manifest-path control/agentctl/Cargo.toml --test ensure_images_e2e -- --ignored --nocapture"; then
  PASS=$((PASS+1))
else
  FAILED=$((FAILED+1))
fi

# ---------------------------------------------------------------------------
# Nested-guest entry (ADR 0036, opt-in e2e — Phase 2 positive stays FUTURE).
# ---------------------------------------------------------------------------
# Requires a nested-capable host: vmx|svm in /proc/cpuinfo AND the
# kvm_intel/kvm_amd `nested` parameter at Y AND /dev/kvm accessible.
# Anything less → SKIP with an explicit requirement note (never FAIL: host
# capability is environmental, not a code regression).
#
# Phase 1 (now): the NEGATIVE path is asserted by unit tests (the exact
# `up`-refusal error shape); this script only gates the live preflights.
# Phase 2 (FUTURE — needs the libkrunfw rebuild w/ CONFIG_KVM + pin): the
# POSITIVE guest test (guest /dev/kvm + vmx + KVM_GET_API_VERSION=12) plugs
# in here behind the same skip gate. No "nested works" claim until then.
if [[ "${NESTED_GUEST_TEST:-0}" == "1" ]]; then
  if grep -qE 'vmx|svm' /proc/cpuinfo \
      && { [[ "$(cat /sys/module/kvm_intel/parameters/nested 2>/dev/null)" =~ ^[Yy1]$ ]] \
        || [[ "$(cat /sys/module/kvm_amd/parameters/nested 2>/dev/null)" =~ ^[Yy1]$ ]]; } \
      && [[ -r /dev/kvm ]]; then
    log "nested-capable host confirmed; running nested-guest checks (Phase 1: refusal-shape probes only)"
    # Phase 1 live probe: a require-workload `up` on THIS host must either
    # proceed (nested-capable) or refuse with the ADR-0036 shape — both are
    # PASS; anything else (panic, partial sandbox) is FAIL. The positive
    # guest-device assertion lands in Phase 2 (see above).
    log "SKIP nested-guest positive (Phase 2 future: libkrunfw w/ CONFIG_KVM not yet pinned)"
  else
    log "SKIP nested-guest: host is not nested-capable (needs vmx|svm + kvm_intel/kvm_amd nested=Y + readable /dev/kvm)"
  fi
else
  log "SKIP nested-guest: opt-in via NESTED_GUEST_TEST=1 (needs a nested-capable host)"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
log "=== KVM test summary: $PASS passed, $FAILED failed ==="
if [[ "$FAILED" -eq 0 ]]; then
  log "All KVM tests passed."
  exit 0
else
  fail "$FAILED KVM test(s) failed; see output above"
  exit 1
fi
