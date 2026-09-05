#!/usr/bin/env bash
# run.sh — host-runnable fixture tests for scripts/msb-generation-converge.sh.
#
# Everything runs under mktemp -d roots with a stub msb (fake-msb.sh) wrapped
# in a fake nix store path (<root>/nix/store/<hash32>-microsandbox-0.6.16/
# bin/msb) so readlink -f + basename + strip exercises the REAL key
# derivation. No KVM, no real nix store, no network. Idempotent; the EXIT
# trap removes every temp root.
#
# Scenarios:
#   a  dry-run on a two-generation fixture mutates nothing
#   b  real converge carries state, never copies run//tmp//bin//lib/, flips,
#      keeps the same-run source; + live-sandbox refusal (exit 2, no flip)
#   c  forward-migrate refusal on a copied db -> fresh-init reset (exit 0,
#      flip, old gen untouched); + sqlite3 integrity_check variant when
#      sqlite3 is on PATH (SKIP otherwise)
#   d  unprobeable generation -> not-proven-quiesced refusal (exit 2)
#   e  resolution edges: single-gen heal then converge; two gens + no
#      current -> ambiguous refusal (exit 2)
#   f  legacy-root absorb: probed first, relocated into generations/legacy,
#      converged legacy->baked in the same run, legacy kept (same-run
#      source exemption); + live pre-generation root refusal (exit 2, db
#      NOT moved, no flip)
#   g  GC keep-rule: current target kept, newest .booted-ok gen kept,
#      non-booted empty gen collected, unprobeable gen kept (fail-closed)
#   h  --check-only exit codes: 0 converged, 1 converge needed (mismatch /
#      legacy-root / heal pending), 2 ambiguous
#   i  staging sweep: dead-pid .converge-tmp-* removed, live-pid kept
#
# Exit 0 when every assertion passes (skips allowed), 1 otherwise.

set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
CONVERGE="$REPO/scripts/msb-generation-converge.sh"
STUB="$HERE/fake-msb.sh"

HASH_A=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
HASH_B=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
KEY_A=aaaaaaaaaaaa
KEY_B=bbbbbbbbbbbb

PASS=0
FAIL=0
SKIP=0
ROOTS=()
NEW_ROOT=""
RC=0
OUT=""

# --- assertions -------------------------------------------------------------

pass() { PASS=$((PASS + 1)); echo "PASS: $*"; }
fail() { FAIL=$((FAIL + 1)); echo "FAIL: $*" >&2; }
skip() { SKIP=$((SKIP + 1)); echo "SKIP: $*"; }

assert_eq() { # label expected actual
  if [[ "$2" == "$3" ]]; then pass "$1"; else fail "$1 (expected [$2], got [$3])"; fi
}
assert_file() { # label path
  if [[ -f "$2" ]]; then pass "$1"; else fail "$1 (missing file: $2)"; fi
}
assert_not_file() { # label path
  if [[ ! -f "$2" ]]; then pass "$1"; else fail "$1 (unexpected file: $2)"; fi
}
assert_dir() { # label path
  if [[ -d "$2" ]]; then pass "$1"; else fail "$1 (missing dir: $2)"; fi
}
assert_not_dir() { # label path
  if [[ ! -d "$2" ]]; then pass "$1"; else fail "$1 (unexpected dir: $2)"; fi
}
assert_absent() { # label path (symlink-aware)
  if [[ ! -e "$2" && ! -L "$2" ]]; then pass "$1"; else fail "$1 (still present: $2)"; fi
}
assert_symlink_target() { # label link expected-target
  local got
  got=$(readlink -f "$2" 2>/dev/null || true)
  assert_eq "$1" "$3" "$got"
}
assert_contains() { # label haystack needle
  if [[ "$2" == *"$3"* ]]; then pass "$1"; else fail "$1 (output missing [$3])"; fi
}

# --- fixture builders -------------------------------------------------------

