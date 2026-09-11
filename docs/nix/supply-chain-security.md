---
type: Reference
resource: https://nix.dev/guides/best-practices.html
title: Supply Chain Security
description: Nix supply chain security for the ai-workbench flake — flake lock auditing, reproducible builds, CVE scanning, dependency review, FOD integrity, and the trust model.
tags: [nix, security, supply-chain, reproducibility, CVE, audit]
timestamp: 2026-07-24T02:00:00Z
---

# Supply Chain Security

## Purpose

This document is the canonical reference for supply chain security in the
workestrate flake. It covers flake lock file auditing, reproducible builds,
CVE scanning, dependency review, fixed-output derivation (FOD) integrity,
source verification, the Nix trust model, and the project's concrete supply
 chain (eleven flake inputs and their lock entries). Every contributor who adds,
updates, or reviews a flake input, a `fetchFromGitHub`/`fetchurl` call, or a
FOD `outputHash` should read this first.

Nix's content-addressed store model makes supply chain attacks harder than in
traditional package managers, but it does not make them impossible. The
defenses here are layered: pinning (lock file), hashing (FOD `outputHash`),
sandboxing (build-time isolation), code review (nixpkgs PR review), and
reproducibility (content-addressed derivations). No single layer is
sufficient; the combination is.

## Sources used

- `.crawl/29-best-practices.md` — https://nix.dev/guides/best-practices.html
  (PRIMARY — lookup paths, reproducible nixpkgs config, reproducible source
  paths, `builtins.path` with `name`)
- `.crawl/09-towards-reproducibility-pinning-nixpkgs.md` —
  https://nix.dev/tutorials/first-steps/towards-reproducibility-pinning-nixpkgs.html
  (pinning via `fetchTarball` + commit hash, `status.nixos.org`)
- `.crawl/43-pinning-nixpkgs.md` — https://nix.dev/reference/pinning-nixpkgs.html
  (pinning mechanisms: `$NIX_PATH`, `-I`, fetchers, URL values)
- `.crawl/35-dependency-management.md` —
  https://nix.dev/guides/recipes/dependency-management.html (`npins` source
  pinning, overriding sources)
- `.crawl/46-flakes.md` — https://nix.dev/concepts/flakes.html (`flake.lock`,
  inputs, `follows`, pure mode, reproducibility caveats)
- `.crawl/60-nix-store-path.md` — https://nix.dev/manual/nix/2.34/store/store-path
  (store path digests, referential integrity)
- `.crawl/62-nix-command-build.md` —
  https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build (`--rebuild`,
  `--repair`, `--update-input`)
- `../nix-purity.md` — project purity rules (FOD `outputHash`, no unhashed
  fetches, `just update-hashes`)

## Related Nix guidance

- [Flake Anatomy](flake-anatomy.md) — `flake.nix` structure, inputs, outputs,
  `flake.lock`, the flake registry, and the flake command set.
- [Purity and Sandboxing](purity-and-sandboxing.md) — eval-time and build-time
  purity, the sandbox, FOD discipline.
- [Store Hygiene and GC](store-hygiene-and-gc.md) — GC roots, store growth,
  `nix store verify`, `nix path-info`.
- [Nix Store and Paths](nix-store-and-paths.md) — store path digests,
  referential integrity, content addressing.
- [Caching and Binary Caches](caching-and-binary-caches.md) — substituters,
  `trusted-substituters`, `require-sigs`.

## Core guidance

### The defense-in-depth model

Nix supply chain security rests on five layers. Each layer addresses a
different attack vector; together they form a chain whose strength is the
minimum of any link.

| Layer | Mechanism | Defends against |
|-------|-----------|-----------------|
| Pinning | `flake.lock` commits + `narHash` | Silent dependency drift, mutable refs |
| Hashing | FOD `outputHash` / `npmDepsHash` / `cargoHash` | Tampered fetch output, MITM |
| Sandboxing | build sandbox, no network in `buildPhase` | Hidden network exfiltration |
| Code review | nixpkgs PR review, ofBorg, Hydra | Malicious PRs, typosquatting |
| Reproducibility | content-addressed derivations, `--rebuild` | Undetected build tampering |

> **From the Nix manual:** "Think of a store path as an opaque, unique
> identifier: The only way to obtain store path is by adding or building store
> objects. A store path will always reference exactly one store object."
> [60-nix-store-path.md] A store path's digest is derived from the inputs that
> produced it, so any change to a dependency changes every downstream path.
> This is the structural property that makes pinning and hashing effective.

