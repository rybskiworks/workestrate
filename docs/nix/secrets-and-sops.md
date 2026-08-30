---
type: Reference
resource: https://nix.dev/guides/best-practices.html
title: Secrets and SOPS
description: Secret management in Nix — why the Nix store leaks secrets, SOPS with age encryption, sops-nix and agenix NixOS modules, and the ai-workbench secret workflow (decrypt-env, write-env, setup-secrets wrappers; the CLI's secret command is workestrate run -- <cmd>).
tags: [nix, secrets, sops, age, sops-nix, agenix]
timestamp: 2026-07-24T01:30:00Z
---

# Secrets and SOPS

## Purpose

Source-verified guidance for secret management in Nix. Covers why the Nix
store leaks secrets, SOPS with age encryption, the sops-nix and agenix NixOS
modules, and the ai-workbench secret workflow (`decrypt-env`, `write-env`,
`setup-secrets` wrappers; the CLI's secret command is `workestrate run -- <cmd>`).
Agents who handle
secrets in a Nix project should follow these rules so secrets never land in
the world-readable Nix store or in plaintext at rest.

## Sources used

- Project file: `docs/secrets.md` — the project's secrets documentation (PRIMARY SOURCE)
- Project file: `flake.nix` — SOPS integration (`decrypt-env`, `write-env`,
  `setup-secrets` packages)
- Project file: `.envrc` — direnv integration and `WORKESTRATE_HOME`
- Project file: `nix/devshells/default.nix` — `sops` and `age` in devshell
- External: https://github.com/getsops/sops — SOPS (Secrets OPerationS)
- External: https://age-encryption.org/ — age encryption
- External: https://github.com/Mic92/sops-nix — sops-nix NixOS module
- External: https://github.com/ryantm/agenix — agenix NixOS secret management
- External: https://nix.dev/guides/best-practices.html — Nix best practices (resource)

## Core guidance

### Why secrets in Nix are special

The Nix store (`/nix/store`) is world-readable: store paths are mode `0444`
by default. Any string interpolated into a derivation ends up in the store
and is readable by any user or process on the machine. Derivations can leak
secrets via build logs, environment variables baked into wrappers, and store
paths. This is why secrets must NEVER be embedded in Nix expressions — they
must be provided at runtime, not build time.

> "Do not put secrets in Nix expressions — Nix strings can leak into the
> world-readable Nix store."
> — docs/secrets.md

### SOPS (Secrets OPerationS)

SOPS (Secrets OPerationS) is a tool that encrypts the VALUES of YAML/JSON/ENV
files while leaving the KEYS in plaintext. It supports multiple key types:
age, GPG, AWS KMS, GCP KMS, and Azure Key Vault. It supports multiple
recipients and key groups (shamir-style threshold). Encrypted files are
diff-friendly (git-trackable) because keys remain visible.

The project uses SOPS with the dotenv input/output type for `.env.enc` files.

> "ai-workbench uses [SOPS](https://github.com/getsops/sops) with an
> [age](https://age-encryption.org/) recipient to keep secrets out of plain
> text in the repo and out of plaintext at rest."
> — docs/secrets.md

### age encryption

age (Actually Good Encryption) is a modern, simple file encryption tool. It
generates an age keypair: a private key (identity) and a public key
(recipient). It supports SSH keys as age recipients (via `ssh-to-age`
conversion). age keys are small, fast, and have no configuration files.

In this project, the age private key lives at
`~/.config/sops/age/ai-workbench-secrets.txt` on the HOST.

> "Key file: `~/.config/sops/age/ai-workbench-secrets.txt` on the HOST
> (project-specific, NEVER in repo, NEVER in bundle, NEVER under
> `.workestrate/` or `$WORKESTRATE_HOME`)"
> — docs/secrets.md

### sops-nix

sops-nix is a NixOS module that decrypts SOPS-encrypted secrets at boot (or
activation) into tmpfs files. Secrets are never written to the world-readable
Nix store; they land in a root-owned tmpfs at `/run/secrets/`. Each secret is
declared with a `sops.secrets.<name>` module option specifying the source file
and per-secret permissions (owner, group, mode). It requires a `.sops.yaml`
config in the repo and the age private key on the machine (or via a host SSH
key). Best suited for NixOS deployments (not pure flake/devshell workflows like
ai-workbench).

### agenix

agenix is an alternative NixOS secret management tool, also built on age.
Secrets are encrypted to a per-host public key derived from the host's SSH host
key. Each secret is an `.age` file in the repo, configured via an `age` module
option and a `secrets.nix` file listing recipients. It decrypts at activation
into `/run/agenix/` with per-secret ownership.

Comparison with sops-nix:

| Dimension | sops-nix | agenix |
|---|---|---|
| Secret file layout | One SOPS file with many keys | One `.age` file per secret |
| Recipient source | Arbitrary recipient lists and key groups | Host SSH host keys |
| Best for | Multi-recipient / key-group deployments | Per-host SSH-key-based deployments |

### The ai-workbench secret workflow

This is the PRIMARY content — drawn from `docs/secrets.md`.

Secrets live as a SOPS-encrypted `.env.enc` file committed in each config
repo (ciphertext-safe). The age private key stays on the HOST.

#### The 7 secrets

| Key | Used for |
|---|---|
| `LITELLM_MASTER_KEY` | Local LiteLLM proxy authentication (any `sk-…` string; `sk-change-me-local-only` is rejected as a known-bad secret) |
| `OPENROUTER_API_KEY` | OpenRouter provider |
| `KIMI_CODE_API_KEY` | Kimi for Coding provider |
| `NEURALWATT_API_KEY` | Neuralwatt provider |
| `MINIMAX_CODING_API_KEY` | MiniMax Coding provider |
| `GITHUB_TOKEN` | GitHub Personal Access Token for agent sandboxes (git operations + API) |
| `ODYSSEUS_ADMIN_PASSWORD` | Odysseus admin login (required because `AUTH_ENABLED=true`; without it Odysseus auto-generates a random password printed to logs) |

#### Where secrets live

| Artifact | Location | Notes |
|---|---|---|
| Per-config-repo `.env.enc` | `$WORKESTRATE_HOME/repos/<name>/.env.enc` | SOPS-encrypted; ciphertext-safe to commit in the config repo |
| Per-config-repo `.sops.yaml` | `$WORKESTRATE_HOME/repos/<name>/.sops.yaml` | SOPS recipient config; no secrets in it |
| User-global secrets | `$WORKESTRATE_HOME/secrets/.env.local.enc` | Applied per-key across all contexts |
| age private key | `~/.config/sops/age/ai-workbench-secrets.txt` (HOST) | NEVER in repo/bundle/`.workestrate/` |

#### The flake packages

From `flake.nix`, the wrappers are `pkgs.writeShellApplication` derivations:

- `decrypt-env`: sets `SOPS_AGE_KEY_FILE` defaulting to
  `~/.config/sops/age/ai-workbench-secrets.txt`, reads `.env.enc` (or
  `$SECRET_FILE`), and runs `sops decrypt --input-type dotenv --output-type
  dotenv`. Prints decrypted secrets to stdout.
- `write-env`: decrypts `.env.enc` to a temp file (mode 0600 via `umask 077`),
  then `mv` to `.env` with `chmod 600`. Refuses to overwrite an existing
  `.env`.
- `setup-secrets`: wraps `scripts/setup-secrets.sh`; sets `SOPS_AGE_KEY_FILE`
  and execs the script. Supports `--config <name> init|update`,
  `--global init|update`, and auto-detect modes.
- The CLI's secret command `workestrate run -- <cmd>`: decrypts `.env.enc`
  and execs `<cmd>` with the secrets in its environment (a CLI subcommand,
  not a flake wrapper).

#### Wrappers reference

| Wrapper | Runs on | Purpose |
|---|---|---|
| `setup-secrets` | Host (age key present) | Create/update `.env.enc` (`--config`) or `.env.local.enc` (`--global`) |
| `decrypt-env` | Host | Print decrypted secrets to stdout |
| `write-env` | Host | Write a short-lived plaintext `.env` (mode 0600) |

The CLI's `workestrate workload up`/`exec` commands decrypt `.env.enc`
themselves and inject the secrets — they are CLI subcommands, not flake
wrappers.

> In a container, the age key is absent, so all of these fail closed by
> design.
> — docs/secrets.md

#### Canonical invocation pattern

```bash
workestrate workload up litellm   # dev shell: workestrate is on PATH
```

#### Security rules

> - The age private key must never be committed. Keep it at
>   `~/.config/sops/age/ai-workbench-secrets.txt` and chmod 0600. NEVER place
>   it under `.workestrate/` or `$WORKESTRATE_HOME` (the repo is
>   agent-reachable via `${CWD}` mounts).
> - `.sops.yaml` may be committed (no secrets in it — only the age recipient).
> - `.env.enc` is ciphertext-safe and may be committed in config repos.
> - Do not put secrets in Nix expressions — Nix strings can leak into the
>   world-readable Nix store.
> - Do not pass secrets as command-line arguments; `setup-secrets`
>   deliberately omits argv-based secret input so values cannot leak into
>   shell history.
> - Do not paste decrypted `.env` into logs, issues, or shell history.
> - `.env` is gitignored; `.env.enc` is committed (in config repos).
> - When using the editor flow, secret values are written to a temp file with
>   mode 0600 and removed in a cleanup trap. They will also appear in your
>   terminal scrollback. Clear scrollback or use a private terminal session if
>   that is a concern.
> — docs/secrets.md

### Key management

age key generation:

```shell
age-keygen -o ~/.config/sops/age/ai-workbench-secrets.txt
```

The key file must be mode 0600. The public key (recipient) is the second line
of the key file, prefixed `# public key: age1...`.

> "Back up this key to a secure location. Without it, `.env.enc` cannot be
> decrypted."
> — docs/secrets.md

SSH keys as age recipients: `ssh-to-age` converts an SSH public key to an age
recipient; `ssh-to-age -private-key` converts an SSH private key to an age
identity. This lets NixOS hosts use their existing SSH host keys for SOPS.

### .sops.yaml configuration

`.sops.yaml` lives at the repo root (or config repo root) and defines creation
rules. Creation rules map file path globs to key groups (lists of recipients).

A typical `.sops.yaml`:

```yaml
creation_rules:
  - path_regex: \.env\.enc$
    age: "age1PLACEHOLDER..."
```

In ai-workbench, `setup-secrets init` replaces the `age1PLACEHOLDER...`
recipient with the actual public key. `.sops.yaml` contains NO secrets — only
the age recipient — so it is safe to commit.

> "`.sops.yaml` may be committed (no secrets in it — only the age recipient)."
> — docs/secrets.md

### Secret rotation

Rotate secret values by running `setup-secrets --config <name> update`
(decrypts, opens editor, re-encrypts). To rotate the age key itself: generate
a new keypair, re-encrypt all `.env.enc` files to the new recipient (update
`.sops.yaml`, then `sops updatekeys .env.enc`), and remove the old key.
Rotation is non-interactive when all required env vars are set.

### CI/CD secret handling

In CI, provide secrets via environment variables (the non-interactive path) or
via stdin (one line per key in `REQUIRED_KEYS` order). The age private key
must be provisioned as a CI secret (e.g. GitHub Actions secret) and written to
`~/.config/sops/age/ai-workbench-secrets.txt` before running `setup-secrets` or
`workestrate run --`.

> "**Env vars:** if all required env vars are set and non-empty, those values
> are used directly."
> "**stdin:** if stdin is not a TTY (e.g. piped from another command or run
> from a test harness), one line per required key is read from stdin in
> `REQUIRED_KEYS` order; empty lines preserve the existing value, non-empty
> lines replace it."
> — docs/secrets.md

Never print decrypted secrets in CI logs; `workestrate run --` injects into the
child process env, not stdout. The validation script (`just validate-secrets`)
is hermetic and safe to run in CI.

## Practical rules

1. Never embed secrets in Nix expressions — the Nix store is world-readable.
2. Use SOPS + age for encrypted secret files committed to the repo.
3. Keep the age private key on the HOST at
   `~/.config/sops/age/ai-workbench-secrets.txt` (mode 0600), never in the
   repo, bundle, `.workestrate/`, or `$WORKESTRATE_HOME`.
4. Use `setup-secrets` to create/update `.env.enc`; never edit ciphertext by
   hand.
5. Use `workestrate run -- <cmd>` to inject secrets into a child process —
   never write plaintext `.env` unless a tool requires it.
6. If a plaintext `.env` is needed, use `write-env` (mode 0600) and `rm .env`
   immediately after.
7. Do not pass secrets as command-line arguments — they leak into shell
   history and `ps`.
8. `.sops.yaml` is safe to commit (contains only the recipient); `.env.enc`
   is ciphertext-safe to commit; `.env` is gitignored.
9. In CI, provision the age key as a CI secret and provide secret values via
   env vars or stdin.
10. Back up the age private key — without it, `.env.enc` is unrecoverable.
11. Rotate secrets via `setup-secrets update`; rotate keys via
    `sops updatekeys`.
12. In a container, the age key is absent by design — secret operations fail
    closed.

## Review checklist

- [ ] No secrets embedded in Nix expressions or derivation strings?
- [ ] age private key at `~/.config/sops/age/ai-workbench-secrets.txt` with mode 0600?
- [ ] age key NOT under repo, bundle, `.workestrate/`, or `$WORKESTRATE_HOME`?
- [ ] `.sops.yaml` committed (recipient only, no secrets)?
- [ ] `.env.enc` committed (ciphertext-safe)?
- [ ] `.env` gitignored and removed after use?
- [ ] `setup-secrets` used for create/update (not hand-editing ciphertext)?
- [ ] `workestrate run -- <cmd>` used to inject secrets into commands?
- [ ] No secrets passed as command-line arguments?
- [ ] No decrypted secrets pasted into logs, issues, or shell history?
- [ ] age private key backed up to a secure location?
- [ ] CI provisions the age key as a secret before running secret operations?

## Implementation checklist

- [ ] Install `sops` and `age` in the devshell (`nix/devshells/default.nix`).
- [ ] Add `decrypt-env`, `write-env`, `setup-secrets` as flake packages
      (`flake.nix`).
- [ ] Create `.sops.yaml` with creation rules mapping `.env.enc` to the age
      recipient.
- [ ] Run `setup-secrets --config <name> init` to create `.env.enc`.
- [ ] Set `SOPS_AGE_KEY_FILE` in wrappers (defaulting to the host path).
- [ ] Ensure wrappers fail closed when the age key is absent (container
      context).
- [ ] Add `.env` to `.gitignore`.
- [ ] Commit `.env.enc` and `.sops.yaml` to the config repo.
- [ ] Provision the age key as a CI secret for non-interactive flows.

## Runtime / debugging checklist

- [ ] `decrypt-env` prints decrypted secrets to stdout (key present)?
- [ ] `setup-secrets init` creates `.env.enc` without error?
- [ ] `workestrate run -- bash -c 'echo $LITELLM_MASTER_KEY'` shows the secret in the child env?
- [ ] `write-env` creates `.env` with mode 0600?
- [ ] "secret file not found" — check `SECRET_FILE` / cwd has `.env.enc`?
- [ ] "error: .env already exists" — `rm .env` and retry `write-env`?
- [ ] Decryption fails in container — expected (key absent, fail-closed)?
- [ ] `LITELLM_MASTER_KEY` placeholder `sk-change-me-local-only` rejected — set a real key?
- [ ] `just validate-secrets` passes the full lifecycle?

## Validation hooks

- `just validate-secrets` — hermetic lifecycle test (setup → decrypt →
  update → write-env → `workestrate run -- env` → `workestrate --help`).
- `nix develop -c decrypt-env` — prints decrypted `.env.enc` to stdout.
- `workestrate run -- bash -c 'echo $LITELLM_MASTER_KEY'` — verifies
  injection.
- `nix develop -c write-env && stat -c '%a' .env` — verifies mode 0600.

> "A non-interactive validation script exercises the full secrets lifecycle
> (`setup-secrets init` → `decrypt-env` → `setup-secrets update` →
> `decrypt-env` → `write-env` → `workestrate run -- env` → `workestrate --help`)
> in an isolated temp directory using a freshly generated test key."
> — docs/secrets.md

## Examples

### Generate an age keypair

```shell
mkdir -p ~/.config/sops/age
age-keygen -o ~/.config/sops/age/ai-workbench-secrets.txt
chmod 600 ~/.config/sops/age/ai-workbench-secrets.txt
# The public key (recipient) is printed to stderr and embedded as a comment:
#   # public key: age1q...
```

### Minimal .sops.yaml

```yaml
creation_rules:
  - path_regex: \.env\.enc$
    age: "age1qzc4l...your-public-key..."
```

### Initialize secrets (interactive)

```shell
nix develop -c setup-secrets --config personal init
```

### Initialize secrets (non-interactive via env vars)

```shell
export LITELLM_MASTER_KEY="sk-..."
export OPENROUTER_API_KEY="sk-or-..."
# ... all 7 keys ...
nix develop -c setup-secrets --config personal init
```

### Update secrets

```shell
nix develop -c setup-secrets --config personal update
```

### Decrypt to stdout

```shell
nix develop -c decrypt-env
```

### Write a short-lived plaintext .env

```shell
nix develop -c write-env
# ... do work ...
rm .env
```

### Run a command with secrets injected

```shell
# Inside the dev shell (workestrate is on PATH):
workestrate workload up litellm

# Arbitrary commands with the same decrypted env (works in any shell; the
# binary is nix-profile-installed):
workestrate run -- bash -c 'echo $LITELLM_MASTER_KEY'
```

### Plan-only commands (no secrets needed)

```shell
nix run . -- workload plan litellm
nix run . -- workload plan pi
```

### sops-nix module (NixOS) — for reference

```nix
{ config, ... }:
{
  sops.defaultSopsFile = ./secrets.yaml;
  sops.age.keyFile = "/var/lib/sops-nix/key.txt";
  sops.secrets.my-api-key = {
    owner = "my-service";
    mode = "0400";
  };
}
```

### agenix (NixOS) — for reference

```nix
# secrets.nix
let
  host1 = "ssh-ed25519 AAAA... host1";
in {
  "my-api-key.age".publicKeys = [ host1 ];
}

# configuration.nix
age.secrets.my-api-key = {
  file = ./secrets/my-api-key.age;
  owner = "my-service";
  mode = "0400";
};
```

## Common mistakes

- Embedding secrets in Nix strings — they land in the world-readable Nix
  store.
- Committing the age private key to the repo — it is agent-reachable via
  `${CWD}` mounts.
- Placing the age key under `.workestrate/` or `$WORKESTRATE_HOME` — same
  exposure.
- Hand-editing `.env.enc` ciphertext — corrupts the file; use
  `setup-secrets update`.
- Passing secrets as command-line arguments — leaks into shell history and
  `ps`.
- Leaving a plaintext `.env` on disk after `write-env` — remove it
  immediately.
- Using the placeholder `sk-change-me-local-only` as `LITELLM_MASTER_KEY` —
  rejected as a known-bad secret.
- Expecting secret operations to work in a container — the age key is absent
  by design (fail-closed).
- Forgetting to back up the age private key — `.env.enc` becomes
  unrecoverable.
- Running `check`/`plan` through `workestrate run --` unnecessarily — those
  commands never read secret env vars.
- Overwriting an existing `.env` with `write-env` — it refuses; `rm .env`
  first.

## Strict vs contextual guidance

### Strict (always follow)

- Never embed secrets in Nix expressions (Nix store is world-readable).
- Keep the age private key on the HOST, never in repo/bundle/`.workestrate/`/
  `$WORKESTRATE_HOME`.
- Use `setup-secrets` for create/update; never hand-edit ciphertext.
- Use `workestrate run -- <cmd>` to inject secrets into commands.
- `.sops.yaml` and `.env.enc` are safe to commit; `.env` is gitignored.
- Do not pass secrets as command-line arguments.
- Back up the age private key.

### Contextual (depends on the project)

- sops-nix vs agenix — depends on NixOS deployment style (single SOPS file
  vs per-secret `.age` files).
- age vs GPG — age is simpler and preferred for new projects; GPG for orgs
  with existing key infrastructure.
- Plaintext `.env` fallback (`write-env`) — only when a tool requires a
  literal file.
- CI secret provisioning method — env vars vs stdin vs mounted key file.
- Key rotation cadence — depends on threat model and compliance requirements.
- Multi-recipient key groups (shamir threshold) — only for high-security
  multi-party deployments.

## Policy decisions for individual repos

- Default to SOPS + age for all secret management.
- Require the age private key at
  `~/.config/sops/age/ai-workbench-secrets.txt` (mode 0600) on the host.
- Require `.sops.yaml` and `.env.enc` committed; `.env` gitignored.
- Require `setup-secrets` for all create/update operations.
- Require `workestrate run -- <cmd>` for commands needing secrets.
- Require the age key provisioned as a CI secret for non-interactive flows.
- Require `just validate-secrets` to pass before merge for secret-workflow
  changes.
- Decide per-repo whether to use sops-nix or agenix for NixOS deployments
  (ai-workbench itself uses the CLI's `workestrate run --` secret command,
  not a NixOS module).

## Related docs

- /docs/secrets.md — the project's secrets documentation (primary source)
- /docs/nix/devshells.md — Nix development shells (sops/age in devshell)
- /docs/nix/flake-anatomy.md — flake outputs (`decrypt-env`, `write-env`,
  `setup-secrets` packages)
- /docs/nix/purity-and-sandboxing.md — Nix purity and sandboxing
- /docs/nix/store-hygiene-and-gc.md — Nix store hygiene

## Related skills

- nix-usage — ai-workbench Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime reference

## Citations

[1] [SOPS — Secrets OPerationS](https://github.com/getsops/sops)
[2] [age — Actually Good Encryption](https://age-encryption.org/)
[3] [sops-nix NixOS module](https://github.com/Mic92/sops-nix)
[4] [agenix — NixOS secret management](https://github.com/ryantm/agenix)
[5] [Nix best practices](https://nix.dev/guides/best-practices.html)
[6] [ai-workbench secrets documentation](../secrets.md)
[7] [ai-workbench flake.nix](../../flake.nix)
[8] [ai-workbench .envrc](../../.envrc)
[9] [ai-workbench devshell](../../nix/devshells/default.nix)