new_root() {
  NEW_ROOT=$(mktemp -d /tmp/msb-gen-converge-test.XXXXXX)
  ROOTS+=("$NEW_ROOT")
}

cleanup() {
  local r
  for r in ${ROOTS[@]+"${ROOTS[@]}"}; do rm -rf "$r"; done
}
trap cleanup EXIT

# Build a fake nix-store msb wrapping the stub; prints the msb path.
make_store_msb() { # root hash32
  local dir="$1/nix/store/$2-microsandbox-0.6.16/bin"
  mkdir -p "$dir"
  cat >"$dir/msb" <<EOF
#!/usr/bin/env bash
exec bash "$STUB" "\$@"
EOF
  chmod +x "$dir/msb"
  printf '%s' "$dir/msb"
}

# Two-generation fixture: current -> gen A (carrying db/sandboxes/volumes/
#secrets plus the never-copied run//tmp//bin//lib/ entries); the baked
# target gen B is absent.
build_two_gen_fixture() { # root
  local genA="$1/.microsandbox/generations/$KEY_A"
  mkdir -p "$genA/db" "$genA/sandboxes/demo" "$genA/volumes/vol1" "$genA/secrets"
  echo "FAKE DB v1" >"$genA/db/msb.db"
  echo "sandbox-state" >"$genA/sandboxes/demo/state.json"
  echo "vol-data" >"$genA/volumes/vol1/data"
  echo "secret" >"$genA/secrets/s1"
  mkdir -p "$genA/run/agent" "$genA/tmp" "$genA/bin" "$genA/lib"
  echo "pid" >"$genA/run/agent/x.sock"
  ln -s "$genA" "$1/.microsandbox/current"
}

# Invoke the converge script against a root with the given baked msb.
# Output lands in $OUT, exit code in $RC.
run_converge() { # root msb_path [args...]
  local root="$1" bin="$2"
  shift 2
  RC=0
  OUT=$(env -u MSB_HOME HOME="$root" MSB_PATH="$bin" bash "$CONVERGE" "$@" 2>&1) || RC=$?
}

# Pre-generation legacy-root fixture: db/ + sandboxes/ directly at the
# msb root, no generations/, no current symlink.
build_legacy_root_fixture() { # root
  mkdir -p "$1/.microsandbox/db" "$1/.microsandbox/sandboxes/demo"
  echo "LEGACY DB v0" >"$1/.microsandbox/db/msb.db"
  echo "sandbox-state" >"$1/.microsandbox/sandboxes/demo/state.json"
}

# --- scenarios --------------------------------------------------------------

scenario_a() {
  echo "--- a: dry-run on a two-generation fixture mutates nothing"
  new_root
  local root="$NEW_ROOT"
  local baked
  baked=$(make_store_msb "$root" "$HASH_B")
  # Exercise readlink -f canonicalization: MSB_PATH is a SYMLINK to the
  # store msb; the key must still derive from the resolved target.
  mkdir -p "$root/wrap/bin"
  ln -s "$baked" "$root/wrap/bin/msb"
  build_two_gen_fixture "$root"
  run_converge "$root" "$root/wrap/bin/msb" --dry-run
  assert_eq "a: dry-run exits 0" "0" "$RC"
  assert_contains "a: key derived through the symlink" "$OUT" "(generation $KEY_B)"
  assert_contains "a: dry-run prints the staged copy" "$OUT" "[dry-run] would stage"
  assert_contains "a: dry-run prints the flip" "$OUT" "would have flipped current -> $KEY_B"
  assert_symlink_target "a: current unchanged" \
    "$root/.microsandbox/current" "$root/.microsandbox/generations/$KEY_A"
  assert_eq "a: gen A db unchanged" "FAKE DB v1" \
    "$(cat "$root/.microsandbox/generations/$KEY_A/db/msb.db")"
  assert_file "a: gen A sandboxes unchanged" \
    "$root/.microsandbox/generations/$KEY_A/sandboxes/demo/state.json"
  assert_file "a: gen A run/ entry survives" \
    "$root/.microsandbox/generations/$KEY_A/run/agent/x.sock"
  assert_not_dir "a: no gen B created" "$root/.microsandbox/generations/$KEY_B"
  if compgen -G "$root/.microsandbox/generations/.converge-tmp-*" >/dev/null; then
    fail "a: staging dir left behind"
  else
    pass "a: no staging dir left"
  fi
}