### Why pinning matters

Without pinning, the same Nix expression can produce different results across
machines and over time. The nix.dev best-practices guide is explicit:

> "This means the value of a lookup path depends on external system state.
> When using lookup paths, the same Nix expression can produce different
> results." [29-best-practices.md]

> "Declare dependencies explicitly using the techniques shown in
> pinning-nixpkgs. Do not use lookup paths, except in minimal examples."
> [29-best-practices.md]

The reproducibility tutorial reinforces this:

> "However, the resulting Nix expression is not fully reproducible."
> [09-towards-reproducibility-pinning-nixpkgs.md]

> "To create fully reproducible Nix expressions, we can pin an exact version
> of Nixpkgs." [09-towards-reproducibility-pinning-nixpkgs.md]

Flakes make pinning automatic: the lock file records the exact revision and
content hash of every input.

> "Nix creates a `flake.lock` to pin dependencies once you run a `nix`
> command." [46-flakes.md]

> "If these dependencies have `inputs` of their own, Nix will check _their_
> lock files to find the versions to use. Using the same versions helps make
> sure programs work as intended, but you can override these." [46-flakes.md]

## Flake lock file auditing

### `flake.lock` structure

The lock file is a JSON document with a `nodes` object (one entry per input,
keyed by input name), a `root` key naming the root node, and a `version`
integer. Each node has:

- `original` — the ref as declared in `flake.nix` (owner, repo, ref, type).
- `locked` — the resolved, pinned state: `owner`, `repo`, `rev` (commit SHA),
  `narHash` (content hash of the fetched tree), `lastModified` (Unix epoch),
  `type`.
- `inputs` (for flake inputs only) — maps the input's own input names to node
  keys or `[ "nodeA" "nodeB" ]` paths (for `follows`).
- `flake` (boolean) — whether the input is a flake (`false` for plain source
  trees).

The project's `flake.lock` (version 7) pins six top-level inputs plus one
transitive input (`rust-analyzer-src`, pulled by `fenix`). See
[The project's supply chain](#the-projects-supply-chain) below for the full
table.

### Inspecting inputs: `nix flake metadata`

```bash
nix flake metadata
```

Lists every input with its resolved revision, last-modified date, and
`narHash`. Use it to confirm an input points at the expected commit before
merging a lock-file update:

```bash
nix flake metadata --json | jq '.locks.nodes.nixpkgs.locked.rev'
```

### Updating inputs

To update a single input (the modern command):

```bash
nix flake update nixpkgs      # update only nixpkgs
nix flake update              # update all inputs
```

The older `--update-input` flag is deprecated:

> "`--update-input` _input-path_ — Update a specific flake input (ignoring its
> previous entry in the lock file). DEPRECATED: Use `nix flake update`
> instead." [62-nix-command-build.md]

After any update, review the diff (`git diff flake.lock`) and confirm each
`rev` and `narHash` changed intentionally. A lock-file change with no
corresponding review is a supply chain event.

### Pinning to a release branch

The project pins `nixpkgs` to a moving branch (`nixos-unstable`). For
production stability, pin to a tested release channel:

> "Picking the commit can be done via status.nixos.org, which lists all the
> releases and the latest commit that has passed all tests."
> [09-towards-reproducibility-pinning-nixpkgs.md]

> "[status.nixos.org] provides: Latest tested commits for each release - use
> when pinning to specific commits; List of active release channels - use when
> tracking latest channel versions" [43-pinning-nixpkgs.md]

To pin to the stable NixOS 26.05 release:

```nix
nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
```

The `follows` mechanism keeps transitive inputs aligned with the root
`nixpkgs`, avoiding duplicate or stale copies:

> "This way, Home Manager's inputs reuse your chosen `nixpkgs`."
> [46-flakes.md]

The project uses `follows` for `fenix`:

