#!/usr/bin/env bash
# host-provision.sh — in-flake host provisioner for the workestrate tool.
#
# This script is the ONLY supported install path for the nix-installed
# `workestrate` binary. Do not `nix profile install` by hand outside this
# script except via the explicit remediation commands it prints.
#
# One idempotent command that:
#   A. runs scripts/host-check.sh,
#   B. syncs the nix-profile-installed `workestrate` binary to the current
#      tree (the ONLY mutation: `nix profile install .#workestrate` when
#      stale, and only when not --check-only),
#   C. runs `workestrate doctor`,
#   D. prints a readiness table + verdict (exit 0 = READY, 1 = NOT READY).
#
# Idempotent and non-destructive: it never boots workloads, never writes
# outside the user's nix profile, and never mutates the repo tree.
#
# Intended for direct host use:
#   ./scripts/host-provision.sh                 # auto-fix (install when stale)
#   ./scripts/host-provision.sh --check-only    # report only, never install
#   ./scripts/host-provision.sh --force         # reinstall even when fresh
#   just host-provision
#
# This script does NOT boot workloads.

set -euo pipefail

ERRORS=0

log()  { echo "[host-provision] $*"; }
warn() { echo "[host-provision] WARNING: $*" >&2; }
fail() { echo "[host-provision] FAIL: $*" >&2; ERRORS=$((ERRORS+1)); }

REPO=$(cd "$(dirname "$0")/.." && pwd)
if [[ ! -f "$REPO/flake.nix" ]]; then
  echo "[host-provision] FAIL: flake.nix not found at $REPO — not a workestrate checkout" >&2
  exit 1
fi
cd "$REPO"

CHECK_ONLY=0
FORCE=0
for arg in "$@"; do
  case "$arg" in
    --check-only) CHECK_ONLY=1 ;;
    --force)      FORCE=1 ;;
    -h|--help)
      cat <<EOF
Usage: $0 [--check-only] [--force]
  --check-only  Report only; never install. Prints reinstall commands when stale.
  --force       Reinstall the binary even when fresh.
Default: auto-fix (install when stale).
EOF
      exit 0 ;;
    *)
      echo "[host-provision] unknown argument: $arg" >&2
      echo "Usage: $0 [--check-only] [--force]" >&2
      exit 2 ;;
  esac
done

# ---------------------------------------------------------------------------
# Step A — host-check.sh (capture exit, do not abort)
# ---------------------------------------------------------------------------
log "Step A: host-check"
set +e
host_out=$(./scripts/host-check.sh 2>&1)
host_check_exit=$?
set -e
echo "$host_out"
if [[ "$host_check_exit" -ne 0 ]]; then
  warn "host-check.sh exited non-zero ($host_check_exit)"
fi

# ---------------------------------------------------------------------------
# Step B — binary sync
# ---------------------------------------------------------------------------
log "Step B: binary sync"
binary_status="UNKNOWN"
want=""
got=""
installed_rev="UNKNOWN"

