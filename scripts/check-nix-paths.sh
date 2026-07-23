#!/usr/bin/env bash
# check-nix-paths.sh — active enforcement of the nix-purity invariants.
#
# Fails on:
#   (1) `nix ... --impure` invocations in shell scripts and nix code.
#       Comments and this linter's own source are skipped.
#   (2) `builtins.getFlake ... toString ...` — impure getFlake with a runtime
#       path string. Use a flake input instead.
#   (3) `builtins.path { ... }` without a `filter =` field — unbounded path
#       copy into the store (closes the B14 class of disk-exhausting evals).
#   (4) `cleanSourceWith { ... }` without a `filter =` field — same problem
#       via the lib helper.
#   (5) Bare repo-root path literals in nix code (outside `src =`/`lockFile =`
#       fields, which are the bounded escape hatches).
#   (6) Impure-pattern references in docs/**/*.md (including docs/migration/):
#       `getFlake ... toString`, `nix eval --impure <arg>`, `toString ./.`.
#       Docs are where these patterns historically leaked into subagent-run
#       regression gates (see docs/nix-store-accumulation-report.md), so the
#       docs tree is scanned too.
#
# Files scanned: *.nix under the repo root (flake.nix, nix/, templates/),
#                plus *.sh under scripts/ and the justfile,
#                plus *.md under docs/ (Check 6).
#
# Allowlists:
#   * Line level: lines matching `# allow: <reason>` are skipped in every
#     scanned file type (belt-and-suspenders in docs). Use sparingly — every
#     allowlist entry is a documented purity exception.
#   * File level (docs only): DOCS_ALLOWLIST below names whole-document
#     discussion contexts where the patterns legitimately appear in narrative
#     prose, code blocks, and rule statements. Line-comments in markdown prose
#     would be fragile and noisy; a file allowlist is explicit, auditable,
#     and in one place. Add a file here ONLY when the document's purpose is
#     to discuss/forbid the pattern, not to invoke it.
#
# Wired into `just lint-nix` and `just verify`.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

violations=()
add_violation() {
    violations+=("$1")
}

# File-level docs allowlist for Check 6. These three files are whole-document
# discussion contexts where the impure patterns appear in narrative prose,
# code blocks, and rule statements — they document/forbid the pattern rather
# than invoke it:
#   docs/nix-purity.md                              — FORBIDS the pattern (rules)
#   docs/nix-store-accumulation-report.md           — incident narrative
#   docs/migration/nix-store-gc-remediation-spec.md — spec BEFORE examples
DOCS_ALLOWLIST=(
    "docs/nix-purity.md"
    "docs/nix-store-accumulation-report.md"
    "docs/migration/nix-store-gc-remediation-spec.md"
)

# Gather the file list (skip this script itself).
self_path="scripts/check-nix-paths.sh"
nix_files=()
while IFS= read -r -d '' f; do
    nix_files+=("$f")
done < <(find flake.nix nix templates -type f -name "*.nix" -print0 2>/dev/null)

sh_files=()
while IFS= read -r -d '' f; do
    [ "$f" = "$self_path" ] && continue
    sh_files+=("$f")
done < <(find scripts justfile -type f \( -name "*.sh" -o -name "justfile" \) -print0 2>/dev/null)

# Gather docs files for Check 6 (skip the file-level allowlist).
docs_files=()
while IFS= read -r -d '' f; do
    skip=0
    for allowed in "${DOCS_ALLOWLIST[@]}"; do
        [ "$f" = "$allowed" ] && skip=1 && break
    done
    [ "$skip" -eq 1 ] && continue
    docs_files+=("$f")
done < <(find docs -type f -name "*.md" -print0 2>/dev/null)

is_allowlisted() {
    case "$1" in *"# allow:"*) return 0 ;; esac
    return 1
}

# ---------------------------------------------------------------------------
# Check 1: `nix ... --impure` invocations (skip comments + this script).
# ---------------------------------------------------------------------------
for f in "${sh_files[@]}" "${nix_files[@]}"; do
    [ -f "$f" ] || continue
    lineno=0
    while IFS= read -r line; do
        lineno=$((lineno + 1))
        if is_allowlisted "$line"; then continue; fi
        # Skip comment lines (first non-space char is #).
        stripped="${line#"${line%%[![:space:]]*}"}"
        case "$stripped" in \#*) continue ;; esac
        case "$line" in
            *nix\ *--impure*|*nix-shell*--impure*)
                add_violation "$f:$lineno: nix invocation with --impure (use a flake input or cleanSourceWith): $line"
                ;;
        esac
    done < "$f"
done

# ---------------------------------------------------------------------------
# Check 2: builtins.getFlake with toString.
# ---------------------------------------------------------------------------
for f in "${nix_files[@]}"; do
    [ -f "$f" ] || continue
    lineno=0
    while IFS= read -r line; do
        lineno=$((lineno + 1))
        if is_allowlisted "$line"; then continue; fi
        # Match `builtins.getFlake` and `toString` anywhere on the same line.
        case "$line" in
            *builtins.getFlake*toString*)
                add_violation "$f:$lineno: impure builtins.getFlake with toString (use a flake input): $line"
                ;;
        esac
    done < "$f"
