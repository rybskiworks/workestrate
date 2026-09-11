---
name: workflow-nix-debugging-04-regression
description: |
  Use only for the regression phase of the Nix debugging workflow.
  Add a check that would have caught the defect. The check must fail
  before the fix and pass after. Do not use for reproduction,
  diagnosis, fixing, or final verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-debugging
  org.phase: regression
  org.phase_order: "04"
---

## Phase purpose

Add a check that would have caught the defect. The check must encode the reproduction from phase `01-reproduce` so the same defect cannot silently return.

## Steps to perform

1. Load `nix-testing` skill.
2. Read `docs/nix/testing.md`.
3. Add a check that fails before the fix and passes after. The form depends on the defect category:
   - **Eval error** → add a `checks.<system>.<name>` derivation (e.g. `runCommand` or `nixosTest`) that evaluates the previously-broken attribute and asserts it succeeds; or add a `nix eval .#<attr>` invocation to a CI script.
   - **Build failure** → add a `checks.<system>.<name>` derivation that builds the previously-failing output and asserts success.
   - **Hash mismatch** → add a check that verifies the FOD hash matches (the `nix build` itself is the check; ensure the FOD is in `checks` or `packages`).
   - **Purity violation** → the `just lint-nix` guard is the regression check; if the violation was not caught by an existing check, add a case to `scripts/check-nix-paths.sh` or a new `runCommand` check that fails on the violating pattern.
   - **Store issue** → add or strengthen a `just store-audit` threshold or a `runCommand` check that fails on stale roots or excessive store growth.
4. The check MUST encode the reproduction from phase `01-reproduce` (same command, same trigger, same assertion that the defect violated).
5. Confirm the check fails before the fix and passes after. Record both observations (command + result) in `evidence`.
6. Apply `constraint-nix-scope-discipline`: keep the regression check scoped to the defect; do not build a broad test suite expansion as part of a bugfix.

## Docs to consult

- `docs/nix/testing.md`.

## Operational skills to load

- `nix-testing`.

## Constraints to apply

- `constraint-nix-scope-discipline` — Keep the regression check scoped to the defect; do not build a broad test suite expansion as part of a bugfix; do not refactor surrounding code; record adjacent issues as follow-ups.

## Validations to run

- `validation-nix-test` — verify the new regression check passes (run `nix flake check` or the specific check derivation to confirm the regression check passes after the fix).

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md). Set `next_phase: 05-verify`, `next_workflow: null`. Record the regression check location (flake output name or script) and its assertion in `tests_run` and `evidence`. Record the fails-before/passes-after confirmation in `evidence`.
