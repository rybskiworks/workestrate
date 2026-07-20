---
name: nix-usage
description: |
  Reference for the ai-workbench Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime. Load when users ask about `nix develop`, `nix build`,
  `nix flake check`, `nix run`, flake outputs, Nix PATH issues, building
  `workestrate`, or why `msb` is downloaded at runtime. Does NOT cover general
  Nix tutorials.
---

# Nix-First Toolchain Management

Nix is the only supported entry point for this project. All builds, dev
tools, and the Rust toolchain come from the flake. If Nix is broken, fix it.

## Triggers

Load this skill when:

- Running or explaining `nix develop`, `nix build`, `nix flake check`, or
  `nix run` inside `/home/node/Development/ai-workbench/`.
- Diagnosing Nix PATH/profile issues (`nix: command not found`, dangling
  `~/.nix-profile`, missing `~/.nix-profile/bin/...`).
- Building `workestrate` (`nix build .#workestrate`).
- Explaining that `msb` is downloaded at runtime by the Microsandbox SDK.
- Modifying `flake.nix`, `flake.lock`, or files under `nix/`.

## Project Context

- **Project root**: `/home/node/Development/ai-workbench/`
- **Container**: Debian 12, single Docker container
- **User**: `node` (uid 1000), no `sudo`, no root
- **Source**: Rust `workestrate` lives in `control/agentctl/`
- **Entry point**: the project flake (`flake.nix` + `nix/`)

## Environment Characteristics

- Single-user Nix at `/nix`, no daemon
- No system C compiler, no `apt-get`, no `dpkg`
- Nix CLI at `/nix/var/nix/profiles/default/bin/nix`
- Profile symlink: `~/.nix-profile` → `/nix/var/nix/profiles/default`
- PATH must include `~/.nix-profile/bin`

## Fixing a Broken Nix Profile

Run as the unprivileged user (no `sudo`):

```bash
mkdir -p ~/.nix-profile
ln -sfn /nix/var/nix/profiles/default ~/.nix-profile
export PATH="$HOME/.nix-profile/bin:$PATH"
nix --version
nix shell nixpkgs#hello -c hello
```

## Verification Commands

```bash
# Nix itself
nix --version
nix shell nixpkgs#hello -c hello

# Flake
cd /home/node/Development/ai-workbench
nix flake check --no-build

# Dev shell tools
nix develop -c cargo --version
nix develop -c rustc --version
nix develop -c gcc --version
nix develop -c just --version

# Package build
nix build .#workestrate
./result/bin/workestrate --version
```

## Working With Flakes

Current outputs:

- `devShells.x86_64-linux.default` — `nix develop`
- `packages.x86_64-linux.workestrate` — `nix build .#workestrate`

There is NO `packages.default`, NO `nix fmt`, and NO `nix run .#default`.

The dev shell (`nix/devshells/default.nix`) provides `cargo`, `clippy`,
`gcc`, `just`, `pkg-config`, `rustc`, `rustfmt` in `nativeBuildInputs`,
plus `git`, `libcap_ng`, and `openssl` in `buildInputs`.

Lock file inputs:

- `nixpkgs` (github:NixOS/nixpkgs/nixos-unstable)
- `pi` (github:georgrybski/pi, `flake=false`)
- `odysseus` (github:georgrybski/odysseus, `flake=false`)

`agents/pi/` is an optional gitignored local override; `workestrate check`
reports it `[MISSING]` if absent. The locked `pi` input is what
`nix build .#workestrate` actually uses.

Update the lock only deliberately:

```bash
nix flake update
```

CI should NOT run `nix flake update`.

## Flake Hygiene

- **Git-tracked sources**: new files must be `git add`-ed before `nix build`
  can see them.
- **`.cargo/config.toml`**: the package derivation filters it out so cargo
  uses Nix's `cc` wrapper. Do not delete it.
- **Source filter**: the derivation excludes `target`, `result`, and
  `result-`.
- **Microsandbox build.rs**: writes to `$HOME/.microsandbox/bin`. The
  derivation sets `HOME=$TMPDIR`. Outside the Nix sandbox, do the same.
