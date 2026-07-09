#!/usr/bin/env bash
set -euo pipefail
set +H

# setup-secrets — bootstrap or update ai-workbench encrypted secrets.
#
# Usage:
#   nix develop -c setup-secrets init
#   nix develop -c setup-secrets update
#
# Secrets can be supplied via environment variables or interactive prompts.
# Command-line argument support is intentionally omitted to avoid leaking
# secrets into shell history.

# Determine the repo root: prefer the directory holding this script
# (works when invoked directly from a clone); fall back to the current
# working directory (works when invoked via a Nix wrapper that copies
# the script into /nix/store and exec's it from the user's CWD).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || true)"
if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/../.sops.yaml" ]; then
  cd "$SCRIPT_DIR/.."
elif [ -f "$PWD/.sops.yaml" ]; then
  cd "$PWD"
else
  echo "[setup-secrets] error: could not locate repo root (.sops.yaml not found)" >&2
  exit 1
fi

: "${SOPS_AGE_KEY_FILE:=$HOME/.config/sops/age/ai-workbench-secrets.txt}"
export SOPS_AGE_KEY_FILE

KEY_DIR="$(dirname "$SOPS_AGE_KEY_FILE")"
SOPS_CONFIG=".sops.yaml"
SECRET_FILE=".env.enc"
SCHEMA_FILE=".env.example"

# Read required keys from .env.example (all non-empty keys that are not
# obviously non-secret config paths).
REQUIRED_KEYS=(
  $(grep -E '^[A-Za-z_][A-Za-z0-9_]*=' .env.example \
    | grep -vE '^(AI_WORKBENCH_.*_DIR)=' \
    | cut -d= -f1 \
    | sort -u)
)

unset HISTFILE
umask 077

TMPFILE=""
SECRET_TMPFILE=""
trap 'rm -f "$TMPFILE" "$SECRET_TMPFILE"; stty echo 2>/dev/null || true' EXIT

log() { echo "[setup-secrets] $*" >&2; }
fail() { echo "[setup-secrets] error: $*" >&2; exit 1; }

require_tools() {
  command -v age-keygen >/dev/null 2>&1 || fail "age-keygen not found; run inside 'nix develop'"
  command -v sops >/dev/null 2>&1 || fail "sops not found; run inside 'nix develop'"
}

ensure_key() {
  if [ -f "$SOPS_AGE_KEY_FILE" ]; then
    log "using existing age key: $SOPS_AGE_KEY_FILE"
    return
  fi

  log "generating age key: $SOPS_AGE_KEY_FILE"
  mkdir -p "$KEY_DIR"
  chmod 700 "$KEY_DIR"
  age-keygen -o "$SOPS_AGE_KEY_FILE"
  chmod 600 "$SOPS_AGE_KEY_FILE"
}

public_key() {
  age-keygen -y "$SOPS_AGE_KEY_FILE"
}

update_sops_config() {
  local pub
  pub="$(public_key)"

  if grep -qE "^[[:space:]]*- &[[:alnum:]_]+ ${pub}$" "$SOPS_CONFIG"; then
    log ".sops.yaml already contains the correct public key"
    return
  fi

  if grep -qF "age1PLACEHOLDER" "$SOPS_CONFIG"; then
    log "updating .sops.yaml with public key"
    sed -i "s|age1PLACEHOLDER[^[:space:]]*|$pub|" "$SOPS_CONFIG"
    return
  fi

  fail ".sops.yaml contains a different public key; update it manually"
}

# Sentinel line written into the editor buffer. The user must delete this
# line to confirm they saved their changes; if it is still present after
# the editor closes, the file is treated as un-saved and the script aborts.
SENTINEL_LINE='# setup-secrets: delete this line to confirm you saved your changes'

