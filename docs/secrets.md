# Secrets Management

ai-workbench uses [SOPS](https://github.com/getsops/sops) with an [age](https://age-encryption.org/) recipient to keep secrets out of plain text in the repo and out of plaintext at rest.

- Encrypted file: `.env.enc` (committed)
- Decrypted form: never committed; `with-secrets` injects env vars into a child process; `write-env` writes a plaintext `.env` you must remove yourself.
- Key file: `~/.config/sops/age/ai-workbench-secrets.txt` (project-specific, NEVER in repo)
- All wrappers (`setup-secrets`, `with-secrets`, `run-with-secrets`, `decrypt-env`, `write-env`) export `SOPS_AGE_KEY_FILE` defaulting to that path.

## One-time setup

1. Install direnv and nix-direnv (optional but recommended):
   - `direnv`: https://direnv.net/
   - `nix-direnv`: https://github.com/nix-community/nix-direnv

2. Generate the project age key and create the encrypted secrets file:
   ```bash
   nix develop -c setup-secrets init
   ```
   `setup-secrets init`:
   - Creates `~/.config/sops/age/ai-workbench-secrets.txt` (mode 0600) if missing.
    - Replaces the `age1PLACEHOLDER...` recipient in `.sops.yaml` with the actual public key.
    - If all four required env vars (`LITELLM_MASTER_KEY`, `OPENROUTER_API_KEY`, `KIMI_CODE_API_KEY`, `MINIMAX_CODING_API_KEY`) are set and non-empty, uses those values directly (non-interactive).
    - Otherwise, opens your default terminal editor (`$EDITOR`, or `nano`/`vi`/`vim` on the host) with a pre-filled buffer of all keys from `.env.example`. Fill in values, delete the `# setup-secrets: delete this line…` sentinel to confirm the save, and exit the editor. The buffer is validated; on errors, the file is re-opened with an `# ERROR:` annotation (up to 3 attempts).
    - Writes encrypted `.env.enc`.

   To use a non-default key path, set `SOPS_AGE_KEY_FILE` in the environment before running `setup-secrets`.

3. After cloning, run `direnv allow` so the dev shell loads automatically when you `cd` into the repo. If you don't use direnv, run `nix develop` manually.

## Updating secrets

```bash
nix develop -c setup-secrets update
```

`update` decrypts `.env.enc`, opens the editor with the existing values pre-filled (plus any keys added to `.env.example` since the last init/update), validates the buffer, and re-encrypts.

Non-interactive updates are supported in two ways:

- If the `LITELLM_MASTER_KEY` environment variable is set and non-empty when `update` is run, that value replaces the existing `LITELLM_MASTER_KEY` in `.env.enc` and all other keys are kept as-is. This is useful for rotating the master key from a script or shell history-free flow.
- If stdin is not a TTY (e.g. piped from another command or run from a test harness), one line per required key is read from stdin in `REQUIRED_KEYS` order; empty lines keep the existing value, non-empty lines replace it. This is the form used by the validation harness.

`init` refuses to overwrite an existing `.env.enc` — use `update` for changes.

## Running `agentctl` commands with secrets

Inside the dev shell, `agentctl` is already on PATH. Use `run-with-secrets` to
decrypt `.env.enc` and run `agentctl` subcommands:

```bash
nix develop
run-with-secrets litellm up
run-with-secrets pi up
run-with-secrets odysseus up
```

`run-with-secrets` always requires `.env.enc` to exist, so it is only for
commands that need decrypted secrets. For plan-only commands that do not need
secrets, use plain `nix run`:

```bash
nix run . -- litellm plan
nix run . -- pi plan
```

For arbitrary commands inside the dev shell that need the same secrets:

```bash
nix develop -c with-secrets nix run . -- litellm up
```

Outside the dev shell you can also run:

```bash
# 'with-secrets' works anywhere; 'run-with-secrets' only inside the dev shell
nix run .#with-secrets -- nix run . -- litellm up
```

## How agentctl validates secrets

When you run `with-secrets nix run . -- litellm up`, the CLI validates that
`LITELLM_MASTER_KEY` and the provider keys defined in `.env.example` are present. Agent `up` commands
require `LITELLM_MASTER_KEY`. If a required secret is missing, the CLI prints
a clear error and exits before starting any sandbox, so the failure is
attributable to the missing secret rather than opaque sandbox-runtime output.
`check` and `plan` commands never read secret environment variables, so they
can run without `run-with-secrets` or `with-secrets`.

Empty values and whitespace-only values are treated as missing, and the
`LITELLM_MASTER_KEY` placeholder value from `.env.example` (`sk-change-me-local-only`)
is rejected as a known-bad secret with a dedicated error.

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

## Validating the workflow

A non-interactive validation script exercises the full secrets lifecycle
(`setup-secrets init` → `decrypt-env` → `setup-secrets update` → `decrypt-env` →
`write-env` → `with-secrets` → `run-with-secrets`) in an isolated temp directory
using a freshly generated test key. It does not touch the real
`~/.config/sops/age/ai-workbench-secrets.txt` or the repo's `.env.enc`.

```bash
just validate-secrets
# or:
nix develop -c scripts/validate-secrets-workflow.sh
```

The script:

1. Creates a temp working directory and a one-off age key inside it.
2. Copies `.sops.yaml` and rewrites the recipient to the test public key.
3. Runs `setup-secrets init` with secrets supplied via env vars
   (`LITELLM_MASTER_KEY`, `OPENROUTER_API_KEY`, `KIMI_CODE_API_KEY`,
   `MINIMAX_CODING_API_KEY`).
4. Decrypts with `decrypt-env` and asserts the four initial values appear.
5. Runs `setup-secrets update` with one changed value and three preserved
   values, then re-decrypts and asserts the change took effect and the other
   three were preserved.
6. Runs `write-env`, asserts `.env` was created with mode `0600` and contains
   the expected values, then removes the file.
7. Runs `with-secrets env` and asserts all four secrets are exported to the
   child process's environment.
8. Runs `run-with-secrets --help` and asserts agentctl's help text appears
   (proving the decrypt+exec path works end-to-end).
9. Cleans up the temp directory and prints a pass/fail summary.

The script is hermetic: it fails fast (exit 1) if the repo already has a real
`.env.enc` or `.env` at the root, and on exit it removes the temp directory
and any files it created.

## Security rules

- The age private key must never be committed. Keep it under `~/.config/sops/age/ai-workbench-secrets.txt` and chmod 0600.
- `.sops.yaml` may be committed (no secrets in it).
- Do not put secrets in Nix expressions — Nix strings can leak into the world-readable Nix store.
- Do not pass secrets as command-line arguments; `setup-secrets` deliberately omits argv-based secret input so values cannot leak into shell history.
- Do not paste decrypted `.env` into logs, issues, or shell history.
- `.env` is gitignored; `.env.enc` is committed.
- When using the editor flow, secret values are written to a temp file with mode 0600 and removed in a cleanup trap. They will also appear in your terminal scrollback. Clear scrollback or use a private terminal session if that is a concern.
