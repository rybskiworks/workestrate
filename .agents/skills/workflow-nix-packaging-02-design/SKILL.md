---
name: workflow-nix-packaging-02-design
description: |
  Use only for the design phase of the Nix packaging workflow. Choose the
  right builder, design the derivation, and plan the FOD strategy. Do not use
  for scoping, implementation, testing, or final verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-packaging
  org.phase: design
  org.phase_order: "02"
---

# Phase 02: design (Nix packaging)

## Phase purpose

Choose the right builder, design the derivation, and plan the FOD strategy so
the implement phase can proceed without mid-flight architectural changes.

## Steps to perform

1. Load `nix-packaging-recipes` — callPackage, buildNpmPackage,
   buildPythonApplication, buildGoModule, bun compile, `pip install --target`,
   FOD hashing, override/overrideAttrs.
2. Load `nix-derivations` — mkDerivation, build phases, phase hooks,
   dependency attributes, source fetchers, FODs.
3. Load `nix-flake-anatomy` — how to wire a new package into
   `packages.<system>`.
4. Read `docs/nix/packaging-recipes.md` and `docs/nix/derivations-and-builds.md`.
5. Decide the builder based on ecosystem:
   - Node/npm → `buildNpmPackage` (needs `npmDepsHash`; compute via
     `nix run nixpkgs#prefetch-npm-deps -- <package-lock.json>`).
   - Python → `buildPythonApplication` (`pyproject = true`) or
     `pip install --target` two-stage FOD for deps not in nixpkgs.
   - Go → `buildGoModule` (needs `vendorHash`; use `lib.fakeHash` first, copy
     `got:` from failure).
   - Bun → `bun build --compile` wrapped in `stdenv.mkDerivation`; use
     `removeReferencesTo` to strip source-tree store refs.
   - Rust → `buildRustPackage` or the repo's `agentctl.nix` pattern with fenix
     toolchain.
   - C/C++/other → `stdenv.mkDerivation` with explicit phases.
6. Decide the source strategy:
   - Remote → add a flake input (`inputs.<name>.url = ...; flake = false;` for
     non-flake repos) and pass to callPackage.
   - Local → use `builtins.path` with `name` + `filter`, or `cleanSourceWith`
     with a `filter` predicate excluding `target/`, `result*`, `node_modules/`.
     NEVER bare `src = ./.`.
7. Decide the FOD strategy: which hash attribute (`npmDepsHash`, `vendorHash`,
   `outputHash`/`bunDeps.outputHash`, `pipDeps.outputHash`,
   `cargoHash`/`cargoLock`). Plan to set `lib.fakeHash`/`lib.fakeSha256` first,
   then compute the real hash via `just update-hashes` or the per-builder
   prefetch command. Never leave `lib.fakeHash` in committed code.
8. Decide dependency placement: build-time tools in `nativeBuildInputs`,
   runtime libraries in `buildInputs`. Set `strictDeps = true` for any package
   that may be cross-compiled.
9. Decide the install layout: app-style (override `installPhase` to reproduce
   runtime tree at `$out`) vs library-style (default). For apps,
   `buildNpmPackage` default is library-style
   (`$out/lib/node_modules/$name/`) — apps expect `dist/` + `node_modules/` +
   `package.json` at root.
10. Decide the flake output attribute name and where to wire it in `flake.nix`
    `packages.${system}` (follow the existing `pi`, `tempest`,
    `odysseus-built`, `opencode-built` pattern).
11. Conditionally load `nix-devshells` if the package must also be available in
    the dev shell (`nativeBuildInputs`/`buildInputs` in
    `nix/devshells/default.nix`).
12. Conditionally load `nix-docker-images` if the package is destined for a
    `dockerTools` image (e.g. `pi-image.nix`, `tempest-image.nix`).

## Docs to consult

- `docs/nix/packaging-recipes.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix/flake-anatomy.md`
- `docs/nix/devshells.md` (if devshell addition needed)
- `docs/nix-purity.md`

## Operational skills to load

- `nix-packaging-recipes`
- `nix-derivations`
- `nix-flake-anatomy`
- (conditional) `nix-devshells` — only if the package must be added to the
  dev shell.
- (conditional) `nix-docker-images` — only if the package feeds a dockerTools
  image.

## Constraints to apply

- Stay within the captured requirements. Design only what the captured
  requirements demand. Do not introduce speculative abstractions, new
  dependencies, or module boundaries beyond what the new code requires.
- `constraint-nix-purity` — no `src = ./.` without filter, no `--impure`, no
  `builtins.getFlake`+`toString`, filtered `builtins.path`/`cleanSourceWith`.
- `constraint-nix-reproducibility` — flake input pinning, FOD hashes declared,
  no mutable refs, no `lib.fakeHash` in committed code.
- `constraint-nix-sandbox-safety` — no network in buildPhase/installPhase, all
  deps via FODs, HOME set to TMPDIR if build.rs writes outside sandbox.
- `constraint-nix-scope-discipline` — no `with` in large scopes, no `rec` when
  `let` suffices, no `<nixpkgs>` channel refs, explicit `lib.` prefixes.

## Validations to run

None — validations run in phase 05 (workflow-nix-packaging-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-packaging-00-orchestration`. Set:

- `outcome` to `pass` once builder, source strategy, FOD strategy, dependency
  placement, install layout, and flake output wiring are decided and recorded.
- `constraints_applied` to include the scope-discipline principle,
  `constraint-nix-purity`, `constraint-nix-reproducibility`,
  `constraint-nix-sandbox-safety`, and `constraint-nix-scope-discipline`.
- `next_phase: 03-implement`.
- `blockers: []` unless a policy decision (e.g. new flake input, new external
  dependency, secret handling) needs human input — in that case set
  `handoff_requires_hil: true` and record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - stay within captured requirements (scope discipline)
  - constraint-nix-purity
  - constraint-nix-reproducibility
  - constraint-nix-sandbox-safety
  - constraint-nix-scope-discipline
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 03-implement
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
