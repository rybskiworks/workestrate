# Writing and maintaining pure derivations in workestrate

This document is the canonical reference for keeping Nix evaluation and
builds pure in the workestrate flake. It records the two purity axes
(eval-time and build-time), the imperative rules that enforce them, the
29 GB-per-eval incident that motivated the rules, the store-growth model
that explains why impurity balloons the store, the `just lint-nix`
enforcement guard, a checklist for adding new derivations, recipe
patterns for fixed-output and image derivations, and the one sanctioned
impure zone (devshell in-tree agent builds). Every contributor adding or
editing a nix derivation under `nix/` or `flake.nix` is expected to read
this first.

## The two purity axes

Purity in Nix has two independent axes. **Both must hold** for a
derivation to be considered pure; either one alone is insufficient.

### (a) Eval-time purity

Eval-time purity is what `nix` sees when it computes the derivation
graph — before any build runs. The evaluation walks the flake's source
closure (the set of paths the flake references) and turns it into store
paths. If that closure is impure — if it includes untracked files, large
build outputs, or directories whose contents change on every edit — then
every evaluation produces a new, different source closure, and Nix
copies the whole thing into the store each time. The rules that govern
this axis are about **source filtering** and **what paths the flake is
allowed to reference**: no `toString ./.`, no `builtins.getFlake`, no
impure `builtins` like `builtins.getEnv` outside controlled wrappers,
and no referencing untracked or large directories (notably `target/`,
`agents/*/build/`, `node_modules/`).

### (b) Build-time purity

Build-time purity is what happens inside the build sandbox once
evaluation has produced a derivation. The sandbox has no network in
`buildPhase`/`installPhase`; all dependencies must arrive via fixed-output
derivations (FODs) with declared `outputHash`es; and `--impure` is
forbidden everywhere. A sandboxed build is necessary but not
sufficient: **eval impurity balloons the store even when the build
itself is sandboxed**, because the impure source closure is copied on
every evaluation regardless of whether the build runs.

## THE RULES

> **Operational quick-reference for agents:** see
> [`.agents/skills/nix-usage`](../.agents/skills/nix-usage/SKILL.md)
> for the anti-accumulation patterns table and verbatim agent rules.

1. Never reference the repo root as a path. Use
   `builtins.path { name = ...; path = ./subdir; filter = ...; }` with
   an explicit `name` and `filter` so only intended files enter the
   store. Bare `./.` or `toString ./.` is forbidden.
2. No unfiltered `cleanSourceWith` / `lib.cleanSource` over the whole
   repo. Always pass a `filter` predicate that excludes `target/`,
   `result*`, `node_modules/`, `agents/*/build/`, `.workestrate/`, and
   any large untracked dir.
3. All dependency fetching goes through fixed-output derivations with an
   `outputHash`: `buildNpmPackage` (uses `npmDepsHash`), FOD pip
   (`fetchPypi`/`fetchFromGitHub` + `outputHash`), FOD bun. No
   `fetchTarball` without a hash, no `builtins.fetchGit` of mutable refs.
