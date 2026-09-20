---
name: nix-usage
description: |
  Reference for the ai-workbench Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime. Load when users ask about `nix develop`, `nix build`,
  `nix flake check`, `nix run`, flake outputs, Nix PATH issues, building
  `workestrate`, or how `msb` is provided in this repo. Does NOT cover general
  Nix tutorials.
---

# Nix-First Toolchain Management

Nix is the only supported entry point for this project. All builds, dev
tools, and the Rust toolchain come from the flake. If Nix is broken, fix it.

## Triggers

Load this skill when:

- Running or explaining `nix develop`, `nix build`, `nix flake check`, or
  `nix run` inside `/home/node/Development/agent-workbench/workestrate/`.
- Diagnosing Nix PATH/profile issues (`nix: command not found`, dangling
  `~/.nix-profile`, missing `~/.nix-profile/bin/...`).
- Building `workestrate` (`nix build .#workestrate`).
- Explaining how `msb` is provided (Nix-pinned `msb-wrapped` in the dev
  shell; `MSB_PATH` baked into the binary).
- Modifying `flake.nix`, `flake.lock`, or files under `nix/`.

## Corpus References

The `docs/nix/` corpus is the canonical source for Nix topic reference. This
skill is the project-specific entry point; the topic docs below hold the full
detail, upstream source URLs, and crawl ledgers.

### Topic docs

| Doc | Scope |
|---|---|
| [`docs/nix/testing.md`](../../../docs/nix/testing.md) | `nix flake check`, `checks` output, `checkPhase`/`doCheck`, `nixosTests`, `testers`, `runCommand` |
| [`docs/nix/store-hygiene-and-gc.md`](../../../docs/nix/store-hygiene-and-gc.md) | GC roots, `nix-collect-garbage`, `nix store optimise`, anti-accumulation patterns, store-audit, the 29 GB incident |
| [`docs/nix/cross-compilation.md`](../../../docs/nix/cross-compilation.md) | `pkgsCross`, `pkgsStatic`, build/host/target, nine dependency types, `qemu-user`, multi-platform flakes |
| [`docs/nix/ci-cd-integration.md`](../../../docs/nix/ci-cd-integration.md) | GitHub Actions, Cachix, `nix flake check` in CI, `--no-link --print-out-paths`, HOST-NIX gates |
| [`docs/nix/nix-store-and-paths.md`](../../../docs/nix/nix-store-and-paths.md) | Store layout, path format, closure semantics, inspection commands |
| [`docs/nix/derivations-and-builds.md`](../../../docs/nix/derivations-and-builds.md) | `mkDerivation`, build phases, dependency attributes, `strictDeps` |
| [`docs/nix/flake-anatomy.md`](../../../docs/nix/flake-anatomy.md) | Flake inputs, outputs, system keying, `checks`/`packages`/`devShells` |
| [`docs/nix/devshells.md`](../../../docs/nix/devshells.md) | `mkShell`, `nativeBuildInputs`/`buildInputs` in dev shells |
| [`docs/nix-purity.md`](../../../docs/nix-purity.md) | Purity rules, `just lint-nix`, source filters, store-growth model |

### Related operational skills

| Skill | Scope |
|---|---|
| [`nix-testing`](../nix-testing/SKILL.md) | Writing and running Nix tests — `nix flake check`, `checkPhase`, `nixosTests`, `runCommand` checks |
| [`nix-store-gc`](../nix-store-gc/SKILL.md) | Store hygiene, GC, `nix-collect-garbage`, `nix store optimise`, store-audit, anti-accumulation |
| [`nix-cross-compilation`](../nix-cross-compilation/SKILL.md) | `pkgsCross`, `pkgsStatic`, build/host/target, `qemu-user`, multi-platform flakes |
| [`nix-ci-cd`](../nix-ci-cd/SKILL.md) | GitHub Actions, Cachix, `nix flake check` in CI, `--no-link --print-out-paths`, HOST-NIX gates |
| [`nix-docker-images`](../nix-docker-images/SKILL.md) | `dockerTools`, `streamLayeredImage`, `buildLayeredImage`, OCI image building |

