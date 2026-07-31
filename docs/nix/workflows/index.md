# Nix Workflow Index

## Purpose

This directory contains order-of-operations guides that future AI coding
agents follow for each SDLC activity in Nix: implementation, code review,
refactoring, debugging, validation, packaging, and hardening. Each workflow
doc describes what happens first, what context is gathered, which
`docs/nix/` topic docs are consulted, which `.agents/skills/` are loaded,
what checks/commands are run in order, what evidence is reported, when human
judgment is needed, and how to avoid mixing unrelated cleanup into the task.

The workflows are deliberately sequential and deterministic. They exist so
that any agent picking up a Nix task executes the same gates in the same
order and reports the same evidence shape, regardless of which model or
session is running.

## When to use

Use this index to choose a workflow before touching Nix code. Pick the
workflow that matches the task type; do not improvise a hybrid. If a task
spans multiple types (for example, fixing a defect that also requires new
feature code), run the primary workflow first and note the secondary workflow
in the report.

## Workflow family

### `docs/nix/workflows/implementation.md`

- **Purpose:** Implement new Nix code (flake outputs, derivations, modules,
  overlays, devShells) from requirements to validated, tested, sandbox-pure
  code.
- **When to use:** Adding a new flake output, derivation, module, overlay, or
  devShell; writing the first Nix code for a new capability.
- **Docs referenced:** `flake-anatomy.md`, `derivations-and-builds.md`,
  `purity-and-sandboxing.md`, `testing.md`, `conventions-and-style.md`,
  `modules-and-config.md`, `overlays.md`, `language-fundamentals.md`.
- **Skills referenced:** `nix-flake-anatomy`, `nix-derivations`,
  `constraint-nix-purity`, `constraint-nix-sandbox-safety`, `nix-testing`.

### `docs/nix/workflows/code-review.md`

- **Purpose:** Review a Nix PR/diff for correctness, purity, sandbox safety,
  store hygiene, secret hygiene, and convention quality.
- **When to use:** Reviewing a pull request, auditing a diff, or performing a
  pre-merge gate on someone else's Nix changes.
- **Docs referenced:** `flake-anatomy.md`, `derivations-and-builds.md`,
  `purity-and-sandboxing.md`, `supply-chain-security.md`,
  `store-hygiene-and-gc.md`, `secrets-and-sops.md`,
  `conventions-and-style.md`, `error-handling-and-debugging.md`.
- **Skills referenced:** `nix-flake-anatomy`, `nix-derivations`,
  `constraint-nix-purity`, `constraint-nix-sandbox-safety`,
  `constraint-nix-store-hygiene`, `constraint-nix-secret-hygiene`.

### `docs/nix/workflows/refactoring.md`

- **Purpose:** Restructure existing Nix code without changing behavior, with
  a passing-test baseline and incremental verification.
- **When to use:** Restructuring flake outputs, splitting modules,
  simplifying derivation structure, replacing a pattern, or cleaning up dead
  code — all without intended behavior change.
- **Docs referenced:** `flake-anatomy.md`, `derivations-and-builds.md`,
  `modules-and-config.md`, `overlays.md`, `conventions-and-style.md`,
  `language-fundamentals.md`.
- **Skills referenced:** `nix-flake-anatomy`, `nix-derivations`,
  `nix-modules`, `nix-overlays`, `constraint-nix-purity`.

### `docs/nix/workflows/debugging.md`

- **Purpose:** Diagnose and fix a Nix defect by category (eval error, build
  failure, hash mismatch, purity violation, store issue), then add a
  regression check.
- **When to use:** Fixing a failing eval, a build failure, a hash mismatch, a
  purity violation, or a store issue.
- **Docs referenced:** `error-handling-and-debugging.md`,
  `language-fundamentals.md`, `derivations-and-builds.md`,
  `purity-and-sandboxing.md`, `store-hygiene-and-gc.md`,
  `nix-store-and-paths.md`, `testing.md`.
- **Skills referenced:** `nix-language`, `nix-derivations`,
  `constraint-nix-purity`, `constraint-nix-sandbox-safety`, `nix-store-gc`,
  `constraint-nix-reproducibility`, `nix-testing`.

### `docs/nix/workflows/validation.md`

- **Purpose:** Run the full validation gate suite (flake check, eval, lint,
  test, store audit, format, supply chain) and report pass/fail per gate.
- **When to use:** Final validation before merge/release, or as a periodic
  health gate on the whole flake.
