#!/usr/bin/env bash
# msb-generation-converge.sh — converge msb state into per-generation homes.
#
# A GENERATION is the mutable state of one pinned msb build. Its key is the
# 12-char prefix of the 32-char base32 hash segment of the resolved
# microsandbox nix store path: readlink -f of the baked msb
# (/nix/store/<hash32>-microsandbox-<ver>/bin/msb) yields the store-dir
# basename; stripping from the first "-microsandbox-" leaves <hash32>, whose
# first 12 chars are the key. A non-store msb (raw PATH install) is key
# "unmanaged" — single-generation legacy behavior, nothing to converge.
#
# Layout: $HOME/.microsandbox/generations/<hash12>/{db,sandboxes,run,...}
# plus $HOME/.microsandbox/current (a symlink; atomic flip = tmp symlink +
# rename under a .flip.lock flock) and per-generation .booted-ok markers
# written by the runtime on first verified up (NOT by this script). 12-char
# keys keep generation paths inside the fork's 59-char unix-socket budget.
#
# ONE resolution rule (mirrors the Rust side,
# control/agentctl/src/microsandbox/generation.rs):
#   non-empty $MSB_HOME verbatim — an explicit override: converge manages
#     only the canonical root, so overrides are out of scope (nothing to do)
#   > current symlink target
#   > current missing + exactly one generation dir -> heal current to it
#   > legacy root has db/ directly -> pre-generation home, ABSORBED as
#     generations/legacy
#   > fresh (no db, no generations).
#
# The legacy devshell cache home ($HOME/.cache/ai-workbench-msb) is absorbed
# FIRST by delegating to scripts/migrate-msb-home.sh: the DB winner rule
# (newest seaql_migrations entry, never older-over-newer, loser preserved in
# a timestamped backup under ~/.cache/workestrate-msb-migrate.<ts>) is
# inherited from that script, not reimplemented here.
#
# Usage:
#   scripts/msb-generation-converge.sh [--dry-run] [--check-only] [-h|--help]
#
#   --dry-run     Print the actions that would be taken; mutate nothing.
#   --check-only  Report converge status only: exit 0 = converged (or
#                 unmanaged / explicit MSB_HOME override — nothing to do),
#                 1 = converge needed (current generation != baked key,
#                 legacy-root absorb pending, single-generation heal pending,
#                 or cache-home migration pending), 2 = error. No msb binary
#                 required.
#   -h, --help    Print this help.
#
# Env: MSB_PATH (baked msb binary; else `command -v msb`), HOME (required).
#
# Safety properties:
#   - Quiesce gate REFUSES (exit 2, never partial) when any generation has
#     live sandboxes or cannot be proven quiesced; per-generation reap
#     commands are printed.
#   - Converge failure (copy error, integrity non-ok, schema refusal)
#     FRESH-INITs the NEW generation (db/ skeleton); the old generation is
#     untouched — it IS the rollback.
#   - The current symlink flips ONLY after a verified converge.
#   - GC keeps exactly {current target, newest other generation carrying
#     .booted-ok}; liveness probes fail-closed (probe error -> KEEP).

set -euo pipefail

DRY_RUN=0
CHECK_ONLY=0

usage() {
  tail -n +2 "$0" | sed -n '1,/^$/p' | sed 's/^# \{0,1\}//' | head -n 70
}

log() { echo "[msb-gen-converge] $*"; }
warn() { echo "[msb-gen-converge] WARNING: $*" >&2; }
die() { echo "[msb-gen-converge] ERROR: $*" >&2; exit "${2:-2}"; }

