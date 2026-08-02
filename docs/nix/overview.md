---
type: Concept
resource: https://nix.dev/index.html
title: Nix Overview
description: Establishes the Nix mental model — what Nix is, purity, reproducibility, the store, derivations, flakes, and how Nix relates to this project.
tags: [nix, overview, fundamentals]
timestamp: 2026-07-24T00:00:00Z
---

# Nix Overview

## Purpose
Establish the Nix mental model: what Nix is, what it deliberately is not, the
distinction between Nix the package manager, Nixpkgs the package collection,
and NixOS the Linux distribution, and the core concepts of purity,
reproducibility, the Nix store, derivations, and flakes. This doc orients
readers to why Nix matters for the workestrator project, where it sits relative
to the Rust/Elixir/Gleam language corpora, and how to think about it correctly.
It is the entry point for the Nix doc set; project-specific purity rules and
flake mechanics live in the sibling docs and in `docs/nix-purity.md`.

## Sources used
- Crawl: `docs/nix/.crawl/01-welcome-to-nix-dev.md` — https://nix.dev/index.html
- Crawl: `docs/nix/.crawl/44-concepts.md` — https://nix.dev/concepts/index.html
- Crawl: `docs/nix/.crawl/45-concepts-faq.md` — https://nix.dev/concepts/faq.html
- Crawl: `docs/nix/.crawl/46-flakes.md` — https://nix.dev/concepts/flakes.html
- Project: `docs/nix-purity.md` — workestrator purity rules and the 29 GB incident
- Project: `flake.nix` — the actual project flake
- Project: `.agents/skills/nix-usage/SKILL.md` — project Nix toolchain reference

## Core guidance

Nix is a tool for people who need computers to do exactly as intended,
repeatably, far into the future (nix.dev welcome). The name *Nix* is derived
from the Dutch word *niks*, meaning *nothing* — build actions do not see
anything that has not been explicitly declared as an input (concepts FAQ,
citing Dolstra 2004).

### What Nix is

Nix is three things at once:

1. **A package manager.** It installs software into a content-addressed store,
   isolates versions so they never conflict, and supports atomic upgrades and
   rollbacks (nix.dev welcome: "Atomic upgrades and rollbacks").
2. **A build system.** It builds software from source in a sandboxed
   environment where only declared inputs are visible, producing reproducible
   artifacts that can be cached and shared via binary caches.
3. **An expression language.** The Nix language is a small, pure, lazy,
   functional language used to describe derivations (build recipes). It is not
   a general-purpose application language; it exists to produce store paths.

With the Nix ecosystem, you can (nix.dev welcome):
- Create reproducible development environments.
- Install software over URLs.
- Transfer software environments between computers.
- Declaratively specify Linux machines (NixOS).
- Run reproducible integration tests using virtual machines.
- Avoid version conflicts with already-installed software.
- Build from source with transparent build caching.
- Cross-compile, remote-build, and remote-deploy.

### What Nixpkgs is

Nixpkgs is the package collection — a massive, community-maintained repository
of Nix expressions describing how to build tens of thousands of software
packages. It is the analog of `apt`'s package archive or `homebrew-core`, but
expressed entirely in the Nix language. The project flake pins `nixpkgs` via
`nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"` and consumes it as
`nixpkgs.legacyPackages.${system}`.

### What NixOS is

NixOS is a Linux distribution built on top of Nix and Nixpkgs. The entire
system configuration — kernel, services, users, packages — is declared in Nix
expressions, making the OS declarative, reproducible, and atomically
rollback- able. The workestrator project does not run NixOS; it runs Nix as a
package manager and build tool on Debian 12 inside a single container.

### What the Nix store is

The Nix store (`/nix/store`) is a content-addressed, append-only filesystem.
Every build output is stored in a path whose name encodes a cryptographic hash
of all inputs that produced it. This means:

- The same inputs always produce the same store path.
- Different inputs produce different paths — multiple versions coexist without
  conflict.
- Paths are immutable once created.
- Garbage collection reclaims paths no longer reachable from any GC root
  (profiles, `result*` symlinks, `nix build` outputs).

The store's content-addressing is what makes Nix reproducible and conflict-free
at the mechanical level. It is also why impure source closures are dangerous:
a dirty source closure changes hash on every edit, producing a new store path
each time and ballooning the store (see `docs/nix-purity.md` — the 29 GB
incident).

### What a derivation is

