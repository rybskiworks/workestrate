---
name: constraint-nix-purity
description: |
  Enforces Nix eval-time and build-time purity invariants during code execution.
  Load when writing or reviewing Nix derivations, `flake.nix`, or files under
  `nix/` — any code that copies paths into the Nix store, declares source
  filters, or wires dependency fetching. Does NOT cover reproducibility/pinning
  (see constraint-nix-reproducibility) or store hygiene/GC (see
  constraint-nix-store-hygiene).
metadata:
  org.kind: constraint
---

# Constraint: Nix Purity (Eval-time and Build-time)

This constraint enforces the two purity axes that keep Nix derivations
hermetic: **eval-time purity** (what `nix` copies into the store when it
computes the derivation graph) and **build-time purity** (what happens inside
the sandbox once a derivation runs). Both axes must hold; either alone is
insufficient. Violations either fail the `just lint-nix` guard, balloon the
store on every evaluation, or break hermeticity.

The motivating incident: an impure eval path previously copied `target/` and
`agents/*/build/` into the store on every evaluation, growing the store at
~1 GB/min to ~29 GB before the source filter was fixed.

## Triggers

Load this skill when:

- Writing or reviewing a Nix derivation under `nix/`, `flake.nix`, or
  `templates/`.
- Writing or reviewing `builtins.path`, `cleanSourceWith`, `lib.cleanSource`,
  or any `src =` assignment in nix code.
- Wiring dependency fetching (`fetchFromGitHub`, `fetchPypi`, `buildNpmPackage`,
  FOD pip, FOD bun).
- Writing `buildPhase` / `installPhase` that might reach the network.
- Reviewing a PR for nix purity correctness.
- Editing shell scripts or the justfile that invoke `nix`.

## Rules

1. Never reference the repo root as a path. Use
   `builtins.path { name = ...; path = ./subdir; filter = ...; }` with an
   explicit `name` and `filter` so only intended files enter the store. Bare
   `./.` or `toString ./.` is forbidden.
2. No unfiltered `cleanSourceWith` / `lib.cleanSource` over the whole repo.
   Always pass a `filter` predicate that excludes `target/`, `result*`,
   `node_modules/`, `agents/*/build/`, `.workestrate/`, and any large
   untracked dir.
3. All dependency fetching goes through fixed-output derivations (FODs) with
   an `outputHash`: `buildNpmPackage` (uses `npmDepsHash`), FOD pip
   (`fetchPypi`/`fetchFromGitHub` + `outputHash`), FOD bun. No
   `fetchTarball` without a hash, no `builtins.fetchGit` of mutable refs.