set +e
build_log=$(mktemp)
want=$(nix build .#workestrate --no-link --print-out-paths 2>"$build_log")
nix_build_exit=$?
set -e
if [[ "$nix_build_exit" -ne 0 ]]; then
  fail "nix build .#workestrate failed (exit $nix_build_exit): $(cat "$build_log")"
  want=""
else
  log "fresh store path: $want"
fi
rm -f "$build_log"

set +e
# Refresh the shell's command hash so `command -v` below cannot resolve a
# stale pre-install path (same refresh as after install in do_install).
hash -r 2>/dev/null || true
got=$(readlink -f "$(command -v workestrate 2>/dev/null)" 2>/dev/null)
got_exit=$?
set -e
if [[ "$got_exit" -ne 0 ]] || [[ -z "$got" ]]; then
  got=""
fi

fresh=0
if [[ -n "$want" ]] && [[ -n "$got" ]] && [[ "$got" = "$want/bin/workestrate" ]]; then
  fresh=1
fi

if [[ -n "$got" ]]; then
  log "installed binary resolves to: $got"
else
  log "no workestrate binary found on PATH"
fi

do_install() {
  # Remove + install the nix profile entry. Returns 0 on success.
  set +e
  nix profile remove workestrate >/dev/null 2>&1
  install_out=$(nix profile install .#workestrate 2>&1)
  install_exit=$?
  set -e
  if [[ "$install_exit" -ne 0 ]]; then
    fail "nix profile install .#workestrate failed (exit $install_exit): $install_out"
    return 1
  fi
  log "nix profile install .#workestrate OK"
  # Refresh the shell's command hash so the fresh binary resolves immediately.
  hash -r
  return 0
}

if [[ -z "$want" ]]; then
  binary_status="FAIL"
elif [[ "$fresh" -eq 1 ]] && [[ "$FORCE" -eq 0 ]]; then
  binary_status="OK"
  log "binary is fresh (matches $want)"
elif [[ "$CHECK_ONLY" -eq 1 ]]; then
  binary_status="STALE"
  warn "installed binary is stale (want=$want, got=${got:-<none>})"
  echo "[host-provision] reinstall on the host with:"
  echo "[host-provision]   nix profile remove workestrate 2>/dev/null || true"
  echo "[host-provision]   nix profile install .#workestrate"
else
  # auto-fix or --force: reinstall
  log "reinstalling (fresh=$fresh, force=$FORCE, check-only=$CHECK_ONLY)"
  if do_install; then
    set +e
    got=$(readlink -f "$(command -v workestrate 2>/dev/null)" 2>/dev/null)
    got_exit=$?
    set -e
    if [[ "$got_exit" -ne 0 ]] || [[ -z "$got" ]]; then got=""; fi
    if [[ -n "$got" ]] && [[ "$got" = "$want/bin/workestrate" ]]; then
      binary_status="OK"
      log "binary now fresh (matches $want)"
    else
      binary_status="FAIL"
      fail "binary still mismatched after install: want=$want got=${got:-<none>}"
    fi
  else
    binary_status="FAIL"
  fi
fi

# Best-effort rev from `workestrate --version` (after the main.rs change it
# prints `0.1.0-<rev>`; older binaries print `0.1.0`).
if [[ -n "$got" ]] && [[ -x "$got" ]]; then
  set +e
  ver_out=$("$got" --version 2>/dev/null | head -n1)
  set -e
  if [[ -n "$ver_out" ]] && [[ "$ver_out" == *-* ]]; then
    installed_rev="${ver_out##*-}"
  fi
fi

# ---------------------------------------------------------------------------
# Step B+ — provision-check asserts (P1/P2/P3). Read-only: they run in BOTH
# modes (including --check-only) and NEVER install anything — every FAIL
# names the exact remediation command instead.
# ---------------------------------------------------------------------------
log "Step B+: provision-check asserts (P1/P2/P3, read-only)"
p1_status="UNKNOWN"
p2_status="UNKNOWN"
p3_status="UNKNOWN"

# --- P1: profile singularity — exactly one `workestrate` entry in
# `nix profile list`. The output shape differs between nix 2.23.3 (multiline
# `Name:`/`Flake:` form, one `Name:` line per entry) and 2.35.1 (indexed
# one-line-per-entry form), so count entry-start lines per detected shape and
# assert the total is exactly 1.
set +e
profile_list_out=$(nix profile list 2>/dev/null)
profile_list_exit=$?
set -e
if [[ "$profile_list_exit" -ne 0 ]]; then
  p1_status="FAIL"
  fail "P1 singularity: \`nix profile list\` failed (exit $profile_list_exit) — cannot verify the profile entry. Remediation: on a nix-capable host run \`nix profile remove workestrate\` then \`nix profile install .#workestrate\` (or add --force to reinstall)"
else
  set +e
  if printf '%s\n' "$profile_list_out" | grep -qE '^Name: '; then
    # nix 2.23.3 multiline shape: one `Name:` line per entry.
    p1_count=$(printf '%s\n' "$profile_list_out" | grep -cE '^Name:.*workestrate' || true)
  else
    # nix 2.35.1 indexed shape: one line per entry.
    p1_count=$(printf '%s\n' "$profile_list_out" | grep -c 'workestrate' || true)
  fi
  set -e
  # grep -c prints 0 on no match; normalize a missing/empty capture to 0.
  p1_count="${p1_count:-0}"
  if [[ "$p1_count" -eq 1 ]]; then
    p1_status="OK"
    log "P1 singularity OK (exactly one workestrate entry in nix profile list)"
  else
    p1_status="FAIL"
    fail "P1 singularity: expected exactly 1 workestrate entry in \`nix profile list\`, got $p1_count. Remediation: \`nix profile remove workestrate\` then \`nix profile install .#workestrate\` (or \`nix profile install .#workestrate --force\`)"
  fi
fi

# --- P2: version identity — store-path want-vs-got (already computed as
# $want/$got) PLUS `workestrate --version` vs the git shortRev
# (inputs.self.shortRev semantics: `git rev-parse --short HEAD`, the
# WORKESTRATE_REV value baked at build). WARN (not FAIL) when the tree is
# dirty (`git status --porcelain` non-empty), since the baked rev then
# legitimately reads "dirty".
p2_fail=0
if [[ -z "$want" ]] || [[ -z "$got" ]]; then
  p2_fail=1
  fail "P2 version identity: cannot compare store paths (want=${want:-<none>} got=${got:-<none>}). Remediation: \`nix profile remove workestrate\` then \`nix profile install .#workestrate\`"
elif [[ "$got" != "$want/bin/workestrate" ]]; then
  p2_fail=1
  fail "P2 version identity: store-path mismatch (want=$want/bin/workestrate got=$got). Remediation: \`nix profile remove workestrate\` then \`nix profile install .#workestrate\`"
else
  log "P2 store-path identity OK ($got)"
fi
set +e
expected_rev=$(git rev-parse --short HEAD 2>/dev/null || echo "UNKNOWN")
tree_porcelain=$(git status --porcelain 2>/dev/null || true)
set -e
if [[ "$installed_rev" == "UNKNOWN" ]]; then
  p2_fail=1
  fail "P2 version identity: installed binary reports no baked rev (\`workestrate --version\` has no -<rev> suffix; git shortRev is $expected_rev). Remediation: \`nix profile remove workestrate\` then \`nix profile install .#workestrate\`"
elif [[ -n "$tree_porcelain" ]]; then
  warn "P2 version identity: tree is dirty — baked rev $installed_rev vs git shortRev $expected_rev is unreliable (WARN, not FAIL)"
  if [[ "$p2_fail" -eq 0 ]]; then p2_status="WARN"; fi
elif [[ "$installed_rev" != "$expected_rev" ]]; then
  p2_fail=1
  fail "P2 version identity: baked-rev mismatch (installed $installed_rev vs git shortRev $expected_rev). Remediation: \`nix profile remove workestrate\` then \`nix profile install .#workestrate\`"
else
  log "P2 baked-rev identity OK ($installed_rev)"
fi
if [[ "$p2_fail" -eq 1 ]]; then
  p2_status="FAIL"
elif [[ "$p2_status" != "WARN" ]]; then
  p2_status="OK"
fi

# --- P3: triple liveness — baked MSB_PATH live (the wrapper's --set MSB_PATH
# target from nix/packages/agentctl.nix exists + executable) + `msb --version`
# runs + `agentd` present (baked MSB_AGENTD_PATH path exists, or `agentd`
# resolvable on PATH). Each FAIL names the exact remediation.
p3_fail=0
# The nix wrapper (--set MSB_PATH/MSB_AGENTD_PATH in nix/packages/agentctl.nix)
# bakes absolute store paths into the installed $got script, either as
# `export MSB_PATH="..."` lines or bare store references — try the precise
# export form first, then fall back to the store-path shape.
set +e
baked_msb=$(grep -oE 'MSB_PATH="[^"]+"' "$got" 2>/dev/null | head -n1 | cut -d'"' -f2 || true)
if [[ -z "${baked_msb:-}" ]]; then
  baked_msb=$(grep -oE '/nix/store/[^" ]*bin/msb' "$got" 2>/dev/null | head -n1 || true)
fi
baked_agentd=$(grep -oE 'MSB_AGENTD_PATH="[^"]+"' "$got" 2>/dev/null | head -n1 | cut -d'"' -f2 || true)
if [[ -z "${baked_agentd:-}" ]]; then
  baked_agentd=$(grep -oE '/nix/store/[^" ]*libexec/agentd' "$got" 2>/dev/null | head -n1 || true)
fi
set -e
if [[ -z "${baked_msb:-}" ]]; then
  p3_fail=1
  fail "P3 msb liveness: no baked MSB_PATH found in the installed wrapper (${got:-<none>}). Remediation: \`nix profile remove workestrate\` then \`nix profile install .#workestrate\`"
elif [[ ! -x "$baked_msb" ]]; then
  p3_fail=1
  fail "P3 msb liveness: baked MSB_PATH target missing/not executable ($baked_msb). Remediation: \`nix profile remove workestrate\` then \`nix profile install .#workestrate\` (and re-enter \`nix develop\` if the store path was garbage-collected)"
else
  log "P3 baked MSB_PATH live ($baked_msb)"
  set +e
  msb_ver_out=$("$baked_msb" --version 2>&1 | head -n1 || true)
  "$baked_msb" --version >/dev/null 2>&1
  msb_ver_exit=$?
  set -e
  if [[ "$msb_ver_exit" -ne 0 ]]; then
    p3_fail=1
    fail "P3 msb liveness: \`$baked_msb --version\` failed (exit $msb_ver_exit): ${msb_ver_out:-<no output>}. Remediation: re-enter \`nix develop\`, and if it persists \`nix profile remove workestrate\` then \`nix profile install .#workestrate\`"
  else
    log "P3 msb --version OK (${msb_ver_out:-ok})"
  fi
fi
if [[ -n "${baked_agentd:-}" ]] && [[ -e "$baked_agentd" ]]; then
  log "P3 agentd present (baked MSB_AGENTD_PATH $baked_agentd)"
else
  set +e
  agentd_which=$(command -v agentd 2>/dev/null || true)
  set -e
  if [[ -n "${agentd_which:-}" ]]; then
    log "P3 agentd present (resolvable on PATH: $agentd_which)"
  else
    p3_fail=1
    fail "P3 msb liveness: agentd missing (no baked MSB_AGENTD_PATH in ${got:-<none>} and \`agentd\` not on PATH). Remediation: re-enter \`nix develop\`, then \`nix profile remove workestrate\` + \`nix profile install .#workestrate\`"
  fi
fi
if [[ "$p3_fail" -eq 1 ]]; then p3_status="FAIL"; else p3_status="OK"; fi

# ---------------------------------------------------------------------------
# Step B½ — MSB home migration (best-effort, never fails provisioning)
# ---------------------------------------------------------------------------
log "Step B½: msb home migration (best-effort)"
set +e
if [[ -x "$REPO/scripts/migrate-msb-home.sh" ]]; then
  mig_check_out=$("$REPO/scripts/migrate-msb-home.sh" --check-only 2>&1)
  mig_check_status=$?
  if [[ "$mig_check_status" -eq 1 ]]; then
    if [[ "$CHECK_ONLY" -eq 1 ]]; then
      warn "msb homes need migration (--check-only: not migrating): $mig_check_out"
      echo "[host-provision] run ./scripts/migrate-msb-home.sh to migrate"
    else
      log "legacy msb home needs migration; running migrate-msb-home.sh (best-effort)"
      "$REPO/scripts/migrate-msb-home.sh" 2>&1 || warn "msb home migration failed; 'workestrate doctor' will flag it"
    fi
  elif [[ "$mig_check_status" -eq 0 ]]; then
    log "msb homes already converged (nothing to do)"
  else
    warn "msb home migration check errored (status=$mig_check_status): $mig_check_out"
  fi
else
  log "migrate-msb-home.sh not present or not executable; skipping msb home migration"
fi
set -e

# ---------------------------------------------------------------------------
# Step C — workestrate doctor (echo verbatim)
# ---------------------------------------------------------------------------
log "Step C: workestrate doctor"
doctor_out=""
doctor_exit=0
if [[ -n "$got" ]] && [[ -x "$got" ]]; then
  set +e
  doctor_out=$("$got" doctor 2>&1)
  doctor_exit=$?
  set -e
  echo "$doctor_out"
  if [[ "$doctor_exit" -ne 0 ]]; then
    warn "workestrate doctor exited non-zero ($doctor_exit)"
  fi
else
  warn "workestrate binary unavailable — skipping doctor"
  doctor_exit=1
fi

# ---------------------------------------------------------------------------
# Step D — readiness table
# ---------------------------------------------------------------------------
log "Step D: readiness table"

# Derive a status word for a named doctor check from its captured output.
# doctor text format: "<name>: <STATUS> (<message>)"
doctor_status() {
  local name="$1"
  if [[ -z "$doctor_out" ]]; then echo "UNKNOWN"; return; fi
  local line
  line=$(echo "$doctor_out" | grep -E "^${name}: " | head -n1 || true)
  if [[ -z "$line" ]]; then echo "UNKNOWN"; return; fi
  local rest="${line#*: }"
  echo "${rest%% *}"
}

row_kvm=$(doctor_status "dev_kvm")
row_nix=$(doctor_status "nix")
row_msb=$(doctor_status "msb")
row_age=$(doctor_status "age_key_file")
row_home=$(doctor_status "home")
row_config_repos=$(doctor_status "config_repos")

# disk: derive from host-check output (line "Free disk in working directory: N GB")
row_disk="UNKNOWN"
if [[ -n "$host_out" ]]; then
  disk_gb=$(echo "$host_out" | grep -oE 'Free disk in working directory: [0-9]+ GB' | grep -oE '[0-9]+' || true)
  if [[ -n "$disk_gb" ]]; then
    if [[ "$disk_gb" -lt 20 ]]; then
      row_disk="WARN"
    else
      row_disk="OK"
    fi
  fi
fi

# sops: bundled in the wrapper PATH for a nix-installed binary (agentctl.nix
# postInstall --prefix PATH : ${pkgs.sops}/bin). OK when the binary is OK;
# UNKNOWN when we can't trust the wrapper.
if [[ "$binary_status" = "OK" ]]; then
  row_sops="OK"
else
  row_sops="UNKNOWN"
fi

# Count FAIL / WARN across the table.
tbl_fail=0
tbl_warn=0
count() {
  local s="$1"
  case "$s" in
    FAIL)  tbl_fail=$((tbl_fail+1)) ;;
    STALE) tbl_fail=$((tbl_fail+1)) ;;
    WARN)  tbl_warn=$((tbl_warn+1)) ;;
  esac
}