# Write a pre-filled dotenv buffer to $1. The caller is responsible for
# creating the temp file with mktemp+chmod 600. Existing values from
# .env.example are stripped so the user always sees empty values.
build_prefilled_buffer() {
  local tmpfile="$1"
  local example_file="${2:-$SCHEMA_FILE}"

  {
    echo "# ai-workbench secrets (will be encrypted to $SECRET_FILE via sops)."
    echo "# Lines starting with '#' are ignored by sops and serve as instructions only."
    echo "# Required keys (from $example_file):"
    local k
    for k in "${REQUIRED_KEYS[@]}"; do
      echo "#   - $k"
    done
    echo "# Fill in real values, save, and exit your editor. The buffer is validated"
    echo "# and encrypted automatically. Delete the SENTINEL line below to confirm."
    echo ""

    # Walk .env.example: keep comments/blanks, rewrite KEY=value to KEY=
    local line key value
    while IFS= read -r line || [ -n "$line" ]; do
      if [ -z "$line" ] || [[ "$line" =~ ^# ]]; then
        printf '%s\n' "$line"
      elif [[ "$line" =~ ^([A-Za-z_][A-Za-z0-9_]*)=(.*)$ ]]; then
        key="${BASH_REMATCH[1]}"
        printf '%s=\n' "$key"
      else
        # Unparseable line: keep as a comment so it survives the round-trip
        printf '# %s\n' "$line"
      fi
    done < "$example_file"

    echo "$SENTINEL_LINE"
  } > "$tmpfile"
}

# Build a buffer for an interactive update: starts from the decrypted
# current .env.enc (so existing values are visible) and appends any
# keys from .env.example that are missing, plus the sentinel line.
build_update_buffer() {
  local output_tmpfile="$1"
  local decrypted_tmpfile="$2"

  cp "$decrypted_tmpfile" "$output_tmpfile"

  local example_file="$SCHEMA_FILE"
  local line key
  while IFS= read -r line || [ -n "$line" ]; do
    if [[ "$line" =~ ^([A-Za-z_][A-Za-z0-9_]*)=(.*)$ ]]; then
      key="${BASH_REMATCH[1]}"
      if ! grep -qE "^${key}=" "$output_tmpfile"; then
        printf '%s=\n' "$key" >> "$output_tmpfile"
      fi
    fi
  done < "$example_file"

  printf '%s\n' "$SENTINEL_LINE" >> "$output_tmpfile"
}

# Pick an editor: prefer $EDITOR if set and resolvable, else nano, vi, vim.
pick_editor() {
  if [ -n "${EDITOR:-}" ] && command -v "$EDITOR" >/dev/null 2>&1; then
    printf '%s' "$EDITOR"
    return 0
  fi
  if command -v nano >/dev/null 2>&1; then
    printf '%s' "nano"
    return 0
  fi
  if command -v vi >/dev/null 2>&1; then
    printf '%s' "vi"
    return 0
  fi
  if command -v vim >/dev/null 2>&1; then
    printf '%s' "vim"
    return 0
  fi
  fail "no editor found; set EDITOR to an absolute path, or install nano/vi/vim on the host"
}

# Capture a stable "fingerprint" of a file's content for change detection.
# Uses sha256sum when available, otherwise falls back to byte count + mtime
# (with sub-second resolution when supported by stat). Defined before
# edit_loop (the only caller) so function-definition order is explicit.
file_state() {
  local f="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" 2>/dev/null | awk '{print $1}'
    return 0
  fi
  if command -v md5sum >/dev/null 2>&1; then
    md5sum "$f" 2>/dev/null | awk '{print $1}'
    return 0
  fi
  # Last-resort fallback: combine byte count with sub-second mtime when possible.
  local size mtime
  size="$(stat -c %s "$f" 2>/dev/null || echo 0)"
  mtime="$(stat -c '%Y.%N' "$f" 2>/dev/null || stat -c %Y "$f" 2>/dev/null || echo 0)"
  printf '%s.%s' "$size" "$mtime"
}

# Open the editor on $tmpfile in a loop. Aborts if the sentinel is still
# present after 5 attempts (i.e. the user did not save their changes).
edit_loop() {
  local tmpfile="$1"
  if ! grep -qF "$SENTINEL_LINE" "$tmpfile"; then
    # Prepend the sentinel so the user has to actively delete it to
    # confirm a save happened.
    local pre
    pre="$(mktemp)"
    printf '%s\n' "$SENTINEL_LINE" > "$pre"
    cat "$tmpfile" >> "$pre"
    mv "$pre" "$tmpfile"
  fi

  local prior_state=""
  prior_state="$(file_state "$tmpfile")"

  local i editor new_state
  for i in 1 2 3 4 5; do
    editor="$(pick_editor)"
    "$editor" "$tmpfile" || log "editor exited non-zero (continuing)"
    new_state="$(file_state "$tmpfile")"
    if grep -qF "$SENTINEL_LINE" "$tmpfile" && [ "$new_state" = "$prior_state" ]; then
      log "no changes detected — re-opening editor" >&2
      continue
    fi
    return 0
  done

  if grep -qF "$SENTINEL_LINE" "$tmpfile"; then
    fail "aborting: file was not saved (sentinel still present)"
  fi
  return 0
}

# Validate the contents of a dotenv-style buffer. Prints errors to stderr
# and returns 0 on success, 1 if any errors were found.
validate_buffer() {
  local tmpfile="$1"
  local errors=0
  local warn_unparseable=0
  declare -A seen
  local line key value trimmed_key trimmed_value

  while IFS= read -r line || [ -n "$line" ]; do
    # Skip blanks and comments
    if [ -z "$line" ] || [[ "$line" =~ ^[[:space:]]*# ]]; then
      continue
    fi
    # Strip a leading "export " (with optional surrounding whitespace)
    local stripped="${line#"${line%%[![:space:]]*}"}"
    stripped="${stripped#export }"

    if [[ "$stripped" =~ ^([A-Za-z_][A-Za-z0-9_]*)=(.*)$ ]]; then
      key="${BASH_REMATCH[1]}"
      value="${BASH_REMATCH[2]}"
      # Trim key whitespace
      trimmed_key="$(printf '%s' "$key" | sed -E 's/^[[:space:]]+|[[:space:]]+$//g')"
      # Trim value whitespace
      trimmed_value="$(printf '%s' "$value" | sed -E 's/^[[:space:]]+|[[:space:]]+$//g')"
      # Strip surrounding matching quotes
      case "$trimmed_value" in
        \"*\") trimmed_value="${trimmed_value#\"}"; trimmed_value="${trimmed_value%\"}" ;;
        \'*\') trimmed_value="${trimmed_value#\'}"; trimmed_value="${trimmed_value%\'}" ;;
      esac

      if [ -n "${seen[$trimmed_key]+set}" ]; then
        echo "[setup-secrets] error: duplicate key: $trimmed_key" >&2
        errors=$((errors + 1))
      else
        seen[$trimmed_key]=1
      fi

      # Only enforce REQUIRED_KEYS validation for the trimmed key
      local is_required=0
      local rk
      for rk in "${REQUIRED_KEYS[@]}"; do
        if [ "$rk" = "$trimmed_key" ]; then
          is_required=1
          break
        fi
      done
      if [ "$is_required" -eq 1 ]; then
        if [ -z "$trimmed_value" ]; then
          echo "[setup-secrets] error: $trimmed_key: value is empty or whitespace-only" >&2
          errors=$((errors + 1))
        fi

      fi
    else
      warn_unparseable=1
    fi
  done < "$tmpfile"

  if [ "$warn_unparseable" -eq 1 ]; then
    echo "[setup-secrets] warning: unparseable lines were ignored" >&2
  fi

  if [ "$errors" -gt 0 ]; then
    return 1
  fi
  return 0
}

# Prepend an "# ERROR: ..." annotation to the temp file. Any previous
# "# ERROR:" lines are stripped first so the user always sees a fresh
# message describing the current validation failures.
prepend_error_annotation() {
  local tmpfile="$1"
  local message="$2"

  local filtered
  filtered="$(mktemp)"
  # Drop any existing "# ERROR:" lines
  grep -v '^# ERROR:' "$tmpfile" > "$filtered" || true

  {
    printf '# ERROR: %s\n' "$message"
    printf '# Fix the issues below and save again.\n'
    printf '# (Previous ERROR: lines were removed automatically.)\n'
    cat "$filtered"
  } > "$tmpfile"
  rm -f "$filtered"
}

# Encrypt $1 to $SECRET_FILE atomically (write to .tmp then mv).
encrypt_dotenv_to_secret() {
  local tmpfile="$1"
  log "encrypting $SECRET_FILE"
  SECRET_TMPFILE="${SECRET_FILE}.tmp"
  if ! sops --config "$SOPS_CONFIG" encrypt --input-type dotenv --output-type dotenv --filename-override "$SECRET_FILE" "$tmpfile" > "$SECRET_TMPFILE"; then
    rm -f "$SECRET_TMPFILE"
    SECRET_TMPFILE=""
    fail "sops encrypt failed; .env.enc not written"
  fi
  mv "$SECRET_TMPFILE" "$SECRET_FILE"
  SECRET_TMPFILE=""
  log "wrote $SECRET_FILE"
}

# Print a human-readable summary of what was encrypted.
print_summary() {
  local tmpfile="$1"
  local keys count
  keys="$(grep -E '^[A-Za-z_][A-Za-z0-9_]*=' "$tmpfile" 2>/dev/null | cut -d= -f1 | tr '\n' ',' | sed 's/,$//' || true)"
  if [ -n "$keys" ]; then
    count="$(printf '%s' "$keys" | tr ',' '\n' | wc -l | tr -d ' ')"
    log "encrypted $count keys: $keys"
  fi
  log "REMINDER: back up $SOPS_AGE_KEY_FILE to a secure location. Without it, .env.enc cannot be decrypted."
}

# If all required env vars are set and non-empty, write them to
# $tmpfile (caller-provided, chmod 600) and return 0. Otherwise return 1.
noninteractive_env_init() {
  local tmpfile="$1"
  local k v
  for k in "${REQUIRED_KEYS[@]}"; do
    v="${!k:-}"
    if [ -z "$v" ]; then
      return 1
    fi
  done

  for k in "${REQUIRED_KEYS[@]}"; do
    v="${!k:-}"
    # Trim value
    v="$(printf '%s' "$v" | sed -E 's/^[[:space:]]+|[[:space:]]+$//g')"
    printf '%s=%s\n' "$k" "$v" >> "$tmpfile"
  done
  return 0
}

# Populate a dotenv file from a list of stdin lines (one per required key,
# in REQUIRED_KEYS order). Empty lines keep the existing value from $decrypted_tmpfile.
# Returns 0 on success, 1 on failure.
populate_from_stdin() {
  local out_tmpfile="$1"
  local decrypted_tmpfile="$2"

  local i=0 k v existing
  while IFS= read -r line; do
    # Only consume up to the number of required keys
    if [ "$i" -ge "${#REQUIRED_KEYS[@]}" ]; then
      break
    fi
    k="${REQUIRED_KEYS[$i]}"
    v="$line"
    # Trim whitespace
    v="$(printf '%s' "$v" | sed -E 's/^[[:space:]]+|[[:space:]]+$//g')"
    if [ -z "$v" ]; then
      # Keep existing value from decrypted file
      existing="$(grep -E "^${k}=" "$decrypted_tmpfile" | head -n 1 | cut -d= -f2- || true)"
      printf '%s=%s\n' "$k" "$existing" >> "$out_tmpfile"
    else
      printf '%s=%s\n' "$k" "$v" >> "$out_tmpfile"
    fi
    i=$((i + 1))
  done
  return 0
}

# Interactive init: build a pre-filled buffer, edit/validate/encrypt.
init_via_editor() {
  local tmp
  tmp="$(mktemp)"
  chmod 600 "$tmp"

  if noninteractive_env_init "$tmp"; then
    log "using env-var values for all required keys (non-interactive init)"
    if ! validate_buffer "$tmp"; then
      fail "env-var values failed validation; fix and retry"
    fi
  else
    build_prefilled_buffer "$tmp"
    TMPFILE="$tmp"

    local attempt
    for attempt in 1 2 3; do
      edit_loop "$tmp"
      if validate_buffer "$tmp"; then
        break
      fi
      # Capture and report validation errors
      local errs=""
      errs="$(validate_buffer "$tmp" 2>&1 1>/dev/null || true)"
      prepend_error_annotation "$tmp" "$errs"
      if [ "$attempt" -eq 3 ]; then
        fail "validation failed after 3 attempts"
      fi
    done
  fi

  encrypt_dotenv_to_secret "$tmp"
  print_summary "$tmp"
  rm -f "$tmp"
  TMPFILE=""
}

# Interactive update: decrypt, then either use stdin / env-var / editor
# to build a new buffer, then validate and re-encrypt.
update_via_editor() {
  local tmp
  tmp="$(mktemp)"
  chmod 600 "$tmp"

  log "decrypting $SECRET_FILE"
  sops --config "$SOPS_CONFIG" decrypt --input-type dotenv --output-type dotenv "$SECRET_FILE" > "$tmp"

  # Decide which path to take:
  # 1. If LITELLM_MASTER_KEY env var is set, do a targeted replace of just
  #    that key, keeping all others unchanged. This works in both interactive
  #    and scripted/CI contexts and has no ordering dependency on REQUIRED_KEYS.
  # 2. Else if stdin is not a TTY, read one line per required key from stdin
  #    (empty = keep existing). This preserves the scriptable interface.
  # 3. Else open the editor.
  if [ -n "${LITELLM_MASTER_KEY:-}" ]; then
    local new_litellm
    new_litellm="$(printf '%s' "$LITELLM_MASTER_KEY" | sed -E 's/^[[:space:]]+|[[:space:]]+$//g')"
    if grep -qE "^LITELLM_MASTER_KEY=" "$tmp"; then
      # Replace the existing line in place (escape sed metachars in the value)
      local escaped_litellm
      escaped_litellm="$(printf '%s' "$new_litellm" | sed -e 's/[\\&|]/\\&/g')"
      sed -i "s|^LITELLM_MASTER_KEY=.*|LITELLM_MASTER_KEY=${escaped_litellm}|" "$tmp"
    else
      printf 'LITELLM_MASTER_KEY=%s\n' "$new_litellm" >> "$tmp"
    fi
    if ! validate_buffer "$tmp"; then
      fail "LITELLM_MASTER_KEY env-var value failed validation; fix and retry"
    fi
  elif [ ! -t 0 ]; then
    local newtmp
    newtmp="$(mktemp)"
    chmod 600 "$newtmp"
    populate_from_stdin "$newtmp" "$tmp"
    rm -f "$tmp"
    tmp="$newtmp"
    TMPFILE="$tmp"

    local attempt
    for attempt in 1 2 3; do
      if validate_buffer "$tmp"; then
        break
      fi
      local errs=""
      errs="$(validate_buffer "$tmp" 2>&1 1>/dev/null || true)"
      prepend_error_annotation "$tmp" "$errs"
      if [ "$attempt" -eq 3 ]; then
        fail "validation failed after 3 attempts"
      fi
    done
  else
    local newtmp
    newtmp="$(mktemp)"
    chmod 600 "$newtmp"
    build_update_buffer "$newtmp" "$tmp"
    rm -f "$tmp"
    tmp="$newtmp"
    TMPFILE="$tmp"

    local attempt
    for attempt in 1 2 3; do
      edit_loop "$tmp"
      if validate_buffer "$tmp"; then
        break
      fi
      local errs=""
      errs="$(validate_buffer "$tmp" 2>&1 1>/dev/null || true)"
      prepend_error_annotation "$tmp" "$errs"
      if [ "$attempt" -eq 3 ]; then
        fail "validation failed after 3 attempts"
      fi
    done
  fi

  encrypt_dotenv_to_secret "$tmp"
  print_summary "$tmp"
  rm -f "$tmp"
  TMPFILE=""
}

cmd_init() {
  require_tools
  ensure_key
  update_sops_config

  log "IMPORTANT: back up $SOPS_AGE_KEY_FILE to a secure location."
  log "Without this key, .env.enc cannot be decrypted."

  if [ -f "$SECRET_FILE" ]; then
    log "$SECRET_FILE already exists; run 'update' to change values"
    return
  fi

  init_via_editor
  log "done. If you use direnv, ensure you've run 'direnv allow'; otherwise use 'nix develop'."
  log "You can now run: workestrate litellm up"
}

cmd_update() {
  require_tools
  [ -f "$SOPS_AGE_KEY_FILE" ] || fail "age key not found at $SOPS_AGE_KEY_FILE; run 'init' first"
  [ -f "$SECRET_FILE" ] || fail "$SECRET_FILE does not exist; run 'init' first"
  update_sops_config
  update_via_editor
  log "done. $SECRET_FILE updated."
}

main() {
  local cmd="${1:-init}"
  case "$cmd" in
    init) cmd_init ;;
    update) cmd_update ;;
    *) fail "unknown command: $cmd (expected init or update)" ;;
  esac
}

main "$@"
