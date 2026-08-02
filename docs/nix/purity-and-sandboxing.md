---
type: Reference
resource: https://nix.dev/guides/best-practices.html
title: Purity and Sandboxing
description: Nix eval-time and build-time purity, sandboxing, fixed-output derivations, impurity patterns, store-growth model, enforcement guards, and nix.conf settings for keeping derivations pure.
tags: [nix, purity, sandbox, FOD, impurity, enforcement]
timestamp: 2026-07-24T01:17:00Z
---

# Purity and Sandboxing

## Purpose

This document is the canonical corpus reference for Nix purity (eval-time and
build-time), sandboxing, fixed-output derivations (FODs), impurity patterns,
the store-growth model, enforcement guards, and `nix.conf` settings for
keeping derivations pure.

It **elevates** `/docs/nix-purity.md` — the repo-root quick reference — into
the corpus structure under `/docs/nix/` with expanded sandboxing, `nix.conf`,
and enforcement coverage. The repo-root `/docs/nix-purity.md` is preserved
as-is and remains the short-form authoritative ruleset; this document is a
strict superset of it.

Intended audience: agents and contributors adding or editing Nix derivations
under `nix/` or `flake.nix`. Every contributor adding or editing a nix
derivation is expected to read this (or the repo-root quick reference) first.

## Sources used

- <https://nix.dev/guides/best-practices.html> — Nix best practices
- <https://nix.dev/manual/nix/2.34/language/derivations> — derivation purity
  (path-copy semantics)
- Local: `/docs/nix-purity.md` — workestrator purity rules (PRIMARY SOURCE
  ELEVATED)
- Local: `/scripts/check-nix-paths.sh` — purity enforcement guard
- Local: `/.agents/skills/nix-usage/SKILL.md` — nix-usage skill
  (anti-accumulation patterns, agent rules)
- Local: `/docs/nix/nix-store-and-paths.md` — store paths, closures, GC,
  store-growth model, 29 GB incident timeline
- Local: `/docs/nix/.crawl/59-nix-language-derivations.md` — derivation
  purity (path-copy semantics)
- Local: `/flake.nix` — real-world FOD and anti-accumulation patterns

### Crawl ledger

SEED:

- <https://nix.dev/guides/best-practices.html>
- <https://nix.dev/manual/nix/2.34/language/derivations>

DISCOVERED & VISITED:

- `/docs/nix-purity.md` — the primary source elevated by this document
- `/scripts/check-nix-paths.sh` — the enforcement guard (6 checks)
- `/.agents/skills/nix-usage/SKILL.md` — anti-accumulation patterns table,
  verbatim agent rules
- `/docs/nix/nix-store-and-paths.md` — store-growth model and 29 GB incident
  timeline (cross-referenced, not duplicated)
- `/docs/nix/.crawl/59-nix-language-derivations.md` — path-copy semantics
- `/flake.nix` — real-world FOD usage, `--no-link --print-out-paths`,
  `cleanSourceWith`

SKIPPED (out of scope / summarized):

- `nix.conf` man page full text — summarized at the level needed for purity
  decisions; full option catalogue is out of scope
- Nix RFCs — out of scope (this document references stable, shipped behavior)

## Core guidance

### The two purity axes

Purity in Nix has two independent axes. **Both must hold** for a derivation
to be considered pure; either one alone is insufficient. [purity]

#### (a) Eval-time purity

Eval-time purity is what `nix` sees when it computes the derivation graph —
before any build runs. The evaluation walks the flake's source closure (the
set of paths the flake references) and turns it into store paths. If that
closure is impure — if it includes untracked files, large build outputs, or
directories whose contents change on every edit — then every evaluation
produces a new, different source closure, and Nix copies the whole thing
into the store each time. [purity]

The rules that govern this axis are about **source filtering** and **what
paths the flake is allowed to reference**: no `toString ./.`, no
`builtins.getFlake`, no impure `builtins` like `builtins.getEnv` outside
controlled wrappers, and no referencing untracked or large directories
(notably `target/`, `agents/*/build/`, `node_modules/`). [purity]

Mechanisms that enforce eval-time purity:

- **`builtins.path` with an explicit `filter`** — copies only the intended
  files into the store.