scenario_b() {
  echo "--- b: real converge carries state, flips, keeps the source"
  new_root
  local root="$NEW_ROOT"
  local baked
  baked=$(make_store_msb "$root" "$HASH_B")
  build_two_gen_fixture "$root"
  run_converge "$root" "$baked"
  assert_eq "b: converge exits 0" "0" "$RC"
  local genB="$root/.microsandbox/generations/$KEY_B"
  assert_eq "b: db carried" "FAKE DB v1" "$(cat "$genB/db/msb.db" 2>/dev/null || echo MISSING)"
  assert_file "b: sandboxes carried" "$genB/sandboxes/demo/state.json"
  assert_file "b: volumes carried" "$genB/volumes/vol1/data"
  assert_file "b: secrets carried" "$genB/secrets/s1"
  assert_not_dir "b: run/ never copied" "$genB/run"
  assert_not_dir "b: tmp/ never copied" "$genB/tmp"
  assert_not_dir "b: bin/ never copied" "$genB/bin"
  assert_not_dir "b: lib/ never copied" "$genB/lib"
  assert_symlink_target "b: current flipped" "$root/.microsandbox/current" "$genB"
  assert_dir "b: source gen kept (same-run exemption)" \
    "$root/.microsandbox/generations/$KEY_A"
  assert_contains "b: flip logged" "$OUT" "flipped current -> $KEY_B"

  echo "--- b2: live sandboxes in any gen -> refuse (exit 2, no flip)"
  new_root
  local root2="$NEW_ROOT"
  local baked2
  baked2=$(make_store_msb "$root2" "$HASH_B")
  build_two_gen_fixture "$root2"
  echo "livebox" >"$root2/.microsandbox/generations/$KEY_A/.fake-live"
  run_converge "$root2" "$baked2"
  assert_eq "b2: live gen refuses with exit 2" "2" "$RC"
  assert_contains "b2: reap command printed" "$OUT" \
    "reap with: MSB_HOME=$root2/.microsandbox/generations/$KEY_A"
  assert_symlink_target "b2: no flip" \
    "$root2/.microsandbox/current" "$root2/.microsandbox/generations/$KEY_A"
  assert_not_dir "b2: no gen B created" "$root2/.microsandbox/generations/$KEY_B"
}

