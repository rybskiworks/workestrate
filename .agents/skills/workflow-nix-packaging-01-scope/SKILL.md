---
name: workflow-nix-packaging-01-scope
description: |
  Use only for the scope phase of the Nix packaging workflow. Identify the
  software to package — its language/ecosystem, dependencies, build system,
  and source location. Do not use for design, implementation, testing,
  verification, or reviewing a diff.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-packaging
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (Nix packaging)

## Phase purpose

Identify the software to package, its language/ecosystem, its dependencies,
its build system, and its source location. This phase produces the packaging
contract every later phase is checked against.

## Steps to perform

1. Capture the intended package: name, version, upstream source (URL/repo),
   language/ecosystem (Node/npm, Python/pip, Go, Bun, Rust, C/C++, other), and
   whether it is a flake input (remote) or local source. Write this down first.
2. Read `docs/nix/packaging-recipes.md` — callPackage convention,
   buildNpmPackage, buildPythonApplication, buildGoModule, bun compile,
   `pip install --target`, FOD hashing, override/overrideAttrs.
3. Read `docs/nix/derivations-and-builds.md` — mkDerivation, build phases,
   phase hooks, dependency attributes (`nativeBuildInputs` vs `buildInputs`),
   source fetchers, FODs.
4. Read `docs/nix/flake-anatomy.md` — `flake.nix` structure, inputs, outputs,
   system keying, how packages are exposed.
5. Read `docs/nix-purity.md` — purity rules, `just lint-nix`, source filters,
   store-growth model.
6. Inspect the existing flake: read `flake.nix` and `nix/packages/` to see the
   established packaging patterns (e.g. `pi.nix`, `tempest.nix`,
   `odysseus.nix`, `opencode.nix`, `agentctl.nix`). Match the repo's
   conventions.
7. Record a one-paragraph summary of the packaging surface: package
   name/version, source (flake input vs local path), language/ecosystem,
   build system, dependency-fetch strategy (FOD hash type), target flake
   output attribute, and whether it needs a devshell addition. This summary is
   the scope contract.

## Docs to consult

- `docs/nix/packaging-recipes.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix/flake-anatomy.md`
- `docs/nix-purity.md`

## Operational skills to load

None are mandatory in the scope phase; skills load in the design and implement
phases. Optionally load `nix-packaging-recipes` if the package's build system
is non-trivial and would benefit from early builder-selection guidance.

## Constraints to apply

- Stay within the captured requirements. Do not fold in related cleanups,
  unrelated refactors, or speculative generalization. If a related cleanup
  appears, record it as a follow-up.

## Validations to run

None — validations run in phase 05 (workflow-nix-packaging-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-packaging-00-orchestration`. Set:

- `outcome` to `pass` once the packaging surface summary and scope contract
  are recorded.
- `constraints_applied` to include the scope-discipline principle.
- `next_phase: 02-design`.
- `blockers: []` unless something prevents proceeding.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - stay within captured requirements (scope discipline)
assumptions:
  - ...
risks:
  - ...
tests_run: []
tests_needed:
  - ...
next_phase: 02-design
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