## Project Context

- **Project root**: `/home/node/Development/agent-workbench/workestrate/`
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
cd /home/node/Development/agent-workbench/workestrate
nix flake check --no-build

# Dev shell tools
just shell
just shell -c cargo --version
just shell -c rustc --version
just shell -c gcc --version
just shell -c just --version

# Package build
nix build .#workestrate
./result/bin/workestrate --version
```

## Working With Flakes

Current outputs:

- `devShells.x86_64-linux.default` — enter via `just shell` (wraps `nix
  develop` with the devenv-root override)
- `packages.x86_64-linux.workestrate` — `nix build .#workestrate`

There is NO `packages.default`, NO `nix fmt`, and NO `nix run .#default`.

The dev shell is a devenv config defined inline in `flake.nix` (perSystem);
its packages list provides `cargo`, `clippy`, `rustc`, `rustfmt`,
`rust-analyzer`, `gcc`, `just`, `pkg-config`, `git`, `libcap_ng`, `openssl`,
`sops`, `age`, `tombi`, `jq`, `curl`, `nodejs_24`, `bun`, python3/3.12, plus
`msb-wrapped` and `workestrate`.

Devshell entry is `just shell`: the flake declares a `devenv-root`
placeholder input and `just shell` passes `--override-input devenv-root
"file+file://<worktree>"` — pure eval needs it; bare `nix develop` is not a
supported entry (fails the `devenv.root != ""` assertion, or silently falls
back to PWD when impure). Guarded `just` recipes self-enshell the same way.

Lock file inputs (root; revs as currently locked in `flake.lock` — re-read
the lock before citing, pins move with deliberate updates):

- `nixpkgs` — `github:NixOS/nixpkgs` @ `a799d3e3886da994fa307f817a6bc705ae538eeb`
- `fenix` — `github:nix-community/fenix` @ `fa09e6473a0dfd673e6cb9a37741aec513b4bb2a` (owned toolchain pin; its `nixpkgs` follows ours)
- `microsandbox-fork` — `github:rybskiworks/microsandbox` @ `78fb3ed12623526ad02f5999047c12953013c395`
- `tooling` — `github:rybskiworks/nix-tooling` @ `2a5796179339a2322e3d01f599e82e3322c8ad4b` (shared devenv modules; its `nixpkgs` follows ours)
- `devenv` — `github:cachix/devenv` @ `97135e80b6e432f41f84f72383e1b8147f33ef0c`
- `flake-parts` — `github:hercules-ci/flake-parts` @ `9d0d87172c374f89da73c1cfe6d81ae62feac1f1`
- `git-hooks` — `github:cachix/git-hooks.nix` @ `27555e2624241fb116b49095df4caaee85a25691`
- `mk-shell-bin` — `github:rrbutani/nix-mk-shell-bin` @ `ff5d8bd4d68a347be5042e2f16caee391cd75887`
- `nix2container` — `github:nlewo/nix2container` @ `76be9608a7f4d6c985d28b0e7be903ae2547df3e`
- `treefmt-nix` — `github:numtide/treefmt-nix` @ `27b3b12a8e6375f28ebe122f07d230ca5459bbfa`
- `devenv-root` — `file:///dev/null` placeholder (devenv root-file thunk)

There are NO `pi`/`odysseus` flake inputs in this repo. `agents/<name>/repo`
directories are optional gitignored local overrides (`workestrate check`
reports them `[MISSING]`); agent sources are pinned in the fleets' own
flakes, not here.

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
- **Runtime daemon**: `msb` is Nix-provided — the `microsandbox` package
  builds it (pinned 0.6.16 at the fork rev above); the dev shell ships
  `msb-wrapped` and the `agentctl` derivation bakes `MSB_PATH` to the store
  binary. No runtime binary download.

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

