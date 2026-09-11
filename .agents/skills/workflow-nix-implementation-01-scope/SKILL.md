---
name: workflow-nix-implementation-01-scope
description: |
  Use only for the scope phase of the Nix implementation workflow. Understand
  requirements, read topic docs, and record the intended behavior and flake
  output surface. Do not use for design, implementation, testing, verification,
  or for reviewing a diff.
allowed-tools: Read Write Edit Bash(nix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-implementation
  org.phase: scope
  org.phase_order: "01"
---

# Phase 01: scope (Nix implementation)

## Phase purpose

Understand requirements, read topic docs, and record the intended behavior and
flake output surface. This phase produces the one-paragraph summary that every
later phase is checked against.

## Steps to perform

1. Capture the intended behavior, inputs (flake inputs, nixpkgs), outputs
   (which flake output class: `packages`/`checks`/`devShells`/`overlays`/
   `nixosModules`/`nixosConfigurations`), error cases (missing hash, impure
   path, eval failure), and any public surface implications of the work. Write
   this down before reading any docs.
2. Read `docs/nix/flake-anatomy.md` — flake inputs, outputs, system keying,
   `checks`/`packages`/`devShells` output classes.
3. Read `docs/nix/derivations-and-builds.md` — `mkDerivation`, build phases,
   dependency attributes (`nativeBuildInputs` vs `buildInputs`), `strictDeps`.
4. Read `docs/nix-purity.md` — purity rules, `just lint-nix`, source filters,
   store-growth model. This is the cautionary-tale doc; scope must respect it.
5. Record a one-paragraph summary of the intended behavior and the flake
   output surface (which output class, which system, which attribute name).
   This summary is the scope contract for the rest of the workflow.

## Docs to consult

- `docs/nix/flake-anatomy.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix-purity.md`

## Operational skills to load

None are mandatory in the scope phase; skills load in the design and implement
phases. Optionally load `nix-flake-anatomy` if the output surface is
non-trivial and would benefit from early output-class guidance.

## Constraints to apply

- `constraint-nix-scope-discipline` — stay within the captured requirements.
  Do not fold in related cleanups, unrelated refactors, or speculative
  generalization (e.g. adding a `default` package, a `nix fmt` formatter, or a
  cross-compilation output the requirements did not ask for). If a related
  cleanup appears, record it as a follow-up.

## Validations to run

None — validations run in phase 05 (workflow-nix-implementation-05-verify).

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-implementation-00-orchestration`. Set:

- `outcome` to `pass` once the one-paragraph summary and flake output surface
  are recorded.
- `constraints_applied` to include `constraint-nix-scope-discipline`.
- `next_phase: 02-design`.
- `blockers: []` unless something prevents proceeding.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-nix-scope-discipline
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
