---
type: Workflow
resource: https://nix.dev/
title: Nix Debugging Workflow
description: Step-by-step workflow for debugging Nix build failures and eval errors — reproduce, diagnose, fix, regression, verify.
tags: [nix, workflow, debugging]
timestamp: 2026-07-24T00:00:00Z
---

# Nix Debugging Workflow

## Purpose

Process for diagnosing and fixing a Nix defect by category — eval error,
build failure, hash mismatch, purity violation, or store issue — then adding a
regression check and running the full check suite. The workflow enforces
root-cause fixes over symptom suppression.

## When to use

Use this workflow when fixing a failing eval, a build failure, a hash
mismatch, a purity violation, or a store issue. Do not use it for implementing
new features (use `implementation.md`), reviewing a diff (use
`code-review.md`), or restructuring without behavior change (use
`refactoring.md`). If the fix requires new public API (a new flake output),
follow this workflow for the fix then `implementation.md` for the API design
steps.

## Order of operations

### 1. Reproduce the issue

Before diagnosing, reproduce the issue reliably. Capture the exact command,
the input, the environment, and the full error message or build log. If the
issue cannot be reproduced, record what is known and what reproduction
attempts were made; do not proceed to a fix on an unreproduced report.

### 2. Categorize

Classify the defect into one of:

- **Eval error** — `nix flake check` or `nix build` fails at evaluation time;
  read the error (infinite recursion, type error, missing attribute,
  `assert` failure).
- **Build failure** — eval succeeds but the build phase fails; read the build
  log (missing source, wrong phase, missing dependency, patch failure).
- **Hash mismatch** — fixed-output derivation produces a different hash than
  recorded; read the expected vs actual hash.
- **Purity violation** — the build fails or produces non-deterministic output
  because of an impure input (`builtins.currentTime`, network access,
  `__noChroot` misuse).
- **Store issue** — `nix build` fails with a store path conflict, missing
  path, or GC-related error.

The category determines which skills and docs to load in step 3.

### 3. Load relevant skills

Load skills and docs matching the category:

- **Eval errors** → `.agents/skills/nix-language/SKILL.md` and
  `docs/nix/language-fundamentals.md`. Use the skill's error-to-fix table
  (infinite recursion, missing attribute, type error).
- **Build failures** → `.agents/skills/nix-derivations/SKILL.md` and
  `docs/nix/derivations-and-builds.md`. Inspect phases, builder, and
  `nativeBuildInputs`/`buildInputs`.
- **Hash mismatches** → `.agents/skills/constraint-nix-purity/SKILL.md` and
  `docs/nix/purity-and-sandboxing.md`. Re-run `just update-hashes` and confirm
  the source is pinned.
- **Purity violations** → `.agents/skills/constraint-nix-purity/SKILL.md` and
  `.agents/skills/constraint-nix-sandbox-safety/SKILL.md` and
  `docs/nix/purity-and-sandboxing.md`.
- **Store issues** → `.agents/skills/nix-store-gc/SKILL.md` and
  `docs/nix/store-hygiene-and-gc.md` and `docs/nix/nix-store-and-paths.md`.

Also load `.agents/skills/constraint-nix-reproducibility/SKILL.md` if the
defect is a non-deterministic build.

### 4. Suggest next

After categorization, suggest the next workflow continuation if the defect
falls outside debugging scope (for example, a defect that requires new feature
code → suggest `workflow-nix-implementation-00-orchestration`). Otherwise
proceed to step 5.

### 5. Fix the root cause

Fix the root cause, not the symptom. Concretely:

- For an eval error, fix the expression (missing attribute, wrong arity,
  infinite recursion) rather than wrapping in `tryEval` to hide it.
- For a build failure, fix the phase/builder/dependency rather than
  `lib.fake` or skipping the phase.
- For a hash mismatch, re-fetch the source, recompute the hash with
  `just update-hashes`, and confirm the source URL is pinned; do not blindly
  update the hash without verifying the source.
- For a purity violation, remove the impure input or justify the sandbox
  escape per `docs/nix/purity-and-sandboxing.md`; do not paper over with
  `__noChroot`.
- For a store issue, fix the GC root or path conflict per
  `docs/nix/store-hygiene-and-gc.md`; do not `nix store delete` to mask the
  conflict.

### 6. Add a regression check

Add a check that fails before the fix and passes after. Place it in the
`checks` output or `passthru.tests` per `docs/nix/testing.md`. The check must
encode the reproduction from step 1 so the same defect cannot silently
return.

### 7. Run the full check suite

```sh
nix flake check
nix build .#<name>
just verify
```

All gates must pass.

### 8. Report

Report the root cause, the fix, and the regression check.

## Docs consulted

- `docs/nix/error-handling-and-debugging.md`
- `docs/nix/language-fundamentals.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/nix-store-and-paths.md`
- `docs/nix/testing.md`

## Skills loaded

- (conditional) `.agents/skills/nix-language/SKILL.md`
- (conditional) `.agents/skills/nix-derivations/SKILL.md`
- (conditional) `.agents/skills/constraint-nix-purity/SKILL.md`
- (conditional) `.agents/skills/constraint-nix-sandbox-safety/SKILL.md`
- (conditional) `.agents/skills/nix-store-gc/SKILL.md`
- (conditional) `.agents/skills/constraint-nix-reproducibility/SKILL.md`
- `.agents/skills/nix-testing/SKILL.md`

## Commands run (in order)

Reproduce:

```sh
<the exact command that reproduces the issue>
```

Fix verification:

```sh
nix flake check
nix build .#<name>
just verify
```

## Evidence to report

- The reproduction command and the original error/build log (from step 1).
- The defect category (from step 2).
- The root cause: a one-paragraph explanation of why the defect occurred.
- The fix: what changed and why it addresses the root cause.
- The regression check: its location, what it asserts, and confirmation it
  fails before the fix and passes after.
- Pass/fail for each command in step 7.

## When human judgment is needed

- Deciding whether a hash mismatch indicates a compromised source (escalate to
  supply-chain review) or a legitimate upstream re-release.
- Judging whether a purity violation fix should restructure the derivation vs.
  accept a justified sandbox escape (consult the `constraint-nix-purity` skill).
- Choosing between fixing an eval error at the source vs. adding validation at
  a boundary (depends on where the invariant truly lives).
- Deciding whether a store conflict indicates a genuine path collision (fix the
  derivation) or a stale GC root (clean the root).
- Escalating when the root cause is in `nixpkgs` rather than the repo's own
  code.

## Avoiding scope creep

- Fix only the reported defect. If adjacent code has issues, record them as
  follow-ups; do not refactor the surrounding code in the same change.
- Do not add features while fixing a bug; if the fix reveals a needed feature,
  open a separate task under `implementation.md`.
- Do not suppress the symptom with `tryEval`, `lib.fake`, or blind hash
  updates; fix the root cause.
- Keep the regression check scoped to the defect; do not build a broad check
  suite expansion as part of a bugfix.
- If the fix touches purity invariants, restrict changes to the purity fix and
  its justification; do not rewrite the derivation's surrounding logic.