- **Relocation paths.** `CARGO_TARGET_DIR` is relocated out of the flake-visible source tree to `${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target` (set by the devshell shellHook and the top-level justfile). The relocation of `agents/<name>/build` outputs into the managed sources store (`~/.local/share/workestrate/sources/<name>/`) is **deferred** (needs fleet changes; see `docs/migration/70-open-items.md`). Until then, `agents/<name>/build` remains the sanctioned in-tree dev zone, excluded from the flake source closure by `.gitignore` and the `agentctl.nix` `cleanSourceWith` filter.

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

- **`just store-audit`** (`scripts/store-audit.py`) — reports the
  top-20 store paths by closure size and flags any `*-source` paths
  referencing `ai-workbench` (impure-path probe). Skips with a one-line
  note when nix is unavailable.

  > **Status (Track 1):** the source-path scan runs with
  > `--warn-if-source-over 50` and `just store-audit` is wired into
  > `just verify` as the final step; the scan is informational (WARN to
  > stderr, always exits 0; non-blocking skip when nix or python3 are
  > unavailable). The earlier V3 `store-delta-check` recipe
  > no longer exists in the justfile.

- **`just gc`** — `nix-collect-garbage --delete-old` + `nix store
  optimise` (dedupe). Reclaims unreachable paths and deduplicates
  content-addressed copies.

## C Toolchain

- `gcc`, `pkg-config`, and `rustc`/`cargo` come from Nix.
- `openssl` is in `buildInputs` for crates that link against it.
- If a build complains about a missing C compiler, you are outside the dev
  shell.

## Anti-patterns

- **NO bare `cargo` or `rustc`** — always use `just shell` or
  `just shell -c ...`.
- **NO `apt-get` / `dpkg`** — add packages to the devenv packages list in
  `flake.nix`.
- **NO `rustup`** — the toolchain comes from Nix.
- **NO bypassing the flake** because it is slow or seems broken — diagnose
  and fix.
- **NO `nix fmt`** — the project does not configure a formatter.
- **NO `nix build .#default`** — there is no such output.
- **NO hand-downloading `msb` binaries** — it is Nix-provided
  (`msb-wrapped` in the dev shell; host profile via `just host-provision`).

## Known Limitations

- Single-user Nix (no daemon). If you see write errors to `/nix/store`,
  run `nix doctor`.
- No `sudo`; system-wide changes are impossible.
- Pre-populated `/nix/store`.
- **Git-filtered sources**: `nix build .` sees only git-tracked files;
  untracked work is invisible until `git add -N` (see Flake hygiene).
  Dirty-worktree warnings are benign.

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| `nix: command not found` | PATH missing Nix profile | `export PATH=$HOME/.nix-profile/bin:$PATH` |
| `~/.nix-profile/bin/...` not found | Dangling profile symlink | Run the profile repair (above) |
| `Path 'X' is not tracked by Git` | Untracked source file | `git add X` |
| `linking with '.../.toolchain/...'` failed | Ran cargo outside `just shell` | Use `just shell -c cargo ...` |
| `error: 'packages.x86_64-linux.default' is not a flake output` | Used `.#default` | Use `.#workestrate`; there is no default |
| `cargo: command not found` / `gcc: command not found` | Outside dev shell | Run inside `just shell` |
| `msb: command not found` | Outside the dev shell and no profile install | Use `just shell` (ships `msb-wrapped`) or `just host-provision`; `MSB_PATH` overrides the resolved binary |
| Build error mentioning `$HOME/.microsandbox/bin` | build.rs writing outside sandbox | Set `HOME=$TMPDIR` (derivation already does this) |
| Store grows ~1GB/min during edits | Impure source filter copying target/ or agents/*/build | Run `just gc` + `just store-audit`; fix the source filter per docs/nix-purity.md |