scenario_c() {
  echo "--- c: forward-migrate refusal on the copied db -> fresh-init reset"
  new_root
  local root="$NEW_ROOT"
  local baked
  baked=$(make_store_msb "$root" "$HASH_B")
  build_two_gen_fixture "$root"
  echo "CORRUPT" >"$root/.microsandbox/generations/$KEY_A/db/msb.db"
  run_converge "$root" "$baked"
  assert_eq "c: reset-path converge exits 0" "0" "$RC"
  local genB="$root/.microsandbox/generations/$KEY_B"
  assert_contains "c: fresh-init logged" "$OUT" "FRESH-INIT"
  assert_dir "c: gen B db skeleton exists" "$genB/db"
  assert_not_file "c: gen B db is fresh (no carried msb.db)" "$genB/db/msb.db"
  assert_not_dir "c: gen B has no carried sandboxes" "$genB/sandboxes"
  assert_symlink_target "c: current flipped to the fresh gen" \
    "$root/.microsandbox/current" "$genB"
  assert_eq "c: gen A untouched (rollback)" "CORRUPT" \
    "$(cat "$root/.microsandbox/generations/$KEY_A/db/msb.db")"

  # Same reset via a literally-corrupt db + sqlite3 integrity_check, when
  # sqlite3 is available (the stub tolerates this db: no CORRUPT marker).
  if command -v sqlite3 >/dev/null 2>&1; then
    echo "--- c-sqlite: integrity_check failure -> fresh-init reset"
    new_root
    local root3="$NEW_ROOT"
    local baked3
    baked3=$(make_store_msb "$root3" "$HASH_B")
    build_two_gen_fixture "$root3"
    printf 'this is not a sqlite database at all' \
      >"$root3/.microsandbox/generations/$KEY_A/db/msb.db"
    run_converge "$root3" "$baked3"
    assert_eq "c-sqlite: integrity failure resets, exit 0" "0" "$RC"
    assert_not_file "c-sqlite: gen B db is fresh" \
      "$root3/.microsandbox/generations/$KEY_B/db/msb.db"
    assert_symlink_target "c-sqlite: current flipped" \
      "$root3/.microsandbox/current" "$root3/.microsandbox/generations/$KEY_B"
  else
    skip "c-sqlite: sqlite3 not on PATH (integrity_check branch untested here)"
  fi
}

scenario_d() {
  echo "--- d: unprobeable generation -> not-proven-quiesced refusal (exit 2)"
  new_root
  local root="$NEW_ROOT"
  local baked
  baked=$(make_store_msb "$root" "$HASH_B")
  build_two_gen_fixture "$root"
  : >"$root/.microsandbox/generations/$KEY_A/.fake-refuse-list"
  run_converge "$root" "$baked"
  assert_eq "d: unprobeable gen refuses with exit 2" "2" "$RC"
  assert_contains "d: not-proven-quiesced text" "$OUT" "cannot prove generation $KEY_A"
  assert_contains "d: manual remediation text" "$OUT" "remediation:"
  assert_symlink_target "d: no flip" \
    "$root/.microsandbox/current" "$root/.microsandbox/generations/$KEY_A"
  assert_not_dir "d: no gen B created" "$root/.microsandbox/generations/$KEY_B"
}

scenario_e() {
  echo "--- e1: current missing + exactly one gen -> heal, then converge"
  new_root
  local root="$NEW_ROOT"
  local baked
  baked=$(make_store_msb "$root" "$HASH_B")
  build_two_gen_fixture "$root"
  rm "$root/.microsandbox/current"
  run_converge "$root" "$baked"
  assert_eq "e1: heal+converge exits 0" "0" "$RC"
  assert_contains "e1: heal logged" "$OUT" "healing missing 'current' symlink -> $KEY_A"
  assert_symlink_target "e1: converged after the heal" \
    "$root/.microsandbox/current" "$root/.microsandbox/generations/$KEY_B"

  echo "--- e2: current missing + two gens -> ambiguous refusal (exit 2)"
  new_root
  local root2="$NEW_ROOT"
  local baked2
  baked2=$(make_store_msb "$root2" "$HASH_B")
  build_two_gen_fixture "$root2"
  rm "$root2/.microsandbox/current"
  mkdir -p "$root2/.microsandbox/generations/cccccccccccc"
  run_converge "$root2" "$baked2"
  assert_eq "e2: ambiguous refuses with exit 2" "2" "$RC"
  assert_contains "e2: ambiguity text" "$OUT" "ambiguous state"
  assert_absent "e2: current left absent" "$root2/.microsandbox/current"
  assert_not_dir "e2: no gen B created" "$root2/.microsandbox/generations/$KEY_B"
}