- **Runtime daemon**: `msb` and the kernel image are NOT bundled in the
  Nix closure. The Microsandbox Rust SDK downloads them on first use.

## Store hygiene

- **Churn model.** Every impure eval copies the source closure into the
  store; with dirty filters each edit produces a new content-addressed
  path. The 29 GB-per-eval incident (see
  [`docs/nix-purity.md`](../../../docs/nix-purity.md)) is the cautionary
  tale: an unfiltered `src = ./.` copied `target/` and `agents/*/build/`
  on every evaluation, accumulating ~1 GB/min until caught.

- **GC cadence.** Run `just gc` regularly to collect unreachable store
  paths; run `just store-audit` when the store feels large to audit
  space consumption and find stale roots.

  > **ASSUMPTION (Track 1/2):** recipe names `gc` and `store-audit` are
  > pending — Track 1/2 own the exact recipe names and flags.

- **Relocation paths.** `agents/<name>/build` outputs and
  `CARGO_TARGET_DIR` are being relocated out of the flake-visible source
  tree so they no longer pollute eval: `agents/<name>/build` → managed
  sources store (`~/.local/share/workestrate/sources/<name>/`);
  `CARGO_TARGET_DIR` → `~/.cache/ai-workbench/agentctl-target`.

  > **ASSUMPTION (Track 1/2):** the relocation targets and the
  > `CARGO_TARGET_DIR` path are pending — Track 1/2 own the final
  > paths.

- **The guard.** `just lint-nix` (backed by `scripts/check-nix-paths.sh`)
  runs in `just verify` and forbids `toString ./`, `getFlake`,
  `--impure`, bare `src = ./.`, and unfiltered `cleanSourceWith`. See
  [`docs/nix-purity.md`](../../../docs/nix-purity.md) for the full rules
  and the store-growth model.

## C Toolchain

- `gcc`, `pkg-config`, and `rustc`/`cargo` come from Nix.
- `openssl` is in `buildInputs` for crates that link against it.
- If a build complains about a missing C compiler, you are outside the dev
  shell.

## Anti-patterns

- **NO bare `cargo` or `rustc`** — always use `nix develop` or
  `nix develop -c ...`.
- **NO `apt-get` / `dpkg`** — add packages to `nix/devshells/default.nix`.
- **NO `rustup`** — the toolchain comes from Nix.
- **NO bypassing the flake** because it is slow or seems broken — diagnose
  and fix.
- **NO `nix fmt`** — the project does not configure a formatter.
- **NO `nix build .#default`** — there is no such output.
- **NO expecting `msb` in the closure** — it is downloaded at runtime by the
  SDK.

## Known Limitations

- Single-user Nix (no daemon). If you see write errors to `/nix/store`,
  run `nix doctor`.
- No `sudo`; system-wide changes are impossible.
- Pre-populated `/nix/store`.
- "Git tree is dirty" warnings are benign because `flake.nix` and `nix/`
  are uncommitted.

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| `nix: command not found` | PATH missing Nix profile | `export PATH=$HOME/.nix-profile/bin:$PATH` |
| `~/.nix-profile/bin/...` not found | Dangling profile symlink | Run the profile repair (above) |
| `Path 'X' is not tracked by Git` | Untracked source file | `git add X` |
| `linking with '.../.toolchain/...'` failed | Ran cargo outside `nix develop` | Use `nix develop -c cargo ...` |
| `error: 'packages.x86_64-linux.default' is not a flake output` | Used `.#default` | Use `.#workestrate`; there is no default |
| `cargo: command not found` / `gcc: command not found` | Outside dev shell | Run inside `nix develop` |
| `msb: command not found` at runtime | SDK has not downloaded it yet | First use downloads it; check network or pre-stage |
| Build error mentioning `$HOME/.microsandbox/bin` | build.rs writing outside sandbox | Set `HOME=$TMPDIR` (derivation already does this) |
| Store grows ~1GB/min during edits | Impure source filter copying target/ or agents/*/build | Run `just gc` + `just store-audit`; fix the source filter per docs/nix-purity.md |