```nix
fenix = {
  url = "github:nix-community/fenix";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

## Reproducible builds

### Pure evaluation mode

Flakes default to pure (hermetic) evaluation, which is the foundation of
reproducibility:

> "Flakes default to pure mode, isolating builds from the host environment.
> This is also called hermetic evaluation, and prevents evaluating
> (non-network) impure functions." [46-flakes.md]

But pure mode is necessary, not sufficient:

> "Even in pure mode, reproducibility is not actually guaranteed."
> [46-flakes.md]

Non-determinism can still arise from: filesystem ordering, timestamps embedded
in build outputs, uninitialized memory, or sources that change between
evaluations. The project's purity rules (see [Purity and
Sandboxing](purity-and-sandboxing.md)) address the eval-time axis; the rules
below address the build-time and verification axes.

### Content-addressed derivations

A fixed-output derivation (FOD) is content-addressed: its output hash is
declared up front, and the build is only accepted if the produced output
matches. This is what permits network during the fetch phase (the fetcher runs
under a FOD) while keeping the main build sandboxed and network-free. The
project enforces this strictly:

> "All dependency fetching goes through fixed-output derivations with an
> `outputHash`: `buildNpmPackage` (uses `npmDepsHash`), FOD pip
> (`fetchPypi`/`fetchFromGitHub` + `outputHash`), FOD bun. No `fetchTarball`
> without a hash, no `builtins.fetchGit` of mutable refs." [nix-purity.md]

### Verifying store path hashes

```bash
nix hash path /nix/store/<digest>-name
```

Computes the hash of a store path's contents. Compare it against the
`narHash` in `flake.lock` to confirm an input was not tampered with after
fetching.

### Inspecting closure contents

```bash
nix path-info --json .#workestrate
```

Returns JSON describing every store path in the closure of an installable,
including references, sizes, and derivation paths. The project's
`just store-audit` recipe uses this to detect oversized source copies:

```bash
nix path-info --all --json | python3 scripts/store-audit.py --warn-if-source-over 50
```

### Rebuilding and repairing

```bash
nix build .#workestrate --rebuild
```

> "`--rebuild` — Rebuild an already built package and compare the result to the
> existing store paths." [62-nix-command-build.md]

If the rebuild produces a different store path, the build is not
reproducible — investigate the source of non-determinism.

```bash
nix build .#workestrate --repair
```

> "`--repair` — During evaluation, rewrite missing or corrupted files in the
> Nix store. During building, rebuild missing or corrupted store paths."
> [62-nix-command-build.md]

`--repair` detects and fixes store corruption (bit rot, disk errors, or
deliberate tampering with store contents).

## Source verification and FOD integrity

### `fetchFromGitHub` with `hash`

Every GitHub source fetch must declare a `hash` (SRI-encoded sha256). The
project's FOD template (see [Purity and Sandboxing](purity-and-sandboxing.md))
shows the pattern:

```nix
src = fetchFromGitHub {
  owner = "example";
  repo = "my-fod";
  rev = "v0.1.0";
  hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
};
```

If the fetched content does not match `hash`, the build fails. This is a
hard cryptographic check — a MITM or compromised mirror cannot substitute
different content without also knowing how to produce a matching hash
collision.

### `fetchurl` with `sha256`

For arbitrary URLs (tarballs, release assets):

```nix
src = fetchurl {
  url = "https://example.com/foo-1.0.tar.gz";
  sha256 = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
};
```

### FOD `outputHash` verification

A full FOD declares `outputHashAlgo`, `outputHashMode`, and `outputHash`:

```nix
outputHashAlgo = "sha256";
outputHashMode = "recursive";
outputHash = "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";
```

The `outputHash` is verified after the build runs. If the produced output
does not match, the build fails. This is the integrity boundary that makes
network-bearing fetch phases safe: the fetcher can download anything, but the
result is only accepted if it hashes to the declared value.

### Reproducible source paths

The directory name of a source path leaks into the store path, breaking
reproducibility:

> "If someone builds the project in a directory with a different name, they
> will get a different store path for `src` and everything that depends on it.
> This can be the cause of needless rebuilds." [29-best-practices.md]

> "Use `builtins.path` with the `name` attribute set to something fixed."
> [29-best-practices.md]

```nix
src = builtins.path { path = ./.; name = "myproject"; };
```

The project enforces this via `scripts/check-nix-paths.sh` (the `lint-nix`
guard in `just verify`), which rejects unfiltered `builtins.path` and
`cleanSourceWith` calls that lack a `filter`.

### `nix store verify`

```bash
nix store verify --recursive .#workestrate
```

Verifies that store paths are signed by trusted keys (when `require-sigs` is
enabled) and that their contents match their registered hashes. This is the
runtime integrity check against store tampering.

## CVE scanning and dependency review

### `nix flake show` and `nix flake archive`

```bash
nix flake show          # list all outputs the flake exposes
nix flake archive --json # compute the full closure (all inputs + outputs)
```

`nix flake archive` is useful for air-gapped review: it copies the entire
flake closure to a store, letting you inspect every transitive input offline.

### CVE scanning tools

Nix does not ship a built-in CVE scanner, but several community tools scan
the nixpkgs closure for known vulnerabilities:

- **`vulnix`** — scans a `nix-store` closure (or a derivation) against the NVD
  CVE database. Reports CVEs by package and version.
- **`nix-audit`** — a lighter alternative that checks a flake's nixpkgs
  revision against known advisories.
- **`nixpkgs` vulnerability database** — nixpkgs tracks security advisories
  in `pkgs/by-name` and via the NixOS security team; the
  [NixOS security page](https://nixos.org/security/) publishes advisories.

Typical workflow:

```bash
# Build the closure, then scan it
nix build .#workestrate --no-link --print-out-paths
vulnix --store-path /nix/store/<digest>-workestrate
```

### nixpkgs review process

nixpkgs has a multi-stage review pipeline that is itself a supply chain
control:

- **PR review** — every nixpkgs PR is reviewed by maintainers before merge.
- **ofBorg** — the community CI bot that builds and tests PRs across
  architectures, running `nixpkgs-review` and mass-rebuild checks.
- **Hydra** — the NixOS continuous-build system that tests the release
  branches; `status.nixos.org` reports which commits have passed all tests.

> "[status.nixos.org] provides: Latest tested commits for each release - use
> when pinning to specific commits" [43-pinning-nixpkgs.md]

Pinning to a Hydra-tested commit (rather than `master` HEAD) means the
nixpkgs revision you depend on has passed the full test suite — a meaningful
supply chain signal.

## Trust model

### `trusted-users`

`trusted-users` in `nix.conf` (or `nix.settings.trustedUsers` on NixOS)
lists users allowed to add binary caches, modify the store, and bypass
certain restrictions. A user not in this list cannot substitute untrusted
paths. Keep this list minimal.

### `trusted-substituters`

`trusted-substituters` lists binary cache URLs whose paths Nix will fetch
without prompting. Only add caches you control or fully trust — a malicious
substituter can serve tampered store paths if signature verification is
disabled.

### `require-sigs`

When `require-sigs = true` (the default), Nix only accepts store paths from
substituters that are signed by a trusted key. This is the cryptographic
backbone of the binary-cache trust model. Disabling it (`require-sigs = false`)
removes the signature check and should never be done in production.

See [Caching and Binary Caches](caching-and-binary-caches.md) for the full
substituter and signature configuration reference.

## Supply chain attack vectors and mitigations

| Attack vector | How it works | Mitigation |
|---------------|--------------|------------|
| **Typosquatting** | A malicious package with a name close to a real one is published to a registry | Pin via `flake.lock` rev; review `fetchFromGitHub` owner/repo; FOD `hash` catches substituted content |
| **Dependency confusion** | A public package shadows a private/internal name, pulling malicious code | Pin internal sources explicitly via flake inputs; never fall back to public registries for private names |
| **Malicious PR** | A compromised or malicious contributor submits a PR to nixpkgs or an upstream | ofBorg/Hydra CI gates; maintainer review; pin to tested commits; `narHash` in lock file |
| **Compromised mirror / MITM** | A fetch URL is redirected to serve tampered content | FOD `outputHash` / `hash` fails on any substitution; `require-sigs` for binary caches |
| **Store tampering** | An attacker with local access modifies `/nix/store` contents | `nix store verify --recursive`; `--repair`; filesystem permissions |
| **Mutable ref drift** | An input tracks a moving branch (`main`, `nixos-unstable`) and silently advances | `flake.lock` pins the exact rev; review lock-file diffs on every update |
| **Unhashed fetch** | `fetchTarball` without a hash, or `builtins.fetchGit` of a mutable ref | Project rule: all fetches go through FODs with `outputHash`; `just lint-nix` enforces |

### The mitigation hierarchy

1. **Pin** — `flake.lock` records exact rev + `narHash` for every input.
2. **Hash** — every fetch declares an `outputHash`/`hash`/`npmDepsHash`.
3. **Sandbox** — no network in `buildPhase`; all data arrives via FODs.
4. **Review** — review lock-file diffs; pin to Hydra-tested commits.
5. **Verify** — `nix store verify`, `--rebuild`, `--repair`, `vulnix`.

## The project's supply chain

The workestrate flake declares eleven top-level inputs: ten GitHub sources
pinned by `flake.lock`, plus `devenv-root` (`file:///dev/null`, non-GitHub).
`fenix`, `tooling`, `devenv`, `treefmt-nix`, `git-hooks`, and `nix2container`
reuse the root `nixpkgs` via `inputs.nixpkgs.follows` (`devenv` additionally
follows `git-hooks`/`flake-parts`); `flake-parts` follows `nixpkgs-lib`.