- **`cleanSourceWith` with a `filter` predicate** — the lib helper that
  excludes `target/`, `result*`, `node_modules/`, `agents/*/build/`,
  `.workestrate/`, and any large untracked dir.
- **No `toString ./.`** — raw working-tree copy is forbidden.
- **No `builtins.getFlake`** — impure self-referential flake fetching is
  forbidden; the flake is entered via its own inputs only.
- **No impure `builtins`** like `builtins.getEnv` outside controlled wrappers.

#### (b) Build-time purity

Build-time purity is what happens inside the build sandbox once evaluation
has produced a derivation. The sandbox has no network in
`buildPhase`/`installPhase`; all dependencies must arrive via fixed-output
derivations (FODs) with declared `outputHash`es; and `--impure` is forbidden
everywhere. [purity]

A sandboxed build is necessary but not sufficient: **eval impurity balloons
the store even when the build itself is sandboxed**, because the impure
source closure is copied on every evaluation regardless of whether the build
runs. [purity]

### Path-copy semantics (why unfiltered paths balloon the store)

From the Nix manual, the environment-variable translation rule for path
values [manual]:

> A _path_ (e.g. `../foo/sources.tar`) causes the referenced file to be
> copied to the store; its location in the store is put in the environment
> variable. The idea is that all sources should reside in the Nix store,
> since all inputs to a derivation should reside in the Nix store.

This is **why** unfiltered paths balloon the store: any path literal
referenced by a derivation is copied wholesale into the store. A bare
`src = ./.` copies the entire repo working tree (including gitignored
`target/` at 16–28 G, `agents/*/build/`, `.workestrate/`) into the store on
every evaluation. Because the dirty source closure changes hash on every
edit, Nix produces a new content-addressed store path each time. [manual]
[purity]

### Fixed-output derivations (FODs)

A fixed-output derivation (FOD) is a derivation whose output is verified by
a declared hash rather than by the build steps. The FOD attributes are:

- `outputHashAlgo` — the hash algorithm (e.g. `"sha256"`).
- `outputHash` — the expected hash of the output.
- `outputHashMode = "recursive"` — hash the output recursively (directories).

FODs **permit network in the fetch phase** because the output is verified by
hash: the fetcher runs under a FOD, not the main build, so a network fetch
inside a FOD does not violate build-time purity. This is how all dependency
fetching works in a pure flake: `fetchFromGitHub`, `fetchPypi`,
`buildNpmPackage` (uses `npmDepsHash`), FOD pip, FOD bun. [purity]

No `fetchTarball` without a hash, no `builtins.fetchGit` of mutable refs.
[purity]

For FOD store path details (how the output hash determines the store path),
see `/docs/nix/nix-store-and-paths.md`. [store]

### Sandbox profiles

Sandboxing is enforced at build time. The relevant advanced-attr / `nix.conf`
concepts:

- **`sandboxBuild`** — build-time sandbox enforcement. When enabled, each
  build runs in an isolated filesystem namespace with no network access in
  `buildPhase`/`installPhase`.
- **`__noChroot`** — a per-derivation escape hatch that disables the sandbox
  for that single derivation. **Dangerous — avoid.** It is only appropriate
  for derivations that genuinely cannot run sandboxed (e.g. some FODs that
  need `/etc` or other host resources). Use sparingly and document why.
- **`sandboxFallback`** — what happens when the sandbox is unavailable on
  the platform (e.g. macOS without the necessary kernel support). When
  fallback is enabled, Nix falls back to an unsandboxed build with a warning;
  when disabled, the build fails instead of falling back.

These are advanced-attr / `nix.conf` concepts. The project's `nix.conf`
settings are documented in the `nix.conf` settings subsection below.

### `nix.conf` settings

The following `nix.conf` settings affect purity and store growth:

- **`sandbox = true`** — enforce the build sandbox. Every build runs in an
  isolated namespace with no network in `buildPhase`/`installPhase`. This is
  the build-time purity enforcement knob. Set to `true` where the platform
  supports it.
- **`auto-optimise-store = true`** — deduplicate identical content across
  store paths by hard-linking. This is **advisory**: it deduplicates
  identical content but is **not** a substitute for the purity rules. It
  reduces vector (c) (content-addressed copies) but does nothing for vectors
  (a) (per-edit churn) and (b) (GC roots). [purity]