A derivation is a build recipe — a Nix expression that describes how to produce
a store path from declared inputs. It specifies a name, a builder (usually a
bash script), input derivations, environment variables, and output hashes. The
Nix evaluator turns the expression into a `.drv` file (a low-level build
description), and the builder runs in a sandbox to produce the output path.

Key properties:
- Derivations are pure functions of their inputs.
- Fixed-output derivations (FODs) declare an `outputHash`; the output is
  verified by hash, not by build steps. This is what permits network access in
  the fetch phase (fetchers run as FODs, not as the main build).
- The workestrator flake uses `stdenv.mkDerivation`, `buildNpmPackage`,
  `dockerTools.streamLayeredImage`, and custom FODs throughout `nix/packages/`.

### What purity is

Purity in Nix has two independent axes. **Both must hold** for a derivation to
be considered pure (see `docs/nix-purity.md`):

1. **Eval-time purity** — what Nix sees when it computes the derivation graph,
   before any build runs. The evaluation walks the flake's source closure and
   turns it into store paths. If the closure is impure (untracked files, large
   build outputs, directories that change on every edit), every evaluation
   produces a new source closure and Nix copies the whole thing into the store
   each time. Rules: no `toString ./.`, no `builtins.getFlake`, no impure
   `builtins` outside controlled wrappers, no referencing untracked or large
   directories (`target/`, `agents/*/build/`, `node_modules/`).

2. **Build-time purity** — what happens inside the build sandbox once
   evaluation has produced a derivation. The sandbox has no network in
   `buildPhase`/`installPhase`; all dependencies must arrive via FODs with
   declared `outputHash`es; `--impure` is forbidden everywhere. A sandboxed
   build is necessary but not sufficient: eval impurity balloons the store
   even when the build itself is sandboxed.

Flakes default to pure mode (hermetic evaluation), which prevents evaluating
non-network impure functions (flakes concept doc). This promotes a style of
writing code more likely to be reproducible, though reproducibility is not
actually guaranteed even in pure mode (flakes concept doc, citing discourse).

### What reproducibility is

Reproducibility means the same inputs produce bit-for-bit identical outputs,
today and in the future. Nix's content-addressed store and sandboxed builds
make this the default goal, but it is not absolute. Residual impurities in
sandboxed builds include (concepts FAQ):
- CPU architecture.
- System current time/date.
- The filesystem used for building (`TMPDIR`).
- Linux kernel parameters (IPv6 capabilities, binfmt interpreters).
- Timing behaviour of parallel build systems.
- Insertion of random values (`/dev/random`, `/dev/urandom`).
- Differences between Nix versions (e.g. new environment variables).

For the workestrator project, reproducibility means: a `nix build .#workestrate`
on any machine with Nix and the same `flake.lock` produces the same binary.
The `flake.lock` pins all transitive inputs; the `fenix` input pins the exact
Rust toolchain (`rustc 1.97.1`); the `nixpkgs` input pins the exact package
set.

### What a flake is

A flake is a self-contained, reproducible source unit — a directory containing
a `flake.nix` file that declares `inputs` and `outputs` with a standard
structure (flakes concept doc). Flakes make it easy to build programs with the
same version by pinning dependencies in a `flake.lock` file.

- `flake.nix` declares inputs (dependencies) and outputs (packages, dev shells,
  apps, checks, etc.).
- `flake.lock` pins the exact revisions of all inputs; Nix checks transitive
  lock files to find the versions to use.
- Flakes default to pure mode (hermetic evaluation).
- Flakes build only tracked files (Git-staged), which helps prevent rebuilds.
- Flakes are an experimental feature (since Nix 2.4) with outstanding design
  issues, but are widely depended on in practice.

The workestrator flake (`flake.nix`) declares inputs for `nixpkgs`, `fenix`
(Rust toolchain), and four agent source repos (`pi`, `odysseus`, `opencode`,
`tempest`), and outputs dev shells, packages, apps, and a `lib` export.

### Why Nix matters for this project

Nix is the only supported entry point for the workestrator project. All
builds, dev tools, and the Rust toolchain come from the flake (nix-usage
skill). Specifically:

- **Toolchain pinning.** The `fenix` input pins the exact `rustc` version; both
  the dev shell and the nix build consume the same `rustToolchain`, so they
  always agree on the compiler.
- **Reproducible agent builds.** Each agent (`pi`, `odysseus`, `opencode`,
  `tempest`) is built via a hermetic nix derivation with a declared dependency
  hash (`npmDepsHash`, `outputHash`, `pipDeps`).
