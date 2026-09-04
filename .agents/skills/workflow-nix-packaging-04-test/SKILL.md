---
name: workflow-nix-packaging-04-test
description: |
  Use only for the test phase of the Nix packaging workflow. Build the
  package, verify its output, and test it in the devshell. Do not use for
  scoping, design, implementation, or final verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-packaging
  org.phase: test
  org.phase_order: "04"
---

# Phase 04: test (Nix packaging)

## Phase purpose

Build the package, verify its output, and test it in the devshell. The Nix
evaluator cannot verify runtime correctness of the packaged binary, so build
and output checks are required.

## Steps to perform

1. Load `nix-packaging-recipes` (build/verify procedures) and `nix-testing`
   (test patterns).
2. Read `docs/nix/testing.md` — `nix flake check`, `checks` output,
   `checkPhase`/`doCheck`, `nixosTests`, `testers`, `runCommand`.
3. Build the package: `nix build .#<name>` (creates `./result` symlink). Use
   `--no-link --print-out-paths` to avoid leaving a GC-root symlink
   (store-hygiene).
4. Verify the output:
   - Inspect `./result` (or the printed store path): expected binaries in
     `bin/`, expected library/runtime tree layout.
   - For a binary: `./result/bin/<binary> --version` (or `--help`) to confirm
     it runs.
   - For a runtime tree (app-style): confirm `dist/`, `node_modules/`,
     `package.json` (or `.deps/` for Python) are present at the expected
     paths.
5. Test in devshell: if the package was added to `nix/devshells/default.nix`,
   run `just shell -c <tool> --version` to confirm it is on PATH.
6. If the derivation has a `checkPhase` (test suite), ensure `doCheck = true`
   and run `nix build .#<name>` (check runs during build) or add a
   `checks.${system}.<name>` entry.
7. Fix failures at the root cause before proceeding. Do not suppress failures
   by setting `doCheck = false` or deleting tests. Common failures: wrong FOD
   hash (recompute), missing `nativeBuildInputs` (e.g. `unzip` for zip src,
   `autoPatchelfHook` for prebuilt native binaries), network in build phase
   (move to FOD), `--ignore-scripts` missing for npm lifecycle hooks.
8. Clean up: remove `result*` symlinks when done (they pin closures forever).

## Docs to consult

- `docs/nix/testing.md`
- `docs/nix/packaging-recipes.md` (review checklist, common mistakes)
- `docs/nix-purity.md`

## Operational skills to load

- `nix-packaging-recipes`
- `nix-testing`

## Constraints to apply

- Stay within the captured requirements. Test the scoped package, not
  unrelated outputs. Do not add tests for speculative future behavior.
- `constraint-nix-store-hygiene` — use `--no-link --print-out-paths`, remove
  `result*` symlinks, no stale GC roots.

## Validations to run

- `validation-nix-build` — runs `nix build .#<name>` and records structured
  pass/fail evidence.

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-packaging-00-orchestration`. Set:

- `outcome` to `pass` once `nix build .#<name>` succeeds, output verified, and
  devshell integration confirmed (if applicable).
- `constraints_applied` to include the scope-discipline principle and
  `constraint-nix-store-hygiene`.
- `tests_run` to list the build + output checks performed, including what each
  verified.
- `next_phase: 05-verify`.
- `blockers: []` unless a build failure needs phase 03 to revisit — in that
  case set `outcome: fail` and record the failing step.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - stay within captured requirements (scope discipline)
  - constraint-nix-store-hygiene
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: ...
    covers: ...
tests_needed:
  - ...
next_phase: 05-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
```
