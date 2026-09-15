# Secrets Management

ai-workbench uses [SOPS](https://github.com/getsops/sops) with an [age](https://age-encryption.org/) recipient to keep secrets out of plain text in the repo and out of plaintext at rest.

Secrets live under the **single tool home** (`$WORKESTRATE_HOME`), not at
the repo root. Each config repo holds its own `.env.enc` + `.sops.yaml`; a
user-global secrets layer applies per-key across all contexts.

- Encrypted file: `.env.enc` (committed in each config repo; ciphertext-safe)
- Decrypted form: never committed; `workestrate run -- <cmd>` injects env vars into a child process; `write-env` writes a plaintext `.env` you must remove yourself.
- Key file: `~/.config/sops/age/ai-workbench-secrets.txt` on the HOST (project-specific, NEVER in repo, NEVER in bundle, NEVER under `.workestrate/` or `$WORKESTRATE_HOME`)
- The `workestrate secrets` CLI defaults the age key path itself (registry `age_key_file` override > `SOPS_AGE_KEY_FILE` env > the default path). `decrypt-env`/`write-env` export `SOPS_AGE_KEY_FILE` defaulting to that path. These run on the host (age key present); in a container the key is absent and they fail closed by design.

## The 7 secrets

| Key | Used for |
|---|---|
| `LITELLM_MASTER_KEY` | Local LiteLLM proxy authentication (any `sk-…` string; `sk-change-me-local-only` is rejected as a known-bad secret) |
| `OPENROUTER_API_KEY` | OpenRouter provider |
| `KIMI_CODE_API_KEY` | Kimi for Coding provider |
| `NEURALWATT_API_KEY` | Neuralwatt provider |
| `MINIMAX_CODING_API_KEY` | MiniMax Coding provider |
| `GITHUB_TOKEN` | GitHub Personal Access Token for agent sandboxes (git operations + API) |
| `ODYSSEUS_ADMIN_PASSWORD` | Odysseus admin login (required because `AUTH_ENABLED=true`; without it Odysseus auto-generates a random password printed to logs) |

The optional keys `AI_WORKBENCH_WORKSPACES_DIR` and
`AI_WORKBENCH_VAR_DIR` are reserved for future use and are not yet
consumed by the code; default paths are used regardless.

## Where secrets live

| Artifact | Location | Notes |
|---|---|---|
| Per-config-repo `.env.enc` | `$WORKESTRATE_HOME/config-repos/<name>/.env.enc` | SOPS-encrypted; ciphertext-safe to commit in the config repo |
| Per-config-repo `.sops.yaml` | `$WORKESTRATE_HOME/config-repos/<name>/.sops.yaml` | SOPS recipient config; no secrets in it |
| User-global secrets | `$WORKESTRATE_HOME/secrets/.env.local.enc` | Applied per-key across all contexts |
| age private key | `~/.config/sops/age/ai-workbench-secrets.txt` (HOST) | NEVER in repo/bundle/`.workestrate/` |

## `workestrate secrets` flows

Secrets are provisioned with the installed CLI — no devshell, no helper
script (the nix wrapper bundles `sops` and `age-keygen`):

```bash
workestrate secrets init    [--config <name> | --config-dir <dir> | --global]
workestrate secrets update  [--config <name> | --config-dir <dir> | --global]
workestrate secrets target <name> [--json]   # inspect the resolved target
workestrate secrets schema                   # required key names from config
```

Secret values are **never accepted as command-line arguments** (they would
leak into shell history): they come from process environment variables,
stdin (non-TTY), or an interactive editor.

The target selectors are mutually exclusive. Targeting precedence:

1. `--config <name>` — registry-backed resolution (same resolver as
   `workestrate secrets target`): the managed store clone of the named
   config repo, honoring the selected tool home, the registry store
   directory, and per-repo `secrets_file`/`age_key_file` overrides (tilde
   expanded; a relative `age_key_file` keeps its invocation-cwd meaning).
   An unknown name is a hard error — it never falls back to a guessed
   directory.
2. `--config-dir <dir>` — the directory itself. It must exist and is never
   created. Relative paths resolve against the invocation directory; spaces,
   quotes, and shell metacharacters in directory names are inert (pure path
   handling, no shell).
3. `--global` — the user-global layer
   (`${XDG_CONFIG_HOME:-~/.config}/workestrate/.env.local.enc`; the
   directory is created when missing).
4. `WORKESTRATE_CONFIG_DIR`, when set.
5. Auto-detect: exactly one registered config repo → resolve as `--config`.
6. A `.sops.yaml` in the current directory → the current directory.

The global `--home <DIR>` flag selects the tool home the registry is read
from and only makes sense together with `--config`.

### `workestrate secrets init --config <name>`

```bash
workestrate secrets init --config personal      # one-time: create .env.enc
workestrate secrets update --config personal    # edit existing values
workestrate --home /path/to/operator-home secrets update --config personal
```

For a config directory not selected through the registry:

```bash
workestrate secrets update --config-dir "/path/to/config repo"
```

`init`:
- Creates `~/.config/sops/age/ai-workbench-secrets.txt` (mode 0600, parent
  0700) via `age-keygen` if missing.
- Replaces the `age1PLACEHOLDER...` recipient in the target's `.sops.yaml`
  with the actual public key; refuses (with a "update it manually" error)
  when the file carries a different recipient.
- If all required env vars are set and non-empty, uses those values directly
  (non-interactive).
- Otherwise, opens your default terminal editor (`$EDITOR`, or
  `nano`/`vi`/`vim`) with a pre-filled buffer of all keys from the
  env-example generator (falling back to the target's `.env.example`). Fill
  in values, delete the `# setup-secrets: delete this line…` sentinel to
  confirm the save, and exit the editor. The buffer is validated; on errors,
  the file is re-opened with an `# ERROR:` annotation (up to 3 attempts).
- Writes the encrypted secrets file (mode 0600) atomically (`.tmp` + rename).
- Refuses to overwrite an existing secrets file — use `update` for changes.

`update`:
- Requires the age key and the secrets file to exist ("run 'init' first").
- Decrypts the secrets file, then picks a path:
  - **Env-var targeted replace:** if ANY schema/env-example key is set
    non-empty in the process env, exactly those keys are replaced in place
    (or appended) and every other value is preserved. The replaced key
    NAMES are reported, never the values. (This generalizes the retired
    script's `LITELLM_MASTER_KEY`-only special case — any key now works the
    same way, which covers scripted key rotation.)
  - **stdin (non-TTY):** one line per required key in required-keys order;
    an empty line keeps the existing value.
  - **Interactive:** opens the editor with the decrypted values plus any
    keys added since the last init/update, validates, re-encrypts.

`REQUIRED_KEYS` is read from the loaded config's `secrets` section (the same
source as `workestrate secrets schema`), falling back to parsing the
target's `.env.example` when the config fails to load.

### `workestrate secrets init|update --global`

Targets the user-global secrets layer at
`$WORKESTRATE_HOME/secrets/.env.local.enc` (XDG config dir in legacy-XDG
layouts). This layer is applied per-key AFTER the context's domain layers
and BEFORE project layers.

```bash
workestrate secrets init --global      # one-time: create .env.local.enc
workestrate secrets update --global    # edit existing user-global values
```

### Deprecated delegates

- `just setup-secrets ...` still works: the recipe is a thin alias for
  `workestrate secrets "$@"` and no longer enters a devshell.
- `scripts/setup-secrets.sh` and the flake `setup-secrets` app print a
  deprecation notice and delegate to `workestrate secrets`, hoisting the
  `init`/`update` verb in front of the target flags so every historical
  argument order keeps working.
- The top-level `workestrate secrets-target` and `workestrate
  secrets-schema` commands remain as hidden compatibility aliases for
  `workestrate secrets target` / `workestrate secrets schema`.

`just secrets-target-check` tests selection and encrypted updates with
disposable homes and fresh test keys (both as Rust integration tests in
`control/agentctl/tests/cmd_secrets.rs` and via
`scripts/test-setup-secrets.py`, which drives the real CLI).
`just shell-arguments-check` covers the recipe's literal argument
forwarding.

## Multi-layer per-key value merge

`workestrate` loads `.env.enc` from EVERY resolved layer in precedence
order, merging decrypted values per-key (later layer wins per key).
Process env is the lowest precedence. Per-key provenance is tracked for
`plan --show-source`.

**Secrets layer resolution order** (lowest → highest precedence):

1. **Process env** (only for defined secrets; lowest precedence)
2. **`WORKESTRATE_CONFIG_DIR`** (single override, bypasses discovery)
3. **Reference config dir** (shipped with tool — no `.env.enc` expected)
4. **Context layers in declared order**, each with its own `.env.enc`
   (per-repo `secrets_file` and `age_key_file` overrides honored from the
   registry `[configs.<name>]`)
5. **User-global secrets** (`$WORKESTRATE_HOME/secrets/.env.local.enc`) —
   applied per-key AFTER the context's domain layers, BEFORE project layers
6. **Trusted project dir** (cwd, if trusted) — may have `.env.enc`
7. **Local overrides dir**

### Failure semantics

- **Undecryptable layer** → WARNING + continue (the layer is skipped; other layers still contribute).
- **Required secret unsatisfied** → hard fail naming the secret + the layers tried + remediation guidance.
- **`secrets = "none"` repos** → skipped silently.

## age key location

The age private key stays at `~/.config/sops/age/ai-workbench-secrets.txt`
on the HOST.

- NEVER in the repo (the repo is agent-reachable via `${CWD}` mounts, so a
  key inside it would be exposed to sandboxes).
- NEVER in the bundle.
- NEVER under `.workestrate/` or `$WORKESTRATE_HOME`.
- In a container, the key is simply absent and secret operations fail
  closed by design.

Back up this key to a secure location. Without it, `.env.enc` cannot be
decrypted.

## Running `workestrate` commands with secrets

Inside the dev shell, `workestrate` is already on PATH. `workestrate
workload up`/`exec` decrypt `.env.enc` themselves and inject the secrets:

```bash
just shell
workestrate workload up litellm           # service: starts detached
workestrate workload up odysseus          # service: starts detached
workestrate workload exec pi              # agent: interactive TUI attach
```

`workestrate run --` always requires `.env.enc` to exist, so it is only for
commands that need decrypted secrets. For plan-only commands that do not
need secrets, use plain `nix run`:

```bash
nix run . -- workload plan litellm
nix run . -- workload plan pi
```

For arbitrary commands that need the same secrets (inside or outside the
dev shell — `workestrate` is nix-profile-installed and works in any shell):

```bash
workestrate run -- bash -c 'echo $LITELLM_MASTER_KEY'
```

## How workestrate validates secrets

When you run `workestrate workload up litellm`, the CLI validates that
`LITELLM_MASTER_KEY` and the provider keys defined in the config's secrets
section are present. Service `up` and agent `exec` commands require
`LITELLM_MASTER_KEY`. If a required secret is missing, the CLI prints a
clear error and exits before starting any sandbox, so the failure is
attributable to the missing secret rather than opaque sandbox-runtime output.
`check` and `plan` commands never read secret environment variables, so
they can run without the `run` secret command.

Empty values and whitespace-only values are treated as missing, and the
`LITELLM_MASTER_KEY` placeholder value (`sk-change-me-local-only`) is
rejected as a known-bad secret with a dedicated error.

## Inspecting secrets

```bash
just shell -c decrypt-env           # prints decrypted .env.enc to stdout
```

## Fallback: short-lived plaintext .env

If a tool needs a literal `.env` file:
```bash
just shell -c write-env              # writes .env with mode 0600
# ... do work ...
rm .env
```

`write-env` refuses to overwrite an existing `.env`.

## Wrappers reference

| Wrapper | Runs on | Purpose |
|---|---|---|
| `workestrate secrets init\|update` | Host (age key present) | Create/update `.env.enc` (`--config`) or `.env.local.enc` (`--global`) — the CLI subcommand, with sops+age bundled in the nix wrapper |
| `setup-secrets` (DEPRECATED) | Host | Alias delegating to `workestrate secrets` |
| `decrypt-env` | Host | Print decrypted secrets to stdout |
| `write-env` | Host | Write a short-lived plaintext `.env` (mode 0600) |

The CLI's secret-consumption command is `workestrate run -- <cmd>` — it decrypts
`.env.enc` and execs `<cmd>` with the secrets in its environment.

In a container, the age key is absent, so all of these fail closed by
design.

## Validating the workflow

A non-interactive validation script exercises the full secrets lifecycle
(`workestrate secrets init` → `decrypt-env` → `workestrate secrets update` →
`decrypt-env` → `write-env` → `workestrate run -- env` → `workestrate --help`) in an
isolated temp directory using a freshly generated test key. It does not
touch the real `~/.config/sops/age/ai-workbench-secrets.txt` or any config
repo's `.env.enc`.

```bash
just validate-secrets
# or:
nix develop --override-input devenv-root "file+file://$HOME/.cache/workestrate/devenv-root/workestrate" -c scripts/validate-secrets-workflow.sh
```

The script:

1. Creates a temp working directory and a one-off age key inside it.
2. Copies `.sops.yaml` and rewrites the recipient to the test public key.
3. Runs `workestrate secrets init` with secrets supplied via env vars (all 7
   required keys).
4. Decrypts with `decrypt-env` and asserts the initial values appear.
5. Runs `workestrate secrets update` with one changed value and the rest
   preserved, then re-decrypts and asserts the change took effect and the
   others were preserved.
6. Runs `write-env`, asserts `.env` was created with mode `0600` and
   contains the expected values, then removes the file.
7. Runs `workestrate run -- env` and asserts all secrets are exported to
   the child process's environment.
8. Runs `workestrate --help` and asserts workestrate's help text appears
   (proving the CLI loads; the decrypt+exec path is covered by step 7).
9. Cleans up the temp directory and prints a pass/fail summary.

The script is hermetic: it fails fast (exit 1) if a real `.env.enc` or
`.env` already exists at the target, and on exit it removes the temp
directory and any files it created.

## Security rules

- The age private key must never be committed. Keep it at `~/.config/sops/age/ai-workbench-secrets.txt` and chmod 0600. NEVER place it under `.workestrate/` or `$WORKESTRATE_HOME` (the repo is agent-reachable via `${CWD}` mounts).
- `.sops.yaml` may be committed (no secrets in it — only the age recipient).
- `.env.enc` is ciphertext-safe and may be committed in config repos.
- Do not put secrets in Nix expressions — Nix strings can leak into the world-readable Nix store.
- Do not pass secrets as command-line arguments; `workestrate secrets` deliberately omits argv-based secret input so values cannot leak into shell history.
- Do not paste decrypted `.env` into logs, issues, or shell history.
- `.env` is gitignored; `.env.enc` is committed (in config repos).
- When using the editor flow, secret values are written to a temp file with mode 0600 and removed in a cleanup trap. They will also appear in your terminal scrollback. Clear scrollback or use a private terminal session if that is a concern.
