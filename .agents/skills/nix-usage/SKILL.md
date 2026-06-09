---
name: nix-usage
description: Use when working with Nix in the ai-workbench project inside its Docker container — when `nix develop` or `nix build` fails, when the Nix user profile is missing or broken, when no C compiler is available, when a fallback toolchain is needed, or when touching the project flake, dev shells, or the `.toolchain/` build path. Triggers on phrases like "fix Nix", "nix not found", "nix profile broken", "no gcc", "fallback toolchain", ".toolchain/", "flake check", "nix develop", "nix build .#agentctl". Do not use for general Nix tutorials unrelated to this project, or for non-Nix toolchain issues.
---

# Nix Usage in the AI Workbench Project

This is how we do Nix in the ai-workbench project. It is the canonical
reference for operating Nix reliably inside the project's Docker
development container, and for knowing when to fall back to the
project-local toolchain at `.toolchain/`.

## When to use this skill

Load or consult this skill when an agent is about to:

- Run `nix develop`, `nix build`, `nix shell`, `nix flake check`, or
  `nix run` inside `/home/node/Development/ai-workbench/`.
- Diagnose a broken or missing Nix installation in the container
  (e.g. `nix: command not found`, dangling `~/.nix-profile`,
  `nix shell` failing on `~/.nix-profile/bin/...`).
- Repair the Nix user profile so `nix` is on `PATH`.
- Build `agentctl` or any other flake output for the project
  (`nix build .#agentctl`, `nix run .#agentctl`).
- Debug a Rust/C build that needs a C toolchain inside this container
  (no system `gcc`/`cc` is installed).
- Decide between the Nix dev shell and the `.toolchain/` fallback when
  `just` detects that `nix` is not on `PATH`.
- Update or modify `flake.nix`, `flake.lock`, or anything under `nix/`.
- Interpret Nix errors specific to this project's setup
  (crane quirks, `.cargo/config.toml` linker filter, "Git tree ... is dirty"
  warning, source-tracking errors for untracked files).
- Add new package or dev-shell entries to the project flake.

## Project context

- **Project**: ai-workbench — a Rust-based agent control plane
  (`agentctl`) with Nix-driven reproducible builds. The `agentctl`
  source lives in `control/agentctl/`; the LiteLLM proxy config and
  helpers live in `infra/litellm/`; operational scripts live in
  `scripts/`.
- **Project root**: `/home/node/Development/ai-workbench/`
  (the `ai-workestrator` subdirectory was removed; the project
  itself now lives at the workspace root).
- **Container**: Debian 12, single Docker container; this is the
  environment the skill describes.
- **User**: uid 1000 (`node`), no `sudo`, no root, no system package manager.
- **Primary build path**: the project flake (`flake.nix` + `nix/`).
- **Fallback build path**: the project-local toolchain at `.toolchain/`
  (rustup + zig cc), used only when Nix is unavailable.
- **Skill location**: this skill lives at
  `.agents/skills/nix-usage/SKILL.md` inside the project root. The
  historical `docs/nix-env-fix-SKILL.md` is kept as a reference but
  is not the active skill.

## Environment Characteristics

- **Container**: Debian 12, single Docker container.
- **User**: uid 1000 (`node`), no `sudo`, no root, no system package manager.
- **No system C compiler**: the base image does not ship `gcc`, `cc`, or
  `pkg-config`. Anything that needs a C toolchain must get it from Nix
  or the bundled fallback at `.toolchain/`.
- **Nix is single-user**: installed at `/nix`, no `nix-daemon`, no
  multi-user setup. The store at `/nix/store` is pre-populated and
  writable by the Nix CLI for normal operations.
- **Nix CLI is at `/nix/var/nix/profiles/default/bin/nix`**. The user
  profile is `~/.nix-profile`. PATH must include the profile bin
  directory for `nix` to be on `$PATH`.

## When Nix Is Broken: The Profile Repair

The user profile is a symlink at `~/.nix-profile` that should point at
`/nix/var/nix/profiles/default`. If it is dangling, all of `nix shell`,
`nix develop`, and `nix build` will fail with errors about
`/home/node/.nix-profile/bin` not existing.

Repair steps (run as the unprivileged user, no sudo needed):

```bash
# 1. Ensure the profile parent directory exists.
mkdir -p ~/.nix-profile

# 2. Re-create the symlink to the system default profile.
ln -sfn /nix/var/nix/profiles/default ~/.nix-profile

# 3. Put the profile on PATH for the current shell.
export PATH="$HOME/.nix-profile/bin:$PATH"

# 4. Verify.
nix --version                 # should print "nix (Nix) 2.34.7" or similar
nix shell nixpkgs#hello -c hello   # should print "Hello, world!"
```

If step 4 fails on `nix shell nixpkgs#hello -c hello` with a network
error, the Nix cache is unreachable. Check `~/.config/nix/nix.conf` and
the network, then retry.

## Verification Commands

Run these in order. If any fails, jump to the matching fix below.

```bash
# 1. Nix binary on PATH and runnable.
nix --version

# 2. Can evaluate and execute a one-shot shell from nixpkgs.
nix shell nixpkgs#hello -c hello

# 3. Can evaluate a flake (no build required).
cd /home/node/Development/ai-workbench
nix flake check --no-build

# 4. Can enter a flake dev shell and find core tools.
nix develop -c cargo --version   # cargo 1.95.0
nix develop -c rustc --version   # rustc 1.95.0
nix develop -c gcc --version     # gcc (GCC) 15.2.0
nix develop -c just --version    # just 1.51.0

# 5. Can build a flake package.
nix build .#agentctl
./result/bin/agentctl --version # agentctl 0.1.0
```