| Input | URL | Flake? | Pinned rev (from `flake.lock`) | narHash (prefix) |
|-------|-----|--------|-------------------------------|------------------|
| `nixpkgs` | `github:NixOS/nixpkgs` | yes | `a799d3e3...` | `sha256-3av0pIjl...` |
| `fenix` | `github:nix-community/fenix` | yes (`follows` nixpkgs) | `fa09e647...` | `sha256-qsmQMPL+...` |
| `microsandbox-fork` | `github:rybskiworks/microsandbox` | no (`flake = false`) | `78fb3ed1...` | `sha256-Mptr2Jwk...` |
| `tooling` | `github:rybskiworks/nix-tooling` | yes (`follows` nixpkgs) | `2a579617...` | `sha256-guDFHPfJ...` |
| `flake-parts` | `github:hercules-ci/flake-parts` | yes | `9d0d8717...` | `sha256-onL0VLf9...` |
| `devenv` | `github:cachix/devenv` | yes | `97135e80...` | `sha256-zxEb+L6m...` |
| `treefmt-nix` | `github:numtide/treefmt-nix` | yes | `27b3b12a8...` | `sha256-WSFCsDSE...` |
| `git-hooks` | `github:cachix/git-hooks.nix` | yes | `27555e26...` | `sha256-nt+lUqYV...` |
| `nix2container` | `github:nlewo/nix2container` | yes | `76be9608...` | `sha256-2lguQpLP...` |
| `mk-shell-bin` | `github:rrbutani/nix-mk-shell-bin` | yes | `ff5d8bd4...` | `sha256-/uEkr1UkJ...` |
| `devenv-root` | `file:///dev/null` | no (`flake = false`) | — | `sha256-d6xi4mKd...` |

