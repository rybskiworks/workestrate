#!/usr/bin/env bash
# check-cargo-lock.sh — lock-guard: pre-resolution Cargo.lock fork-pin check.
#
# A broken lock breaks cargo resolution itself (circular: `cargo test` cannot
# even resolve), so this bash+python3-stdlib script gives actionable output
# BEFORE cargo runs. It invokes NO cargo/nix commands and runs in <1s.
#
# Checks (control/agentctl/Cargo.lock):
#   A1. every microsandbox-* entry has NO registry source/checksum
#       (i.e. patched to the fork via [patch.crates-io], not crates.io).
#   A2. every microsandbox-* version == X, where X is parsed from
#       control/agentctl/Cargo.toml ([dependencies] microsandbox +
#       [dev-dependencies] microsandbox-image `=X` pins — never literals here).
#   A3. smoltcp satisfies the fork's requirement parsed from
#       control/agentctl/vendor/microsandbox-fork/Cargo.toml
#       [workspace.dependencies] (prefix match on major.minor), AND
#       tokio-socks is present in the lock.
#
# Vendor states:
#   vendor-absent (symlink target missing, e.g. fresh clone without
#     `nix develop` ever run) -> SKIP exit 0 with a note (A1/A2 still checked).
#   vendor-present-but-wrong (smoltcp mismatch) -> FAIL exit 1.
#
# Overrides (for synthetic fixtures — never dirty the real lock):
#   CARGO_LOCK_OVERRIDE, CARGO_TOML_OVERRIDE, VENDOR_TOML_OVERRIDE
#
# Wired into `just lock-guard`, FIRST in the `verify` chain.

set -euo pipefail

REPO=$(cd "$(dirname "$0")/.." && pwd)
LOCK="${CARGO_LOCK_OVERRIDE:-$REPO/control/agentctl/Cargo.lock}"
MANIFEST="${CARGO_TOML_OVERRIDE:-$REPO/control/agentctl/Cargo.toml}"
VENDOR_MANIFEST="${VENDOR_TOML_OVERRIDE:-$REPO/control/agentctl/vendor/microsandbox-fork/Cargo.toml}"

if ! command -v python3 >/dev/null 2>&1; then
    echo "lock-guard: FAIL: python3 not found on PATH" >&2
    exit 1
fi

python3 - "$LOCK" "$MANIFEST" "$VENDOR_MANIFEST" <<'PY'
import sys
import tomllib

lock_path, manifest_path, vendor_path = sys.argv[1], sys.argv[2], sys.argv[3]
failures = []
notes = []

FIX = (
    "cause: bare `cargo ...` run outside the devshell rewrote "
    "control/agentctl/Cargo.lock without the [patch.crates-io] fork table, "
    "so crates.io copies replaced the fork pins\n"
    "fix: enter `nix develop`, then run `cargo update -w` inside the devshell "
    "ONLY, re-run ./scripts/check-cargo-lock.sh, and review the diff "
    "(`git diff control/agentctl/Cargo.lock`)"
)

# --- Parse X from control/agentctl/Cargo.toml (=X pins, never literals) ---
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

if not dep_pin or not dev_pin:
    print("lock-guard: FAIL: could not parse =X pins for microsandbox "
          "([dependencies]) / microsandbox-image ([dev-dependencies]) from "
          f"{manifest_path} (got {dep_pin!r} / {dev_pin!r})")
    print(FIX)
    sys.exit(1)
if not dep_pin.startswith("=") or not dev_pin.startswith("="):
    failures.append(
        f"Cargo.toml pins must be exact (=X style), got microsandbox={dep_pin!r} "
        f"microsandbox-image={dev_pin!r}"
    )
    X = None
else:
    X = dep_pin[1:]
    if dev_pin[1:] != X:
        failures.append(
            f"Cargo.toml fork pins disagree: microsandbox={dep_pin!r} vs "
            f"microsandbox-image={dev_pin!r} (expected both ={X})"
        )