## Working With Flakes

The project flake at
`/home/node/Development/ai-workbench/flake.nix` provides:

- `nix develop` — full dev shell (rustc, cargo, just, gcc, openssl, git, jq,
  yq, curl, ripgrep, fd, cosign, postgresql_16, rust-analyzer, rustfmt,
  clippy).
- `nix build .#agentctl` — build the agentctl binary via crane.
- `nix build .#default` — alias for the agentctl build.
- `nix run .#agentctl` / `nix run .#default` — build and run.
- `nix fmt` — run nixpkgs-fmt on all `.nix` files.

Flake inputs are pinned in `flake.lock`. Update with
`nix flake update` (CI should not do this; the lock is intentional).

### Flake Hygiene Specific to This Project

- **Source tracking**: Nix flakes read files via the parent git tree.
  New source files must be `git add`-ed (or `git add -f` if `.gitignore`
  excludes them) before `nix build` / `nix flake check` can see them.
  The `control/agentctl` directory must be a regular tracked directory,
  not a gitlink or submodule, or Nix will refuse to read it.
- **`.cargo/config.toml`**: the project pins the linker to
  `.toolchain/ld_zigcc` for the no-Nix fallback. The Nix build filters
  that file out (see `nix/packages/agentctl.nix`) so cargo picks up
  Nix's bundled `cc` wrapper instead. Do not delete the file from the
  source tree — the filter handles it.
- **`buildPackage` quirks**: crane's `crateNameFromCargoToml` helper
  is brittle with relative `src` paths in some Nix versions. The
  package passes `pname`, `version`, and `cargoToml` explicitly.

## C Toolchain

- The Nix dev shell provides `gcc 15.2.0` and `cc` (the gcc-wrapper)
  via `nixpkgs.gcc`. No system compiler is required.
- `pkg-config` and `openssl` are also in the dev shell; the
  `openssl` package exposes both the `openssl` binary and the `lib`
  / `dev` outputs needed by cargo crates that link against it.
- If a Rust build script complains about a missing C compiler, you
  are probably not inside the dev shell. Always run
  `nix develop -c cargo build ...` rather than bare `cargo build`.

## Known Limitations

- **Single-user Nix**: no daemon, no `nix-daemon` service to restart.
  Killing the user session does not affect long-running builds
  (Nix detaches them under `/nix/var/nix/builds`).
- **No `sudo`**: nothing can be installed system-wide. Do not try
  `apt-get install` or `dpkg`; both require privileges you do not
  have. Nix is the only installer.
- **Pre-populated store**: `/nix/store` is large and pre-populated.
  It is not directly user-writable, but `nix build` can add to it
  transparently. If you see `Read-only file system` while writing
  into `/nix/store/...`, the Nix daemon is misbehaving — check
  `/nix/var/log/nix` and run `nix doctor`.
- **Git is dirty warning**: flake evaluation always warns
  "Git tree ... is dirty" because `flake.nix`, `nix/`, and
  `justfile` are uncommitted in the working tree. This is benign;
  ignore it.
- **Build sandbox can be slow on first run**: crane fetches the
  nixpkgs vendor closure and compiles `cargo` itself before
  compiling the crate. First `nix build .#agentctl` takes several
  minutes; subsequent builds are cached.

## Fallback: The `.toolchain/` Path

If Nix is broken and you cannot repair it, the project keeps a
project-local toolchain at `.toolchain/` (rustup + zig cc). It is
used by the `just` recipes when `nix` is not on PATH; the
detection lives in the justfile at the project root.

Manual fallback build:

```bash
cd /home/node/Development/ai-workbench/control/agentctl
CARGO_HOME=../.toolchain/cargo \
RUSTUP_HOME=../.toolchain/rustup \
CC=../.toolchain/cc \
PATH=../.toolchain/cargo/bin:$PATH \
    cargo build --target x86_64-unknown-linux-gnu --bin agentctl
```

Note the `CC=../.toolchain/cc`: the wrapper script invokes
`zig cc` internally, so `zig` does not need to be on PATH.

The fallback only matters for environments that lack Nix entirely.
Inside this container, prefer `nix develop` — it is faster, more
reproducible, and is the canonical build path for the project.

## How to Recognize Each Failure Mode

| Symptom | Cause | Fix |
|---------|-------|-----|
| `nix: command not found` | PATH missing Nix profile | `export PATH=$HOME/.nix-profile/bin:$PATH` |
| `nix shell ... : No such file or directory` for `~/.nix-profile/bin/...` | Dangling profile symlink | Run the profile repair (above) |
| `error: Path 'X' ... is not tracked by Git` | New file added to flake but not staged | `git add X` |
| `unable to infer crate name and version` | crane's auto-detection failed | Add `pname`/`version`/`cargoToml` to the crane call |
| `linking with '.../.toolchain/ld_zigcc' failed` | `.cargo/config.toml` linker in a Nix build | Filter that file out in `nix/packages/agentctl.nix` (already done) |
| `error: access to absolute path '...' is forbidden in pure evaluation mode` | Absolute path passed to a flake | Use a relative path or pass `--impure` |
| `cargo: command not found` (not in Nix shell) | Ran `cargo` outside `nix develop` | Prefix with `nix develop -c ...` |
| `gcc: command not found` | Ran outside `nix develop` and outside `.toolchain/` fallback | Either enter `nix develop` or use the `.toolchain/cc` wrapper |