- **Reproducible images.** Docker images are built via
  `dockerTools.streamLayeredImage` / `buildLayeredImage`, producing
  content-addressed tarballs loaded into Microsandbox.
- **Reproducible dev shells.** `nix develop` produces a shell with all tools
  (Rust, Node, Bun, Python, sops, msb) at pinned versions, so every contributor
  has the same environment.
- **Purity enforcement.** `just lint-nix` (via `scripts/check-nix-paths.sh`)
  statically guards against the impurity patterns that caused the 29 GB store
  incident.

### Relationship to other corpora

Nix is infrastructure, not a language. It provides the build environment,
toolchain, and dependency management for the code written in Rust, Elixir, and
Gleam. The language corpora (`docs/rust/`, `docs/elixir/`, `docs/gleam/`)
cover how to write and validate code in those languages; the Nix corpus covers
how to build and reproduce that code. The corpora are complementary:

- `docs/rust/` — Rust language, Cargo, tooling. The Rust toolchain itself comes
  from the flake (`fenix`), but Rust code mechanics are corpus-scoped.
- `docs/elixir/` — Elixir/OTP. Nix provides the Erlang/OTP runtime if needed.
- `docs/gleam/` — Gleam language. Nix provides the Gleam compiler if needed.
- `docs/nix/` — this corpus. Build system, purity, flake mechanics, store
  hygiene.

## Practical rules
- Treat Nix as the single source of truth for toolchain and dependency
  versions. Do not install tools via `apt-get`, `cargo install`, `npm install
  -g`, or any other channel. Everything comes from the flake.
- Treat purity as two independent axes. A sandboxed build is not enough; the
  eval-time source closure must also be pure. See `docs/nix-purity.md`.
- Never reference the repo root as a path (`src = ./.`). Use `builtins.path`
  with an explicit `name` and `filter`.
- All dependency fetching goes through fixed-output derivations with declared
  hashes. No `fetchTarball` without a hash, no `builtins.fetchGit` of mutable
  refs.
- No `--impure` anywhere: not in scripts, not in the justfile, not in ad-hoc
  commands.
- Pin everything. The `flake.lock` is the reproducibility contract; do not
  update it without understanding the consequences.
- Run `just gc` regularly to collect unreachable store paths and deduplicate.

## Review checklist
- [ ] Is the toolchain sourced from the flake, not from the host system?
- [ ] Are all dependency hashes declared (`npmDepsHash`, `outputHash`,
      `pipDeps`)?
- [ ] Is `--impure` absent from all nix invocations?
- [ ] Are source paths filtered (`builtins.path` with `filter`), not bare
      `./.`?
- [ ] Does `just lint-nix` pass?
- [ ] Is the `flake.lock` committed and up to date?
- [ ] Are large untracked directories (`target/`, `agents/*/build/`,
      `node_modules/`) excluded from the flake source closure?

## Implementation checklist
- [ ] Confirm `nix --version` works and PATH includes `~/.nix-profile/bin`.
- [ ] Confirm `nix develop` produces a shell with the expected tools.
- [ ] Confirm `nix build .#<name>` succeeds for the target package.
- [ ] Confirm `just lint-nix` passes (the static purity guard).
- [ ] Confirm `flake.lock` is committed.
- [ ] If adding a new derivation, follow the checklist in `docs/nix-purity.md`
      ("How to add a new derivation").

## Validation hooks
- `nix --version` — confirms Nix is installed and on PATH.
- `nix flake check` — validates flake structure and runs checks.
- `nix build .#<name>` — builds a package; success confirms the derivation is
  eval-pure and build-pure.
- `nix develop` — enters the dev shell; success confirms the shell derivation
  evaluates.
- `just lint-nix` — runs `scripts/check-nix-paths.sh`, the static purity guard
  that catches forbidden patterns (`--impure`, unfiltered `builtins.path`,
  `builtins.getFlake` + `toString`, `../` outside escape-hatch fields).
- `just gc` — runs `nix-collect-garbage --delete-old` + `nix store optimise`.
- `just store-audit` — reports top-20 store paths by closure size and flags
  unbounded `*-source` paths referencing `ai-workbench`.

## Examples

A minimal flake (from the flakes concept doc):

```nix
{
  description = "My example flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }: {
    packages.x86_64-linux = {
      default = self.packages.x86_64-linux.hello;
      hello = nixpkgs.legacyPackages.x86_64-linux.hello;
    };
  };
}
```

Building and running from a flake:

