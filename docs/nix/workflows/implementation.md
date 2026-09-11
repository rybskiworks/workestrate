---
type: Workflow
resource: https://nix.dev/
title: Nix Implementation Workflow
description: Step-by-step workflow for implementing new Nix code — flake outputs, derivations, modules, and packaging.
tags: [nix, workflow, implementation]
timestamp: 2026-07-24T00:00:00Z
---

# Nix Implementation Workflow

## Purpose

Process for implementing new Nix code — flake outputs, derivations, modules,
overlays, devShells, and NixOS configurations — from requirements to validated,
tested, sandbox-pure code. The workflow enforces a deterministic order:
understand requirements, load skills, design the structure, write the Nix
expression, write tests, run gates, and report.

## When to use

Use this workflow when adding new Nix code: a new flake output, a new
derivation, a new module or overlay, a new devShell, or the first
implementation of a Nix capability. Do not use it for reviewing a diff (use
`code-review.md`), restructuring without behavior change (use
`refactoring.md`), fixing a defect (use `debugging.md`), or running a final
gate (use `validation.md`). For onboarding a new package into the flake, use
`packaging.md` instead of this workflow.

## Order of operations

### 1. Understand requirements

Before writing code, capture the intended behavior, inputs, outputs, error
cases, and any public surface implications (new flake output, new attribute
path, new module option). Read the relevant topic docs:

- `docs/nix/flake-anatomy.md` — flake schema, outputs, inputs, schema
  conventions.
- `docs/nix/derivations-and-builds.md` — derivation structure, phases, fixed
  output derivations, builder selection.
- `docs/nix/purity-and-sandboxing.md` — sandbox constraints, impure inputs,
  `__noChroot`, network access.

Record a one-paragraph summary of the intended behavior and the public surface
so later steps can be checked against it.

### 2. Load relevant skills

Load the operational skills that match the implementation area:

- `.agents/skills/nix-flake-anatomy/SKILL.md` — flake schema, outputs, inputs.
- `.agents/skills/nix-derivations/SKILL.md` — derivation structure, phases,
  builder selection.
- `.agents/skills/constraint-nix-purity/SKILL.md` — sandbox purity invariants.
- `.agents/skills/constraint-nix-sandbox-safety/SKILL.md` — sandbox safety
  rules.

Load additional skills only if the implementation area requires them (for
example, `.agents/skills/nix-modules/SKILL.md` for module options,
`.agents/skills/nix-overlays/SKILL.md` for overlays).

### 3. Design the structure

Read `docs/nix/modules-and-config.md` for module/option structure and
`docs/nix/overlays.md` for overlay composition. Consult the
`.agents/skills/nix-flake-anatomy/SKILL.md` skill for output wiring and
attribute path conventions. Decide file layout (`nix/packages/<name>.nix`,
`nix/modules/<name>.nix`, `nix/devshells/default.nix`) and apply it
consistently with the existing flake structure.

### 4. Write the Nix expression

Implement the body of the code applying the purity rules from the
`constraint-nix-purity` and `constraint-nix-sandbox-safety` skills: no network
access in the sandbox, no impure inputs (`builtins.currentTime`,
`builtins.fetchTarball` without hash), deterministic store paths. Choose
builders per `docs/nix/derivations-and-builds.md` rather than reaching for a
raw `mkDerivation` when a higher-level builder exists.

### 5. Write tests alongside code

Write checks under `checks` outputs and `passthru.tests` per
`docs/nix/testing.md` and the `.agents/skills/nix-testing/SKILL.md` skill.
Cover the eval path, the build path, and edge cases (empty input, missing
input, cross-system). Add `nix flake check`-discoverable checks so they run
in the standard gate.

### 6. Run checks

Run the gates in this exact order; stop and fix before proceeding if a gate
fails:

```sh
nix flake check
nix build .#<name>
just lint-nix
just verify
```

If any gate fails, fix the root cause (do not suppress with `lib.fake` or
`builtins.trace` unless the convention policy explicitly permits it). Re-run
from `nix flake check`.

### 7. Report evidence

Report what was implemented, the public surface added, the tests added, and the
output of each gate in step 6.

## Docs consulted

- `docs/nix/flake-anatomy.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/testing.md`
- `docs/nix/conventions-and-style.md`
- `docs/nix/modules-and-config.md`
- `docs/nix/overlays.md`
- `docs/nix/language-fundamentals.md`

## Skills loaded

- `.agents/skills/nix-flake-anatomy/SKILL.md`
- `.agents/skills/nix-derivations/SKILL.md`
- `.agents/skills/constraint-nix-purity/SKILL.md`
- `.agents/skills/constraint-nix-sandbox-safety/SKILL.md`
- `.agents/skills/nix-testing/SKILL.md`
- (conditional) `.agents/skills/nix-modules/SKILL.md`
- (conditional) `.agents/skills/nix-overlays/SKILL.md`

## Commands run (in order)

```sh
nix flake check
nix build .#<name>
just lint-nix
just verify
```

## Evidence to report

- One-paragraph summary of intended behavior and public surface (from step 1).
- List of files added/modified.
- List of tests/checks added with what each covers.
- Raw output (or pass/fail) of each command in step 6.
- Any policy decisions deferred to the repo (for example, builder choice,
  sandbox escape approval).

## When human judgment is needed

- Choosing a builder when the repo has no recorded policy
  (`buildNpmPackage` vs `mkDerivation` vs custom).
- Deciding whether a new flake output needs a `passthru.tests` entry or a
  `checks` entry.
- Choosing between `nixpkgs` master and a pinned input when the tradeoff is
  not clear from the skill guidance.
- Approving a sandbox escape (`__noChroot`, `unsafe-setup-hooks`) — purity
  decisions belong to the repo owner.
- Deciding whether a fixed-output derivation is justified; if so, hand off to
  the `constraint-nix-purity` skill and `docs/nix/purity-and-sandboxing.md`.

## Avoiding scope creep

- Implement only the requirements captured in step 1. If a related cleanup
  appears, record it as a follow-up rather than folding it into this task.
- Do not refactor unrelated modules that the new code touches only at a call
  site; restrict changes to the new code and its direct integration points.
- Do not add flake inputs, overlays, or options beyond what the new code
  requires; if a flake change is needed, keep it minimal and note it in the
  report.
- If a gate fails because of pre-existing code, fix only the new code's
  contribution and surface the pre-existing issue as a follow-up.
