---
name: workflow-nix-implementation-02-design
description: |
  Use only for the design phase of the Nix implementation workflow. Make module
  structure, derivation design, flake output design, and error strategy
  decisions up front. Do not use for scoping, implementation, testing, or final
  verification.
allowed-tools: Read Write Edit Bash(nix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-implementation
  org.phase: design
  org.phase_order: "02"
---

# Phase 02: design (Nix implementation)

## Phase purpose

Make module structure, derivation design, flake output design, and error
strategy decisions up front so the implement phase can proceed without
mid-flight architectural changes.

## Steps to perform

1. Load `nix-flake-anatomy` — flake inputs, outputs, system keying, output
   class selection (`packages`/`checks`/`devShells`/`overlays`/`nixosModules`).
2. Load `nix-derivations` — `mkDerivation`, build phases, dependency attributes
   (`nativeBuildInputs` vs `buildInputs`), `strictDeps`, FODs.
3. Load `nix-modules` if the work introduces a NixOS module or an overlay.
4. Read `docs/nix/flake-anatomy.md` for output-class and system-keying
   conventions.
5. Read `docs/nix/derivations-and-builds.md` for `mkDerivation` phase order and
   dependency-attribute selection.
6. Read `docs/nix-purity.md` for the source-filter and FOD rules that the
   design must respect.
7. Decide the module file layout up front:
   - New derivation under `nix/<name>.nix` vs inline in `flake.nix`.
   - Whether a `nix/` subdirectory module is needed (`nix/packages/`,
     `nix/checks/`, `nix/devshells/`).
   - Apply the chosen layout consistently across the implementation.
8. Decide the derivation design:
   - `mkDerivation` vs `stdenv.mkDerivation` vs a FOD (`outputHash`).
   - `nativeBuildInputs` (build-time tools) vs `buildInputs` (runtime libs);
     set `strictDeps = true` unless there is a documented reason not to.
   - Source filter: `builtins.path { name = ...; path = ...; filter = ...; }`
     with an explicit `name` and `filter` — never bare `src = ./.`.
   - `CARGO_TARGET_DIR` relocation if the derivation invokes cargo in-tree.
9. Decide the flake output design:
   - Which output class and which system key (`x86_64-linux` only, or
     `flake-utils.lib.eachDefaultSystem`).
   - Attribute name (there is NO `packages.default` in this project; do not
     invent one unless the requirements demand it).
   - Whether a `checks.<system>` entry is needed for the test phase.
10. Decide the error strategy:
    - Eval-time errors: use `throw`/`assert` for genuine misconfiguration
      (missing input, wrong system); use `lib.warn` only for non-fatal
      deprecations. Never `abort` for expected conditions.
    - Build-time failures: surface via `checkPhase`/`installPhase` exit codes;
      do not swallow with `|| true`.
    - FOD hash mismatches: fail loudly with the expected vs actual hash; do
      not silently update the hash without confirming the upstream is trusted.
11. Conditionally load `nix-cross-compilation` if the work targets multiple
    systems or uses `pkgsCross`/`pkgsStatic`.

## Docs to consult

- `docs/nix/flake-anatomy.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix-purity.md`
- `docs/nix/devshells.md` (if a devShell is being added)

## Operational skills to load

- `nix-flake-anatomy`
- `nix-derivations`
- (conditional) `nix-modules` — only if a NixOS module or overlay is introduced.
- (conditional) `nix-devshells` — only if a devShell is being added.
- (conditional) `nix-cross-compilation` — only if multi-system/cross is in scope.

## Constraints to apply

- `constraint-nix-scope-discipline` — design only what the captured
  requirements demand. Do not introduce speculative output classes, a
  `default` package, a `nix fmt` formatter, or extra flake inputs beyond what
  the new code requires.
- `constraint-nix-purity` — the design must use filtered `builtins.path` /
  `cleanSourceWith` with a `filter`, FODs with `outputHash` for all fetching,
  and no `--impure` / no `getFlake (toString ./.)`. Decide the source filter
  predicate up front so the implement phase does not reach for bare `src = ./.`.
- `constraint-nix-reproducibility` — all flake inputs must be pinned via
  `flake.lock`; no `fetchTarball` without a hash; no mutable `builtins.fetchGit`
  refs.

## Validations to run

None — validations run in phase 05 (workflow-nix-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-implementation-00-orchestration`. Set:

- `outcome` to `pass` once module layout, derivation design, flake output
  design, source filter, and error strategy are decided and recorded.
- `constraints_applied` to include `constraint-nix-scope-discipline`,
  `constraint-nix-purity`, and `constraint-nix-reproducibility`.
- `next_phase: 03-implement`.
- `blockers: []` unless a policy decision (e.g. FOD hash trust, new flake
  input) needs human input — in that case set `handoff_requires_hil: true` and
  record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-nix-scope-discipline
  - constraint-nix-purity
  - constraint-nix-reproducibility
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
