---
type: Workflow
resource: https://nix.dev/
title: Nix Refactoring Workflow
description: Step-by-step workflow for refactoring Nix code without behavior change — baseline, plan, execute, verify, confirm.
tags: [nix, workflow, refactoring]
timestamp: 2026-07-24T00:00:00Z
---

# Nix Refactoring Workflow

## Purpose

Process for restructuring existing Nix code without changing behavior. The
workflow enforces a passing-test baseline before any change, incremental steps
with verification after each, and a final no-regression check. The defining
constraint is: behavior must not change.

## When to use

Use this workflow when restructuring flake outputs, splitting modules,
simplifying derivation structure, replacing a pattern, extracting a helper, or
cleaning up dead code — all without intended behavior change. Do not use it
for adding features (use `implementation.md`), fixing a defect (use
`debugging.md`), or reviewing a diff (use `code-review.md`). If behavior must
change, switch to `implementation.md` and record the intended behavior change
explicitly; do not smuggle a behavior change through a refactoring pass.

## Order of operations

### 1. Understand current behavior

Before changing anything, read the code being refactored and the relevant
topic docs so the refactor is grounded in the corpus rules:

- `docs/nix/flake-anatomy.md` — flake schema, outputs, inputs.
- `docs/nix/derivations-and-builds.md` — derivation structure, phases.
- `docs/nix/modules-and-config.md` — module/option structure.
- `docs/nix/overlays.md` — overlay composition.
- `docs/nix/conventions-and-style.md` — naming, attribute paths.
- `docs/nix/language-fundamentals.md` — let, attrset, function idioms.

Record a one-paragraph description of the current behavior and the refactoring
goal (what structure is being improved and why), so later steps can verify the
goal was met without behavior change.

### 2. Ensure tests pass BEFORE refactoring (baseline)

Establish a green baseline. If tests do not pass before the refactor, stop and
fix the baseline first (use `debugging.md`) — a refactor on a red baseline
cannot be verified.

```sh
nix build .#<name>
just verify-full
```

If test coverage is thin in the area being refactored, add characterization
checks that pin the current behavior before refactoring. These checks are
part of the refactor, not separate work.

### 3. Load relevant skills based on refactoring area

Load skills matching the refactor's focus:

- `.agents/skills/nix-flake-anatomy/SKILL.md` — for flake output refactors.
- `.agents/skills/nix-derivations/SKILL.md` — for derivation structure
  refactors.
- `.agents/skills/nix-modules/SKILL.md` — for module/option refactors.
- `.agents/skills/nix-overlays/SKILL.md` — for overlay refactors.
- `.agents/skills/constraint-nix-purity/SKILL.md` — if purity invariants are
  touched.

### 4. Make changes incrementally

Refactor in small steps, each independently verifiable. One step might be:
extract a helper function, split a module, replace a hand-rolled builder with
a `buildXxxPackage`, or consolidate overlapping overlays. Commit or checkpoint
after each step so a bad step can be reverted without losing the whole
refactor.

### 5. Run checks after each step

After each step, run the gates in this order:

```sh
nix build .#<name>
just lint-nix
just verify-full
```

If any gate fails, revert the step and redo it. Do not proceed to the next
step on a red gate. `nix build` is the primary no-regression signal;
`just lint-nix` catches dead code and unused bindings introduced by the
refactor.

### 6. Verify no behavior change

After all steps are complete, run the full test suite again and confirm the
output matches the baseline from step 2 (same checks pass, same store paths,
no new failures). If characterization checks were added in step 2, confirm
they still pass unchanged. If any check had to change, that is a behavior
change — either justify it explicitly and switch to `implementation.md`, or
revert.

### 7. Check for dead code and unused bindings

```sh
just lint-nix
nix flake check
```

Linting catches dead code, unused `let` bindings, and unreachable attributes
introduced by the refactor. Remove anything the linter flags rather than
suppressing it, unless the repo convention policy explicitly allows the
suppression.

### 8. Report

Report what changed, why, and the evidence of no regression.

## Docs consulted

- `docs/nix/flake-anatomy.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix/modules-and-config.md`
- `docs/nix/overlays.md`
- `docs/nix/conventions-and-style.md`
- `docs/nix/language-fundamentals.md`

## Skills loaded

- `.agents/skills/nix-flake-anatomy/SKILL.md`
- `.agents/skills/nix-derivations/SKILL.md`
- (conditional) `.agents/skills/nix-modules/SKILL.md`
- (conditional) `.agents/skills/nix-overlays/SKILL.md`
- (conditional) `.agents/skills/constraint-nix-purity/SKILL.md`

## Commands run (in order)

Baseline:

```sh
nix build .#<name>
just verify-full
```

After each incremental step:

```sh
nix build .#<name>
just lint-nix
just verify-full
```

Final:

```sh
just lint-nix
nix flake check
```

## Evidence to report

- One-paragraph description of current behavior and the refactoring goal (from
  step 1).
- Baseline test result before refactoring (from step 2).
- List of incremental steps taken, with the gate results after each.
- Final test result confirming no regression (from step 6).
- Final lint and flake check result (from step 7).
- List of files changed.
- Explicit statement that no behavior change occurred, or a flagged behavior
  change with a handoff to `implementation.md`.

## When human judgment is needed

- Deciding whether a flake output change is a refactor (no behavior change) or
  a behavior change requiring `implementation.md`.
- Choosing between two valid patterns from `docs/nix/conventions-and-style.md`
  when the tradeoff is not clear from the skill guidance.
- Deciding whether to add characterization checks before refactoring when
  coverage is thin.
- Judging whether a builder swap (`mkDerivation` → `buildNpmPackage`) is a
  refactor or a behavior change.
- Approving a refactor that touches purity invariants (hand off to the
  `constraint-nix-purity` skill).

## Avoiding scope creep

- Refactor only the structure described in step 1. If an unrelated cleanup
  appears, record it as a follow-up rather than folding it in.
- Do not fix defects encountered during the refactor; switch to
  `debugging.md` for the defect and resume the refactor afterward.
- Do not add features during a refactor; if a feature suggests itself, open a
  separate task under `implementation.md`.
- Do not change flake inputs, sandbox policy, or convention policy as part of
  a refactor; those are separate concerns with their own policy gates.
- Keep each incremental step small enough that a revert is cheap; if a step
  grows, split it.
