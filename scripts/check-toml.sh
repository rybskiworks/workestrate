#!/usr/bin/env bash
# check-toml.sh — repo-wide tombi TOML gates (spec 15).
#
# Runs:
#   (1) tombi availability + version guard (TOMBI_REQUIRED; spec §1.2 — a
#       stale 0.11.6 on PATH cannot silently lint with the wrong config keys).
#   (2) `tombi format --check` over the repo include set (tombi.toml §2.1).
#   (3) `tombi lint --error-on-warnings` over the same set (includes schema
#       validation of config.reference/workestrate.toml via [[schemas]]).
#
# TOMBI_OFFLINE=true pins offline mode: no remote schema-catalog fetches.
#
# Wired into `just tombi-check` and `just verify`; the devshell provides
# tombi 1.2.5 (nix-tooling/packages/tombi.nix via inputs.tooling).

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

TOMBI_REQUIRED="1.2.5"

if ! command -v tombi >/dev/null 2>&1; then
    echo "FAIL: tombi not found on PATH." >&2
    echo "      Run from a plain shell: just tombi-check (self-enshelling)" >&2
    exit 1
fi

# `tombi --version` prints "tombi 1.2.5 (x86_64-unknown-linux-musl)" — the
# version is field 2 ($NF is the target triple).
tombi_version="$(tombi --version | awk '{print $2}')"
if [ "$tombi_version" != "$TOMBI_REQUIRED" ]; then
    echo "FAIL: tombi version mismatch: got '${tombi_version}', require '${TOMBI_REQUIRED}'." >&2
    echo "      The repo tombi.toml uses 1.x config keys ([files] include/exclude);" >&2
    echo "      run from a plain shell: just tombi-check (self-enshelling)" >&2
    exit 1
fi

export TOMBI_OFFLINE=true

if ! tombi format --check; then
    echo "FAIL: tombi format --check reported diffs." >&2
    echo "      Run 'just shell -c \"tombi format\"' at the repo root and commit the result." >&2
    exit 1
fi

if ! tombi lint --error-on-warnings; then
    echo "FAIL: tombi lint reported errors/warnings." >&2
    echo "      Fix the reported TOML files (see tombi.toml for the include set)." >&2
    exit 1
fi

echo "OK: tombi ${tombi_version} format + lint clean (include set in tombi.toml)."
