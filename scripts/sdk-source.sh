#!/usr/bin/env bash
# Select a caller-supplied SDK source without overwriting a local checkout.
set -euo pipefail

fail() {
    printf 'workestrate: %s\n' "$1" >&2
    exit 1
}

[[ $# -eq 2 || $# -eq 3 ]] || fail 'sdk-source.sh requires checkout root and SDK source'
allow_unlocked=false
if [[ $# -eq 3 ]]; then
    [[ "$3" == --allow-unlocked ]] || fail 'unknown SDK source option'
    allow_unlocked=true
fi
for value in "$1" "$2"; do
    [[ "$value" == /* && "$value" != *$'\n'* && "$value" != *$'\r'* ]] \
        || fail 'checkout root and SDK source must be absolute single-line paths'
done
repo_root=$(realpath -e -- "$1") || fail 'cannot resolve checkout root'
source_root=$(realpath -e -- "$2") || fail 'cannot resolve SDK source'
[[ "$repo_root" != / && -d "$repo_root" ]] || fail 'invalid checkout root'
[[ -f "$repo_root/flake.nix" && -f "$repo_root/control/agentctl/Cargo.toml" \
    && -f "$repo_root/config.reference/workestrate.toml" ]] || fail 'not a Workestrate checkout'
[[ -d "$source_root" && -f "$source_root/Cargo.toml" \
    && -f "$source_root/sdk/rust/Cargo.toml" \
    && -f "$source_root/crates/protocol/Cargo.toml" ]] || fail 'SDK source is not a complete workspace'
case "$source_root" in
    "$repo_root"|"$repo_root"/*) fail 'SDK source must remain outside the checkout' ;;
esac

# Reject symlinked parents before updating the source link.
for parent in "$repo_root/control" "$repo_root/control/agentctl"; do
    [[ -d "$parent" && ! -L "$parent" ]] || fail 'SDK link parent must be a real directory'
done
vendor_dir="$repo_root/control/agentctl/vendor"
[[ ! -L "$vendor_dir" ]] || fail 'vendor parent must not be a symlink'
if [[ -e "$vendor_dir" && ! -d "$vendor_dir" ]]; then
    fail 'vendor parent must be a real directory'
fi
vendor_link="$vendor_dir/microsandbox-fork"
if [[ ! -L "$vendor_link" && -e "$vendor_link" ]]; then
    if [[ -d "$vendor_link" && "$allow_unlocked" == true ]]; then
        printf 'workestrate: vendor/microsandbox-fork is a real directory (unlocked); leaving it alone\n' >&2
        exit 0
    fi
    fail 'vendor/microsandbox-fork is not a symlink; preserve local work and resolve it explicitly'
fi

mkdir -p -- "$vendor_dir"
if [[ -L "$vendor_link" ]]; then
    current=$(realpath -e -- "$vendor_link" 2>/dev/null || true)
    if [[ "$current" != "$source_root" ]]; then
        # -T/-n replaces the symlink itself, never a file within its target.
        ln -sfnT -- "$source_root" "$vendor_link"
        printf 'workestrate: SDK source symlink refreshed to the pinned workspace\n' >&2
    fi
else
    # No force when absent: a concurrent creator is a refusal, not an overwrite.
    ln -s -- "$source_root" "$vendor_link"
fi
[[ -L "$vendor_link" && "$(realpath -e -- "$vendor_link")" == "$source_root" ]] \
    || fail 'SDK source symlink verification failed'
printf '%s\n' "$source_root"