- **`max-jobs`** — the number of build jobs Nix will run in parallel.
  Performance tuning; no purity impact.
- **`cores`** — the number of cores each build job may use. Performance
  tuning; no purity impact.

### The 29 GB-per-eval incident

> An impure eval path previously caused Nix to copy `target/` and
> `agents/*/build/` into the store on every evaluation. Because those
> directories were large and changed on every edit, each evaluation
> produced a new content-addressed store path containing the full dirty
> source closure. The store grew at roughly 1 GB/min and reached
> approximately 29 GB before the impurity was caught and the source filter
> was fixed. [purity]

The mechanism is straightforward: a dirty source closure changes hash on
every edit, so Nix produces a new store path each time, and those paths are
kept alive by live GC roots (dev-shell profiles, `result*` symlinks, `nix
build` outputs). Garbage collection could not reclaim them because the roots
were still live. This incident is the motivating case for every rule below
and for the `just lint-nix` guard. [purity]

For the expanded timeline table (7 GC cycles, cumulative ~76 GiB freed) and
the ranked root causes, see `/docs/nix/nix-store-and-paths.md`. [store]

### The store-growth model

The store grows along three vectors. Understanding all three is necessary to
keep the store bounded. [purity]

#### (a) Per-edit churn

> Each evaluation with an impure source filter produces a new store path.
> With a dirty `target/` or `agents/*/build/` in the closure, every `nix
> develop` or `nix build` copies the whole closure again. This is the
> dominant growth vector during active development — it is what produced
> the 29 GB incident. [purity]

#### (b) GC roots

> Live `result*` symlinks, dev-shell profiles, and `nix build` outputs keep
> paths alive. A path that is unreachable is collectable; a path held by a
> live root is not. Stale `result*` symlinks pointing at long-gone paths,
> and accumulated dev-shell profiles, are the second growth vector.
> [purity]

#### (c) Content-addressed copies

> The same content under different store paths is duplicated until `nix
> store optimise` (or `auto-optimise-store`) deduplicates it. This is the
> third vector: even when content is identical, distinct paths consume
> space until deduplication runs. [purity]

For the expanded version of the store-growth model (including the hygiene
cadence and `auto-optimise-store` advisory note), see
`/docs/nix/nix-store-and-paths.md`. [store]

### Sanctioned impure zone: devshell in-tree agent builds

The dev shell (`nix develop`) building agents in-tree
(`agents/<name>/build`, `just dev-build-pi`) is the **one** sanctioned
impure zone. It is acceptable because: [purity]

- **Tree-side, not store-side.** The build output lives in the working tree
  (`agents/<name>/build`), not in `/nix/store`. It is never a store path.
  [purity]
- **Store-inert.** Because it is not a store path, it does not participate
  in GC root accounting and does not produce content-addressed copies.
  [purity]
- **Does not feed back into flake evaluation**, as long as the source
  filters exclude `agents/*/build/`. The dev shell's in-tree build is a
  developer convenience loop (fast, hashless, editable), not a purity claim.
  [purity]

This is a developer convenience loop, not a purity claim. The moment such an
output is referenced by a nix derivation (e.g. a derivation that does `src =
agents/pi/build`), the purity rules apply in full: that reference is
forbidden, and the build output must instead be produced by a proper
FOD-backed derivation (`.#pi-bun`, etc.) or relocated out of the
flake-visible source tree into the managed sources store. [purity]

The `CARGO_TARGET_DIR` relocation detail (canonical path
`${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target`) is covered
in rule 8 below and in the Practical rules section. [purity]

### HOST-GATE convention

This container has no nix; any `nix build` claim in this document is based
on documented Nix semantics, not runtime verification. Derivations that can
only be verified on a host with nix carry a `# HOST-GATE:` comment.
[purity]

## Practical rules

> **Operational quick-reference for agents:** see
> [`.agents/skills/nix-usage`](../../.agents/skills/nix-usage/SKILL.md)
> for the anti-accumulation patterns table and verbatim agent rules.
> [purity]

1. Never reference the repo root as a path. Use
   `builtins.path { name = ...; path = ./subdir; filter = ...; }` with
   an explicit `name` and `filter` so only intended files enter the
   store. Bare `./.` or `toString ./.` is forbidden. [purity]

