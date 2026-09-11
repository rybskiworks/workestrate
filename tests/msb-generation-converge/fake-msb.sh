#!/usr/bin/env bash
# fake-msb.sh — stub `msb` for the msb-generation-converge fixture tests
# (tests/msb-generation-converge/run.sh). No KVM, no real db, no network.
#
# All state is keyed off MSB_HOME (default ${HOME}/.microsandbox):
#
#   --version   Print `msb 0.6.16-test`.
#   list        Exit 1 when $MSB_HOME/.fake-refuse-list exists (simulates an
#               UNPROBEABLE generation: schema refusal / unreadable db).
#               Exit 1 when $MSB_HOME/db/msb.db contains the literal CORRUPT
#               AND the home basename starts with `.converge-tmp-` (simulates
#               the NEW baked binary's forward-migrate refusal surfacing only
#               against the STAGED copy — the source generation's own runtime
#               tolerates its db, so the quiesce-gate probe of the source
#               still succeeds).
#               Otherwise print a `NAME` header plus any live sandboxes
#               recorded one-per-line in $MSB_HOME/.fake-live.
#   down --all  Truncate $MSB_HOME/.fake-live (reap the fake live set).
#
# The stub NEVER writes state on `list` (a successful probe against a home
# whose db/ exists touches nothing else) and supports nothing else.

set -euo pipefail

HOME_DIR="${MSB_HOME:-${HOME:-.}/.microsandbox}"

cmd="${1:-}"
case "$cmd" in
  --version)
    echo "msb 0.6.16-test"
    ;;
  list)
    if [[ -f "$HOME_DIR/.fake-refuse-list" ]]; then
      echo "fake-msb: refusing 'list' (marker .fake-refuse-list present)" >&2
      exit 1
    fi
    base=$(basename "$HOME_DIR")
    if [[ "$base" == .converge-tmp-* && -f "$HOME_DIR/db/msb.db" ]] &&
      grep -q CORRUPT "$HOME_DIR/db/msb.db" 2>/dev/null; then
      echo "fake-msb: forward-migrate refused: staged db schema unsupported (CORRUPT marker)" >&2
      exit 1
    fi
    echo "NAME"
    if [[ -f "$HOME_DIR/.fake-live" ]]; then
      grep -vE '^\s*$' "$HOME_DIR/.fake-live" || true
    fi
    ;;
  down)
    if [[ "${2:-}" == "--all" ]]; then
      if [[ -f "$HOME_DIR/.fake-live" ]]; then : >"$HOME_DIR/.fake-live"; fi
    else
      echo "fake-msb: unsupported down args: ${*:2}" >&2
      exit 1
    fi
    ;;
  *)
    echo "fake-msb: unsupported command: ${cmd:-<none>}" >&2
    exit 1
    ;;
esac
