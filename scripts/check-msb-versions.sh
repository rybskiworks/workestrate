#!/usr/bin/env bash
# check-msb-versions.sh — Phase-0 observability pin check (build-time assert).
#
# The microsandbox version pin ("0.6.16") and the fork rev pin are
# hand-maintained strings in several places. This bash+python3-stdlib script
# asserts they agree, with NO cargo/nix/network calls, and runs in <1s.
#
# Checks:
#   V1. version "0.6.16" in nix/packages/microsandbox.nix +
#       nix/packages/agentd.nix +
#       nix/packages/microsandbox-filesystem-patched.nix (each `version = "X"`).
#   V2. control/agentctl/Cargo.toml `=X` pins ([dependencies] microsandbox +
#       [dev-dependencies] microsandbox-image — never literals here).
#   V3. control/agentctl/src/commands/versions.rs MSB_VERSION_PIN.
#   R1. fork rev in flake.lock (microsandbox-fork locked.rev + original.rev).
#   R2. fork rev in flake.nix (the microsandbox-fork url + pin comment).
#   R3. control/agentctl/src/commands/versions.rs FORK_REV_PIN.
#
# Overrides (for synthetic fixtures — never dirty the real tree):
#   REPO (repo root; default: script dir's parent)
#
# Wired into `just versions-check`, in the `verify` chain after
# `toolchain-check`.
#
# Exit 1 with an actionable message on drift; exit 0 OK.

set -euo pipefail

REPO="${REPO:-$(cd "$(dirname "$0")/.." && pwd)}"
MSB_NIX="$REPO/nix/packages/microsandbox.nix"
AGENTD_NIX="$REPO/nix/packages/agentd.nix"
FS_NIX="$REPO/nix/packages/microsandbox-filesystem-patched.nix"
MANIFEST="$REPO/control/agentctl/Cargo.toml"
VERSIONS_RS="$REPO/control/agentctl/src/commands/versions.rs"
FLAKE_NIX="$REPO/flake.nix"
FLAKE_LOCK="$REPO/flake.lock"

if ! command -v python3 >/dev/null 2>&1; then
    echo "msb-versions-check: FAIL: python3 not found on PATH" >&2
    exit 1
fi

python3 - "$MSB_NIX" "$AGENTD_NIX" "$FS_NIX" "$MANIFEST" "$VERSIONS_RS" "$FLAKE_NIX" "$FLAKE_LOCK" <<'PY'
import json
import re
import sys
import tomllib

msb_nix, agentd_nix, fs_nix, manifest_path, versions_rs, flake_nix, flake_lock = sys.argv[1:8]
failures = []

FIX = (
    "fix: update ALL of these to the same pin, then re-run "
    "./scripts/check-msb-versions.sh:\n"
    "  nix/packages/microsandbox.nix, nix/packages/agentd.nix,\n"
    "  nix/packages/microsandbox-filesystem-patched.nix (version = \"X\"),\n"
    "  control/agentctl/Cargo.toml (=X pins, then `cargo update -w` inside "
    "`just shell` ONLY),\n"
    "  control/agentctl/src/commands/versions.rs (MSB_VERSION_PIN),\n"
    "  flake.nix + flake.lock (fork rev, via "
    "`nix flake lock --update-input microsandbox-fork` on a nix-capable host)"
)


def read(path):
    try:
        with open(path, encoding="utf-8") as f:
            return f.read()
    except OSError as e:
        failures.append(f"cannot read {path}: {e}")
        return ""


def nix_version(path, text):
    m = re.search(r'version\s*=\s*"([^"]+)"', text)
    if not m:
        failures.append(f'{path}: no `version = "X"` found')
        return None
    return m.group(1)


# --- V1: the three nix package versions ---
nix_versions = {}
for path in (msb_nix, agentd_nix, fs_nix):
    v = nix_version(path, read(path))
    if v is not None:
        nix_versions[path] = v

# --- V2: Cargo.toml =X pins (never literals) ---
with open(manifest_path, "rb") as f:
    manifest = tomllib.load(f)
try:
    dep_pin = manifest["dependencies"]["microsandbox"]["version"]
except KeyError:
    dep_pin = None