- **Docs referenced:** `flake-anatomy.md`, `conventions-and-style.md`,
  `testing.md`, `store-hygiene-and-gc.md`, `supply-chain-security.md`.
- **Skills referenced:** `validation-nix-flake-check`, `validation-nix-eval`,
  `validation-nix-lint`, `validation-nix-test`, `validation-nix-format`,
  `validation-nix-supply-chain`.

### `docs/nix/workflows/packaging.md`

- **Purpose:** Onboard a new package into the ai-workbench flake — write the
  derivation, wire into flake outputs, compute FOD hashes, verify purity.
- **When to use:** Adding a new package to the flake: a new derivation under
  `nix/packages/`, a new `packages` output, or the first packaging of an
  upstream project.
- **Docs referenced:** `packaging-recipes.md`, `derivations-and-builds.md`,
  `flake-anatomy.md`, `purity-and-sandboxing.md`,
  `supply-chain-security.md`.
- **Skills referenced:** `nix-packaging-recipes`, `nix-derivations`,
  `nix-flake-anatomy`, `constraint-nix-purity`,
  `constraint-nix-sandbox-safety`, `nix-testing`.

### `docs/nix/workflows/hardening.md`

- **Purpose:** Production-hardening pass on the flake — eval-time purity,
  build-time sandbox safety, store hygiene, secret hygiene, supply chain.
- **When to use:** Hardening the flake for production: tightening sandbox
  policy, adding store hygiene, enforcing secret indirection, pinning inputs,
  or adding supply-chain gates.
- **Docs referenced:** `purity-and-sandboxing.md`,
  `store-hygiene-and-gc.md`, `supply-chain-security.md`,
  `secrets-and-sops.md`, `deployment-and-runtime.md`,
  `caching-and-binary-caches.md`.
- **Skills referenced:** `constraint-nix-purity`,
  `constraint-nix-sandbox-safety`, `constraint-nix-store-hygiene`,
  `constraint-nix-secret-hygiene`, `constraint-nix-reproducibility`.

## Decision tree

Choose a workflow by matching the task type:

- **Task: write new feature / flake output / derivation / module / devShell**
  - Use `docs/nix/workflows/implementation.md`
  - If the new code is a fix for a known defect, also run
    `docs/nix/workflows/debugging.md` step 6 (regression check) and step 8
    (report root cause).
- **Task: onboard a new package into the flake**
  - Use `docs/nix/workflows/packaging.md`
  - If the package requires a new module or devShell, follow with
    `implementation.md` for the module.
- **Task: review a PR / diff / someone else's changes**
  - Use `docs/nix/workflows/code-review.md`
  - If the review surfaces a defect, hand off to
    `docs/nix/workflows/debugging.md` for the fix (do not fix in the review
    pass).
- **Task: restructure existing code without behavior change**
  - Use `docs/nix/workflows/refactoring.md`
  - If behavior must change, switch to `implementation.md` and record the
    intended behavior change explicitly.
- **Task: fix a defect (eval error, build failure, hash mismatch, purity
  violation, store issue)**
  - Use `docs/nix/workflows/debugging.md`
  - If the fix requires new public API (a new flake output), follow with
    `implementation.md` for the API design steps.
- **Task: final validation gate before merge/release**
  - Use `docs/nix/workflows/validation.md`
  - Run it after one of the above workflows has produced code, not instead of
    them.
- **Task: production-hardening pass on the flake**
  - Use `docs/nix/workflows/hardening.md`
  - On pass, chain to `workflow-rust-validation-00-orchestration` for the
    cross-language final gate.

## Workflow map

| Workflow | File path |
|---|---|
| Implementation | `docs/nix/workflows/implementation.md` |
| Code review | `docs/nix/workflows/code-review.md` |
| Refactoring | `docs/nix/workflows/refactoring.md` |
| Debugging | `docs/nix/workflows/debugging.md` |
| Validation | `docs/nix/workflows/validation.md` |
| Packaging | `docs/nix/workflows/packaging.md` |
| Hardening | `docs/nix/workflows/hardening.md` |

## Relationship to skills and topic docs

Workflows are the orchestration layer. They do not restate the rules that
live in `docs/nix/` topic docs or the operational recipes that live in
`.agents/skills/`. A workflow tells an agent which docs to read, which skills
to load, and which commands to run in order; the docs and skills supply the
actual rules. Always consult the referenced doc and load the referenced skill
rather than relying on memory.
