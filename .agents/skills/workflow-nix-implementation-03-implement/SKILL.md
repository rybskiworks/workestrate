---
name: workflow-nix-implementation-03-implement
description: |
  Use only for the implement phase of the Nix implementation workflow. Write
  Nix code following purity rules and the design decided in phase 02. Do not
  use for scoping, design, test-only work, or final verification.
allowed-tools: Read Write Edit Bash(nix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-implementation
  org.phase: implement
  org.phase_order: "03"
---

# Phase 03: implement (Nix implementation)

## Phase purpose

Write the Nix code following purity rules and the derivation/flake-output design
decided in phase 02.

## Steps to perform

1. Load `nix-language` — idiomatic Nix syntax, `let`/`in`, `with`, attribute
   sets, lambdas, `lib` helpers.
2. Load `nix-derivations` — `mkDerivation`, build phases, dependency
   attributes, `strictDeps`.
3. Implement the derivation applying the design from phase 02:
   - Source filter: `builtins.path { name = ...; path = ./subdir; filter = ...; }`
     with an explicit `name` and `filter`. Never bare `src = ./.` or
     `toString ./.`.
   - Dependency attributes: `nativeBuildInputs` for build-time tools,
     `buildInputs` for runtime libs; set `strictDeps = true`.
   - FODs: every `fetchFromGitHub`/`fetchPypi`/`buildNpmPackage` carries an
     `outputHash`/`npmDepsHash`/`cargoHash`. No `fetchTarball` without a hash.
4. Implement the flake output wiring in `flake.nix` (or a `nix/<name>.nix`
   module imported by `flake.nix`) per the output-class decision from phase 02.
   Key the output to the correct system; do not invent a `packages.default`
   unless the requirements demand it.
5. Apply the error strategy decided in phase 02: `throw`/`assert` for genuine
   misconfiguration, `lib.warn` for non-fatal deprecations, never `abort` for
   expected conditions, never `|| true` in `checkPhase`/`installPhase`.
6. If any impure construct is tempting (`--impure`, `builtins.getFlake`,
   `toString ./.`), apply `constraint-nix-purity`: stop, use a native `.#` ref
   or a filtered `builtins.path` instead, and flag the temptation in the
   handoff risks.
7. Stage new files (`git add -N <file>`) so flake evaluation can see them —
   untracked files are invisible to `.#` refs (classic flakes gotcha).

## Docs to consult

- `docs/nix/derivations-and-builds.md`
- `docs/nix-purity.md`
- `docs/nix/flake-anatomy.md`

## Operational skills to load

- `nix-language`
- `nix-derivations`
- (conditional) `nix-modules` — only if a NixOS module or overlay is being
  written.
- (conditional) `nix-devshells` — only if a devShell is being written.

## Constraints to apply

- `constraint-nix-purity` — enforce eval-time and build-time purity: filtered
  `builtins.path`/`cleanSourceWith` with a `filter`, FODs with `outputHash`,
  no `--impure`, no `getFlake (toString ./.)`, no bare `src = ./.`, no network
  in `buildPhase`/`installPhase`.
- `constraint-nix-reproducibility` — all flake inputs pinned via `flake.lock`;
  no mutable fetches; deterministic build phases.
- `constraint-nix-sandbox-safety` — applies if the derivation does anything
  non-hermetic in the sandbox: justify it, or move the work to a FOD.
- `constraint-nix-scope-discipline` — implement only the requirements captured
  in phase 01; record related cleanups as follow-ups rather than folding them
  in.

## Validations to run

None — validations run in phase 05 (workflow-nix-implementation-05-verify).
A fast `nix eval .#<attr>.drvPath` sanity check is permitted but not required;
the full gate suite runs in phase 05.

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-implementation-00-orchestration`. Set:

- `outcome` to `pass` once the code is written and evaluates conceptually
  against the purity rules and the phase-02 design.
- `constraints_applied` to include every constraint listed above that applied
  (include `constraint-nix-sandbox-safety` only if a non-hermetic sandbox
  action was introduced).
- `next_phase: 04-test`.
- `blockers: []` unless an impurity temptation or FOD hash trust issue needs
  human review — in that case set `handoff_requires_hil: true` and record the
  reason.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-nix-purity
  - constraint-nix-reproducibility
  - constraint-nix-scope-discipline
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