2. No unfiltered `cleanSourceWith` / `lib.cleanSource` over the whole
   repo. Always pass a `filter` predicate that excludes `target/`,
   `result*`, `node_modules/`, `agents/*/build/`, `.workestrate/`, and
   any large untracked dir. [purity]

3. All dependency fetching goes through fixed-output derivations with an
   `outputHash`: `buildNpmPackage` (uses `npmDepsHash`), FOD pip
   (`fetchPypi`/`fetchFromGitHub` + `outputHash`), FOD bun. No
   `fetchTarball` without a hash, no `builtins.fetchGit` of mutable refs.
   [purity]

4. No network in `buildPhase`/`installPhase`. If a build needs data, it
   must be committed (e.g. pi's model catalogs) or fetched via a FOD
   before the build runs. [purity]

5. No `--impure` anywhere: not in scripts, not in the justfile, not in
   ad-hoc `nix build --impure` commands. [purity]

6. No `getFlake (toString ./.)` or self-referential flake fetching. The
   flake is entered via its own inputs only. [purity]

7. Large untracked directories (`target/`, `agents/*/build/`,
   `node_modules/`) must NEVER be referenced by nix code — not even
   transitively via a parent `src = ./.`. [purity]

8. `CARGO_TARGET_DIR` for any in-tree cargo invocation must point outside
   the source tree the flake sees (canonical:
   `~/.cache/ai-workbench/agentctl-target`), so cargo artifacts never
   become part of the nix source closure. [purity]

> **Confirmed:** the canonical `CARGO_TARGET_DIR` is
> `${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target` (default
> `~/.cache/ai-workbench/agentctl-target` when `XDG_CACHE_HOME` is unset).
> It is exported by the Nix devshell shellHook
> (`nix/devshells/default.nix`) and by the top-level `justfile`
> (`export CARGO_TARGET_DIR := ...` evaluated at just-parse time), so both
> `nix develop` sessions and bare `just` invocations relocate cargo
> artifacts out of the source tree. [purity]

## Review checklist

When reviewing a derivation for purity, verify each rule:

- [ ] **Rule 1:** No bare `src = ./.` or `toString ./.`. Local sources use
      `builtins.path { name = ...; path = ./subdir; filter = ...; }`.
      *(Guard: does NOT catch bare `src = ./.` — check 5 matches `../`
      only. Rule 1 is authoritative even when the guard passes.)* [purity]
- [ ] **Rule 2:** `cleanSourceWith` / `lib.cleanSource` has a `filter`
      predicate excluding `target/`, `result*`, `node_modules/`,
      `agents/*/build/`, `.workestrate/`. *(Guard: check 4 catches
      `cleanSourceWith { ... }` without `filter =` in the following 15
      lines.)* [guard]
- [ ] **Rule 3:** All dependency fetching goes through FODs with
      `outputHash` / `npmDepsHash` / `cargoHash`. No `fetchTarball`
      without a hash. *(Guard: not directly checked — manual review.)*
- [ ] **Rule 4:** No network in `buildPhase`/`installPhase`. Data is
      committed or fetched via a FOD. *(Guard: not directly checked —
      manual review; `sandbox = true` enforces at build time.)*
- [ ] **Rule 5:** No `--impure` in scripts, justfile, or nix code.
      *(Guard: check 1 catches `nix ... --impure` and
      `nix-shell ... --impure`.)* [guard]
- [ ] **Rule 6:** No `getFlake (toString ./.)`. *(Guard: check 2 catches
      `builtins.getFlake` combined with `toString` on the same line.)*
      [guard]
- [ ] **Rule 7:** No `../` parent-directory path literals outside
      `src =`/`lockFile =`/`path =` escape-hatch fields. *(Guard: check 5
      catches `../` literals in nix assignments outside escape-hatch
      fields.)* [guard]
- [ ] **Rule 8:** `CARGO_TARGET_DIR` points outside the source tree
      (`~/.cache/ai-workbench/agentctl-target`). *(Guard: not directly
      checked — manual review.)* [purity]
- [ ] **`builtins.path` with `filter`:** *(Guard: check 3 catches
      `builtins.path { ... }` without `filter =` in the following 15
      lines.)* [guard]
- [ ] **Docs:** no impure-pattern references in `docs/**/*.md`
      (`getFlake ... toString`, `nix eval --impure <arg>`,
      `toString ./.`). *(Guard: check 6, unless file is in
      `DOCS_ALLOWLIST`.)* [guard]

## Implementation checklist

How to add a new derivation: [purity]

1. **Decide the source filter.** Use `builtins.path` with an explicit
   `name` and `filter`, or consume a flake input. Never `src = ./.`.
2. **Decide the dependency strategy.** All deps go through a FOD with
   `outputHash` / `npmDepsHash` / `cargoHash`. No `fetchTarball` without
   a hash.
3. **Confirm no network in build/install.** Any data the build needs is
   committed or fetched via a FOD before the build runs.
4. **Confirm no `--impure`.** Not in the derivation, not in a wrapping
   script, not in the justfile.
5. **Run `just lint-nix`.** The guard must pass.
6. **Build with `nix build .#<name>`.** Confirm it succeeds.
7. **If a hash is unknown**, use `fakeHash` / `lib.fakeSha256` as a
   placeholder, then run the `update-hashes` recipe to prefetch the
   real hash.

   > **Confirmed:** `just update-hashes` runs
   > `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json`
   > (tempest `npmDepsHash`) and `nix build .#opencode-built` /
   > `.#odysseus-built --no-link` to surface the `got:` sha256 for
   > opencode `bunDeps.outputHash` and odysseus `pipDeps.outputHash`. It
   > prints the hashes; the operator manually inlines each `got:` value
   > into the matching `nix/packages/*.nix` file (the recipe prints
   > step-by-step instructions). It does not auto-rewrite the files.
   > [purity]

8. **Add a HOST-GATE note** if the build can only be verified on a host
   with nix. This container has no nix; any `nix build` claim here is
   based on documented Nix semantics, not runtime verification. [purity]

## Validation hooks

### `just lint-nix` / `scripts/check-nix-paths.sh`

`scripts/check-nix-paths.sh` is a bash `case`-pattern guard (not grep) wired
into `just verify` as the `lint-nix` recipe. It scans `*.nix` under
`flake.nix`/`nix`/`templates` plus `*.sh` under `scripts/` and the
`justfile`, plus `*.md` under `docs/` (check 6). [purity] [guard]

The guard catches **6 checks** (from the script's own header comments):

1. **`nix ... --impure` invocations in shell scripts and nix code.**
   Comments and the linter's own source are skipped. Matches
   `*nix\ *--impure*` and `*nix-shell*--impure*`. [guard]
2. **`builtins.getFlake` combined with `toString` on the same line**
   — impure self-referential flake fetching. Use a flake input instead.
   Matches `*builtins.getFlake*toString*`. [guard]
3. **`builtins.path { ... }` without a `filter =` field in the following
   15 lines** — unbounded path copy into the store (closes the
   disk-exhausting eval class). Uses an awk block that scans lines
   `start` through `start + 15` for `filter =`. [guard]
4. **`cleanSourceWith { ... }` without a `filter =` field in the
   following 15 lines** — same problem via the lib helper. Same awk
   block as check 3. [guard]
5. **`../` (parent-directory) path literals in nix code outside the
   bounded `src =` / `lockFile =` / `path =` escape-hatch fields.**
   Uses awk to strip quoted strings, then flags lines with a nix
   assignment (`=`) and a `../` literal on the RHS, unless the field
   is `src`, `lockFile`, or `path`. [guard]
6. **Impure-pattern references in `docs/**/*.md`** (including
   `docs/migration/`): `getFlake ... toString`, `nix eval --impure <arg>`
   (invocation form — a trailing flag/argument is required so
   backtick-quoted prose mentions like `` `nix eval --impure` against a ``
   do not match), and `toString ./.`. Docs are where these patterns
   historically leaked into subagent-run regression gates. [guard]

#### Allowlists

- **Line-level:** lines matching `# allow: <reason>` are skipped in every
  scanned file type (belt-and-suspenders in docs). Use sparingly — every
  allowlist entry is a documented purity exception. [guard]
- **File-level (docs only):** `DOCS_ALLOWLIST` names whole-document
  discussion contexts where the patterns legitimately appear in narrative
  prose, code blocks, and rule statements. The 3 allowlisted docs are:
  [guard]
  - `docs/nix-purity.md` — FORBIDS the pattern (rules)
  - `docs/nix-store-accumulation-report.md` — incident narrative
  - `docs/migration/nix-store-gc-remediation-spec.md` — spec BEFORE examples

#### Guard limitations

The guard does **NOT** catch bare `src = ./.` (current-dir literal): check 5
matches `../` only. Rule 1 above still forbids bare `src = ./.` — the guard
is a static heuristic, not a prover, and rule 1 remains authoritative even
when the guard passes. [purity]

The guard is a **static heuristic, not a full eval-purity prover**. It
catches the common impurity patterns but cannot prove a derivation is pure.
The rules in "Practical rules" above remain authoritative even when the
guard passes. [purity]

### `just store-audit` / `scripts/store-audit.py`

- Reports the top-20 store paths by closure size (via
  `nix path-info --all --json` + a python reducer).
- Flags any `*-source` paths referencing `ai-workbench` (via
  `nix path-info --all | grep -E "ai-workbench.*-source$"`). Non-empty
  `*-source` output indicates an unbounded source copy that should be
  bounded by a `cleanSourceWith` filter. [purity]
- `--fail-if-source-over 50` is wired into `just verify` (V2 landed) —
  `verify` now fails when `*-source` paths exceed the 50M threshold.
  [usage]
- Skips with a one-line note when nix is unavailable.

For full detail, see `/docs/nix/nix-store-and-paths.md`. [store]

### `just gc`

Runs `nix-collect-garbage --delete-old` + `nix store optimise` (dedupe).
Reclaims unreachable store paths and deduplicates content-addressed copies
(vectors (b) and (c) of the store-growth model). [purity] [usage]

### `just store-delta-check` (V3)

Exists as a recipe but is **NOT** wired into `verify` — it is a periodic
host/CI check that fails on new source-path copies exceeding the <50M
criterion from the remediation spec. [usage]

## Examples

### (a) FOD template

A fixed-output derivation using `stdenv.mkDerivation`: [purity]

```nix
{ lib, stdenv, fetchFromGitHub }:

stdenv.mkDerivation {
  pname = "my-fod";
  version = "0.1.0";

  # Source comes from a flake input or a hashed fetch — never ./.
  src = fetchFromGitHub {
    owner = "example";
    repo = "my-fod";
    rev = "v0.1.0";
    # hash = lib.fakeSha256; # TODO: replace via just update-hashes
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  # Fixed-output declaration — the output is verified by hash, not by
  # the build steps. This is what permits network in the fetch phase
  # (the fetcher runs under a FOD, not the main build).
  outputHashAlgo = "sha256";
  outputHashMode = "recursive";
  # outputHash = lib.fakeSha256; # TODO: replace via just update-hashes
  outputHash = "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";

  # No network here. All deps arrived via the FOD fetch above.
  buildPhase = ''
    runHook preBuild
    make build
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r dist/* $out/
    runHook postInstall
  '';

  # HOST-GATE: hash verified on host via just update-hashes
  # (this container has no nix; the hash above is a placeholder).
}
```

The `fakeHash` / `lib.fakeSha256` convention marks a hash as
not-yet-prefetched: `hash = lib.fakeSha256; # TODO: replace via just update-hashes`.
Running `just update-hashes` prefetches the real hash and prints it; the
operator manually inlines the `got:` value into the matching
`nix/packages/*.nix` file (the recipe prints step-by-step instructions).
[purity]

### (b) Image recipe

`streamLayeredImage` is preferred over `buildLayeredImage` because it
streams the tarball to stdout and avoids materializing a large image path in
the Nix store. A minimal example: [purity]

```nix
{ pkgs }:

pkgs.dockerTools.streamLayeredImage {
  name = "my-service";
  tag = "latest";

  contents = [
    pkgs.nodejs_22
    # runtime deps here
  ];

  config = {
    Cmd = [ "node" "server.js" ];
    WorkingDir = "/app";
    Env = [ "NODE_ENV=production" ];
  };
}
```

Build and load: [purity]

```bash
nix build .#myImage
docker load < result
```

**Why `streamLayeredImage` over `buildLayeredImage`:** `streamLayeredImage`
streams the tarball to stdout, so no large image path is materialized in the
Nix store. `buildLayeredImage` materializes a 0.5–1G tarball as a store
path, which accumulates across image revisions. For new images, always use
`streamLayeredImage`. [purity] [usage]

### (c) `builtins.path` with filter

A correct example showing `builtins.path` with an explicit `name` and
`filter`:

```nix
# Only .nix and .toml files under ./config enter the store.
# target/, result*, node_modules/ are excluded by the filter predicate.
src = builtins.path {
  name = "my-config";
  path = ./config;
  filter = path: type:
    let base = baseNameOf path; in
    type == "directory" ||
    lib.hasSuffix ".nix" base ||
    lib.hasSuffix ".toml" base;
};
```

### (d) `cleanSourceWith` with filter

A correct example showing `cleanSourceWith` with a `filter` predicate:

```nix
src = lib.cleanSourceWith {
  src = ./.;
  filter = path: type:
    let base = baseNameOf path; in
    type == "directory" ||
    !(lib.hasSuffix ".md" base) && !(lib.hasPrefix "result" base);
};
```

### (e) Anti-pattern: `src = ./.` (forbidden)

```nix
# FORBIDDEN — copies the entire repo working tree (including gitignored
# target/ at 16-28G, agents/*/build/, .workestrate/) into the store on
# every evaluation. This is the 29GB incident class.
src = ./.;
```

Use `builtins.path { name = ...; path = ./subdir; filter = ...; }` or a
flake input instead. [purity]

### (f) Real-world pattern from `flake.nix`: `--no-link --print-out-paths`

The `load-images` derivation in `flake.nix` uses
`--no-link --print-out-paths` to avoid GC root leaks: [flake]

```nix
load-one = name: ''
  echo "Loading ${name}..."
  out=$(nix build .#${name} --no-link --print-out-paths)
  gunzip -c "$out" | msb load -t ${name}:latest
'';
```

From the comment in `flake.nix`: [flake]

> Anti-accumulation: uses `--no-link --print-out-paths` so no /tmp GC root
> is created. The previous `--out-link /tmp/<name>.tar.gz` form left a
> symlink + tarball in /tmp that survived across runs and was never
> garbage-collected by nix. The new form pipes the store path directly into
> `gunzip | msb load`, leaving no /tmp residue.

## Common mistakes

| Mistake | Consequence |
| --- | --- |
| Bare `src = ./.` | Unbounded store copy — the 29 GB incident class. The guard does NOT catch this (check 5 matches `../` only); rule 1 is authoritative. [purity] |
| `cleanSourceWith` without `filter` | Same unbounded store copy via the lib helper. Guard check 4 catches this. [guard] |
| `builtins.path { ... }` without `filter` | Unbounded path copy into the store. Guard check 3 catches this. [guard] |
| `builtins.getFlake (toString ./.)` | Impure self-referential flake fetch — copies the raw working tree. Guard check 2 catches this. [guard] |
| `nix eval --impure` in gates | Copies the working tree each run — 16–35G per eval. Guard check 1 catches `--impure`; check 6 catches docs references. [guard] |
| `--out-link /tmp/foo.tar.gz` | Pins the closure forever via a GC root symlink. Use `--no-link --print-out-paths` instead. [flake] |
| `buildLayeredImage` for new images | 0.5–1G tarballs materialized in the store. Use `streamLayeredImage`. [purity] [usage] |
| Forgetting `git add` new files | Untracked files are invisible to `.#` refs (classic flakes gotcha). [usage] |
| `nix flake update` in CI | Multi-GB rebuild — new nixpkgs revision pulls a new toolchain/closure. Update deliberately, never in CI. [usage] |
| Treating `auto-optimise-store` as a substitute for purity rules | It only reduces vector (c) (content-addressed copies); it does nothing for vectors (a) and (b). [purity] |

## Strict vs contextual guidance

| Rule | Strict (always) | Contextual (depends) |
| --- | --- | --- |
| `builtins.path` with `filter` for local `src` | Strict | — |
| `cleanSourceWith` with `filter` predicate | Strict | — |
| FOD for all source fetching | Strict | — |
| No network in `buildPhase`/`installPhase` | Strict | — |
| No `--impure` | Strict | — |
| `streamLayeredImage` over `buildLayeredImage` (new images) | Strict | — |
| `--no-link --print-out-paths` over `--out-link` | Strict | — |
| `sandbox = true` in `nix.conf` | Strict (where platform supports) | — |
| `CARGO_TARGET_DIR` relocation (in-tree cargo the flake sees) | Strict | — |
| `auto-optimise-store` | — | Contextual (advisory; not a purity substitute) |
| `__noChroot` | — | Contextual (almost never; only for derivations that genuinely cannot run sandboxed, e.g. some FODs needing `/etc`) |
| `max-jobs` / `cores` | — | Contextual (performance tuning; no purity impact) |

## Policy decisions for individual repos

The `ai-workbench` repo applies the general rules above via the following
repo-specific mechanisms:

- **`just lint-nix` guard** (`scripts/check-nix-paths.sh`): a bash
  `case`-pattern static guard wired into `just verify`. It scans `*.nix`
  under `flake.nix`/`nix`/`templates` plus `*.sh` under `scripts/` and the
  `justfile`, plus `*.md` under `docs/` (check 6). It fails the gate on the
  6 checks documented in Validation hooks. Allowlists: line-level
  `# allow: <reason>` and file-level `DOCS_ALLOWLIST` (3 docs). [guard]
- **`just update-hashes` recipe**: prefetches `npmDepsHash` (tempest),
  `bunDeps.outputHash` (opencode), and `pipDeps.outputHash` (odysseus). It
  prints the `got:` hashes; the operator manually inlines each into the
  matching `nix/packages/*.nix` file. It does not auto-rewrite the files.
  [purity]
- **`just gc` and `just store-audit` hygiene cadence**: `just gc` runs
  `nix-collect-garbage --delete-old` + `nix store optimise` (dedupe). `just
  store-audit` reports the top-20 store paths by closure size and flags any
  `*-source` paths referencing `ai-workbench` (indicating an unbounded
  source copy that should be bounded by a `cleanSourceWith` filter).
  [purity]
- **HOST-GATE convention**: This container has no nix; all `nix build`
  claims are based on documented Nix semantics, not runtime verification.
  Derivations that can only be verified on a host with nix carry a
  `# HOST-GATE:` comment. [purity]
- **Authoritative purity reference**: `/docs/nix-purity.md` is the
  repo-root quick reference (the source this doc elevates). This corpus doc
  (`/docs/nix/purity-and-sandboxing.md`) is the expanded reference with
  sandboxing, `nix.conf`, and enforcement coverage.
- **Devshell gcroot pinning**: the devshell toolchain is gcroot-pinned at
  `/nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`. If stale after
  a `flake.lock` change, re-pin with
  `nix build .#devShells.x86_64-linux.default --out-link /nix/var/nix/gcroots/per-user/node/ai-workbench-devshell`.
  Rollback: `rm` the gcroot + `nix-collect-garbage -d`. [usage]

## Related docs

- `/docs/nix-purity.md` — repo-root purity quick reference (the source this
  doc elevates)
- `/docs/nix/nix-store-and-paths.md` — store path types, closures, GC,
  store-growth model, 29 GB incident timeline
- `/docs/nix/derivations-and-builds.md` — derivations, stdenv, build
  phases, FODs
- `/docs/nix/flake-anatomy.md` — flake structure
- `/docs/nix/devshells.md` — dev shells

## Related skills

- `.agents/skills/nix-usage` — Nix flake, dev shell, Rust toolchain, and
  Microsandbox runtime reference (anti-accumulation patterns table and
  verbatim agent rules)

## Citations

1. [Nix Best Practices](https://nix.dev/guides/best-practices.html)
2. [Nix Language — Derivations](https://nix.dev/manual/nix/2.34/language/derivations)
3. Local: `/docs/nix-purity.md` — workestrator purity rules (PRIMARY SOURCE
   ELEVATED)
4. Local: `/scripts/check-nix-paths.sh` — purity enforcement guard
5. Local: `/.agents/skills/nix-usage/SKILL.md` — nix-usage skill
   (anti-accumulation patterns, agent rules)
6. Local: `/docs/nix/nix-store-and-paths.md` — store paths, closures, GC,
   store-growth model
7. Local: `/docs/nix/.crawl/59-nix-language-derivations.md` — derivation
   purity (path-copy semantics)
8. Local: `/flake.nix` — real-world FOD and anti-accumulation patterns