echo
echo "=== readiness ==="
printf "  %-14s %-8s %s\n" "ROW" "STATUS" "DETAIL"
binary_detail="rev=$installed_rev want=${want:-<none>}"
if [[ "$binary_status" != "OK" ]] && [[ -n "$got" ]]; then
  binary_detail="$binary_detail got=$got"
fi
printf "  %-14s %-8s %s\n" "binary" "$binary_status" "$binary_detail"
count "$binary_status"
printf "  %-14s %-8s %s\n" "kvm" "$row_kvm" ""
count "$row_kvm"
printf "  %-14s %-8s %s\n" "nix" "$row_nix" ""
count "$row_nix"
printf "  %-14s %-8s %s\n" "disk" "$row_disk" ""
count "$row_disk"
printf "  %-14s %-8s %s\n" "msb" "$row_msb" ""
count "$row_msb"
printf "  %-14s %-8s %s\n" "sops" "$row_sops" "(bundled for nix-installed binary)"
count "$row_sops"
printf "  %-14s %-8s %s\n" "age-key" "$row_age" ""
count "$row_age"
printf "  %-14s %-8s %s\n" "home" "$row_home" ""
count "$row_home"
printf "  %-14s %-8s %s\n" "config_repos" "$row_config_repos" ""
count "$row_config_repos"
printf "  %-14s %-8s %s\n" "singularity" "$p1_status" "(P1: exactly one workestrate profile entry)"
count "$p1_status"
printf "  %-14s %-8s %s\n" "version" "$p2_status" "(P2: store-path + baked-rev identity)"
count "$p2_status"
printf "  %-14s %-8s %s\n" "msb-live" "$p3_status" "(P3: MSB_PATH + msb --version + agentd)"
count "$p3_status"

echo
if [[ "$tbl_fail" -eq 0 ]]; then
  echo "verdict: READY"
  exit 0
else
  echo "verdict: NOT READY ($tbl_fail FAIL, $tbl_warn WARN)"
  exit 1
fi
