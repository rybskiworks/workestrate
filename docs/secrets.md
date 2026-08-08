# Secrets Management

ai-workbench uses [SOPS](https://github.com/getsops/sops) with an [age](https://age-encryption.org/) recipient to keep secrets out of plain text in the repo and out of plaintext at rest.

Secrets live under the **single tool home** (`$WORKESTRATE_HOME`), not at
the repo root. Each config repo holds its own `.env.enc` + `.sops.yaml`; a
user-global secrets layer applies per-key across all contexts.

- Encrypted file: `.env.enc` (committed in each config repo; ciphertext-safe)
- Decrypted form: never committed; `workestrate run -- <cmd>` injects env vars into a child process; `write-env` writes a plaintext `.env` you must remove yourself.
- Key file: `~/.config/sops/age/ai-workbench-secrets.txt` on the HOST (project-specific, NEVER in repo, NEVER in bundle, NEVER under `.workestrate/` or `$WORKESTRATE_HOME`)
- All wrappers (`setup-secrets`, `decrypt-env`, `write-env`) export `SOPS_AGE_KEY_FILE` defaulting to that path. These wrappers run on the host (age key present); in a container the key is absent and they fail closed by design.

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
| Per-config-repo `.env.enc` | `$WORKESTRATE_HOME/repos/<name>/.env.enc` | SOPS-encrypted; ciphertext-safe to commit in the config repo |
| Per-config-repo `.sops.yaml` | `$WORKESTRATE_HOME/repos/<name>/.sops.yaml` | SOPS recipient config; no secrets in it |
| User-global secrets | `$WORKESTRATE_HOME/secrets/.env.local.enc` | Applied per-key across all contexts |
| age private key | `~/.config/sops/age/ai-workbench-secrets.txt` (HOST) | NEVER in repo/bundle/`.workestrate/` |

## setup-secrets flows

`setup-secrets.sh` supports two mutually exclusive targeting modes:

### `setup-secrets --config <name> init|update`

Targets a specific config repo's `.env.enc` + `.sops.yaml` in
`$WORKESTRATE_HOME/repos/<name>/`.

```bash
setup-secrets --config personal init      # one-time: create .env.enc
setup-secrets --config personal update    # edit existing values
```

`init`:
- Creates `~/.config/sops/age/ai-workbench-secrets.txt` (mode 0600) if missing.
- Replaces the `age1PLACEHOLDER...` recipient in the config repo's `.sops.yaml` with the actual public key.
- If all required env vars are set and non-empty, uses those values directly (non-interactive).
- Otherwise, opens your default terminal editor (`$EDITOR`, or `nano`/`vi`/`vim` on the host) with a pre-filled buffer of all keys from `workestrate generate-env-example`. Fill in values, delete the `# setup-secrets: delete this line…` sentinel to confirm the save, and exit the editor. The buffer is validated; on errors, the file is re-opened with an `# ERROR:` annotation (up to 3 attempts).
- Writes encrypted `.env.enc` in the config repo dir.
- Refuses to overwrite an existing `.env.enc` — use `update` for changes.

`update`:
- Decrypts the config repo's `.env.enc`, opens the editor with existing values pre-filled (plus any keys added since the last init/update), validates, and re-encrypts.

### `setup-secrets --global init|update`

Targets the user-global secrets layer at
`$WORKESTRATE_HOME/secrets/.env.local.enc`. This layer is applied per-key
AFTER the context's domain layers and BEFORE project layers.

```bash
setup-secrets --global init      # one-time: create .env.local.enc
setup-secrets --global update    # edit existing user-global values
```

`--global` and `--config` are **mutually exclusive**.

### No flags (auto-detect)

Without `--global` or `--config`, `setup-secrets` auto-detects a single
registered config repo, or falls back to the repo root (backwards compat).

### Non-interactive input

- **Env vars:** if all required env vars are set and non-empty, those values are used directly.
- **stdin:** if stdin is not a TTY (e.g. piped from another command or run from a test harness), one line per required key is read from stdin in `REQUIRED_KEYS` order; empty lines preserve the existing value, non-empty lines replace it.

`REQUIRED_KEYS` is read from `workestrate secrets-schema` (falls back to
`.env.example` grep).

> **LANDING (Track B):** per-repo override alignment — `setup-secrets`
> reading per-repo `secrets_file` and `age_key_file` from the registry
> when `--config <name>` is used — is landing in parallel via Track B.
> Until it lands, users can set `SOPS_AGE_KEY_FILE` manually as a
> workaround. The Rust `load_secrets()` already honors these per-repo
> overrides; only the shell wrapper needs alignment.

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

Inside the dev shell, `workestrate` is already on PATH. The CLI's secret
command is `workestrate run -- <cmd>` — it decrypts `.env.enc` and runs
`<cmd>` with the secrets injected into its environment:

```bash
nix develop
workestrate run -- workload up litellm      # service: starts detached
workestrate run -- workload up odysseus     # service: starts detached
workestrate run -- workload exec pi         # agent: interactive TUI attach
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

When you run `workestrate run -- workload up litellm`, the CLI validates that
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
nix develop -c decrypt-env          # prints decrypted .env.enc to stdout
```

## Fallback: short-lived plaintext .env

If a tool needs a literal `.env` file:
```bash
nix develop -c write-env            # writes .env with mode 0600
# ... do work ...
rm .env
```

`write-env` refuses to overwrite an existing `.env`.

## Wrappers reference

| Wrapper | Runs on | Purpose |
|---|---|---|
| `setup-secrets` | Host (age key present) | Create/update `.env.enc` (`--config`) or `.env.local.enc` (`--global`) |
| `decrypt-env` | Host | Print decrypted secrets to stdout |
| `write-env` | Host | Write a short-lived plaintext `.env` (mode 0600) |

The CLI's secret command is `workestrate run -- <cmd>` — it decrypts
`.env.enc` and execs `<cmd>` with the secrets in its environment (a CLI
subcommand, not a flake wrapper).

In a container, the age key is absent, so all of these fail closed by
design.

## Validating the workflow

A non-interactive validation script exercises the full secrets lifecycle
(`setup-secrets init` → `decrypt-env` → `setup-secrets update` →
`decrypt-env` → `write-env` → `workestrate run -- env` → `workestrate --help`) in an
isolated temp directory using a freshly generated test key. It does not
touch the real `~/.config/sops/age/ai-workbench-secrets.txt` or any config
repo's `.env.enc`.

```bash
just validate-secrets
# or:
nix develop -c scripts/validate-secrets-workflow.sh
```

The script:

1. Creates a temp working directory and a one-off age key inside it.
2. Copies `.sops.yaml` and rewrites the recipient to the test public key.
3. Runs `setup-secrets init` with secrets supplied via env vars (all 7
   required keys).
4. Decrypts with `decrypt-env` and asserts the initial values appear.
5. Runs `setup-secrets update` with one changed value and the rest
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
- Do not pass secrets as command-line arguments; `setup-secrets` deliberately omits argv-based secret input so values cannot leak into shell history.
- Do not paste decrypted `.env` into logs, issues, or shell history.
- `.env` is gitignored; `.env.enc` is committed (in config repos).
- When using the editor flow, secret values are written to a temp file with mode 0600 and removed in a cleanup trap. They will also appear in your terminal scrollback. Clear scrollback or use a private terminal session if that is a concern.
