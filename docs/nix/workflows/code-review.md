---
type: Workflow
resource: https://nix.dev/
title: Nix Code Review Workflow
description: Step-by-step workflow for reviewing Nix code changes — purity, reproducibility, scope discipline, and verdict.
tags: [nix, workflow, code-review]
timestamp: 2026-07-24T00:00:00Z
---

# Nix Code Review Workflow

## Purpose

Process for reviewing a Nix PR/diff for correctness, purity, sandbox safety,
store hygiene, secret hygiene, and convention quality. The workflow enforces a
deterministic order: understand the change, load review skills, run the
automated gates, then perform the human-judgment review using the per-doc
checklists, and report findings with severity levels.

## When to use

Use this workflow when reviewing a pull request, auditing a diff, or
performing a pre-merge gate on someone else's Nix changes. Do not use it for
implementing new code (use `implementation.md`), refactoring without behavior
change (use `refactoring.md`), fixing a defect (use `debugging.md`), or
running the full validation suite (use `validation.md`). If the review
surfaces a defect, hand off to `debugging.md` for the fix; do not fix in the
review pass.

## Order of operations

### 1. Understand the change

Before judging the code, capture what the change is and what it intends. Read
the PR description and the diff. Record: which files changed, what the stated
intent is, whether the change touches flake outputs, derivations, modules,
overlays, devShells, secrets, or the sandbox. This summary drives which docs
and skills to consult in later steps.

### 2. Load review skills

Load the operational skills that match a Nix review:

- `.agents/skills/nix-flake-anatomy/SKILL.md` — flake schema, output wiring,
  attribute path conventions.
- `.agents/skills/nix-derivations/SKILL.md` — derivation structure, phases,
  builder selection.
- `.agents/skills/constraint-nix-purity/SKILL.md` — sandbox purity
  invariants, impure inputs.
- `.agents/skills/constraint-nix-sandbox-safety/SKILL.md` — sandbox safety
  rules, `__noChroot` policy.

Load `.agents/skills/constraint-nix-store-hygiene/SKILL.md` and
`.agents/skills/constraint-nix-secret-hygiene/SKILL.md` as well if the diff
touches the store, GC roots, or secret handling.

### 3. Check lints

```sh
just lint-nix
```

A clean lint is the baseline. If it does not pass, report a blocker and stop
the review.

### 4. Check flake

```sh
nix flake check
```

`nix flake check` evaluates all outputs and runs `checks`. Any failure is a
blocker. Review any new `__noChroot` or sandbox escape for justification
against the policy in `docs/nix/purity-and-sandboxing.md`.

### 5. Check build

```sh
nix build .#<name>
```

The changed derivation must build. If a derivation was deleted or weakened,
flag it. New outputs should have accompanying `checks` (see
`docs/nix/testing.md`).

### 6. Review for correctness and design

With the automated gates green, perform the human-judgment review across these
dimensions, consulting the per-doc review checklists:

- **Flake anatomy** — `docs/nix/flake-anatomy.md` and the
  `nix-flake-anatomy` skill. Look for mis-wired outputs, missing `systems`,
  unpinned inputs.
- **Derivation correctness** — `docs/nix/derivations-and-builds.md` and the
  `nix-derivations` skill. Look for missing phases, wrong builder, missing
  `meta` attributes.
- **Purity and sandbox** — `docs/nix/purity-and-sandboxing.md` and the
  `constraint-nix-purity` / `constraint-nix-sandbox-safety` skills. Look for
  impure inputs, unjustified `__noChroot`, network access in the sandbox.
- **Supply chain** — `docs/nix/supply-chain-security.md`. Look for unpinned
  inputs, missing hashes, `builtins.fetchTarball` without a fixed output.
- **Store hygiene** — `docs/nix/store-hygiene-and-gc.md`. Look for leaked
  `result*` symlinks, missing GC roots.