scenario_f() {
  echo "--- f1: legacy-root absorb -> generations/legacy -> converge to baked"
  new_root
  local root="$NEW_ROOT"
  local baked
  baked=$(make_store_msb "$root" "$HASH_B")
  build_legacy_root_fixture "$root"
  run_converge "$root" "$baked"
  assert_eq "f1: absorb+converge exits 0" "0" "$RC"
  assert_contains "f1: absorb logged" "$OUT" "absorbed pre-generation state as generation 'legacy'"
  assert_not_dir "f1: root db moved out" "$root/.microsandbox/db"
  assert_not_dir "f1: root sandboxes moved out" "$root/.microsandbox/sandboxes"
  local legacy="$root/.microsandbox/generations/legacy"
  local genB="$root/.microsandbox/generations/$KEY_B"
  assert_dir "f1: legacy gen kept post-run (same-run source exemption)" "$legacy"
  assert_eq "f1: legacy db carried" "LEGACY DB v0" "$(cat "$legacy/db/msb.db" 2>/dev/null || echo MISSING)"
  assert_file "f1: legacy sandboxes carried" "$legacy/sandboxes/demo/state.json"
  assert_symlink_target "f1: current flipped to the baked gen" "$root/.microsandbox/current" "$genB"
  assert_eq "f1: baked gen db carried" "LEGACY DB v0" "$(cat "$genB/db/msb.db" 2>/dev/null || echo MISSING)"
  assert_file "f1: baked gen sandboxes carried" "$genB/sandboxes/demo/state.json"

  echo "--- f2: live pre-generation root -> refuse (exit 2, db NOT moved, no flip)"
  new_root
  local root2="$NEW_ROOT"
  local baked2
  baked2=$(make_store_msb "$root2" "$HASH_B")
  build_legacy_root_fixture "$root2"
  echo "livebox" >"$root2/.microsandbox/.fake-live"
  run_converge "$root2" "$baked2"
  assert_eq "f2: live root refuses with exit 2" "2" "$RC"
  assert_contains "f2: refusal names the root" "$OUT" "LIVE sandboxes in the pre-generation root"
  assert_contains "f2: reap command names MSB_HOME=<root>" "$OUT" \
    "reap with: MSB_HOME=$root2/.microsandbox "
  assert_file "f2: root db NOT moved" "$root2/.microsandbox/db/msb.db"
  assert_file "f2: root sandboxes NOT moved" "$root2/.microsandbox/sandboxes/demo/state.json"
  assert_not_dir "f2: no generations/legacy created" "$root2/.microsandbox/generations/legacy"
  assert_absent "f2: no flip (current absent)" "$root2/.microsandbox/current"
}

scenario_g() {
  echo "--- g: GC keep-rule {current, newest .booted-ok} + fail-closed probes"
  new_root
  local root="$NEW_ROOT"
  local baked
  baked=$(make_store_msb "$root" "$HASH_B")
  build_two_gen_fixture "$root"
  run_converge "$root" "$baked"
  assert_eq "g: initial converge exits 0" "0" "$RC"
  local gens="$root/.microsandbox/generations"
  # Plant: a .booted-ok gen, a plain empty gen, and an unprobeable gen.
  mkdir -p "$gens/gggggggggggg/db" "$gens/hhhhhhhhhhhh/db" "$gens/iiiiiiiiiiii/db"
  : >"$gens/gggggggggggg/.booted-ok"
  : >"$gens/iiiiiiiiiiii/db/msb.db"
  : >"$gens/iiiiiiiiiiii/.fake-refuse-list"
  run_converge "$root" "$baked"
  assert_eq "g: already-converged run exits 0" "0" "$RC"
  assert_contains "g: no-op converge" "$OUT" "already converged on generation $KEY_B"
  assert_dir "g: current target kept" "$gens/$KEY_B"
  assert_dir "g: .booted-ok gen kept" "$gens/gggggggggggg"
  assert_contains "g: booted-ok keep logged" "$OUT" "newest generation carrying .booted-ok"
  assert_not_dir "g: non-booted empty gen collected" "$gens/hhhhhhhhhhhh"
  assert_dir "g: unprobeable gen kept (fail-closed)" "$gens/iiiiiiiiiiii"
  assert_contains "g: fail-closed keep logged" "$OUT" "liveness probe errored"
  assert_not_dir "g: previous source collected (no exemption on a no-op run)" "$gens/$KEY_A"
}

