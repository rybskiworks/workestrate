---
name: workflow-nix-code-review-02-analyze
description: |
  Use only for the analyze phase of the Nix code-review workflow. Categorize
  the diff by dimension and load the operational review skills that match. Do
  not use for scoping, running gates, manual review, or issuing a verdict.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-code-review
  org.phase: analyze
  org.phase_order: "02"
---

# Phase 02: analyze (Nix code review)

## Phase purpose

Categorize the diff by dimension and load the operational review skills that
match the categories, so phase 04-review has the right skills and docs ready.

## Steps to perform

1. Categorize the diff by dimension: does it touch purity/source-filters,
   reproducibility/pinning (flake.lock, FOD hashes), sandbox safety, expression
   scope (`with`/`rec`/`<nixpkgs>`), secret handling, store hygiene/GC roots,
   derivation build phases, devShells, modules/overlays, or the justfile's
   nix-adjacent recipes? Record the categories in `risks`.
2. Load `nix-flake-anatomy` — flake inputs, outputs, system keying,
   `checks`/`package`/`devShells` structure.
3. Load `nix-derivations` — `mkDerivation`, build phases, dependency
   attributes, `strictDeps`. This is **mandatory if a derivation is present**
   in the diff.
4. Load `constraint-nix-purity` — eval-time and build-time purity: source
   filters, `--impure`, `builtins.path`/`cleanSourceWith` without `filter =`,
       `builtins.getFlake` + `toString`. This is **mandatory if the diff touches
   source filters or path-copying code**.
5. Load `constraint-nix-reproducibility` — flake input pinning, fetcher hashes,
   FOD output verification, flake.lock discipline. This is **mandatory if the
   diff touches `flake.lock`, FODs, or fetcher hashes**.
6. Load `constraint-nix-sandbox-safety` — sandbox enabled, no network in
   buildPhase/installPhase, `__noChroot` justification, all deps declared,
   `HOME=$TMPDIR`. Load if the diff touches derivation build phases.
7. Load `constraint-nix-secret-hygiene` — no embedded secrets, SOPS/age,
   runtime resolution via `os.environ/`. Load if the diff touches secret
   references or service configuration.
8. Load `constraint-nix-store-hygiene` — GC root discipline, `result*`
   symlinks, `--no-link --print-out-paths`, `streamLayeredImage`. Load if the
   diff touches store-path-producing commands or image builds.
9. Conditionally load `nix-testing`, `nix-devshells`, `nix-modules`,
   `nix-overlays`, `nix-packaging-recipes` if the diff is large or touches
   those areas. When in doubt, load them.
10. Apply `constraint-nix-scope-discipline`: categorization covers only the
    diff; do not expand scope to untouched code.

## Docs to consult

- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix/flake-anatomy.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/supply-chain-security.md`

## Operational skills to load

- `nix-flake-anatomy`
- `nix-derivations` (if a derivation is present)
- `constraint-nix-purity` (if source filters or path-copying code present)
- `constraint-nix-reproducibility` (if flake.lock/FODs/fetcher hashes present)
- `constraint-nix-sandbox-safety` (if build phases present)
- `constraint-nix-secret-hygiene` (if secret references present)
- `constraint-nix-store-hygiene` (if store-path-producing commands present)
- `nix-testing` (conditional)
- `nix-devshells` (conditional)
- `nix-modules` (conditional)
- `nix-overlays` (conditional)
- `nix-packaging-recipes` (conditional)

## Constraints to apply

- `constraint-nix-scope-discipline` — categorization covers only the diff; do
  not expand scope to untouched code.

## Validations to run

None — validations run in phase 03 (workflow-nix-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-nix-code-review-00-orchestration`. Set:

- `outcome`: `pass` if categorization completed and skills loaded; `partial`
  if a conditional skill was deliberately not loaded (record why in
  `assumptions`).
- `constraints_applied`: `constraint-nix-scope-discipline`.
- `risks`: the dimensions the diff touches.
- `assumptions`: any conditional-load decisions and their rationale.
- `next_phase`: `03-check`.
- `next_workflow`: `null`.
