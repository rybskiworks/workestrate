#!/bin/sh
# pre-push hook — pure sh (POSIX), degrades cleanly outside nix-capable envs.
#
# Tracked canonical copy: scripts/git-hooks/pre-push.sh
# Installed copy: .git/hooks/pre-push (per-clone state, NOT tracked).
# Install after cloning:
#   cp scripts/git-hooks/pre-push.sh .git/hooks/pre-push && chmod +x .git/hooks/pre-push
#
# Tier-3 rationale: the full check gate is `just check` = cargo fmt --check +
# cargo clippy -D warnings + cargo check (self-enshelling via nix develop
# where the repo justfile does so). clippy compiles the crate, so it is far
# too heavy for pre-commit — pre-commit gets rustfmt-only (tier 2). pre-push
# is the right place for the compile-grade gate: it runs once per push, not
# once per commit.
#
# Behavior: run `just check` only when `just` AND `nix` are on PATH AND the
# repo justfile declares a `check` recipe; otherwise print ONE skip message
# to stderr and exit 0 (skip-with-message, never bricks the push).
#
# git pre-push passes the remote name + URL as argv and ref specs on stdin;
# this hook intentionally ignores both (never reads stdin, so it can't hang).

label="$(basename "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null)"
[ -n "$label" ] || label="repo"

# Anchor at the repo top level so manual runs behave like git-invoked runs.
cd "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || true

if command -v just >/dev/null 2>&1 && command -v nix >/dev/null 2>&1; then
    if just --summary 2>/dev/null | tr ' ' '\n' | grep -qx check; then
        just check || exit 1
        exit 0
    fi
fi
echo "$label pre-push: just+nix+check-recipe not all present; skipping just check gate — run \`just verify\` manually before relying on this" >&2
exit 0
