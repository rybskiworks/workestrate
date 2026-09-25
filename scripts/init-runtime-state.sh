#!/usr/bin/env bash
# Initialize a fresh canonical runtime home for the package's paired runtime.
set -euo pipefail
export LC_ALL=C

fail() { printf 'workestrate-init-state: %s\n' "$*" >&2; exit 1; }
dry_run=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) dry_run=1 ;;
    -h|--help)
      printf '%s\n' 'Usage: workestrate-init-state [--dry-run]' \
        'Create a fresh private ~/.microsandbox generation for the paired runtime.' \
        'Refuses existing state and explicit MSB_HOME overrides; performs no migration.'
      exit 0 ;;
    *) fail "unknown argument: $arg" ;;
  esac
done

[[ -z "${MSB_HOME:-}" ]] || fail 'MSB_HOME is set; this command only initializes the canonical home'
[[ "${HOME:-}" == /* && -d "$HOME" ]] || fail 'HOME must name an existing absolute directory'
operator_home=$(readlink -e -- "$HOME")
[[ "$(stat -c %u -- "$operator_home")" == "$(id -u)" ]] || fail 'HOME must be owned by the invoking account'
runtime_home="$operator_home/.microsandbox"
[[ ! -e "$runtime_home" && ! -L "$runtime_home" ]] || fail "refusing existing state at $runtime_home"

: "${WORKESTRATE_INIT_MSB:?the packaged launcher must supply its paired runtime}"
paired_msb=$(readlink -e -- "$WORKESTRATE_INIT_MSB") || fail 'paired runtime is unavailable'
[[ -f "$paired_msb" && -x "$paired_msb" && "$paired_msb" == */bin/msb ]] || fail 'paired runtime must be an executable bin/msb'
runtime_dir=$(dirname -- "$(dirname -- "$paired_msb")")
runtime_name=$(basename -- "$runtime_dir")
[[ "$runtime_name" =~ ^([a-z0-9]{32})-microsandbox-.+$ ]] || fail 'paired runtime has no Nix generation identity'
generation_key=${BASH_REMATCH[1]:0:12}
generation="$runtime_home/generations/$generation_key"
if [[ "$dry_run" -eq 1 ]]; then
  printf 'Would initialize %s (generation %s)\n' "$runtime_home" "$generation_key"
  exit 0
fi

umask 077
created=0
on_exit() {
  local status=$?
  if [[ "$status" -ne 0 && "$created" -eq 1 ]]; then
    printf 'Initialization incomplete; preserved %s for inspection. No existing state was removed.\n' "$runtime_home" >&2
  fi
}
trap on_exit EXIT
# Exclusive mkdir is the final race-safe refusal of an existing root. Do not
# recursively remove a partially initialized root if another process used it.
mkdir -m 0700 -- "$runtime_home"
created=1
mkdir -m 0700 -- "$runtime_home/generations" "$generation"
(set -o noclobber; : > "$runtime_home/.flip.lock")
ln -s -- "generations/$generation_key" "$runtime_home/current"
printf 'Initialized %s (generation %s). No workloads were started.\n' "$runtime_home" "$generation_key"
