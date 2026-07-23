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
  tale. See the [anti-accumulation patterns table](#anti-accumulation-patterns--agent-rules)
  below for the full pattern → safe-alternative → guard mapping.

- **GC cadence.** Run `just gc` regularly to collect unreachable store
  paths; run `just store-audit` when the store feels large to audit
  space consumption and find stale roots.

  > **Confirmed:** `just gc` runs `nix-collect-garbage --delete-old` + `nix store optimise`; `just store-audit` reports the top-20 store paths by size and flags `*-source` paths referencing `ai-workbench`.

- **Relocation paths.** `CARGO_TARGET_DIR` is relocated out of the flake-visible source tree to `${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target` (set by the devshell shellHook and the top-level justfile). The relocation of `agents/<name>/build` outputs into the managed sources store (`~/.local/share/workestrate/sources/<name>/`) is **deferred** (needs config-repo changes; see `docs/migration/70-open-items.md`). Until then, `agents/<name>/build` remains the sanctioned in-tree dev zone, excluded from the flake source closure by `.gitignore` and the `agentctl.nix` `cleanSourceWith` filter.

- **The guard.** `just lint-nix` (backed by `scripts/check-nix-paths.sh`)
  is the ACTIVE gate wired into `just verify`. It catches `--impure`
  invocations, `builtins.getFlake` + `toString`, `builtins.path { ... }`
  without `filter =`, `cleanSourceWith { ... }` without `filter =`, and
  `../` path literals outside `src =`/`lockFile =`/`path =` fields. It
  does NOT catch bare `src = ./.` — rule 1 in
  [`docs/nix-purity.md`](../../../docs/nix-purity.md) remains
  authoritative even when the guard passes. See the [guards reference](#guards-as-they-exist)
  below for the full check inventory and the V2/V3 landing notes.

## Anti-accumulation patterns & agent rules

This is the operational quick-reference for agents running nix tasks on
this repo. The incident class it prevents: agents running impure
evaluation gates that copy the raw working tree into the store on every
invocation, accumulating 16–35G per eval until the disk fills. The
git-tracked repo is ~735 files / ~12M; an impure eval can copy 16–28G
of gitignored `target/` alone.

### Patterns table

| # | Pattern | What it does | Safe alternative | Guard |
|---|---|---|---|---|
| a | Impure/path-mode flake eval: `nix eval --impure`, `builtins.getFlake (toString ./.)`, unfiltered `builtins.path { path = ./.; }` / `src = ./.` | Copies the raw working tree (gitignored `target/` 16–28G, `agents/*/build`, `.workestrate/`) into the store — up to ~35G per invocation; content-addressed so every dirty edit produces a new path | Native flake refs `nix eval .#attr` (git-filtered, ~12M) or filtered `builtins.path { path = ./subdir; filter = ...; }` | `just lint-nix` (checks 1–4); `just store-audit` flags `*-source` paths |
| b | GC-root leaks: `result` symlinks, `/tmp/*.tar.gz` out-links, `nix profile install` | Pins closures forever — unreachable paths survive GC | `--no-link --print-out-paths` (no `result` symlink); `just gc` to collect | `just store-audit` (flags `*-source` paths); manual inspection |
| c | `nix flake update` on rolling nixpkgs | Multi-GB rebuild — new nixpkgs revision pulls new toolchain/closure | Update deliberately (never in CI); run `just gc` after | Operational discipline (no static guard) |
| d | `buildLayeredImage` for new images | 0.5–1G tarballs materialized in the store | `streamLayeredImage` (streams to stdout, no store path) — documented default in `docs/nix-purity.md` | Operational discipline (`docs/nix-purity.md` recipe patterns) |
| e | Per-edit churn: `nix develop` after edits | ~50–100MB per unique source state (content-addressed copies accumulate) | `auto-optimise-store` + `just gc` cadence | `just store-audit` (top-20 report); `just gc` |

### Agent rules (verbatim)

1. **Regression/eval gates use ONLY native `.#` refs.** NEVER `--impure`.
   NEVER `toString ./.` or `getFlake` on this repo. NEVER `builtins.path`
   on the repo root without a `filter =` field.
2. **Stage new files (`git add -N`) before eval** — untracked files are
   invisible to `.#` refs (classic flakes gotcha; the error looks like a
   missing file/attr).
3. **Clean up out-links** — remove `result*` symlinks and `/tmp/*.tar.gz`
   out-links when done; they pin closures forever.
4. **Run `just lint-nix` before committing** nix-adjacent changes.
5. **After `nix flake update`, run `just gc`** — a new nixpkgs revision
   pulls a multi-GB toolchain closure.
6. **The devshell toolchain is gcroot-pinned** (Work Item 3, commit
   a315f24) at
   `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. If it is
   stale after a `flake.lock` change, re-pin:
   ```bash
   nix build .#devShells.x86_64-linux.default --out-link \
     /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
   ```
   Rollback: `rm` the gcroot + `nix-collect-garbage -d`.

### Guards (as they exist)

- **`just lint-nix`** (`scripts/check-nix-paths.sh`) — ACTIVE gate,
  wired into `just verify`. Fails on: (1) `nix ... --impure`, (2)
  `builtins.getFlake` + `toString` on the same line, (3) `builtins.path
  { ... }` without `filter =` in the following 15 lines, (4)
  `cleanSourceWith { ... }` without `filter =` in the following 15
  lines, (5) `../` path literals outside `src =`/`lockFile =`/`path =`
  escape-hatch fields. **Does NOT catch bare `src = ./.`** — rule 1 in
  `docs/nix-purity.md` remains authoritative even when the guard passes.
  Allowlist: `# allow: <reason>` on a line skips it.

- **`just store-audit`** (`scripts/store-audit.py`) — currently
  PASSIVE / NON-BLOCKING (always exits 0 so `verify` cannot fail on
  it). Reports the top-20 store paths by closure size and flags any
  `*-source` paths referencing `ai-workbench` (impure-path probe). Skips
  with a one-line note when nix is unavailable.

  > **Landing (Track 1):** a fail-threshold (V2) will make store-audit
  > block `verify` when `*-source` paths exceed a threshold, and a
  > store-delta check (V3) will fail on new source-path copies exceeding
  > the <50M criterion from the remediation spec. Until V2/V3 land,
  > store-audit is advisory only — rely on `just lint-nix` as the
  > active gate.

- **`just gc`** — `nix-collect-garbage --delete-old` + `nix store
  optimise` (dedupe). Reclaims unreachable paths and deduplicates
  content-addressed copies.

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