4. No network in `buildPhase` / `installPhase`. If a build needs data, it
   must be committed (e.g. pi's model catalogs) or fetched via a FOD before
   the build runs.
5. No `--impure` anywhere: not in scripts, not in the justfile, not in
   ad-hoc `nix build --impure` commands.
6. No `getFlake (toString ./.)` or self-referential flake fetching. The
   flake is entered via its own inputs only.
7. Large untracked directories (`target/`, `agents/*/build/`,
   `node_modules/`) must NEVER be referenced by nix code — not even
   transitively via a parent `src = ./.`.
8. `CARGO_TARGET_DIR` for any in-tree cargo invocation must point outside
   the source tree the flake sees (canonical:
   `~/.cache/ai-workbench/agentctl-target`), so cargo artifacts never become
   part of the nix source closure.

## References

- Docs: `docs/nix/purity-and-sandboxing.md` (canonical corpus reference),
  `docs/nix-purity.md` (repo-root quick reference).
- Operational skill: `nix-usage` (anti-accumulation patterns table, agent
  rules).
- Enforcement guard: `scripts/check-nix-paths.sh` (6 checks, wired into
  `just lint-nix` / `just verify`).

## Out of scope

- Reproducibility / dependency pinning / `flake.lock` discipline — see
  `constraint-nix-reproducibility`.
- Store hygiene, GC roots, `result*` symlink cleanup, `just gc` cadence —
  see `constraint-nix-store-hygiene`.
- Devshell in-tree agent builds (`agents/<name>/build`, `just dev-build-pi`)
  — the one sanctioned impure zone. The build output lives in the working
  tree, not `/nix/store`; it is store-inert and does not feed back into flake
  evaluation as long as source filters exclude `agents/*/build/`. The moment
  such an output is referenced by a nix derivation, the purity rules apply in
  full.

## Violation examples

### Bare `src = ./.` (forbidden — the 29 GB incident class)

```nix
# FORBIDDEN — copies the entire repo working tree (including gitignored
# target/ at 16-28G, agents/*/build/, .workestrate/) into the store on
# every evaluation.
src = ./.;
```

Correct — `builtins.path` with explicit `name` and `filter`:

```nix
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

### `cleanSourceWith` without a `filter` predicate

```nix
# FORBIDDEN — unbounded store copy via the lib helper (guard check 4).
src = lib.cleanSourceWith { src = ./.; };
```

Correct — pass a `filter` excluding `target/`, `result*`, `node_modules/`:

```nix
src = lib.cleanSourceWith {
  src = ./.;
  filter = path: type:
    let base = baseNameOf path; in
    type == "directory" ||
    !(lib.hasSuffix ".md" base) && !(lib.hasPrefix "result" base);
};
```

### `builtins.getFlake (toString ./.)` (impure self-referential fetch)

```nix
# FORBIDDEN — impure self-referential flake fetching; copies the raw
# working tree. Guard check 2 catches this.
let flake = builtins.getFlake (toString ./.); in ...
```

Correct — use native `.#` refs (git-filtered) or a flake input.

### `nix build --impure` in a script or justfile

```bash
# FORBIDDEN — --impure bypasses the sandbox and copies the working tree.
nix build --impure .#myPackage
nix eval --impure --expr '...'
```

Correct — use native flake refs: `nix build .#myPackage`,
`nix eval .#packages.x86_64-linux.myPackage.drvPath`.

### `fetchTarball` without a hash

```nix
# FORBIDDEN — mutable fetch with no outputHash; not reproducible, not pure.
src = builtins.fetchTarball { url = "https://example.com/foo.tar.gz"; };
```

Correct — use `fetchFromGitHub` / `fetchPypi` with a declared `hash`, or a
FOD with `outputHash`.

### Network in `buildPhase`

```nix
# FORBIDDEN — sandbox has no network in buildPhase/installPhase.
buildPhase = ''
  curl https://example.com/model.bin -o $out/model.bin
'';
```

Correct — fetch via a FOD before the build, or commit the data.

## How to check

```bash
just lint-nix                    # runs scripts/check-nix-paths.sh (6 checks)
just verify                      # full gate suite (includes lint-nix + store-audit)
```

The guard (`scripts/check-nix-paths.sh`) catches:

1. `nix ... --impure` invocations in shell scripts and nix code.
2. `builtins.getFlake` combined with `toString` on the same line.
3. `builtins.path { ... }` without a `filter =` field in the following 15
   lines.
4. `cleanSourceWith { ... }` without a `filter =` field in the following 15
   lines.
5. `../` parent-directory path literals outside `src =` / `lockFile =` /
   `path =` escape-hatch fields.
6. Impure-pattern references in `docs/**/*.md`.

Guard limitations: the guard does NOT catch bare `src = ./.` (check 5 matches
`../` only). Rule 1 remains authoritative even when the guard passes. The
guard is a static heuristic, not a full eval-purity prover.

Manual review checklist:

- No bare `src = ./.` or `toString ./.` anywhere in nix code.
- Every `builtins.path` and `cleanSourceWith` has a `filter` predicate.
- All dependency fetching goes through FODs with `outputHash` /
  `npmDepsHash` / `cargoHash`.
- No `fetchTarball` without a hash; no `builtins.fetchGit` of mutable refs.
- No network in `buildPhase` / `installPhase`.
- No `--impure` in derivations, wrapping scripts, or the justfile.
- `CARGO_TARGET_DIR` points outside the source tree.
