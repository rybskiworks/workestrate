#!/usr/bin/env bash
# check-msb-versions.sh — check the SDK version and runtime source pins.
#
# Cargo dependency pins and the reported runtime pins must agree with the
# Microsandbox flake input. This bash+python3-stdlib script checks that contract
# without cargo/nix/network calls. Nix separately checks the consumed fork's
# package and SDK versions; runtime packaging belongs to the fork.
#
# Checks:
#   - Exact Microsandbox dependency versions match MSB_VERSION_PIN.
#   - The literal microsandbox-fork URL pins a full commit revision.
#   - The resolved flake.lock input and FORK_REV_PIN match that source.
#
# Overrides (for synthetic fixtures — never dirty the real tree):
#   REPO (repo root; default: script dir's parent)
#
# Wired into `just versions-check`, before the devshell-dependent verify gates.
#
# Exit 1 with an actionable message on drift; exit 0 OK.

set -euo pipefail

REPO="${REPO:-$(cd "$(dirname "$0")/.." && pwd)}"
MANIFEST="$REPO/control/agentctl/Cargo.toml"
VERSIONS_RS="$REPO/control/agentctl/src/commands/versions.rs"
FLAKE_NIX="$REPO/flake.nix"
FLAKE_LOCK="$REPO/flake.lock"

if ! command -v python3 >/dev/null 2>&1; then
    echo "msb-versions-check: FAIL: python3 not found on PATH" >&2
    exit 1
fi

python3 - "$MANIFEST" "$VERSIONS_RS" "$FLAKE_NIX" "$FLAKE_LOCK" <<'PY'
import json
import re
import sys
import tomllib

manifest_path, versions_rs, flake_nix, flake_lock = sys.argv[1:5]
failures = []

FIX = (
    "fix: align the SDK version and fork source pins, then re-run "
    "./scripts/check-msb-versions.sh:\n"
    "  control/agentctl/Cargo.toml (exact =X.Y.Z dependency pins),\n"
    "  control/agentctl/src/commands/versions.rs (MSB_VERSION_PIN and FORK_REV_PIN),\n"
    "  flake.nix + flake.lock (full fork revision; relock with "
    "`nix flake update microsandbox-fork` on a nix-capable host)\n"
    "  Runtime package and SDK versions are checked during Nix evaluation."
)


def read(path):
    try:
        with open(path, encoding="utf-8") as f:
            return f.read()
    except OSError as e:
        failures.append(f"cannot read {path}: {e}")
        return ""


# Exact Cargo pins must identify a complete version, not a compatible range.
try:
    manifest = tomllib.loads(read(manifest_path))
except tomllib.TOMLDecodeError as e:
    failures.append(f"Cargo.toml: cannot parse manifest: {e}")
    manifest = {}
cargo_versions = {}
for section in ("dependencies", "dev-dependencies", "build-dependencies"):
    entries = manifest.get(section, {})
    if not isinstance(entries, dict):
        failures.append(f"Cargo.toml: [{section}] must be a table")
        continue
    required = {"dependencies": "microsandbox", "dev-dependencies": "microsandbox-image"}.get(section)
    if required and required not in entries:
        failures.append(f"Cargo.toml: [{section}] {required} version pin missing")
    for name, dependency in entries.items():
        if name != "microsandbox" and not name.startswith("microsandbox-"):
            continue
        pin = dependency.get("version") if isinstance(dependency, dict) else dependency
        match = re.fullmatch(
            r"=([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?)",
            pin,
        ) if isinstance(pin, str) else None
        if not match:
            failures.append(f"Cargo.toml: [{section}] {name} pin must be exact (=X.Y.Z), got {pin!r}")
        else:
            cargo_versions[f"{section}.{name}"] = match.group(1)

rs_text = read(versions_rs)


def rust_pin(name):
    values = re.findall(rf'(?m)^\s*(?:pub\s+)?const\s+{name}\s*:\s*&str\s*=\s*"([^\"]+)"', rs_text)
    if len(values) != 1:
        failures.append(f"{versions_rs}: expected one {name} string constant")
        return None
    return values[0]


rs_version = rust_pin("MSB_VERSION_PIN")
seen_versions = set(cargo_versions.values())
if rs_version is not None:
    seen_versions.add(rs_version)
if len(seen_versions) > 1:
    detail = ", ".join(
        [f"Cargo.toml:{name}={v}" for name, v in cargo_versions.items()]
        + ([f"versions.rs={rs_version}"] if rs_version else [])
    )
    failures.append(f"msb version pins disagree: {detail}")

# Inspect the actual input declaration, not an unrelated URL or pin comment.
flake_text = re.sub(
    r'("(?:\\.|[^"\\])*")|#[^\n]*|/\*.*?\*/',
    lambda match: match.group(1) or "",
    read(flake_nix),
    flags=re.DOTALL,
)
urls = re.findall(
    r'\bmicrosandbox-fork\s*(?:\.\s*url\s*=\s*"([^"]+)"|=\s*\{[^{}]*?\burl\s*=\s*"([^"]+)")',
    flake_text,
)
fork_url = next((value for value in urls[0] if value), None) if len(urls) == 1 else None
url_match = re.fullmatch(r"github:([^/]+)/microsandbox/([0-9a-f]{40})", fork_url or "")
if url_match is None:
    failures.append("flake.nix: microsandbox-fork URL must pin one full 40-hex GitHub revision")
flake_rev = url_match.group(2) if url_match else None

# Resolve the root input's node, whose lockfile name need not match the input.
lock_revs = []
try:
    lock = json.loads(read(flake_lock))
    nodes = lock["nodes"]
    fork_node = nodes[lock["root"]]["inputs"]["microsandbox-fork"]
    fork = nodes[fork_node]
    if fork.get("flake") is False:
        failures.append("flake.lock: microsandbox-fork must provide flake packages (flake=false is stale)")
    for key in ("locked", "original"):
        source = fork.get(key, {})
        rev = source.get("rev")
        if not isinstance(rev, str) or not re.fullmatch(r"[0-9a-f]{40}", rev):
            failures.append(f"flake.lock: microsandbox-fork {key}.rev must be a full 40-hex revision")
        else:
            lock_revs.append(rev)
        if url_match and (
            source.get("type"), source.get("owner"), source.get("repo")
        ) != ("github", url_match.group(1), "microsandbox"):
            failures.append(f"flake.lock: microsandbox-fork {key} source does not match flake.nix URL")
except (KeyError, TypeError, ValueError, AttributeError) as e:
    failures.append(f"flake.lock: cannot parse microsandbox-fork rev: {e}")

rs_rev = rust_pin("FORK_REV_PIN")
if rs_rev is not None and not re.fullmatch(r"[0-9a-f]{40}", rs_rev):
    failures.append(f"{versions_rs}: FORK_REV_PIN is not a full 40-hex rev: {rs_rev!r}")

seen_revs = set(lock_revs)
if flake_rev is not None:
    seen_revs.add(flake_rev)
if rs_rev is not None:
    seen_revs.add(rs_rev)
if len(seen_revs) > 1:
    failures.append(
        "fork rev pins disagree: "
        + ", ".join(
            [f"flake.lock={r}" for r in lock_revs]
            + ([f"flake.nix={flake_rev}"] if flake_rev else [])
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