# --- Parse control/agentctl/Cargo.lock ---
with open(lock_path, "rb") as f:
    lock = tomllib.load(f)
packages = lock.get("package", [])
msb_pkgs = [p for p in packages if p.get("name", "").startswith("microsandbox")]
if not msb_pkgs:
    failures.append(f"no microsandbox-* entries found in {lock_path}")

# --- A1: no registry source/checksum on microsandbox-* entries ---
for p in msb_pkgs:
    name, src, cks = p.get("name"), p.get("source"), p.get("checksum")
    if src is not None or cks is not None:
        failures.append(
            f"A1: {name}@{p.get('version')} has registry linkage "
            f"(source={src!r} checksum={'<present>' if cks else None}) — "
            "expected fork patch (no source/checksum)"
        )

# --- A2: every microsandbox-* version == X ---
if X is not None:
    for p in msb_pkgs:
        if p.get("version") != X:
            failures.append(
                f"A2: {p.get('name')} version mismatch: expected {X}, "
                f"got {p.get('version')}"
            )

# --- A3: smoltcp satisfies the fork requirement + tokio-socks present ---
by_name = {}
for p in packages:
    by_name.setdefault(p.get("name"), []).append(p)

smoltcp_entries = by_name.get("smoltcp", [])
lock_smoltcp = smoltcp_entries[0].get("version") if smoltcp_entries else None
has_tokio_socks = "tokio-socks" in by_name
if not has_tokio_socks:
    failures.append("A3: tokio-socks missing from Cargo.lock")

try:
    with open(vendor_path, "rb") as f:
        vendor = tomllib.load(f)
    vendor_req = vendor["workspace"]["dependencies"]["smoltcp"]["version"]
    vendor_live = True
except (OSError, KeyError) as e:
    vendor_live = False
    vendor_req = None
    notes.append(
        f"SKIP A3 requirement check: vendor manifest absent ({vendor_path}: {e}) — "
        "fresh clone without `nix develop` ever run; A1/A2 still enforced"
    )

if vendor_live:
    req_mm = ".".join(str(vendor_req).split(".")[:2])
    got_mm = ".".join(str(lock_smoltcp).split(".")[:2]) if lock_smoltcp else None
    if lock_smoltcp is None:
        failures.append(
            f"A3: smoltcp missing from Cargo.lock (fork requires {vendor_req})"
        )
    elif got_mm != req_mm:
        failures.append(
            f"A3: smoltcp version mismatch: expected {vendor_req}.x "
            f"(major.minor {req_mm}), got {lock_smoltcp}"
        )
    else:
        notes.append(
            f"A3 OK: lock smoltcp {lock_smoltcp} satisfies fork requirement "
            f"{vendor_req} (major.minor {req_mm})"
        )
else:
    # Vendor-absent fallback: literal 0.14.x expectation + tokio-socks (above).
    if lock_smoltcp is None:
        failures.append("A3: smoltcp missing from Cargo.lock (fallback expects 0.14.x)")
    elif not str(lock_smoltcp).startswith("0.14."):
        failures.append(
            f"A3: smoltcp version mismatch (vendor absent, fallback): "
            f"expected 0.14.x, got {lock_smoltcp}"
        )
    else:
        notes.append(
            f"A3 OK (fallback): lock smoltcp {lock_smoltcp} matches 0.14.x; "
            "re-run inside `nix develop` once the vendor symlink exists "
            "for the authoritative fork-requirement check"
        )

for n in notes:
    print(f"lock-guard: {n}")

if failures:
    print("lock-guard: FAIL:")
    for fl in failures:
        print(f"  - {fl}")
    print(FIX)
    sys.exit(1)

print(f"lock-guard: OK (microsandbox-*{X and ('==' + X) or ''}, "
      f"smoltcp={lock_smoltcp}, tokio-socks present)")
PY
