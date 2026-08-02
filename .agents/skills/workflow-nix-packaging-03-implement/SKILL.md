---
name: workflow-nix-packaging-03-implement
description: |
  Use only for the implement phase of the Nix packaging workflow. Write the
  derivation in `nix/packages/<name>.nix`, wire it into flake outputs, and
  compute FOD hashes. Do not use for scoping, design, test-only work, or final
  verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-packaging
  org.phase: implement
  org.phase_order: "03"
---

# Phase 03: implement (Nix packaging)

## Phase purpose

Write the derivation in `nix/packages/<name>.nix`, wire it into flake outputs,
and compute FOD hashes, following the design decided in phase 02.

## Steps to perform

1. Load `nix-packaging-recipes`, `nix-derivations`, `nix-flake-anatomy`.
2. Write the derivation file at `nix/packages/<name>.nix` following the design
   from phase 02:
   - Function signature taking an attrset of dependencies (callPackage
     convention): `{ lib, stdenv, ... }:` (or the builder-specific args like
     `{ buildNpmPackage, nodejs, ... }:`).
   - Set `pname` + `version` (NOT bare `name` — RFC 0035).
   - Set `src` per the source strategy (flake input passed as arg, or filtered
     `builtins.path`/`cleanSourceWith`).
   - Set the FOD hash attribute to `lib.fakeHash`/`lib.fakeSha256` initially
     (to be computed next).
   - Set `nativeBuildInputs` (build-time tools) and `buildInputs` (runtime
     libs) per the dependency placement decision. Set `strictDeps = true` if
     cross-compilation is possible.
   - Override `installPhase` for app-style packages (reproduce runtime tree at
     `$out`). Include `runHook preInstall`/`runHook postInstall`.
   - Set `meta.platforms` (and `meta.mainProgram` if there is a primary
     binary).
3. Wire the package into `flake.nix`:
   - If remote source: add `inputs.<name> = { url = ...; flake = false; };` to
     `inputs`, add the input name to the
     `outputs = { self, nixpkgs, ..., <name>, ... }:` destructure, and
     `pkgs.callPackage ./nix/packages/<name>.nix { <name> = <name>; };` in the
     `let` bindings.
   - Add the package to `packages.${system}` attrset (follow the existing
     pattern, e.g. `pi = pi-built;`).
4. Stage new files: `git add -N nix/packages/<name>.nix` (and `git add`
   flake.nix changes) — untracked files are invisible to `.#` refs (classic
   flakes gotcha).
5. Compute FOD hashes:
   - Run `just update-hashes` if the package matches the script's known set, OR
     run the per-builder prefetch command:
     - npm: `nix run nixpkgs#prefetch-npm-deps -- <package-lock.json>` → inline
       `sha256-...` into `npmDepsHash`.
     - Go: build once with `lib.fakeHash`, copy `got:` `sha256-...` into
       `vendorHash`.
     - Bun/pip: build once with `lib.fakeHash`, copy `got:` `sha256-...` into
       `bunDeps.outputHash`/`pipDeps.outputHash`.
   - Inline the computed `sha256-...` value into the derivation. NEVER leave
     `lib.fakeHash` in committed code.
6. Verify the derivation evaluates: `nix eval .#<name>.drvPath` (succeeds even
   with fakeHash — confirms drv instantiation; does NOT confirm the build).
7. If the package must be in the dev shell, add it to
   `nix/devshells/default.nix` `nativeBuildInputs`/`buildInputs` and pass it
   through the devshell call.

## Docs to consult

- `docs/nix/packaging-recipes.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix/flake-anatomy.md`
- `docs/nix-purity.md`

## Operational skills to load

- `nix-packaging-recipes`
- `nix-derivations`
- `nix-flake-anatomy`
- (conditional) `nix-devshells` (if devshell addition needed)

## Constraints to apply

- `constraint-nix-purity` — filtered sources only, no `src = ./.`, no
  `--impure`.
- `constraint-nix-reproducibility` — real FOD hashes (no `lib.fakeHash`
  committed), flake input pinned with a ref.
- `constraint-nix-sandbox-safety` — no network in build/install phases, deps
  via FODs, HOME=$TMPDIR if needed.
- `constraint-nix-scope-discipline` — no `with`/`rec`/`<nixpkgs>`, explicit
  `lib.`.
- `constraint-nix-secret-hygiene` — never embed secrets in derivations; use
  SOPS/age, resolve at runtime.
- Stay within the captured requirements. Implement only the requirements
  captured in phase 01; record related cleanups as follow-ups rather than
  folding them in.

## Validations to run

None — validations run in phase 05 (workflow-nix-packaging-05-verify). (Note:
`nix eval .#<name>.drvPath` is a sanity check, not a validation gate.)

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-packaging-00-orchestration`. Set:

- `outcome` to `pass` once the derivation is written, wired into flake
  outputs, files staged with `git add -N`, and FOD hashes computed and
  inlined.
- `constraints_applied` to include every constraint listed above that applied.
- `next_phase: 04-test`.
- `blockers: []` unless a hash cannot be computed (e.g. no nix on host) — in
  that case set `handoff_requires_hil: true` and record the reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-nix-purity
  - constraint-nix-reproducibility
  - constraint-nix-sandbox-safety
  - constraint-nix-scope-discipline
  - constraint-nix-secret-hygiene
  - stay within captured requirements (scope discipline)
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 04-test
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