4. No network in `buildPhase`/`installPhase`. If a build needs data, it
   must be committed (e.g. pi's model catalogs) or fetched via a FOD
   before the build runs.
5. No `--impure` anywhere: not in scripts, not in the justfile, not in
   ad-hoc `nix build --impure` commands.
6. No `getFlake (toString ./.)` or self-referential flake fetching. The
   flake is entered via its own inputs only.
7. Large untracked directories (`target/`, `agents/*/build/`,
   `node_modules/`) must NEVER be referenced by nix code — not even
   transitively via a parent `src = ./.`.
8. `CARGO_TARGET_DIR` for any in-tree cargo invocation must point outside
   the source tree the flake sees (canonical:
   `~/.cache/ai-workbench/agentctl-target`), so cargo artifacts never
   become part of the nix source closure.

> **Confirmed:** the canonical `CARGO_TARGET_DIR` is `${XDG_CACHE_HOME:-$HOME/.cache}/ai-workbench/agentctl-target` (default `~/.cache/ai-workbench/agentctl-target` when `XDG_CACHE_HOME` is unset). It is exported by the Nix devshell shellHook (`nix/devshells/default.nix`) and by the top-level `justfile` (`export CARGO_TARGET_DIR := ...` evaluated at just-parse time), so both `nix develop` sessions and bare `just` invocations relocate cargo artifacts out of the source tree.

## Why: the 29GB-per-eval incident

An impure eval path previously caused Nix to copy `target/` and
`agents/*/build/` into the store on every evaluation. Because those
directories were large and changed on every edit, each evaluation
produced a new content-addressed store path containing the full dirty
source closure. The store grew at roughly 1 GB/min and reached
approximately 29 GB before the impurity was caught and the source filter
fixed.

The mechanism is straightforward: a dirty source closure changes hash
on every edit, so Nix produces a new store path each time, and those
paths are kept alive by live GC roots (dev-shell profiles, `result*`
symlinks, `nix build` outputs). Garbage collection could not reclaim
them because the roots were still live. This incident is the motivating
case for every rule above and for the `just lint-nix` guard below.

## The store-growth model

The store grows along three vectors. Understanding all three is
necessary to keep the store bounded.

### (a) Per-edit churn

Each evaluation with an impure source filter produces a new store path.
With a dirty `target/` or `agents/*/build/` in the closure, every
`nix develop` or `nix build` copies the whole closure again. This is
the dominant growth vector during active development — it is what
produced the 29 GB incident.

### (b) GC roots

Live `result*` symlinks, dev-shell profiles, and `nix build` outputs
keep paths alive. A path that is unreachable is collectable; a path held
by a live root is not. Stale `result*` symlinks pointing at long-gone
paths, and accumulated dev-shell profiles, are the second growth vector.

### (c) Content-addressed copies

The same content under different store paths is duplicated until
`nix store optimise` (or `auto-optimise-store`) deduplicates it. This
is the third vector: even when content is identical, distinct paths
consume space until deduplication runs.

### Hygiene cadence

- Run `just gc` regularly to collect unreachable store paths.

> **Confirmed:** `just gc` runs `nix-collect-garbage --delete-old` followed by `nix store optimise` (dedupe). It reclaims unreachable store paths and deduplicates content-addressed copies (vectors (b) and (c) of the store-growth model).

- Run `just store-audit` when the store feels large, to audit what is
  consuming space and find stale roots.

> **Confirmed:** `just store-audit` reports the top-20 store paths by closure size (via `nix path-info --all --json` + a python reducer) and flags any `*-source` paths referencing `ai-workbench` (via `nix path-info --all | grep -E "ai-workbench.*-source$"`). Non-empty `*-source` output indicates an unbounded source copy that should be bounded by a `cleanSourceWith` filter.

- `auto-optimise-store` is advisory: it deduplicates identical content
  but is **not** a substitute for the purity rules. It reduces vector
  (c) but does nothing for vectors (a) and (b).

## Enforcement: `just lint-nix` (check-nix-paths.sh) in `just verify`

`scripts/check-nix-paths.sh` is a static guard wired into `just verify`
as the `lint-nix` recipe. The guard uses bash `case` patterns (not grep)
to scan for forbidden patterns and fails the gate on any match.

> **Confirmed:** `scripts/check-nix-paths.sh` is a bash `case`-pattern guard (not grep) wired into `just verify` as the `lint-nix` recipe. It scans `*.nix` under `flake.nix`/`nix`/`templates` plus `*.sh` under `scripts/` and the `justfile`. Allowlist: lines containing `# allow: <reason>` are skipped.

The guard catches:

- `nix ... --impure` and `nix-shell ... --impure` invocations (in shell scripts, the justfile, and nix code)
- `builtins.getFlake` combined with `toString` on the same line (impure self-referential flake fetching)
- `builtins.path { ... }` without a `filter =` field in the following 15 lines (unbounded store copy)
- `cleanSourceWith { ... }` without a `filter =` field in the following 15 lines (same problem via lib)
- `../` (parent-directory) path literals in nix assignments, outside the bounded `src =` / `lockFile =` / `path =` escape-hatch fields

The guard does NOT catch `src = ./.` (current-dir literal): check 5 matches `../` only. Rule 1 above still forbids bare `src = ./.` — the guard is a static heuristic, not a prover, and rule 1 remains authoritative even when the guard passes.

The guard is a **static heuristic, not a full eval-purity prover**. It
catches the common impurity patterns but cannot prove a derivation is
pure. The rules in "THE RULES" above remain authoritative even when the
guard passes.

## How to add a new derivation (checklist)

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

   > **Confirmed:** `just update-hashes` runs `nix run nixpkgs#prefetch-npm-deps -- agents/tempest/repo/package-lock.json` (tempest `npmDepsHash`) and `nix build .#opencode-built` / `.#odysseus-built --no-link` to surface the `got:` sha256 for opencode `bunDeps.outputHash` and odysseus `pipDeps.outputHash`. It prints the hashes; the operator manually inlines each `got:` value into the matching `nix/packages/*.nix` file (the recipe prints step-by-step instructions). It does not auto-rewrite the files.

8. **Add a HOST-GATE note** if the build can only be verified on a host
   with nix. This container has no nix; any `nix build` claim here is
   based on documented Nix semantics, not runtime verification.

## Recipe patterns

### (a) FOD template

A fixed-output derivation using `stdenv.mkDerivation`:

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
Running `just update-hashes` prefetches the real hash and prints it; the operator manually inlines the `got:` value into the matching `nix/packages/*.nix` file (the recipe prints step-by-step instructions).

### (b) Image recipe

`streamLayeredImage` is preferred over `buildLayeredImage` because it
streams the tarball to stdout and avoids materializing a large image
path in the Nix store. A minimal example:

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

Build and load:

```bash
nix build .#myImage
docker load < result
```

For the full image-building guidance (when to use `streamLayeredImage`
vs `buildLayeredImage` vs `buildImage`, layer caching, digest pinning),
see [`.agents/skills/nix-docker-images/SKILL.md`](../.agents/skills/nix-docker-images/SKILL.md).

## Sanctioned impure zone: devshell in-tree agent builds

The dev shell (`nix develop`) building agents in-tree
(`agents/<name>/build`, `just dev-build-pi`) is the **one** sanctioned
impure zone. It is acceptable because:

- **Tree-side, not store-side.** The build output lives in the working
  tree (`agents/<name>/build`), not in `/nix/store`. It is never a store
  path.
- **Store-inert.** Because it is not a store path, it does not
  participate in GC root accounting and does not produce content-addressed
  copies.
- **Does not feed back into flake evaluation**, as long as the source
  filters exclude `agents/*/build/`. The dev shell's in-tree build is a
  developer convenience loop (fast, hashless, editable), not a purity
  claim.

This is a developer convenience loop, not a purity claim. The moment
such an output is referenced by a nix derivation (e.g. a derivation that
does `src = agents/pi/build`), the purity rules in "THE RULES" apply in
full: that reference is forbidden, and the build output must instead be
produced by a proper FOD-backed derivation (`.#pi-bun`, etc.) or
relocated out of the flake-visible source tree into the managed sources
store.

> **Confirmed (partial):** the `CARGO_TARGET_DIR` relocation has landed (see rule 8). The relocation of `agents/<name>/build` outputs into the managed sources store (`~/.local/share/workestrate/sources/<name>/`) is **deferred** — it requires config-repo changes (the `source build`/`source clone` commands and `WORKESTRATE_<NAME>_BUILD` resolution path must point at the sources store). See `docs/migration/70-open-items.md` ("Agent build-output relocation"). Until relocation lands, `agents/<name>/build` remains the sanctioned in-tree dev zone (`just dev-build-pi` writes there), kept out of the flake source closure by `.gitignore` (`agents/*/build`) and the `agentctl.nix` `cleanSourceWith` filter (for the `control/agentctl` tree). The purity rules above hold regardless: the rules require only that the source filters exclude these paths, which they do.
