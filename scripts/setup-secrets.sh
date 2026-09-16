#!/usr/bin/env bash
# setup-secrets — DEPRECATED delegate for `workestrate secrets`.
#
# The provisioning engine now lives in the workestrate CLI:
#
#   workestrate secrets init    [--config <name> | --config-dir <dir> | --global]
#   workestrate secrets update  [--config <name> | --config-dir <dir> | --global]
#
# This script remains only as a compatibility shim. It accepts the historical
# argument shapes (init/update before OR after the --config/--config-dir/
# --global flags; --home anywhere, now a global CLI flag) and forwards them
# to `workestrate secrets`. Secret values are never accepted as command-line
# arguments: they come from environment variables, stdin (non-TTY), or an
# interactive editor.

set -euo pipefail

echo "[setup-secrets] DEPRECATED: use 'workestrate secrets' directly; this script delegates to it." >&2

# Hoist the init/update verb in front of the target-selector flags: the
# historical script accepted options before or after the command, while the
# CLI parses them as flags OF the verb.
action=""
rest=()
for arg in "$@"; do
  if [ -z "$action" ] && { [ "$arg" = "init" ] || [ "$arg" = "update" ]; }; then
    action="$arg"
  else
    rest+=("$arg")
  fi
done

if [ -n "$action" ]; then
  exec workestrate secrets "$action" "${rest[@]}"
fi

# No verb given: --help passes through to the CLI's group help; anything else
# defaults to `init` (the historical script's default command).
for arg in "$@"; do
  case "$arg" in
    -h|--help) exec workestrate secrets "$@" ;;
  esac
done
exec workestrate secrets init "$@"
