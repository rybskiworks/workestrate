---
name: constraint-nix-secret-hygiene
description: |
  Enforces Nix secret-hygiene invariants during code execution — never embed
  secrets in Nix expressions, use SOPS/age for secret management, resolve
  secrets at runtime via os.environ/ not eval-time, and use the
  with-secrets wrapper to inject secrets into child processes. Load when
  writing or reviewing Nix code that handles secrets, derivations, or
  service configuration. Does NOT cover build sandbox purity (see
  constraint-nix-sandbox-safety) or Nix expression scoping (see
  constraint-nix-scope-discipline).
metadata:
  org.kind: constraint
---

# Constraint: Nix Secret Hygiene

The Nix store (`/nix/store`) is world-readable: store paths are mode 0444 by
default, so any string interpolated into a derivation ends up readable by any
user or process on the host. Secrets must therefore be provided at runtime,
never at build time. Violations leak secrets into the world-readable store,
build logs, or shell history, where they persist indefinitely and are
effectively impossible to revoke cleanly. A leaked store path remains
readable until garbage-collected, and build logs are often retained for
debugging long after the build that produced them. The only safe model is:
ciphertext at rest (`.env.enc`), plaintext only in process memory at runtime,
injected by the `with-secrets` wrapper.

## Triggers

Load this skill when:

- Writing or reviewing Nix derivations that reference API keys, passwords, or tokens.
- Configuring services that consume secrets (LiteLLM, Odysseus, agents).
- Using `builtins.getEnv` or environment-variable access in Nix code.
- Reviewing a PR that touches `.env.enc`, `.sops.yaml`, or secret-wrapper packages.

## Rules

1. Never put secrets in Nix expressions — Nix strings can leak into the
   world-readable Nix store (store paths are mode 0444 by default).
   - Treat every string in a derivation as if it will be published; if it is
     secret, it must not appear in a `.nix` file.
2. Never hardcode API keys, passwords, or tokens in `.nix` files
   (e.g. `apiKey = "sk-..."`).
   - This includes placeholder-looking values that happen to be real, and
     "local-only" keys committed for convenience.
3. Use SOPS with age for secret management; secrets live as `.env.enc`
   (ciphertext-safe to commit).
   - The `.env.enc` file is the single source of truth; `.env` (plaintext) is
     gitignored and generated on demand.
4. Secrets must come from `os.environ/` at runtime, not eval-time —
   `builtins.getEnv` in a derivation is forbidden outside controlled wrappers.
   - Eval-time access bakes the value into the store path; runtime access
     keeps the secret out of `/nix/store`.
5. Never commit the age private key to git (even encrypted) — keep it at
   `~/.config/sops/age/ai-workbench-secrets.txt` (mode 0600), never under
   `.workestrate/` or `$WORKESTRATE_CONFIG`.
   - The key decrypts every secret in the repo; committing it (even
     encrypted) defeats the entire SOPS/age model.
6. `decrypt-env` writes to stdout only; `write-env` writes plaintext `.env`
   with mode 0600 (via `umask 077` + `chmod 600`) and refuses to overwrite an
   existing `.env`.
   - Refusing to overwrite prevents clobbering a hand-edited `.env` and
     avoids accidental plaintext exposure of a stale file.
7. `with-secrets` wrapper validates required env vars are set before running
   the child process; never pass secrets as command-line arguments (they
   leak into shell history and `ps`).
   - Command-line arguments are visible to every process on the host via
     `/proc` and to shell history files.

## References

- Docs: `docs/nix/secrets-and-sops.md`, `docs/secrets.md`.
- Sibling skill: `nix-usage` (ai-workbench Nix flake, dev shell, secret wrappers).

## Out of scope

- Build sandbox purity and `__noChroot` — see `constraint-nix-sandbox-safety`.
- Nix expression scoping (`with`, `rec`, `<nixpkgs>`) — see
  `constraint-nix-scope-discipline`.

## Violation examples

### Hardcoded API key in a Nix expression

```nix
# FORBIDDEN: secret lands in the world-readable Nix store
{ pkgs }:
pkgs.writeShellApplication {
  name = "litellm-proxy";
  text = ''
    export LITELLM_MASTER_KEY="sk-abc123..."
    litellm up
  '';
}
```

Correct: never embed the key in the derivation. Use `with-secrets` to inject
`LITELLM_MASTER_KEY` into the child process env at runtime:
`just shell -c with-secrets nix run . -- litellm up`. The derivation text
stays secret-free; the key is read from the environment only when the child
process starts, so it never enters `/nix/store`.

### `builtins.getEnv` in a derivation body

```nix
# FORBIDDEN: eval-time secret access bakes the value into the store
stdenv.mkDerivation {
  pname = "my-service";
  version = "0.1.0";
  configurePhase = ''
    echo "${builtins.getEnv "API_KEY"}" > config/secrets.json
  '';
}
```

Correct: fetch the secret at runtime via a FOD or inject via `with-secrets`;
never interpolate `builtins.getEnv` into a derivation string.
`builtins.getEnv` evaluates at eval time, so the secret value becomes part of
the store path and is permanently readable by anyone with store access.

### Passing a secret as a command-line argument

```bash
# FORBIDDEN: leaks into shell history and ps
nix run . -- litellm up --master-key "sk-abc123..."
```

Correct: `just shell -c with-secrets nix run . -- litellm up` —
`with-secrets` injects `LITELLM_MASTER_KEY` into the child env without
exposing it on the command line. The key never appears in `ps`,
`/proc/<pid>/cmdline`, or shell history.

### Using the placeholder `sk-change-me-local-only` as `LITELLM_MASTER_KEY`

```bash
# FORBIDDEN: rejected as a known-bad secret
export LITELLM_MASTER_KEY="sk-change-me-local-only"
```

Correct: set a real `sk-...` value via `setup-secrets --config <name> init`
(interactive or via env vars). The placeholder is explicitly rejected by the
secrets lifecycle as a known-bad value; using it silently disables auth
instead of failing loudly.

## How to check

```bash
grep -rn "sk-\|api_key\|password\|token" *.nix nix/ flake.nix   # no secret literals
just lint-nix                                                    # static purity guard
just validate-secrets                                            # hermetic secrets lifecycle test
just shell -c decrypt-env                                        # prints decrypted .env.enc to stdout
stat -c '%a' ~/.config/sops/age/ai-workbench-secrets.txt        # must be 600
```

A clean run produces no `grep` hits in `.nix` files, `just validate-secrets`
exits 0, and the age key file reports mode `600`. Any `grep` hit on a `sk-`
literal or `builtins.getEnv` interpolation inside a derivation string is a
blocking failure.

Manual review:

- No secret literals (sk-..., passwords, tokens) in any `.nix` file.
- No `builtins.getEnv` interpolating secrets into derivation strings.
- age private key at `~/.config/sops/age/ai-workbench-secrets.txt` with mode
  0600, never under `.workestrate/` or `$WORKESTRATE_CONFIG`.
- `.env.enc` and `.sops.yaml` committed; `.env` gitignored.
- Secrets injected via `with-secrets`/`run-with-secrets`, never as
  command-line arguments.
- No secret values echoed in build logs or `echo` statements inside
  derivation phases.
