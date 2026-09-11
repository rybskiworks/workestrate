#!/usr/bin/env bash
# migrate-msb-home.sh — one-way migration from the legacy devshell msb home
# ($HOME/.cache/ai-workbench-msb) to the canonical SDK home
# ($HOME/.microsandbox, the microsandbox_utils::resolve_home default).
#
# The canonical home root is $HOME/.microsandbox; under the generations
# model the ONLY runtime home is $HOME/.microsandbox/current (a symlink into
# generations/<hash12>): the workestrate + msb-wrapped nix wrappers default
# unset/empty MSB_HOME to $HOME/.microsandbox/current (see
# docs/runtime-provisioning.md and
# docs/migration/50-decisions/0037-msb-state-generations.md), the
# devshell no longer exports MSB_HOME/MSB_PATH, and the Rust mirrors
# (reconcile.rs/policy_file.rs msb_home()) treat empty MSB_HOME as unset.
# This script remains the legacy-cache-home → canonical-root migration,
# which the generation converge delegates to. It reconciles pre-convergence
# state where msb DBs/caches exist under BOTH homes (skew) or only under
# the legacy home (unmigrated).
#
# Idempotent: re-running after a successful migration reports "nothing to do".
#
# Usage:
#   scripts/migrate-msb-home.sh [--dry-run] [--check-only] [--rollback <ts>] [-h|--help]
#
#   --dry-run     Print the actions that would be taken; mutate nothing.
#   --check-only  Report skew status only: exit 0 = no action needed
#                 (canonical-only or fresh), 1 = needs migration (legacy-only
#                 or skewed), 2 = error. No msb binary required.
#   --rollback <ts>
#                 Restore both homes from the backup recorded under
#                 $HOME/.cache/workestrate-msb-migrate.<ts>/.
#   -h, --help    Print this help.
#
# Backups: timestamped dirs under $HOME/.cache/workestrate-msb-migrate.<ts>/
# holding `legacy/` + `canon/` copies of both pre-migration homes. The DB
# winner rule never copies older-over-newer: the DB with the newest
# seaql_migrations entry (via sqlite3 when available, else newest mtime)
# wins; the loser survives in the backup. Rollback restores from the backup.
#
# Preconditions for migrate: `msb` reachable (MSB_PATH or PATH) and
# `msb list` empty under BOTH homes (no live sandboxes); otherwise aborts.

set -euo pipefail

LEGACY="$HOME/.cache/ai-workbench-msb"
CANON="$HOME/.microsandbox"
BACKUP_PARENT="$HOME/.cache"

DRY_RUN=0
CHECK_ONLY=0
ROLLBACK_TS=""

usage() {
  tail -n +2 "$0" | sed -n '1,/^$/p' | sed 's/^# \{0,1\}//' | head -n 40
}

log() { echo "[migrate-msb-home] $*"; }
warn() { echo "[migrate-msb-home] WARNING: $*" >&2; }
die() { echo "[migrate-msb-home] ERROR: $*" >&2; exit "${2:-2}"; }

# Mutating step: echo the command; skip it under --dry-run.
run() {
  echo "+ $*"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    "$@"
  fi
}

msb_bin() {
  if [[ -n "${MSB_PATH:-}" ]]; then
    printf '%s' "$MSB_PATH"
  else
    command -v msb || true
  fi
}

# cp flavor: GNU --reflink=auto when supported, else plain cp -a.
cp_a() {
  if cp --help 2>/dev/null | grep -q -- '--reflink'; then
    cp -a --reflink=auto "$@"
  else
    cp -a "$@"
  fi
}

