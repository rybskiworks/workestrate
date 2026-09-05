#!/bin/sh
# pre-commit hook — pure sh (POSIX), degrades cleanly outside devshells.
#
# Tracked canonical copy: scripts/git-hooks/pre-commit.sh
# Installed copy: .git/hooks/pre-commit (per-clone state, NOT tracked).
# Reinstall after cloning:
#   cp scripts/git-hooks/pre-commit.sh .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
#
# CAUTION (devenv clobber): entering a devenv shell would run git-hooks.nix's
# installer, which moves this fallback to .git/hooks/pre-commit.legacy and
# installs a store-path'd generated hook (dangles after GC; no secret gate).
# That clobber is DISABLED by default via nix-tooling devenvModules/base.nix
# `git-hooks.install.enable = false`. The CAUTION applies only if a consumer
# re-enables installation — then re-run the cp above after shell entry.
#
# Background: a previous `prek install` wrote a shim that exec'd a hardcoded
# nix-store prek binary against a generated .pre-commit-config.yaml. Nix
# garbage collection deleted both, so EVERY commit failed unless passed
# --no-verify. This shim never references store paths and never hard-fails
# when the nix-provided toolchain is absent. Hook exactness is enforced by
# `nix flake check` / CI, not by this gate.
#
# Tiered gates, in order:
#   (1) always-run secret-material guard (POSIX grep only) — unconditional;
#   (2) tier-1 fast linters over staged files (typos, nixfmt, statix,
#       deadnix) — present-run / absent-skip-with-message;
#   (3) tombi TOML gates when tombi is on PATH (absent, or version mismatch,
#       warns and skips — never fails);
#   (4) tier-2 in-devshell rustfmt --check at the crate's edition when
#       rustfmt is on PATH and .rs files are staged;
#   (5) prek delegation only when BOTH the prek binary AND a non-dangling
#       repo-root config are present.
# `.git/hooks/pre-commit.legacy`, when present, is a historical backup left
# by `prek install`, not part of the chain (same as pre-commit semantics).

label="$(basename "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null)"
[ -n "$label" ] || label="repo"
fail=0

# git runs hooks with cwd at the repo top level; anchor there anyway so manual
# runs from a subdirectory behave the same. Stderr is suppressed: outside a
# repo (or with a restricted git) the guards see an empty index and pass.
cd "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || true

# Staged paths (added/copied/modified). Word-splitting over newline-separated
# paths below is intentional: these repos forbid spaces in tracked paths.
staged=$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null)
nix_files=$(printf '%s\n' "$staged" | grep -E '[^ ]\.nix$' 2>/dev/null || true)
rs_files=$(printf '%s\n' "$staged" | grep -E '[^ ]\.rs$' 2>/dev/null || true)

# Canonical tombi version source: nix-tooling share/tombi-version (single-
# sourced by packages/tombi.nix). Repos without that file fall back to the
# literal; keep the fallback in sync with nix-tooling's share/tombi-version.
if [ -f share/tombi-version ]; then
    TOMBI_REQUIRED="$(tr -d '[:space:]' < share/tombi-version 2>/dev/null)"
fi
[ -n "$TOMBI_REQUIRED" ] || TOMBI_REQUIRED="1.2.5"

# --- (1) secret-material guard (staged paths only) ---------------------------
if printf '%s\n' "$staged" | grep -qE '(^|/)([^/]*\.agekey|age\.txt|[^/]*\.pem|id_rsa[^/]*|\.env)$'; then
    echo "$label pre-commit: refusing secret material (*.agekey, age.txt, *.pem, id_rsa*, .env):" >&2
    printf '%s\n' "$staged" | grep -E '(^|/)([^/]*\.agekey|age\.txt|[^/]*\.pem|id_rsa[^/]*|\.env)$' >&2
    fail=1
fi
[ "$fail" -eq 0 ] || exit 1

