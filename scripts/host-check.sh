#!/usr/bin/env bash
set -euo pipefail

ERRORS=0

log() { echo "[host-check] $*"; }
warn() { echo "[host-check] WARNING: $*" >&2; }
fail() { echo "[host-check] FAIL: $*" >&2; ERRORS=$((ERRORS+1)); }

# OS / arch
if [[ "$(uname -s)" != "Linux" ]]; then
  fail "Microsandbox runtime requires Linux; found $(uname -s)"
fi
if [[ "$(uname -m)" != "x86_64" ]]; then
  fail "Microsandbox runtime requires x86_64; found $(uname -m)"
fi

# KVM
if [[ -c /dev/kvm ]]; then
  log "KVM device present: /dev/kvm"
  if [[ -r /dev/kvm ]] && [[ -w /dev/kvm ]]; then
    log "KVM device is readable and writable by current user"
  else
    warn "KVM device exists but is not accessible; you may need to add user to 'kvm' group and re-login"
  fi
else
  fail "KVM device not found at /dev/kvm; enable virtualization in BIOS and load the kvm/kvm_intel/kvm_amd modules"
fi

# CPU flags
if grep -qE 'vmx|svm' /proc/cpuinfo; then
  log "CPU virtualization flags detected"
else
  warn "No vmx/svm flag in /proc/cpuinfo; virtualization may be disabled in firmware"
fi

# Nix
if command -v nix >/dev/null 2>&1; then
  log "nix found: $(command -v nix)"
  if nix flake --help >/dev/null 2>&1; then
    log "Nix flakes appear enabled"
  else
    fail "Nix flakes are not enabled; add 'experimental-features = nix-command flakes' to ~/.config/nix/nix.conf or /etc/nix/nix.conf"
  fi
else
  fail "nix not found; install Nix: https://nixos.org/download/ or run: curl -L https://nixos.org/nix/install | sh"
fi

# Memory
MEM_MB=$(awk '/MemTotal/ {print int($2/1024)}' /proc/meminfo)
log "Memory: ${MEM_MB} MB"
if [[ "$MEM_MB" -lt 4096 ]]; then
  warn "Less than 4 GB RAM; sandboxes may be constrained"
fi

# Disk
DISK_GB=$(df -BG . 2>/dev/null | awk 'NR==2 {gsub(/G/,"",$4); print $4}')
log "Free disk in working directory: ${DISK_GB} GB"
if [[ "$DISK_GB" -lt 20 ]]; then
  warn "Less than 20 GB free disk space"
fi

if [[ "$ERRORS" -eq 0 ]]; then
  log "Host looks ready for ai-workbench. Run: nix run . -- workload plan litellm"
  exit 0
else
  fail "$ERRORS check(s) failed; fix above issues before running the workbench"
  exit 1
fi