# Newest-migration marker for a msb DB: the latest seaql_migrations name via
# sqlite3 when available, else the file mtime. Prints "<marker>"; empty DB
# path (missing file) prints "".
db_marker() {
  local db="$1"
  if [[ ! -f "$db" ]]; then
    echo ""
    return
  fi
  if command -v sqlite3 >/dev/null 2>&1; then
    local name
    name=$(sqlite3 "$db" "SELECT name FROM seaql_migrations ORDER BY name DESC LIMIT 1;" 2>/dev/null || true)
    if [[ -n "$name" ]]; then
      echo "mig:$name"
      return
    fi
  fi
  local mtime
  if mtime=$(stat -c '%Y' "$db" 2>/dev/null); then
    echo "mtime:$mtime"
  elif mtime=$(stat -f '%m' "$db" 2>/dev/null); then
    echo "mtime:$mtime"
  else
    echo "mtime:0"
  fi
}

# Pure emptiness predicate over captured `msb list` output: drops the header
# row (NAME/INSTANCE/SANDBOX...), blank lines, and explicit empty-store
# markers; anything left is a live sandbox.
list_output_empty() {
  local out="$1" rest
  rest=$(echo "$out" | grep -vE '^\s*$' | grep -vE '^(NAME|INSTANCE|SANDBOX)\b' | grep -vE 'No .* (found|sandbox)' || true)
  [[ -z "$rest" ]]
}

msb_image_count() {
  local home="$1" bin="$2" out
  out=$(MSB_HOME="$home" "$bin" image ls 2>/dev/null) || { echo "unknown"; return 0; }
  # The trailing `|| true` absorbs grep's exit-1-when-nothing-selected under
  # `pipefail`; wc still prints the (zero) count.
  echo "$out" | grep -vE '^\s*$' | grep -vE '^REFERENCE\b' | grep -vE '^No .* found\.' | wc -l | tr -d ' ' || true
}

# Skew status over the two db files: prints one of fresh|canonical|unmigrated|skewed.
skew_status() {
  local legacy_db="$LEGACY/db/msb.db" canon_db="$CANON/db/msb.db"
  local l=0 c=0
  [[ -f "$legacy_db" ]] && l=1
  [[ -f "$canon_db" ]] && c=1
  if [[ "$l" -eq 1 && "$c" -eq 1 ]]; then echo "skewed"
  elif [[ "$l" -eq 1 ]]; then echo "unmigrated"
  elif [[ "$c" -eq 1 ]]; then echo "canonical"
  else echo "fresh"
  fi
}

do_check_only() {
  local st
  st=$(skew_status)
  case "$st" in
    fresh) log "no msb DB under legacy ($LEGACY) or canonical ($CANON): nothing to do"; return 0 ;;
    canonical) log "canonical DB exists, no legacy DB: nothing to do"; return 0 ;;
    unmigrated) log "legacy DB exists but canonical is missing: needs migration"; return 1 ;;
    skewed) log "legacy AND canonical DBs exist: needs migration (skewed)"; return 1 ;;
  esac
}

do_rollback() {
  local ts="$1"
  local backup="$BACKUP_PARENT/workestrate-msb-migrate.$ts"
  [[ -d "$backup" ]] || die "backup not found: $backup"
  log "rolling back from $backup"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "[dry-run] would restore LEGACY=$LEGACY from $backup/legacy"
    log "[dry-run] would restore CANON=$CANON from $backup/canon"
    return 0
  fi
  if [[ -d "$backup/legacy" ]]; then
    rm -rf "$LEGACY"
    cp_a "$backup/legacy" "$LEGACY"
    log "restored $LEGACY"
  else
    warn "no legacy backup in $backup; leaving $LEGACY alone"
  fi
  if [[ -d "$backup/canon" ]]; then
    rm -rf "$CANON"
    cp_a "$backup/canon" "$CANON"
    log "restored $CANON"
  else
    warn "no canon backup in $backup; leaving $CANON alone"
  fi
  log "rollback complete (any *.migrated-away.* dirs left for manual cleanup)"
}