scenario_h() {
  echo "--- h: --check-only exit codes"
  new_root
  local root="$NEW_ROOT"
  local baked
  baked=$(make_store_msb "$root" "$HASH_B")
  build_two_gen_fixture "$root"
  run_converge "$root" "$baked" --check-only
  assert_eq "h1: mismatch -> 1" "1" "$RC"
  assert_contains "h1: needs-converge text" "$OUT" "needs converge"
  run_converge "$root" "$baked"
  assert_eq "h1: real converge exits 0" "0" "$RC"
  run_converge "$root" "$baked" --check-only
  assert_eq "h2: converged -> 0" "0" "$RC"
  assert_contains "h2: converged text" "$OUT" "converged: current generation == baked key"

  new_root
  local root2="$NEW_ROOT"
  local baked2
  baked2=$(make_store_msb "$root2" "$HASH_B")
  build_legacy_root_fixture "$root2"
  run_converge "$root2" "$baked2" --check-only
  assert_eq "h3: legacy-root pending -> 1" "1" "$RC"
  assert_contains "h3: absorb-pending text" "$OUT" "absorption into generations/legacy pending"
  assert_file "h3: check-only moved nothing" "$root2/.microsandbox/db/msb.db"

  new_root
  local root3="$NEW_ROOT"
  local baked3
  baked3=$(make_store_msb "$root3" "$HASH_B")
  build_two_gen_fixture "$root3"
  rm "$root3/.microsandbox/current"
  run_converge "$root3" "$baked3" --check-only
  assert_eq "h4: heal pending -> 1" "1" "$RC"
  assert_contains "h4: heal-pending text" "$OUT" "heal pending"
  assert_absent "h4: check-only did not heal" "$root3/.microsandbox/current"

  new_root
  local root4="$NEW_ROOT"
  local baked4
  baked4=$(make_store_msb "$root4" "$HASH_B")
  build_two_gen_fixture "$root4"
  rm "$root4/.microsandbox/current"
  mkdir -p "$root4/.microsandbox/generations/cccccccccccc"
  run_converge "$root4" "$baked4" --check-only
  assert_eq "h5: ambiguous -> 2" "2" "$RC"
  assert_contains "h5: ambiguous text" "$OUT" "ambiguous"
}

scenario_i() {
  echo "--- i: staging sweep removes dead-pid dirs, keeps live-pid dirs"
  new_root
  local root="$NEW_ROOT"
  local baked
  baked=$(make_store_msb "$root" "$HASH_B")
  build_two_gen_fixture "$root"
  run_converge "$root" "$baked"
  assert_eq "i: initial converge exits 0" "0" "$RC"
  local gens="$root/.microsandbox/generations"
  # $$ is this runner's pid — alive for the whole run. 999999 is dead.
  mkdir -p "$gens/.converge-tmp-999999" "$gens/.converge-tmp-$$"
  run_converge "$root" "$baked"
  assert_eq "i: sweep run exits 0" "0" "$RC"
  assert_not_dir "i: dead-pid staging removed" "$gens/.converge-tmp-999999"
  assert_dir "i: live-pid staging kept" "$gens/.converge-tmp-$$"
  assert_contains "i: live-pid keep logged" "$OUT" "pid $$ is alive"
}

main() {
  scenario_a
  scenario_b
  scenario_c
  scenario_d
  scenario_e
  scenario_f
  scenario_g
  scenario_h
  scenario_i
  echo
  echo "msb-generation-converge fixtures: $PASS passed, $FAIL failed, $SKIP skipped"
  [[ "$FAIL" -eq 0 ]]
}

main "$@"