done

# ---------------------------------------------------------------------------
# Check 3 + 4: builtins.path / cleanSourceWith without filter.
# ---------------------------------------------------------------------------
check_filter_in_block() {
    local file="$1" start_line="$2"
    awk -v start="$start_line" '
        NR >= start && NR <= start + 15 {
            if ($0 ~ /[[:space:]]filter[[:space:]]*=/) found=1
        }
        END { exit found ? 0 : 1 }
    ' "$file"
}

for f in "${nix_files[@]}"; do
    [ -f "$f" ] || continue
    lineno=0
    while IFS= read -r line; do
        lineno=$((lineno + 1))
        if is_allowlisted "$line"; then continue; fi
        case "$line" in
            *builtins.path*\{*)
                if ! check_filter_in_block "$f" "$lineno"; then
                    add_violation "$f:$lineno: builtins.path { ... } without filter = (unbounded store copy)"
                fi
                ;;
            *cleanSourceWith*\{*)
                if ! check_filter_in_block "$f" "$lineno"; then
                    add_violation "$f:$lineno: cleanSourceWith { ... } without filter = (unbounded store copy)"
                fi
                ;;
        esac
    done < "$f"
done

# ---------------------------------------------------------------------------
# Check 5: repo-root path literals in nix code (outside src=/lockFile= fields).
# Uses process substitution so add_violation mutates the parent shell's array
# (a `cmd | while read` form would silently lose violations to a subshell).
# ---------------------------------------------------------------------------
for f in "${nix_files[@]}"; do
    [ -f "$f" ] || continue
    while IFS= read -r v; do
        add_violation "$v"
    done < <(awk -v file="$f" '
        {
            line = $0
            # Skip comment lines (first non-space is #).
            stripped = line
            sub(/^[ \t]+/, "", stripped)
            if (substr(stripped, 1, 1) == "#") next
            # Strip double-quoted and single-quoted strings so path literals
            # INSIDE strings (e.g. shell-hook lines like
            # `export VAR="$HOME/.cache/ai-workbench-msb"`) do not match.
            # This is a conservative heuristic: it cannot see multi-line
            # nix `''''` strings, but those are caught indirectly because
            # the lines inside rarely have a top-level nix `=` assignment.
            gsub(/"[^"]*"/, "", line)
            gsub(/'\''[^'\'']*'\''/, "", line)
            # Only flag lines that have a nix assignment AND a ../ literal
            # on the RHS. This filters out shell commands like `cd ../..`
            # inside buildPhase strings (no `=` on those lines).
            if (line ~ /=.*(\.\.\/)+/) {
                # Allow the bounded escape-hatch field names.
                if (line ~ /[[:space:]]src[[:space:]]*=/) next
                if (line ~ /[[:space:]]lockFile[[:space:]]*=/) next
                if (line ~ /[[:space:]]path[[:space:]]*=/) next
                if (line ~ /# allow:/) next
                printf "%s:%d: repo-root path literal in nix code (use cleanSourceWith + src, or a flake input): %s\n", file, NR, $0
            }
        }
    ' "$f")
done

# ---------------------------------------------------------------------------
# Check 6: impure-pattern references in docs/**/*.md.
# Patterns: `getFlake ... toString`, `nix eval --impure <arg>` (invocation
# form — a trailing flag/argument is required so backtick-quoted prose
# mentions like "`nix eval --impure` against a ..." do not match), and
# `toString ./.`. Files in DOCS_ALLOWLIST are skipped entirely (gathered
# above); `# allow:` per-line suppression also applies.
# ---------------------------------------------------------------------------
for f in "${docs_files[@]}"; do
    [ -f "$f" ] || continue
    lineno=0
    while IFS= read -r line; do
        lineno=$((lineno + 1))
        if is_allowlisted "$line"; then continue; fi
        case "$line" in
            *getFlake*toString*)
                add_violation "$f:$lineno: docs reference to getFlake with toString (impure self-referential flake fetching): $line"
                ;;
            *"nix eval --impure -"*|*"nix eval --impure '"*)
                add_violation "$f:$lineno: docs reference to 'nix eval --impure' invocation (use the git-filtered .# ref form): $line"
                ;;
            *"toString ./."*)
                add_violation "$f:$lineno: docs reference to 'toString ./.' (raw working-tree copy — use a flake input or the .# ref form): $line"
                ;;
        esac
    done < "$f"
done

# ---------------------------------------------------------------------------
# Report.
# ---------------------------------------------------------------------------
if [ "${#violations[@]}" -gt 0 ]; then
    echo "FAIL: ${#violations[@]} nix-purity violation(s) found:" >&2
    echo "" >&2
    for v in "${violations[@]}"; do
        echo "  $v" >&2
    done
    echo "" >&2
    echo "To suppress a false positive, append '# allow: <reason>' to the line." >&2
    echo "Use sparingly — every allowlist entry is a documented purity exception." >&2
    exit 1
fi

echo "OK: no nix-purity violations (scanned ${#nix_files[@]} nix files + ${#sh_files[@]} shell/justfile files + ${#docs_files[@]} docs files)."
