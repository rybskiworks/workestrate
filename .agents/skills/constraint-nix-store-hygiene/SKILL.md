---
name: constraint-nix-store-hygiene
description: |
  Enforces Nix store hygiene invariants during code execution — GC root
  discipline, no stale `result*` symlinks, `--no-link --print-out-paths` for
  CI builds, `streamLayeredImage` over `buildLayeredImage`, devshell gcroot
  pinning, and the `just gc` / `just store-audit` cadence. Load when running
  nix commands, managing GC roots, building images, or reviewing store-growth
  patterns. Does NOT cover eval-time purity (see constraint-nix-purity) or
  dependency pinning (see constraint-nix-reproducibility).
metadata:
  org.kind: constraint
---

# Constraint: Nix Store Hygiene

This constraint enforces the hygiene rules that keep the Nix store bounded.
The store is append-only and content-addressed: every evaluation or build
copies inputs into the store as new, immutable paths. Nothing is ever
overwritten. Without hygiene discipline, the store grows along three vectors
— per-edit churn, GC root accumulation, and content-addressed duplication —
until the disk fills.

The motivating incident: the store grew to ~29 GB from a single impure eval
path, and across 7 GC cycles a cumulative ~76 GiB was freed as subagent
sessions repeatedly ran impure gates. GC treats the symptom; hygiene
discipline treats the cause.

## Triggers

Load this skill when:

- Running `nix build`, `nix develop`, or `nix eval` commands.
- Managing GC roots (`result*` symlinks, `--out-link`, profile generations).
- Building Docker images with `dockerTools`.
- Reviewing CI scripts or justfile recipes that invoke `nix`.
- Auditing store growth (`just store-audit`, `nix path-info`).
- Editing `flake.lock` (devshell gcroot re-pinning).

## Rules

1. Never leave stale `result*` symlinks. `nix build` (without `--no-link`)
   creates a `result` symlink in the working directory that pins the entire
   build closure forever until removed. Remove `result*` symlinks when done,
   or use `--no-link --print-out-paths` to avoid creating them.
2. Never use `buildLayeredImage` when `streamLayeredImage` suffices.
   `buildLayeredImage` materializes a 0.5–1 GB tarball as a store path that
   accumulates across image revisions. `streamLayeredImage` streams the
   tarball to stdout, leaving no large store path. For new images, always use
   `streamLayeredImage`.
3. Always use `--no-link --print-out-paths` for CI builds. Bare `nix build`
   creates a `result` symlink (a GC root). `--out-link /tmp/foo.tar.gz` pins
   the closure via a symlink + file in `/tmp` that survives across runs. The
   `--no-link --print-out-paths` form pipes the store path directly to
   stdout, leaving no residue.
4. Never run `nix eval --impure` in the container (HOST-NIX). This container
   has no nix; any `nix eval --impure` claim is based on documented
   semantics, not runtime verification. Impure eval copies the raw working
   tree (16–35 GB) into the store on every invocation. Use native `.#` refs
   (git-filtered, ~12 MB) instead.
