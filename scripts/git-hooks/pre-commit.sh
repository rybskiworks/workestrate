#!/bin/sh
# pre-commit hook — pure sh, degrades cleanly outside devshells.
#
# Tracked canonical copy: scripts/git-hooks/pre-commit.sh
# Installed copy: .git/hooks/pre-commit (per-clone state, NOT tracked).
# Reinstall after cloning:
#   cp scripts/git-hooks/pre-commit.sh .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
#
# Background: a previous `prek install` wrote a shim that exec'd a hardcoded
# nix-store prek binary against a generated .pre-commit-config.yaml. Nix
# garbage collection deleted both, so EVERY commit failed unless passed
# --no-verify. This shim never references store paths and never hard-fails
# when the nix-provided toolchain is absent:
#   - inside a devenv shell, devenv's git-hooks integration (see flake.nix
#     `devenv.shells.default` imports of the shared nix-tooling modules)
#     manages formatting/linting with the pinned toolchain;
#   - on a bare host or container without nix, the fast POSIX-only guards
#     below still run, while formatter/linter gates skip with a message.
# Hook exactness is enforced by `nix flake check` / CI, not by this gate.
#
# Order: (1) always-run secret-material guard (POSIX grep only);
# (2) tombi TOML gates when tombi is on PATH (absent, or version mismatch,
# warns and skips — never fails); (3) prek delegation only when BOTH the
# prek binary AND a non-dangling repo-root config are present.
# `.git/hooks/pre-commit.legacy`, when present, is a historical backup left
# by `prek install`, not part of the chain (same as pre-commit semantics).

label="$(basename "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null)"
[ -n "$label" ] || label="repo"
TOMBI_REQUIRED="1.2.5"
fail=0

# git runs hooks with cwd at the repo top level; anchor there anyway so manual
# runs from a subdirectory behave the same. Stderr is suppressed: outside a
# repo (or with a restricted git) the guards see an empty index and pass.
cd "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || true

# --- (1) secret-material guard (staged paths only) ---------------------------
staged=$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null)
if printf '%s\n' "$staged" | grep -qE '(^|/)([^/]*\.agekey|age\.txt|[^/]*\.pem|id_rsa[^/]*|\.env)$'; then
    echo "$label pre-commit: refusing secret material (*.agekey, age.txt, *.pem, id_rsa*, .env):" >&2
    printf '%s\n' "$staged" | grep -E '(^|/)([^/]*\.agekey|age\.txt|[^/]*\.pem|id_rsa[^/]*|\.env)$' >&2
    fail=1
fi
[ "$fail" -eq 0 ] || exit 1

# --- (2) tombi TOML gates (best-effort; never brick the commit) --------------
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

# --- (3) prek delegation (only when binary AND config both resolve) ----------
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