- **Secret hygiene** — `docs/nix/secrets-and-sops.md`. Look for hardcoded
  secrets, missing `sops`/`agenix` indirection.
- **Conventions** — `docs/nix/conventions-and-style.md`. Check naming,
  attribute path conventions, file layout.
- **Error handling** — `docs/nix/error-handling-and-debugging.md`. Check
  `lib.optional` vs `lib.optionals`, `throw` vs `assert`, eval-time errors.

### 7. Use the review checklists

Each topic doc has a `## Review checklist` section. Walk the relevant
checklists explicitly and record which checklist items passed, failed, or were
not applicable. Do not paraphrase checklist items; cite them.

### 8. Report findings with severity levels

Classify every finding under one of these severity levels:

- **blocker** — must be fixed before merge: lint failure, `nix flake check`
  failure, build failure, purity violation, hardcoded secret, unpinned input.
- **major** — should be fixed before merge but may be deferred with owner
  approval: missing `checks` for a new output, wrong builder, missing `meta`.
- **minor** — fix encouraged but not blocking: naming nit, redundant
  `lib.optional`, missing `meta.description`.
- **nit** — optional polish: comment wording, attribute ordering.
- **praise** — positive callout: notably clean derivation, excellent test
  coverage, idiomatic use of a pattern from `docs/nix/conventions-and-style.md`.

### 9. Issue the verdict

On approve, chain to `workflow-nix-validation-00-orchestration` for the final
gate. On changes-requested, route to the appropriate orchestrator:
`workflow-nix-implementation-00-orchestration` for missing features,
`workflow-nix-refactoring-00-orchestration` for restructuring, or
`workflow-nix-debugging-00-orchestration` for defects.

## Docs consulted

- `docs/nix/flake-anatomy.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/supply-chain-security.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/secrets-and-sops.md`
- `docs/nix/conventions-and-style.md`
- `docs/nix/error-handling-and-debugging.md`

## Skills loaded

- `.agents/skills/nix-flake-anatomy/SKILL.md`
- `.agents/skills/nix-derivations/SKILL.md`
- `.agents/skills/constraint-nix-purity/SKILL.md`
- `.agents/skills/constraint-nix-sandbox-safety/SKILL.md`
- (conditional) `.agents/skills/constraint-nix-store-hygiene/SKILL.md`
- (conditional) `.agents/skills/constraint-nix-secret-hygiene/SKILL.md`

## Commands run (in order)

```sh
just lint-nix
nix flake check
nix build .#<name>
```

## Evidence to report

- One-paragraph summary of the change and its intent (from step 1).
- Pass/fail for each command in steps 3–5 with raw output on failure.
- List of findings, each with: file:line, severity, the checklist item or doc
  rule violated, and a concrete suggested fix.
- Explicit statement of which `## Review checklist` items were walked and their
  pass/fail/N-A status.
- A final verdict: approve, request changes, or block.

## When human judgment is needed

- Deciding whether a `__noChroot` is justified by repo purity policy.
- Judging whether a sandbox escape is necessary (this is rarely fully
  mechanical; consult the `constraint-nix-sandbox-safety` skill and
  `docs/nix/purity-and-sandboxing.md`).
- Weighing an unpinned input against the PR's stated intent.
- Deciding whether a missing `checks` entry is a blocker or a major, given the
  change's risk.
- Escalating a supply-chain concern that the reviewer cannot fully resolve to a
  human reviewer.

## Avoiding scope creep

- Review only the diff. Do not request changes to code the diff does not touch,
  even if it has pre-existing issues; record those as separate follow-ups.
- Do not ask the author to refactor for style preferences beyond what
  `just lint-nix` and the repo convention policy enforce.
- Do not combine a review with a parallel implementation task; if the review
  reveals a needed feature, open a separate task under `implementation.md`.
- Keep severity assignments consistent with the definitions above; do not
  inflate a nit to a major to force a change.