5. Pin the devshell gcroot when `flake.lock` changes. The devshell toolchain
   (~5–7 GB of Rust/LLVM/GCC) is gcroot-pinned at
   `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. After a
   `flake.lock` change (fenix/nixpkgs bumps), re-pin or the pinned closure
   goes stale and agents fetch the new toolchain anyway.
6. Run `just gc` regularly. `just gc` runs `nix-collect-garbage --delete-old`
   + `nix store optimise` (dedupe), reclaiming unreachable paths and
   deduplicating content-addressed copies (vectors (b) and (c) of the
   store-growth model). Run after `nix flake update`, after large builds,
   and as a periodic cadence.
7. Run `just store-audit` when the store feels large. Reports the top-20
   store paths by closure size and flags any `*-source` paths referencing
   `ai-workbench` (indicating an unbounded source copy that should be bounded
   by a `cleanSourceWith` filter). Wired into `just verify` with a 50 MB
   threshold.

## References

- Docs: `docs/nix/store-hygiene-and-gc.md` (canonical reference — GC commands,
  GC roots, store-growth model, 29 GB incident timeline, anti-accumulation
  patterns, store-audit tooling), `docs/nix/nix-store-and-paths.md` (store
  path types, closures, inspection commands).
- Operational skill: `nix-usage` (anti-accumulation patterns table, agent
  rules, devshell gcroot pin instructions).
- Constraint skill: `constraint-nix-purity` (the `--impure` rule overlaps;
  load both when reviewing store-growth root causes).

## Out of scope

- Eval-time source filtering (`builtins.path` with `filter`,
  `cleanSourceWith`) — see `constraint-nix-purity`.
- Dependency pinning / `flake.lock` discipline — see
  `constraint-nix-reproducibility`.
- `nix.conf` performance tuning (`max-jobs`, `cores`) — no hygiene impact.

## Violation examples

### Stale `result` symlink (pins closure forever)

```bash
# FORBIDDEN — creates result symlink that pins the build closure.
nix build .#myPackage
ls -la result  # ← lingers, pins the closure until manually removed
```

Correct — use `--no-link --print-out-paths`:

```bash
out=$(nix build .#myPackage --no-link --print-out-paths)
# No result symlink created; store path piped to stdout.
```

### `--out-link /tmp/foo.tar.gz` (pins via /tmp residue)

```bash
# FORBIDDEN — leaves a symlink + tarball in /tmp that survives across runs
# and is never garbage-collected by nix.
nix build .#myImage --out-link /tmp/myImage.tar.gz
```

Correct — pipe the store path directly (from `flake.nix` `load-images`):

```bash
out=$(nix build .#${name} --no-link --print-out-paths)
gunzip -c "$out" | msb load -t ${name}:latest
```

### `buildLayeredImage` for a new image (store bloat)

```nix
# FORBIDDEN — materializes a 0.5-1G tarball as a store path; accumulates
# across image revisions.
pkgs.dockerTools.buildLayeredImage {
  name = "my-service";
  contents = [ pkgs.nodejs_22 ];
}
```

Correct — use `streamLayeredImage` (streams to stdout, no store path):

```nix
pkgs.dockerTools.streamLayeredImage {
  name = "my-service";
  tag = "latest";
  contents = [ pkgs.nodejs_22 ];
  config = { Cmd = [ "node" "server.js" ]; };
}
```

### `nix eval --impure` in the container (HOST-NIX)

```bash
# FORBIDDEN — copies the raw working tree (16-35G) into the store on every
# invocation. This container has no nix; this is the 29 GB incident class.
nix eval --impure --expr 'let f = (builtins.getFlake (toString ./.)).packages.x86_64-linux.pi-image; in f.drvPath'
```

Correct — use the native git-filtered `.#` ref form:

```bash
nix eval .#packages.x86_64-linux.pi-image.drvPath
# Store delta: 0 MB (git-filtered, ~12M closure)
```

### Stale devshell gcroot after `flake.lock` change

```bash
# PROBLEM — after nix flake update, the pinned devshell closure is stale;
# agents fetch the new toolchain anyway (no breakage, just no benefit).
nix flake update
# forgot to re-pin the devshell gcroot
```

Correct — re-pin after `flake.lock` changes:

```bash
nix build .#devShells.x86_64-linux.default --out-link \
  /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell
# Rollback: rm the gcroot + nix-collect-garbage -d
```

## How to check

```bash
just store-audit                 # top-20 paths + *-source probe (50 MB gate)
just gc                          # nix-collect-garbage -d + nix store optimise
nix path-info --all --json | python3 scripts/store-audit.py
nix path-info --all | grep -E "ai-workbench.*-source$"  # unbounded source copies
nix-store --gc --print-dead      # preview what GC would delete
du -sh /nix/store                # quick store size check
```

Manual review checklist:

- No stale `result*` symlinks in the working directory.
- CI scripts use `--no-link --print-out-paths`, not bare `nix build` or
  `--out-link`.
- New images use `streamLayeredImage`, not `buildLayeredImage`.
- No `nix eval --impure` anywhere (overlaps with purity constraint).
- Devshell gcroot re-pinned after `flake.lock` changes.
- `just gc` run after `nix flake update` and after large builds.
- `just store-audit` run when the store feels large; no `*-source` paths
  exceeding 50 MB.
