#!/usr/bin/env bash
# Select a caller-owned Cargo cache without placing artifacts in the checkout.
set -euo pipefail

fail() {
    printf 'workestrate: %s\n' "$1" >&2
    exit 1
}

[[ $# -eq 1 ]] || fail 'cargo-target.sh requires the checkout root'
repo_root=$1
[[ "$repo_root" == /* && -d "$repo_root" ]] || fail 'checkout root must be an existing absolute directory'
[[ "$repo_root" != *$'\n'* ]] || fail 'checkout root must not contain a newline'

target=${CARGO_TARGET_DIR:-}
if [[ -z "$target" ]]; then
    if [[ -n "${XDG_CACHE_HOME:-}" ]]; then
        target=$XDG_CACHE_HOME/ai-workbench/agentctl-target
    else
        [[ -n "${HOME:-}" ]] || fail 'HOME is required when no Cargo target or XDG cache is supplied'
        target=$HOME/.cache/ai-workbench/agentctl-target
    fi
fi

[[ "$target" == /* ]] || fail 'CARGO_TARGET_DIR must be absolute'
[[ "$target" != *$'\n'* ]] || fail 'CARGO_TARGET_DIR must not contain a newline'
canonical_root=$(realpath -m -- "$repo_root" 2>/dev/null) || fail 'cannot resolve checkout root'
canonical_target=$(realpath -m -- "$target" 2>/dev/null) || fail 'cannot resolve CARGO_TARGET_DIR'
[[ "$canonical_root" != / ]] || fail 'the filesystem root cannot be used as a checkout root'
# GNU realpath -m may leave a looping symlink unresolved. A canonical spelling
# must not retain an existing symlink in any prefix; missing directories are fine.
prefix=$canonical_target
while [[ "$prefix" != / ]]; do
    [[ ! -L "$prefix" ]] || fail 'cannot fully resolve CARGO_TARGET_DIR'
    prefix=${prefix%/*}
    [[ -n "$prefix" ]] || prefix=/
done
case "$canonical_target" in
    "$canonical_root"|"$canonical_root"/*)
        fail 'CARGO_TARGET_DIR must remain outside the checkout, including through symlinks'
        ;;
esac

# Validate the resolved location, but preserve the caller's literal spelling.
# Cargo, not this selector or shell entry, creates the directory when needed.
printf '%s\n' "$target"