# Mutating step: echo the command; skip it under --dry-run.
run() {
  echo "+ $*"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    "$@"
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

msb_bin() {
  if [[ -n "${MSB_PATH:-}" ]]; then
    printf '%s' "$MSB_PATH"
  else
    command -v msb || true
  fi
}

# Pure emptiness predicate over captured `msb list` output (same semantics
# as migrate-msb-home.sh): drops the header row, blank lines, and explicit
# empty-store markers; anything left is a live sandbox.
list_output_empty() {
  local out="$1" rest
  rest=$(echo "$out" | grep -vE '^\s*$' | grep -vE '^(NAME|INSTANCE|SANDBOX)\b' | grep -vE 'No .* (found|sandbox)' || true)
  [[ -z "$rest" ]]
}

# Generation key from an msb path — mirrors
# generation.rs::generation_key_from_msb_path: canonicalize, require the
# <store-dir>/bin/msb tail shape, split the store-dir basename at the FIRST
# "-microsandbox-", require the left segment to be exactly 32 lowercase
# base32 chars, take its first 12. Any mismatch -> "unmanaged".
gen_key_from_msb_path() {
  local p="$1" canonical bindir storedir base hash
  canonical=$(readlink -f "$p" 2>/dev/null || true)
  [[ -n "$canonical" ]] || { echo "unmanaged"; return; }
  [[ "$(basename "$canonical")" == "msb" ]] || { echo "unmanaged"; return; }
  bindir=$(dirname "$canonical")
  [[ "$(basename "$bindir")" == "bin" ]] || { echo "unmanaged"; return; }
  storedir=$(dirname "$bindir")
  base=$(basename "$storedir")
  [[ "$base" == *"-microsandbox-"* ]] || { echo "unmanaged"; return; }
  hash=${base%%-microsandbox-*}
  if [[ ${#hash} -eq 32 && "$hash" =~ ^[a-z0-9]{32}$ ]]; then
    echo "${hash:0:12}"
  else
    echo "unmanaged"
  fi
}

# Absolute paths of generation dirs under $GENS (any name, so 'legacy'
# counts), excluding interrupted-converge staging debris.
list_gen_dirs() {
  [[ -d "$GENS" ]] || return 0
  local e
  for e in "$GENS"/*; do
    [[ -d "$e" ]] || continue
    case "$(basename "$e")" in
      .converge-tmp-*) continue ;;
    esac
    echo "$e"
  done
}

# Liveness/emptiness probe for one generation dir. Echoes one of:
#   empty       list succeeded and parsed empty
#   live        list succeeded with live sandboxes
#   error       list errored AND the generation has a db (not-proven-quiesced)
#   nodb-error  list errored and there is no db (treated as empty)
gen_probe() {
  local gen="$1" bin="$2" rc=0 out=""
  out=$(MSB_HOME="$gen" "$bin" list 2>&1) || rc=$?
  if [[ "$rc" -ne 0 ]]; then
    if [[ -f "$gen/db/msb.db" ]]; then echo "error"; else echo "nodb-error"; fi
    return
  fi
  if list_output_empty "$out"; then echo "empty"; else echo "live"; fi
}

# mv -T (rename over a symlink) when GNU mv is available; else rm + mv.
rename_over() {
  local src="$1" dst="$2"
  if mv --help 2>/dev/null | grep -q -- '-T'; then
    mv -T "$src" "$dst"
  else
    rm -f "$dst"
    mv "$src" "$dst"
  fi
}

# Atomic flip of the 'current' symlink: tmp symlink + rename, under the
# .flip.lock flock when flock is available (best-effort without it).
flip_current() {
  local target="$1"
  local tmp="$ROOT/.current.tmp-$$"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "+ ln -s $target $tmp && mv -T $tmp $CURRENT   (under .flip.lock flock)"
    return 0
  fi
  mkdir -p "$ROOT"
  if command -v flock >/dev/null 2>&1; then
    (
      flock -x 9
      rm -f "$tmp"
      ln -s "$target" "$tmp"
      rename_over "$tmp" "$CURRENT"
    ) 9>"$LOCK"
  else
    warn "flock not available; flipping 'current' via tmp symlink + rename WITHOUT .flip.lock coordination"
    rm -f "$tmp"
    ln -s "$target" "$tmp"
    rename_over "$tmp" "$CURRENT"
  fi
}

# State entries converge/absorb carry between homes. NEVER: run/, tmp/,
# bin/, lib/ (runtime/build artifacts).
STATE_ENTRIES=(db sandboxes volumes snapshots secrets tls ssh mount-policy config.json)

# Rule 4 absorb: a pre-generation home (db/ directly at the root) becomes
# generations/legacy; current flips to it atomically.
absorb_legacy_root() {
  log "PRE-GENERATION HOME: $ROOT has db/ directly — absorbing state into generations/legacy"
  run mkdir -p "$GENS/legacy"
  local base entry
  for base in "${STATE_ENTRIES[@]}"; do
    entry="$ROOT/$base"
    [[ -e "$entry" ]] || continue
    if [[ "$DRY_RUN" -eq 1 ]]; then
      echo "+ mv $entry $GENS/legacy/$base   (cp -a --reflink=auto + rm fallback across filesystems)"
    else
      if ! mv "$entry" "$GENS/legacy/$base" 2>/dev/null; then
        cp_a "$entry" "$GENS/legacy/$base"
        rm -rf "$entry"
      fi
    fi
  done
  for base in run tmp bin lib; do
    [[ -e "$ROOT/$base" ]] && log "leaving runtime/build entry at root (never absorbed): $base"
  done
  flip_current "$GENS/legacy"
  log "absorbed pre-generation state as generation 'legacy'"
}

# Quiesce gate: probe EVERY existing generation dir; REFUSE (never partial)
# on live sandboxes or unproven quiescence.
quiesce_gate() {
  local gens=() g st refuse=0
  mapfile -t gens < <(list_gen_dirs)
  if [[ "${#gens[@]}" -eq 0 ]]; then
    log "quiesce gate: no generations to probe"
    return 0
  fi
  for g in "${gens[@]}"; do
    st=$(gen_probe "$g" "$BIN")
    case "$st" in
      empty) log "quiesce gate: $(basename "$g") empty" ;;
      nodb-error) log "quiesce gate: $(basename "$g") has no db and 'msb list' errored; treating as empty" ;;
      live)
        warn "quiesce gate: LIVE sandboxes in generation $(basename "$g") ($g)"
        echo "[msb-gen-converge]   reap with: MSB_HOME=$g $BIN down --all" >&2
        refuse=1 ;;
      error)
        warn "quiesce gate: cannot prove generation $(basename "$g") ($g) quiesced: it has a db but 'MSB_HOME=$g $BIN list' errored"
        echo "[msb-gen-converge]   remediation: run 'MSB_HOME=$g $BIN list' manually, resolve the error (or remove the generation if disposable), then re-run converge" >&2
        refuse=1 ;;
    esac
  done
  if [[ "$refuse" -eq 1 ]]; then
    die "quiesce gate REFUSED: live or unproven generations (see above); converge never proceeds partially"
  fi
  return 0
}

# Converge state into generations/<baked key> from src_dir ("" when fresh).
# Any failure -> rm staging + FRESH-INIT the new generation (old untouched).
converge_to_baked() {
  local src_dir="$1"
  local staging="$GENS/.converge-tmp-$$"
  local final="$GENS/$KEY"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "[dry-run] would stage $staging from ${src_dir:-<fresh>} (reflink copy of db/msb.db(+wal/shm), sandboxes, volumes, snapshots, secrets, tls, ssh, mount-policy, config.json; NEVER run/tmp/bin/lib)"
    log "[dry-run] would forward-migrate + verify with: MSB_HOME=$staging $BIN list (+ sqlite3 integrity_check when available)"
    log "[dry-run] on ANY failure would rm -rf staging and FRESH-INIT $final (db/ skeleton; old generation untouched)"
    log "[dry-run] on success would mv staging -> $final and re-verify on the final dir"
    return 0
  fi
  mkdir -p "$GENS"
  rm -rf "$staging"
  mkdir -p "$staging/db"
  local ok=1 f e
  if [[ -n "$src_dir" ]]; then
    for f in msb.db msb.db-wal msb.db-shm; do
      [[ -f "$src_dir/db/$f" ]] && { cp_a "$src_dir/db/$f" "$staging/db/$f" || ok=0; }
    done
    for e in sandboxes volumes snapshots secrets tls ssh mount-policy config.json; do
      [[ -e "$src_dir/$e" ]] && { cp_a "$src_dir/$e" "$staging/$e" || ok=0; }
    done
  fi
  # Forward-migrate with the NEW baked binary (schema refusal = failure).
  local rc=0 out=""
  if [[ "$ok" -eq 1 ]]; then
    out=$(MSB_HOME="$staging" "$BIN" list 2>&1) || rc=$?
    if [[ "$rc" -ne 0 ]]; then
      warn "forward-migrate failed: 'MSB_HOME=$staging $BIN list' exited $rc: $out"
      ok=0
    elif ! list_output_empty "$out"; then
      warn "post-migrate list not empty: $out"
      ok=0
    fi
  fi
  if [[ "$ok" -eq 1 && -f "$staging/db/msb.db" ]]; then
    if command -v sqlite3 >/dev/null 2>&1; then
      local integrity
      integrity=$(sqlite3 "$staging/db/msb.db" "PRAGMA integrity_check;" 2>&1 || echo "probe-failed")
      if [[ "$integrity" != "ok" ]]; then
        warn "sqlite integrity_check on staging db: $integrity"
        ok=0
      fi
    else
      log "sqlite3 not available; skipping PRAGMA integrity_check"
    fi
  fi
  if [[ "$ok" -ne 1 ]]; then
    rm -rf "$staging"
    warn "CONVERGE FAILED — FRESH-INIT: resetting state for generation $KEY (deterministic reset; the old generation is untouched and IS the rollback)"
    rm -rf "$final"
    mkdir -p "$final/db"
    local rc2=0 out2=""
    out2=$(MSB_HOME="$final" "$BIN" list 2>&1) || rc2=$?
    [[ "$rc2" -eq 0 ]] || die "fresh-init verify failed: 'MSB_HOME=$final $BIN list' exited $rc2: $out2"
    log "fresh-init verified: new binary initialized $final"
    return 0
  fi
  # Success: move staging into place (a stale same-key dir was quiesced by
  # the gate above and is safe to replace).
  if [[ -e "$final" ]]; then
    log "replacing stale quiesced generation dir $final"
    rm -rf "$final"
  fi
  mv "$staging" "$final"
  # Re-verify integrity + list on the final dir.
  local rc3=0 out3=""
  out3=$(MSB_HOME="$final" "$BIN" list 2>&1) || rc3=$?
  [[ "$rc3" -eq 0 ]] || die "post-move verify failed: 'MSB_HOME=$final $BIN list' exited $rc3: $out3 (generation dir in place but unverified; 'current' NOT flipped; previous generation untouched)"
  if command -v sqlite3 >/dev/null 2>&1 && [[ -f "$final/db/msb.db" ]]; then
    local integrity3
    integrity3=$(sqlite3 "$final/db/msb.db" "PRAGMA integrity_check;" 2>&1 || echo "probe-failed")
    [[ "$integrity3" == "ok" ]] || die "post-move sqlite integrity_check: $integrity3 ('current' NOT flipped; previous generation untouched)"
  fi
  log "converged state verified at $final"
}

# GC sweep: keep EXACTLY {current target, newest other gen carrying
# .booted-ok}; everything else is probed fail-closed before removal. Only
# reached after the flip completed (or when already converged), so the
# migration source can never be collected by an incomplete converge.
gc_sweep() {
  local cur_target="" keep_cur="" keep_booted="" newest_ts=0
  cur_target=$(readlink -f "$CURRENT" 2>/dev/null || true)
  [[ -n "$cur_target" ]] && keep_cur=$(basename "$cur_target")
  local gens=() g base ts st
  mapfile -t gens < <(list_gen_dirs)
  for g in "${gens[@]}"; do
    base=$(basename "$g")
    [[ "$base" == "$keep_cur" ]] && continue
    if [[ -f "$g/.booted-ok" ]]; then
      ts=$(stat -c '%Y' "$g/.booted-ok" 2>/dev/null || echo 0)
      if [[ "$ts" -gt "$newest_ts" ]]; then newest_ts=$ts; keep_booted="$base"; fi
    fi
  done
  for g in "${gens[@]}"; do
    base=$(basename "$g")
    if [[ "$base" == "$keep_cur" ]]; then
      continue
    fi
    if [[ -n "$keep_booted" && "$base" == "$keep_booted" ]]; then
      log "GC: keeping $base (newest generation carrying .booted-ok)"
      continue
    fi
    st=$(gen_probe "$g" "$BIN")
    case "$st" in
      live) log "GC: keeping $base (live sandboxes)" ;;
      error) log "GC: keeping $base (liveness probe errored — fail-closed)" ;;
      nodb-error|empty)
        log "GC: collecting generation $base (empty, not current, not newest booted-ok)"
        run rm -rf "$g" ;;
    esac
  done
  # Interrupted-converge staging debris.
  local d
  for d in "$GENS"/.converge-tmp-*; do
    [[ -d "$d" ]] || continue
    log "GC: removing interrupted-converge staging dir $(basename "$d")"
    run rm -rf "$d"
  done
  # Migration backup sweep: keep the NEWEST, remove older ones.
  local backups=() b first=1
  if compgen -G "$HOME/.cache/workestrate-msb-migrate.*" >/dev/null; then
    mapfile -t backups < <(ls -dt "$HOME"/.cache/workestrate-msb-migrate.* 2>/dev/null)
  fi
  for b in "${backups[@]}"; do
    if [[ "$first" -eq 1 ]]; then
      first=0
      continue
    fi
    log "GC: removing old migration backup $b (newest kept)"
    run rm -rf "$b"
  done
}

# --check-only: read-only status. 0 = converged/nothing-to-do,
# 1 = converge needed, 2 = error.
do_check_only() {
  # Cache-home migration pending?
  if [[ ! -d "$ROOT/db" && ! -d "$GENS" && -f "$LEGACY_CACHE_DB" ]]; then
    if [[ -x "$MIGRATE" ]]; then
      local mig_rc=0
      "$MIGRATE" --check-only >/dev/null 2>&1 || mig_rc=$?
      if [[ "$mig_rc" -eq 1 ]]; then
        log "needs converge: legacy cache-home migration pending ($LEGACY_CACHE_DB)"
        return 1
      elif [[ "$mig_rc" -ne 0 ]]; then
        log "error: migrate-msb-home.sh --check-only exited $mig_rc"
        return 2
      fi
    else
      warn "legacy cache home present but $MIGRATE is not executable"
    fi
  fi
  # Resolution (read-only).
  local cur_gen="" target=""
  if [[ -L "$CURRENT" || -e "$CURRENT" ]]; then
    target=$(readlink -f "$CURRENT" 2>/dev/null || true)
    if [[ -n "$target" && -d "$target" ]]; then
      cur_gen=$(basename "$target")
    fi
  fi
  if [[ -z "$cur_gen" ]]; then
    local gens=()
    mapfile -t gens < <(list_gen_dirs)
    if [[ "${#gens[@]}" -gt 1 ]]; then
      log "error: ambiguous — 'current' missing with multiple generation dirs (${gens[*]##*/}); manually repoint $CURRENT"
      return 2
    elif [[ "${#gens[@]}" -eq 1 ]]; then
      log "needs converge: single generation $(basename "${gens[0]}") with no 'current' symlink (heal pending)"
      return 1
    elif [[ -d "$ROOT/db" ]]; then
      log "needs converge: pre-generation home (db/ at $ROOT) — absorption into generations/legacy pending"
      return 1
    fi
  fi
  if [[ "$cur_gen" == "$KEY" ]]; then
    log "converged: current generation == baked key ($KEY)"
    return 0
  fi
  log "needs converge: current generation '${cur_gen:-<fresh>}' != baked key $KEY"
  return 1
}

do_converge() {
  log "baked msb: $BIN (generation $KEY)"

  # Legacy cache-home absorption FIRST (delegated to migrate-msb-home.sh;
  # the DB winner rule + timestamped backups are inherited from it).
  if [[ ! -d "$ROOT/db" && ! -d "$GENS" && -f "$LEGACY_CACHE_DB" ]]; then
    if [[ -x "$MIGRATE" ]]; then
      local mig_rc=0 mig_out=""
      mig_out=$("$MIGRATE" --check-only 2>&1) || mig_rc=$?
      case "$mig_rc" in
        0) log "legacy cache home present but migrate reports nothing to do" ;;
        1)
          log "legacy cache home ($HOME/.cache/ai-workbench-msb) detected — delegating to migrate-msb-home.sh before generation converge"
          if [[ "$DRY_RUN" -eq 1 ]]; then
            log "[dry-run] would run: $MIGRATE"
          else
            "$MIGRATE" || die "migrate-msb-home.sh failed; resolve it (see $MIGRATE --help) and re-run converge"
            log "cache-home migration complete (state now at canonical root; absorbed below if pre-generation)"
          fi
          ;;
        *) die "migrate-msb-home.sh --check-only errored: $mig_out" ;;
      esac
    else
      warn "legacy cache home present but $MIGRATE is not executable; skipping cache-home migration"
    fi
  fi

  # Resolve the home per the ONE rule (MSB_HOME override handled in main).
  local cur_gen="" target=""
  if [[ -L "$CURRENT" || -e "$CURRENT" ]]; then
    target=$(readlink -f "$CURRENT" 2>/dev/null || true)
    if [[ -n "$target" && -d "$target" ]]; then
      cur_gen=$(basename "$target")
      log "current generation: $cur_gen ($target)"
    else
      warn "dangling 'current' symlink ($CURRENT); treating as missing"
    fi
  fi
  if [[ -z "$cur_gen" ]]; then
    local gens=()
    mapfile -t gens < <(list_gen_dirs)
    if [[ "${#gens[@]}" -gt 1 ]]; then
      die "ambiguous state: 'current' is missing and ${#gens[@]} generation dirs exist (${gens[*]##*/}). Remediation: point $CURRENT at the intended generation (ln -s) and re-run converge"
    elif [[ "${#gens[@]}" -eq 1 ]]; then
      cur_gen=$(basename "${gens[0]}")
      log "healing missing 'current' symlink -> $cur_gen"
      flip_current "${gens[0]}"
    elif [[ -d "$ROOT/db" ]]; then
      absorb_legacy_root
      cur_gen="legacy"
    else
      log "fresh msb home (no db/, no generations/)"
    fi
  fi

  # Already converged?
  if [[ "$cur_gen" == "$KEY" ]]; then
    log "already converged on generation $KEY"
    gc_sweep
    log "converge complete (no-op)"
    return 0
  fi

  # Quiesce gate (refuse, never partial).
  quiesce_gate

  # Converge to the baked generation.
  local src_dir=""
  [[ -n "$cur_gen" ]] && src_dir="$GENS/$cur_gen"
  converge_to_baked "$src_dir"

  # ONLY THEN flip current atomically.
  flip_current "$GENS/$KEY"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "[dry-run] would have flipped current -> $KEY (nothing was mutated)"
  else
    log "flipped current -> $KEY"
  fi
  if [[ -n "$cur_gen" ]]; then
    log "previous generation '$cur_gen' left untouched (it IS the rollback): rollback = re-run converge after reinstalling the previous wrapper generation, or manually repoint 'current'"
  fi

  gc_sweep
  log "converge complete: ${cur_gen:-<fresh>} -> $KEY"
}

# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --check-only) CHECK_ONLY=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
done

[[ -z "${HOME:-}" ]] && die "HOME is unset; cannot resolve the msb state root"

ROOT="$HOME/.microsandbox"
GENS="$ROOT/generations"
CURRENT="$ROOT/current"
LOCK="$ROOT/.flip.lock"
LEGACY_CACHE_DB="$HOME/.cache/ai-workbench-msb/db/msb.db"
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
MIGRATE="$SCRIPT_DIR/migrate-msb-home.sh"

BIN=$(msb_bin)
if [[ -z "$BIN" ]]; then
  KEY="unmanaged"
else
  KEY=$(gen_key_from_msb_path "$BIN")
fi

if [[ "$KEY" == "unmanaged" ]]; then
  log "msb (${BIN:-<none found>}) is not a nix-store microsandbox path — key 'unmanaged': single-generation legacy behavior, nothing to converge"
  exit 0
fi

if [[ -n "${MSB_HOME:-}" ]]; then
  log "MSB_HOME override is set ($MSB_HOME): converge manages only the canonical root ($ROOT); explicit overrides are out of scope — nothing to do"
  exit 0
fi

if [[ "$CHECK_ONLY" -eq 1 ]]; then
  do_check_only
  exit $?
fi

do_converge
