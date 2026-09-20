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
# --global flags; --config anywhere, now a global CLI flag) and forwards them
# to `workestrate secrets`. Secret values are never accepted as command-line
# arguments: they come from environment variables, stdin (non-TTY), or an
# interactive editor.

set -euo pipefail

echo "[setup-secrets] DEPRECATED: use 'workestrate secrets' directly; this script delegates to it." >&2

# Hoist the init/update verb in front of the target-selector flags: the
# historical script accepted options before or after the command, while the
# CLI parses them as flags OF the verb. Options that take a value
# (--config/--fleet/--fleet-dir, both `--opt value` and `--opt=value`
# forms) are consumed together with their value, so a value named
# init/update is never mistaken for the verb; a missing value errors out
# before delegating to the CLI.
action=""
rest=()
saw_help=0
while [ $# -gt 0 ]; do
  case "$1" in
    --config|--fleet|--fleet-dir)
      opt="$1"
      if [ $# -lt 2 ]; then
        echo "[setup-secrets] ERROR: $opt requires a value" >&2
        exit 1
      fi
      rest+=("$1" "$2")
      shift 2
      ;;
    --config=*|--fleet=*|--fleet-dir=*)
      rest+=("$1")
      shift
      ;;
    -h|--help)
      saw_help=1
      rest+=("$1")
      shift
      ;;
    init|update)
      if [ -z "$action" ]; then
        action="$1"
      else
        rest+=("$1")
      fi
      shift
      ;;
    *)
      rest+=("$1")
      shift
      ;;
  esac
done

if [ -n "$action" ]; then
  exec workestrate secrets "$action" "${rest[@]}"
fi

# No verb given: --help passes through to the CLI's group help; anything else
# defaults to `init` (the historical script's default command).
if [ "$saw_help" -eq 1 ]; then
  exec workestrate secrets "${rest[@]}"
fi
exec workestrate secrets init "${rest[@]}"