# --- (2) tier-1 fast linters (present-run / absent-skip) ---------------------
if command -v typos >/dev/null 2>&1; then
    if [ -n "$staged" ]; then
        typos -- $staged || exit 1
    fi
else
    echo "$label pre-commit: typos not found; skipping typos gate" >&2
fi

if command -v nixfmt >/dev/null 2>&1; then
    if [ -n "$nix_files" ]; then
        nixfmt --check -- $nix_files || exit 1
    fi
else
    echo "$label pre-commit: nixfmt not found; skipping nixfmt gate" >&2
fi

if command -v statix >/dev/null 2>&1; then
    if [ -n "$nix_files" ]; then
        for f in $nix_files; do
            statix check -- "$f" || exit 1
        done
    fi
else
    echo "$label pre-commit: statix not found; skipping statix gate" >&2
fi

if command -v deadnix >/dev/null 2>&1; then
    if [ -n "$nix_files" ]; then
        deadnix --fail -- $nix_files || exit 1
    fi
else
    echo "$label pre-commit: deadnix not found; skipping deadnix gate" >&2
fi

# --- (3) tombi TOML gates (best-effort; never brick the commit) --------------
if command -v tombi >/dev/null 2>&1; then
    tombi_version="$(tombi --version 2>/dev/null | awk '{print $2}')"
    if [ "$tombi_version" != "$TOMBI_REQUIRED" ]; then
        echo "$label pre-commit: tombi version mismatch (found '${tombi_version:-unknown}', want $TOMBI_REQUIRED); skipping tombi gates — exactness via \`nix flake check\`" >&2
    else
        tombi format --check || exit 1
        tombi lint --error-on-warnings || exit 1
    fi
else
    echo "$label pre-commit: tombi not found; skipping tombi gates" >&2
fi

# --- (4) tier-2 in-devshell rustfmt gate (staged .rs files only) -------------
if [ -n "$rs_files" ]; then
    if command -v rustfmt >/dev/null 2>&1; then
        manifest=""
        if [ -f ./Cargo.toml ]; then
            manifest=./Cargo.toml
        elif [ -f ./control/agentctl/Cargo.toml ]; then
            manifest=./control/agentctl/Cargo.toml
        else
            echo "$label pre-commit: no Cargo.toml found; skipping rustfmt gate" >&2
        fi
        if [ -n "$manifest" ]; then
            # Edition follows the crate's Cargo.toml (single source of truth);
            # edition 2024 would break pre-2024 crates (`gen` becomes reserved).
            crate_edition=$(sed -n 's/^edition *= *"\([^"]*\)".*/\1/p' "$manifest" | head -1)
            [ -n "$crate_edition" ] || crate_edition="2021"
            if ! rustfmt --edition "$crate_edition" --check -- $rs_files; then
                echo "$label pre-commit: rustfmt check failed — run \`cargo fmt\` or \`just fmt\`" >&2
                exit 1
            fi
        fi
    else
        echo "$label pre-commit: rustfmt not found; skipping rustfmt gate" >&2
    fi
fi

# --- (5) prek delegation (only when binary AND config both resolve) ----------
root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
cfg=""
if [ -n "$root" ]; then
    for c in "$root/.pre-commit-config.yaml" "$root/.pre-commit-config.yml"; do
        if [ -f "$c" ]; then
            cfg="$c"
            break
        elif [ -e "$c" ] || [ -L "$c" ]; then
            echo "$label pre-commit: generated $c is dangling (nix GC); skipping prek delegation" >&2
        fi
    done
fi
if [ -n "$cfg" ]; then
    if command -v prek >/dev/null 2>&1; then
        here="$(cd "$(dirname "$0")" && pwd)"
        # Flags mirror the previously generated shim (minus the dead store path).
        prek hook-impl --hook-dir "$here" --script-version 4 --hook-type=pre-commit --config="$cfg" -- "$@" || exit 1
    else
        echo "$label pre-commit: prek not found but $cfg exists; skipping prek delegation" >&2
    fi
fi
exit 0