do_migrate() {
  local bin
  bin=$(msb_bin)
  if [[ -z "$bin" ]]; then
    if [[ "$DRY_RUN" -eq 1 ]]; then
      warn "[dry-run] msb not reachable (MSB_PATH/PATH); a real run would abort here"
      bin="<msb>"
    else
      die "msb not reachable (set MSB_PATH or put msb on PATH)"
    fi
  fi

  local st
  st=$(skew_status)
  if [[ "$st" == "fresh" || "$st" == "canonical" ]]; then
    log "status=$st: nothing to do"
    return 0
  fi
  log "status=$st: migrating LEGACY=$LEGACY -> CANON=$CANON"

  local ts
  ts=$(date +%Y%m%d-%H%M%S)
  local backup="$BACKUP_PARENT/workestrate-msb-migrate.$ts"
  log "backup root: $backup (rollback: $0 --rollback $ts)"

  # Preconditions: no live sandboxes under either home.
  if [[ "$DRY_RUN" -eq 1 && "$bin" == "<msb>" ]]; then
    log "[dry-run] would require 'msb list' empty under both homes"
  else
    local rc=0 out=""
    out=$(MSB_HOME="$LEGACY" "$bin" list 2>&1) || rc=$?
    if [[ "$rc" -ne 0 ]]; then
      die "precondition failed: 'MSB_HOME=$LEGACY $bin list' errored (aborting): $out"
    fi
    if ! list_output_empty "$out"; then
      die "precondition failed: live sandboxes under LEGACY ($LEGACY); run 'MSB_HOME=$LEGACY $bin down --all' first"
    fi
    rc=0
    out=$(MSB_HOME="$CANON" "$bin" list 2>&1) || rc=$?
    if [[ "$rc" -ne 0 ]]; then
      # A missing canonical DB makes `list` fail on first run — that is the
      # fresh-canonical case, not a blocker. Anything else aborts.
      if [[ ! -f "$CANON/db/msb.db" ]]; then
        log "canonical home has no DB yet (fresh); treating canonical list as empty"
      else
        die "precondition failed: 'MSB_HOME=$CANON $bin list' errored (aborting): $out"
      fi
    elif ! list_output_empty "$out"; then
      die "precondition failed: live sandboxes under CANON ($CANON); run 'MSB_HOME=$CANON $bin down --all' first"
    fi
  fi

  # Image parity baseline (best-effort; warn-only on probe failure).
  local before_count="unknown"
  if [[ "$bin" != "<msb>" ]]; then
    before_count=$(msb_image_count "$CANON" "$bin")
    local legacy_count
    legacy_count=$(msb_image_count "$LEGACY" "$bin")
    log "image counts before: canonical=$before_count legacy=$legacy_count"
  fi

  # Backup both homes (timestamped; loser-preserving).
  run mkdir -p "$backup"
  if [[ -e "$LEGACY" ]]; then
    run cp_a "$LEGACY" "$backup/legacy"
  else
    log "no legacy home to back up"
  fi
  if [[ -e "$CANON" ]]; then
    run cp_a "$CANON" "$backup/canon"
  else
    log "no canonical home to back up (fresh)"
  fi

  # DB winner (never older-over-newer): newest seaql_migrations entry, else
  # newest mtime; single-DB and no-DB cases fall through to fresh.
  local legacy_db="$LEGACY/db/msb.db" canon_db="$CANON/db/msb.db"
  local lmark cmark
  lmark=$(db_marker "$legacy_db")
  cmark=$(db_marker "$canon_db")
  log "db markers: legacy='${lmark:-<none>}' canonical='${cmark:-<none>}'"
  if [[ -z "$lmark" && -z "$cmark" ]]; then
    log "neither home has a DB; canonical starts fresh"
    run mkdir -p "$CANON/db"
  elif [[ -n "$lmark" && -z "$cmark" ]]; then
    log "only legacy DB exists; adopting it as canonical"
    run mkdir -p "$CANON/db"
    run cp_a "$legacy_db" "$canon_db"
  elif [[ -z "$lmark" && -n "$cmark" ]]; then
    log "only canonical DB exists; keeping it"
  else
    if [[ "$lmark" > "$cmark" ]]; then
      log "legacy DB is newer ($lmark > $cmark); adopting legacy as canonical (loser preserved in backup)"
      run cp_a "$legacy_db" "$canon_db"
    else
      log "canonical DB is newest-or-tied ($cmark >= $lmark); keeping canonical (loser preserved in backup)"
    fi
  fi

  # Cache merge: no-clobber copy of legacy top-level entries, skipping the
  # build/runtime dirs (bin/lib/run/tmp/sandboxes) and db/ (handled above).
  if [[ -d "$LEGACY" ]]; then
    for entry in "$LEGACY"/*; do
      [[ -e "$entry" ]] || continue
      base=$(basename "$entry")
      case "$base" in
        bin|lib|run|tmp|sandboxes|db) log "skipping $base/ (build/runtime/DB-managed)"; continue ;;
      esac
      echo "+ cp -a -n [--reflink=auto] $entry $CANON/$base"
      if [[ "$DRY_RUN" -eq 0 ]]; then
        mkdir -p "$CANON"
        if cp --help 2>/dev/null | grep -q -- '--reflink'; then
          cp -a -n --reflink=auto "$entry" "$CANON/$base" 2>/dev/null || cp -a -n "$entry" "$CANON/$base" || true
        else
          cp -a -n "$entry" "$CANON/$base" || true
        fi
      fi
    done
  fi

  # Cutover: rename the legacy home away + leave a README with the rollback.
  local retired="$LEGACY.migrated-away.$ts"
  run mv "$LEGACY" "$retired"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    cat > "$retired/README.migrated-away.txt" <<EOF
This legacy msb home was migrated to the canonical home on $(date -u +%Y-%m-%dT%H:%M:%SZ).

  legacy (this dir): $retired
  canonical:         $CANON
  backup:            $backup

The canonical home ($CANON) is the SDK default (MSB_HOME unset/empty).
To undo: $0 --rollback $ts
After verifying the canonical home, this directory can be deleted.
EOF
  else
    log "[dry-run] would write $retired/README.migrated-away.txt"
  fi

  # Verify: image parity + sqlite integrity + doctor (all warn-only).
  if [[ "$bin" != "<msb>" && "$DRY_RUN" -eq 0 ]]; then
    local after_count
    after_count=$(msb_image_count "$CANON" "$bin")
    if [[ "$before_count" != "unknown" && "$after_count" != "unknown" && "$before_count" != "$after_count" ]]; then
      warn "image count changed: before=$before_count after=$after_count (legacy images may need reload)"
    else
      log "image count after: $after_count (before: $before_count)"
    fi
    if command -v sqlite3 >/dev/null 2>&1; then
      local integrity
      integrity=$(sqlite3 "$canon_db" "PRAGMA integrity_check;" 2>&1 || echo "probe-failed")
      if [[ "$integrity" == "ok" ]]; then
        log "sqlite integrity_check: ok"
      else
        warn "sqlite integrity_check: $integrity"
      fi
    else
      warn "sqlite3 not available; skipping PRAGMA integrity_check"
    fi
    if command -v workestrate >/dev/null 2>&1; then
      workestrate doctor >/dev/null 2>&1 || warn "'workestrate doctor' reports issues (see output above)"
      log "workestrate doctor probed (warn-only)"
    else
      warn "workestrate not on PATH; skipping doctor probe"
    fi
  else
    log "[dry-run] would verify: 'MSB_HOME=$CANON $bin image ls' parity, sqlite integrity_check, workestrate doctor"
  fi

  log "migration complete. backup=$backup rollback: $0 --rollback $ts"
}

# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --check-only) CHECK_ONLY=1; shift ;;
    --rollback)
      [[ $# -ge 2 ]] || die "--rollback requires a <ts> argument"
      ROLLBACK_TS="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
done

[[ -z "${HOME:-}" ]] && die "HOME is unset; cannot resolve msb homes"

if [[ -n "$ROLLBACK_TS" ]]; then
  [[ "$CHECK_ONLY" -eq 0 ]] || die "--rollback cannot combine with --check-only"
  do_rollback "$ROLLBACK_TS"
  exit 0
fi

if [[ "$CHECK_ONLY" -eq 1 ]]; then
  do_check_only
  exit $?
fi

do_migrate
