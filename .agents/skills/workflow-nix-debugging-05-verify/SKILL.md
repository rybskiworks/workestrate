---
name: workflow-nix-debugging-05-verify
description: |
  Use only for the verify phase of the Nix debugging workflow.
  Run the full check suite (`nix flake check`, `nix build`, `just
  verify`) and report the root cause, fix, and regression check. Do
  not use for reproduction, diagnosis, fixing, or regression testing.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-debugging
  org.phase: verify
  org.phase_order: "05"
---

## Phase purpose

Run the full check suite and report the root cause, fix, and regression check. This is the final phase.

## Steps to perform

1. Run gates in this exact order; all must pass: `nix flake check --no-build` (eval validation); `nix build .#<name>` (the previously-failing output builds); `just verify` (full project verification including `lint-nix` and `store-audit`).
2. If the fix touched a FOD or flake input, also run `nix flake update --dry-run` to confirm no unintended input changes (do NOT actually run `nix flake update` unless that was the fix).
3. Run validation skills: `validation-nix-flake-check`; `validation-nix-build`; `validation-nix-lint`.
4. Report all of the following (aggregate from prior phases):
   - the reproduction command and the original error output (from phase `01-reproduce`);
   - the defect category (from phase `02-diagnose`);
   - the root cause: a one-paragraph explanation of why the defect occurred;
   - the fix: what changed and why it addresses the root cause (from phase `03-fix`);
   - the regression check: location, assertion, and fails-before/passes-after confirmation (from phase `04-regression`);
   - pass/fail for each gate above.

## Docs to consult

None new. Findings reference docs cited in earlier phases (`docs/nix/error-handling-and-debugging.md`, `docs/nix-purity.md`, `docs/nix/store-hygiene-and-gc.md`, `docs/nix/language-fundamentals.md`, `docs/nix/derivations-and-builds.md`, `docs/nix/testing.md`).

## Operational skills to load

None new.

## Constraints to apply

- `constraint-nix-scope-discipline` — Verify only the reported defect's fix; do not make additional changes to make a gate pass; do not refactor surrounding code; record adjacent issues as follow-ups.

## Validations to run

- `validation-nix-flake-check` (`nix flake check --no-build`);
- `validation-nix-build` (`nix build .#<name>`);
- `validation-nix-lint` (`just lint-nix`).

If a FOD or flake input was touched, also run `nix flake update --dry-run` as a command to confirm no unintended input drift.

## Handoff output

Return the handoff YAML schema (see orchestration SKILL.md), including the verification-phase extra fields:
- `validations_run` — list of validation skills executed (`validation-nix-flake-check`, `validation-nix-build`, `validation-nix-lint`);
- `constraints_checked` — list of constraint skills audited against the diff;
- `evidence` — reproduction command, original error, category, root cause, fix, regression check location/assertion, per-gate pass/fail;
- `failures` — list of gates that failed (empty if all passed);
- `not_fully_checkable` — list of aspects that could not be fully validated and why (e.g., `nix build` requires a Nix-capable host; `nix flake check` requires nix on the host).

Set `next_phase: null`, `next_workflow: null`. Unless the fix revealed a need for new flake outputs or derivation structure, in which case set `next_workflow: workflow-nix-implementation` (the debugging workflow is complete; the implementation work is a separate workflow).
