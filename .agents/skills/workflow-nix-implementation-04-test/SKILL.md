---
name: workflow-nix-implementation-04-test
description: |
  Use only for the test phase of the Nix implementation workflow. Write and run
  tests — checkPhase, nixosTests, nix flake check. Do not use for scoping,
  design, implementation, or final verification.
allowed-tools: Read Write Edit Bash(nix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-implementation
  org.phase: test
  org.phase_order: "04"
---

# Phase 04: test (Nix implementation)

## Phase purpose

Write tests alongside the code and run `nix flake check` to confirm the
implementation behaves as scoped in phase 01.

## Steps to perform

1. Load `nix-testing` — `nix flake check`, `checks` output, `checkPhase`/
   `doCheck`, `nixosTests`, `testers`, `runCommand` checks.
2. Read `docs/nix/testing.md` — the canonical testing reference.
3. Write a `checks.<system>.<name>` derivation for the new code:
   - For a package: a `runCommand` or `testers.runNixOSTest` that builds the
     package and asserts the binary/version/output exists.
   - For a derivation with a test suite: set `doCheck = true` and a
     `checkPhase` that runs the suite; the `checks` output builds it.
   - For a NixOS module: a `nixosTests` entry via `testers.runNixOSTest`.
4. Cover: happy path (build succeeds, expected output present), each error
   branch (missing hash fails loudly, impure path rejected), and edge cases
   (empty input, wrong system key, FOD hash mismatch surfaces the expected vs
   actual hash).
5. Run `nix flake check --no-build` first (eval-only, fast) to catch
   attribute/eval errors; then `nix build .#checks.x86_64-linux.<name>` to run
   the check derivation.
6. Fix failures at the root cause before proceeding. Do not suppress failures
   with `|| true`, `doCheck = false`, or by deleting checks.

## Docs to consult

- `docs/nix/testing.md`

## Operational skills to load

- `nix-testing`

## Constraints to apply

- `constraint-nix-scope-discipline` — test the scoped behavior, not unrelated
  outputs the new code touches only at a call site. Do not add checks for
  speculative future outputs.
- `constraint-nix-purity` — checks must themselves be pure: no `--impure`, no
  network in the check derivation's `buildPhase`.

## Validations to run

- `validation-nix-test` — runs `nix flake check` (and/or
  `nix build .#checks.x86_64-linux.<name>`) and records structured pass/fail
  evidence for the check suite.

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-implementation-00-orchestration`. Set:

- `outcome` to `pass` once `nix flake check` passes and the check set covers
  happy path, error branches, and edge cases.
- `constraints_applied` to include `constraint-nix-scope-discipline` and
  `constraint-nix-purity`.
- `tests_run` to list every check added with what each covers.
- `next_phase: 05-verify`.
- `blockers: []` unless a check reveals a defect that needs phase 03 to revisit
  — in that case set `outcome: fail` and record the failing check.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-nix-scope-discipline
  - constraint-nix-purity
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