```bash
# Build the default package
nix build .#default

# Run an app
nix run .#default

# Run a package from nixpkgs directly
nix run nixpkgs#hello -- --greeting "hello from flakes"
```

The workestrator project's entry points (from `flake.nix`):

```bash
# Enter the dev shell (all tools pinned)
nix develop

# Build the workestrate binary
nix build .#workestrate

# Run workestrate
nix run .#default

# Build a workload image
nix build .#pi-image
```

## Common mistakes
- **Installing tools outside Nix.** Using `apt-get`, `cargo install`, or
  `npm install -g` breaks reproducibility. Everything comes from the flake.
- **Bare `src = ./.`.** Copies the entire repo (including `target/`,
  `node_modules/`) into the store on every evaluation. This caused the 29 GB
  incident. Use `builtins.path` with a `filter`.
- **Unfiltered `cleanSourceWith`.** Same problem as bare `./.` — large
  untracked directories enter the source closure. Always pass a `filter`
  predicate.
- **`--impure` in scripts or ad-hoc commands.** Breaks the reproducibility
  contract. Forbidden everywhere.
- **Forgetting to commit `flake.lock`.** The lock file is the reproducibility
  contract; an uncommitted lock file means the next evaluation may resolve to
  different inputs.
- **Treating flakes as stable.** Flakes are an experimental feature with
  outstanding design issues. They work, but be aware of the limitations.
- **Assuming pure mode guarantees reproducibility.** Even in pure mode,
  reproducibility is not actually guaranteed (residual impurities: CPU arch,
  time, kernel params, random values, Nix version differences).
- **Confusing Nix, Nixpkgs, and NixOS.** Nix is the tool; Nixpkgs is the
  package collection; NixOS is the Linux distribution. The workestrator
  project uses Nix + Nixpkgs, not NixOS.

## Strict vs contextual guidance
- Strict: no `--impure`, no bare `src = ./.`, no unfiltered `cleanSourceWith`,
  no `builtins.getFlake` + `toString`, all deps via FODs with declared hashes,
  `flake.lock` committed. These are project-level and non-negotiable (enforced
  by `just lint-nix` and the purity rules in `docs/nix-purity.md`).
- Contextual: which nixpkgs channel to track (`nixos-unstable` vs a stable
  release), whether to use `follows` for transitive inputs, whether to use
  `streamLayeredImage` vs `buildLayeredImage` for a given image, whether to
  enable `auto-optimise-store`. These are decided per project / per image.

## Policy decisions for individual repos
- Which nixpkgs channel should the project track (`nixos-unstable` vs a pinned
  stable release)?
- Should transitive inputs use `follows` to deduplicate `nixpkgs`?
- Is `auto-optimise-store` enabled, or is `just gc` run manually?
- Which image builder is preferred (`streamLayeredImage` vs
  `buildLayeredImage` vs `buildImage`) for each workload?
- Are flakes the only entry point, or is `flake-compat` / non-flake access
  supported?
- What is the GC root hygiene cadence (how often is `just gc` run)?

## Related docs
- `docs/nix-purity.md` — canonical purity rules, the 29 GB incident, store-growth
  model, `just lint-nix` enforcement, derivation checklist, recipe patterns.
- `docs/nix/flakes.md` — flake mechanics, inputs/outputs, `flake.lock`, `follows`
  (to be created).
- `docs/nix/store-and-gc.md` — store model, GC roots, hygiene cadence,
  `just gc` / `just store-audit` (to be created).
- `docs/nix/derivations.md` — derivation anatomy, FODs, `stdenv.mkDerivation`,
  `buildNpmPackage`, `dockerTools` (to be created).
- `docs/nix/dev-shells.md` — `nix develop`, shell hooks, toolchain pinning
  (to be created).
- `docs/nix/language-basics.md` — the Nix expression language: lazy evaluation,
  attributes, let-in, import (to be created).

## Related skills
- `nix-usage` — project Nix flake, dev shell, Rust toolchain, Microsandbox
  runtime reference.

# Citations

[1] [Welcome to nix.dev](https://nix.dev/index.html)
[2] [Nix concepts](https://nix.dev/concepts/index.html)
[3] [Nix concepts FAQ](https://nix.dev/concepts/faq.html)
[4] [Flakes](https://nix.dev/concepts/flakes.html)
[5] [Nix: A Safe and Policy-Free System for Software Deployment (Dolstra, LISA 2004)](https://edolstra.github.io/pubs/nspfssd-lisa2004-final.pdf)
