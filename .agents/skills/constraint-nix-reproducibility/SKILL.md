---
name: constraint-nix-reproducibility
description: |
  Enforces Nix reproducibility invariants during code execution — flake input
  pinning, fetcher hashes, FOD output verification, flake.lock discipline, and
  no mutable refs. Load when building packages, creating fixed-output
  derivations, pinning dependencies, or editing flake inputs. Does NOT cover
  eval-time source filtering (see constraint-nix-purity) or store hygiene/GC
  (see constraint-nix-store-hygiene).
metadata:
  org.kind: constraint
---

# Constraint: Nix Reproducibility

This constraint enforces the reproducibility rules that make Nix builds
bit-for-bit repeatable across machines and time. Reproducibility is distinct
from purity: a pure derivation is hermetic (no impure inputs), while a
reproducible build produces the same output from the same inputs every time.
A build can be pure but not reproducible (e.g. a FOD with a mutable fetch
URL), or reproducible but not pure (e.g. a sandboxed build reading
`builtins.getEnv`). This constraint covers the reproducibility axis.

The core principle: every input to a derivation must be pinned to an
immutable, hash-verified artifact. Mutable references — channels, unhashed
fetches, uncommitted lock files — break reproducibility silently.

## Triggers

Load this skill when:

- Building packages with `nix build` or creating fixed-output derivations.
- Pinning dependencies (flake inputs, `fetchFromGitHub`, `fetchPypi`,
  `fetchTarball`, `builtins.fetchGit`).
- Editing `flake.nix` inputs or running `nix flake update`.
- Reviewing a PR for reproducibility / supply-chain correctness.
- Creating or auditing `flake.lock`.

## Rules

1. All flake inputs must be pinned in `flake.lock`. Every `inputs.*` entry
   in `flake.nix` must resolve to a locked revision with a recorded `narHash`
   / `lastModified`. No floating inputs (`github:owner/repo` without a `ref`
   that pins to a commit or tag).
2. All fetchers must have a hash / `outputHash`. `fetchFromGitHub` needs
   `hash`, `fetchPypi` needs `hash`, `fetchTarball` needs `sha256`,
   `builtins.fetchGit` needs a pinned `rev` + `ref`. No hashless fetches.
3. No mutable references. No `fetchTarball` without a hash, no
   `builtins.fetchGit` of mutable refs (e.g. `master` without a `rev`), no
   `builtins.fetchurl` without a `sha256`. Mutable refs produce different
   outputs over time.
4. No `--impure` anywhere. This duplicates the purity constraint
   (`constraint-nix-purity` rule 5) but from the reproducibility angle:
   `--impure` allows `builtins.getEnv`, `builtins.currentTime`, and other
   non-deterministic builtins to enter evaluation, breaking bit-for-bit
   reproducibility.
5. FOD outputs must be verified by hash. Every fixed-output derivation must
   declare `outputHashAlgo`, `outputHash`, and `outputHashMode`. A FOD
   without a hash is not a FOD — it is an unverified fetch. Use
   `lib.fakeSha256` as a placeholder during development, then prefetch the
   real hash via `just update-hashes`.
6. `flake.lock` must be committed to version control. An uncommitted
   `flake.lock` means every machine resolves inputs independently,
   producing different builds. `flake.lock` is the reproducibility anchor.
7. No channels (`<nixpkgs>`) in flake code. Channel references
   (`import <nixpkgs> {}`) resolve to whatever `NIX_PATH` points at on the
   host — a mutable, machine-specific, unversioned input. Flakes must use
   `inputs.nixpkgs` (pinned via `flake.lock`) instead.

## References

- Docs: `docs/nix/supply-chain-security.md` (supply-chain and pinning),
  `docs/nix/purity-and-sandboxing.md` (FOD semantics, sandboxing).
- Operational skill: `nix-usage` (anti-accumulation patterns, agent rules).
- Constraint skill: `constraint-nix-purity` (eval-time / build-time purity —
  the `--impure` rule overlaps; load both when reviewing a derivation).

## Out of scope

- Eval-time source filtering (`builtins.path` with `filter`,
  `cleanSourceWith`) — see `constraint-nix-purity`.