Plus one transitive input: `rust-analyzer-src`
(`github:rust-lang/rust-analyzer` nightly, `1174734d...`), pulled by `fenix`.

### Trust posture

- **`nixpkgs`** — the NixOS/nixpkgs repository, pinned to rev `a799d3e3`.
  Trust derives from the ofBorg/Hydra review pipeline. For production, pin to
  a tested stable release (`nixos-26.05`).
- **`microsandbox-fork`, `tooling`** — sources under
  `github:rybskiworks` (org moved from `georgrybski` 2026-08-31).
  `microsandbox-fork` is a non-flake source tree (`flake = false`) consumed
  via `callPackage`/vendoring; `tooling` is the shared nix-tooling flake.
  Trust derives from the fork owner's review of upstream; the `narHash` in
  `flake.lock` pins the exact tree.
- **`fenix`** — `github:nix-community/fenix`, a flake providing the pinned
  Rust toolchain. `inputs.nixpkgs.follows = "nixpkgs"` ensures it reuses the
  root nixpkgs, avoiding a second nixpkgs revision in the closure.
- **`flake-parts`, `devenv`, `treefmt-nix`, `git-hooks`, `nix2container`,
  `mk-shell-bin`** — community flakes (dev shell, formatting, git hooks,
  image building), each pinned by exact rev + `narHash` in `flake.lock`.

### FOD hash verification workflow: `just update-hashes`

The project's FOD hashes (`npmDepsHash`, `bunDeps.outputHash`,
`pipDeps.outputHash`) are verified via the `just update-hashes` recipe. It
does not auto-rewrite files; it surfaces the correct `got:` hash for manual
inlining:

```bash
just update-hashes
```

This runs:

1. `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json`
   — prefetches the tempest `npmDepsHash`.
2. `nix build .#opencode-built --no-link` — fails with `lib.fakeHash` and
   prints the `got:` sha256 for `bunDeps.outputHash`.
3. `nix build .#odysseus-built --no-link` — same for `pipDeps.outputHash`.

The operator manually inlines each `got:` value into the matching
`nix/packages/*.nix` file, then re-runs
`nix build .#tempest .#opencode-built .#odysseus-built` to confirm.