try:
    dev_pin = manifest["dev-dependencies"]["microsandbox-image"]
    if isinstance(dev_pin, dict):
        dev_pin = dev_pin.get("version")
except KeyError:
    dev_pin = None
cargo_versions = []
if not dep_pin or not dev_pin:
    failures.append(
        "Cargo.toml: could not parse =X pins for microsandbox "
        f"([dependencies]) / microsandbox-image ([dev-dependencies]) "
        f"(got {dep_pin!r} / {dev_pin!r})"
    )
else:
    if not dep_pin.startswith("=") or not dev_pin.startswith("="):
        failures.append(
            "Cargo.toml pins must be exact (=X style), got "
            f"microsandbox={dep_pin!r} microsandbox-image={dev_pin!r}"
        )
    else:
        cargo_versions = [dep_pin[1:], dev_pin[1:]]

# --- V3: versions.rs MSB_VERSION_PIN ---
rs_text = read(versions_rs)
m = re.search(r'MSB_VERSION_PIN\s*:\s*&str\s*=\s*"([^"]+)"', rs_text)
rs_version = m.group(1) if m else None
if rs_version is None:
    failures.append(f"{versions_rs}: MSB_VERSION_PIN not found")

# --- V1+V2+V3 agreement ---
seen_versions = set(nix_versions.values()) | set(cargo_versions)
if rs_version is not None:
    seen_versions.add(rs_version)
if len(seen_versions) > 1:
    detail = ", ".join(
        [f"{p}={v}" for p, v in sorted(nix_versions.items())]
        + [f"Cargo.toml={v}" for v in cargo_versions]
        + ([f"versions.rs={rs_version}"] if rs_version else [])
    )
    failures.append(f"msb version pins disagree: {detail}")

# --- R1: flake.lock microsandbox-fork revs ---
lock_revs = []
try:
    with open(flake_lock, encoding="utf-8") as f:
        lock = json.load(f)
    fork = lock["nodes"]["microsandbox-fork"]
    for key in ("locked", "original"):
        rev = fork.get(key, {}).get("rev")
        if rev:
            lock_revs.append(rev)
        else:
            failures.append(f"flake.lock: microsandbox-fork {key}.rev missing")
except (OSError, KeyError, ValueError) as e:
    failures.append(f"flake.lock: cannot parse microsandbox-fork rev: {e}")

# --- R2: flake.nix fork revs (url + pin comment only, not every input pin) ---
flake_text = read(flake_nix)
flake_revs = set(re.findall(r"microsandbox/([0-9a-f]{40})", flake_text))
flake_revs |= set(re.findall(r"[Pp]inned fork rev ([0-9a-f]{40})", flake_text))
if not flake_revs:
    failures.append("flake.nix: no microsandbox-fork rev found (url or pin comment)")

# --- R3: versions.rs FORK_REV_PIN ---
m = re.search(r'FORK_REV_PIN\s*:\s*&str\s*=\s*"([^"]+)"', rs_text)
rs_rev = m.group(1) if m else None
if rs_rev is None:
    failures.append(f"{versions_rs}: FORK_REV_PIN not found")
elif not re.fullmatch(r"[0-9a-f]{40}", rs_rev):
    failures.append(f"{versions_rs}: FORK_REV_PIN is not a full 40-hex rev: {rs_rev!r}")

# --- R1+R2+R3 agreement ---
seen_revs = set(lock_revs) | set(flake_revs)
if rs_rev is not None:
    seen_revs.add(rs_rev)
if len(seen_revs) > 1:
    failures.append(
        "fork rev pins disagree: "
        + ", ".join(
            [f"flake.lock={r}" for r in lock_revs]
            + [f"flake.nix={r}" for r in sorted(flake_revs)]
            + ([f"versions.rs={rs_rev}"] if rs_rev else [])
        )
    )

if failures:
    print("msb-versions-check: FAIL:")
    for fl in failures:
        print(f"  - {fl}")
    print(FIX)
    sys.exit(1)

print(
    "msb-versions-check: OK "
    f"(msb=={rs_version or '?'}; fork=={(rs_rev or '?')[:8]})"
)
PY