- Store hygiene, GC roots, `result*` symlink cleanup — see
  `constraint-nix-store-hygiene`.
- Docker image layering strategy (`streamLayeredImage` vs
  `buildLayeredImage`) — see `constraint-nix-store-hygiene` rule 2 and
  `nix-usage`.

## Violation examples

### `fetchTarball` without a hash

```nix
# FORBIDDEN — mutable fetch, no outputHash; not reproducible.
src = builtins.fetchTarball {
  url = "https://example.com/foo.tar.gz";
  # missing: sha256 = "..."
};
```

Correct — declare the hash:

```nix
src = fetchFromGitHub {
  owner = "example";
  repo = "foo";
  rev = "v1.2.3";
  hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
};
```

### `<nixpkgs>` channel reference in flake code

```nix
# FORBIDDEN — channel reference resolves to host NIX_PATH; mutable and
# machine-specific.
{ pkgs ? import <nixpkgs> {} }:
stdenv.mkDerivation { ... }
```

Correct — use the flake input (pinned via `flake.lock`):

```nix
{ nixpkgs, ... }:
let pkgs = nixpkgs.legacyPackages.x86_64-linux; in
pkgs.stdenv.mkDerivation { ... }
```

### Uncommitted `flake.lock`

```bash
# FORBIDDEN — every machine resolves inputs independently.
git status
# ?? flake.lock   ← must be committed
```

Correct — commit `flake.lock` alongside `flake.nix` changes:

```bash
git add flake.nix flake.lock
git commit -m "feat: add myPackage derivation"
```

### Floating flake input (no pinned ref)

```nix
# FORBIDDEN — floating input; resolves to latest commit on default branch.
inputs.some-lib.url = "github:owner/repo";
```

Correct — pin to a ref (tag or commit):

```nix
inputs.some-lib.url = "github:owner/repo/v2.1.0";
# flake.lock records the exact rev + narHash
```

### FOD without `outputHash`

```nix
# FORBIDDEN — a "FOD" without a hash is an unverified fetch.
stdenv.mkDerivation {
  pname = "my-fod";
  outputHashAlgo = "sha256";
  outputHashMode = "recursive";
  # missing: outputHash = "sha256-...";
  buildPhase = '' curl https://example.com/data.bin -o $out/data.bin '';  # also violates purity rule 4
}
```

Correct — declare the hash (use `lib.fakeSha256` during development, then
prefetch via `just update-hashes`):

```nix
outputHashAlgo = "sha256";
outputHashMode = "recursive";
outputHash = lib.fakeSha256;  # TODO: replace via just update-hashes
```

### `nix flake update` in CI

```bash
# FORBIDDEN — multi-GB rebuild; new nixpkgs revision pulls a new
# toolchain/closure. Update deliberately, never in CI.
nix flake update  # in a CI step
```

Correct — update `flake.lock` deliberately on a workstation, commit it, run
`just gc` after.

## How to check

```bash
nix flake check                  # validates flake structure + builds
nix flake metadata               # shows resolved inputs + revisions
nix flake lock --no-update       # fails if flake.lock is out of sync
git diff --exit-code flake.lock  # ensure lock is committed (CI gate)
just lint-nix                    # catches --impure, getFlake+toString
```

Inspect `flake.lock` manually:

- Every `nodes.*.locked` entry has `rev`, `narHash`, `lastModified`.
- No `nodes.*.original.ref` is a bare branch name without a corresponding
  pinned `rev` in `locked`.
- `nixpkgs` is pinned to a specific commit, not `github:NixOS/nixpkgs/nixpkgs-unstable`
  floating.

Manual review checklist:

- Every `inputs.*` in `flake.nix` has a corresponding `flake.lock` entry
  with `narHash`.
- Every fetcher (`fetchFromGitHub`, `fetchPypi`, `fetchTarball`,
  `builtins.fetchGit`) has a `hash` / `sha256` / `outputHash`.
- No `import <nixpkgs>` anywhere in flake code.
- `flake.lock` is committed and in sync with `flake.nix`.
- No `--impure` in any nix invocation (overlaps with purity constraint).
- FODs declare `outputHashAlgo` + `outputHash` + `outputHashMode`.