> **Confirmed (from `nix-purity.md`):** `just update-hashes` runs
> `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json`
> (tempest `npmDepsHash`) and `nix build .#opencode-built` /
> `.#odysseus-built --no-link` to surface the `got:` sha256 for opencode
> `bunDeps.outputHash` and odysseus `pipDeps.outputHash`. It prints the
> hashes; the operator manually inlines each `got:` value into the matching
> `nix/packages/*.nix` file. It does not auto-rewrite the files.

The `fakeHash` / `lib.fakeSha256` convention marks a hash as
not-yet-prefetched:

```nix
hash = lib.fakeSha256; # TODO: replace via just update-hashes
```

## Validation hooks

The project enforces supply chain controls through these gates (all wired
into `just verify` unless noted):

| Gate | Command | What it checks |
|------|---------|----------------|
| `lint-nix` | `just lint-nix` (`scripts/check-nix-paths.sh`) | Rejects `--impure`, unfiltered `builtins.path`/`cleanSourceWith` without `filter`, parent-dir path literals |
| `store-audit` | `just store-audit` | Reports top-20 store paths by closure size; fails if any `ai-workbench`-source path exceeds 50 MB (impure source copy) |
| `gc` | `just gc` | `nix-collect-garbage --delete-old` + `nix store optimise` (reclaims unreachable + dedupes) |
| `update-hashes` | `just update-hashes` | Surfaces correct FOD hashes for manual inlining (not in `verify`) |
| Lock-file review | `git diff flake.lock` | Manual: confirm every `rev`/`narHash` change is intentional |

### HOST-GATE note

This container has no `nix` on `PATH`. Any `nix build`, `nix flake
metadata`, `nix store verify`, or `vulnix` claim here is based on documented
Nix semantics and the project's `justfile`/`nix/packages/*.nix`, not runtime
verification. Run these commands on a nix-capable host.

## Review checklist

Before merging a change that touches the supply chain:

- [ ] `flake.lock` diff reviewed: every changed `rev` and `narHash` is
      intentional and traces to a reviewed commit.
- [ ] No new `fetchTarball` without a hash, no `builtins.fetchGit` of a
      mutable ref, no `fetchurl`/`fetchFromGitHub` without `hash`/`sha256`.
- [ ] Every new FOD declares `outputHashAlgo`, `outputHashMode`, and a real
      `outputHash` (not `lib.fakeSha256`).
- [ ] `just update-hashes` run and the `got:` values inlined for any new or
      changed FOD.
- [ ] `just lint-nix` passes (no `--impure`, no unfiltered source paths).
- [ ] `just store-audit` passes (no oversized `*-source` paths).
- [ ] New `fetchFromGitHub` owner/repo reviewed for typosquatting.
- [ ] If a new binary cache was added, it is in `trusted-substituters` and
      `require-sigs` remains `true`.
- [ ] `nixpkgs` pin reviewed: prefer a Hydra-tested commit or stable release
      for production paths.

## Related docs

- [Flake Anatomy](flake-anatomy.md)
- [Purity and Sandboxing](purity-and-sandboxing.md)
- [Store Hygiene and GC](store-hygiene-and-gc.md)
- [Nix Store and Paths](nix-store-and-paths.md)
- [Caching and Binary Caches](caching-and-binary-caches.md)
- [Nix Commands](nix-commands.md)
- [Packaging Recipes](packaging-recipes.md)

## Related skills

- [`.agents/skills/nix-usage`](../../.agents/skills/nix-usage/SKILL.md) —
  flake, dev shell, and `msb` runtime operational reference.

## Citations

[1] [Best practices](https://nix.dev/guides/best-practices.html) — nix.dev
[2] [Towards reproducibility: pinning Nixpkgs](https://nix.dev/tutorials/first-steps/towards-reproducibility-pinning-nixpkgs.html) — nix.dev
[3] [Pinning Nixpkgs](https://nix.dev/reference/pinning-nixpkgs.html) — nix.dev
[4] [Automatically managing remote sources with npins](https://nix.dev/guides/recipes/dependency-management.html) — nix.dev
[5] [Flakes](https://nix.dev/concepts/flakes.html) — nix.dev
[6] [Store Path](https://nix.dev/manual/nix/2.34/store/store-path) — Nix 2.34 manual
[7] [nix build](https://nix.dev/manual/nix/2.34/command-ref/new-cli/nix3-build) — Nix 2.34 manual
[8] [status.nixos.org](https://status.nixos.org/) — NixOS release/channel test status
[9] [NixOS security advisories](https://nixos.org/security/) — NixOS security team
[10] [Purity and Sandboxing (project)](purity-and-sandboxing.md) — ai-workbench docs
