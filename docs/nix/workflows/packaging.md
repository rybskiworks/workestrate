---
type: Workflow
resource: https://nix.dev/
title: Nix Packaging Workflow
description: Step-by-step workflow for onboarding a new package or dependency into the Nix flake.
tags: [nix, workflow, packaging]
timestamp: 2026-07-24T00:00:00Z
---

# Nix Packaging Workflow

## Purpose

Process for onboarding a new package into the ai-workbench flake — writing the
derivation in `nix/packages/<name>.nix`, wiring it into flake outputs,
computing fixed-output derivation hashes, and verifying purity. The workflow
enforces a deterministic order: scope the package, design the builder, write
the derivation, test it, and verify.

## When to use

Use this workflow when adding a new package to the flake: a new derivation
under `nix/packages/`, a new `packages` output, or the first packaging of an
upstream project. Do not use it for general flake implementation (use
`implementation.md`), reviewing a diff (use `code-review.md`), or fixing a
defect (use `debugging.md`). If the package requires a new module or devShell,
run this workflow for the package then `implementation.md` for the module.

## Order of operations

### 1. Understand requirements

Before writing the derivation, capture the upstream source, version, license,
build system, and runtime dependencies. Read the relevant topic docs:

- `docs/nix/packaging-recipes.md` — recipe catalog for common build systems.
- `docs/nix/derivations-and-builds.md` — derivation structure, phases,
  fixed-output derivations.
- `docs/nix/flake-anatomy.md` — flake schema, `packages` output wiring.
- `docs/nix/purity-and-sandboxing.md` — sandbox constraints, FOD hashes.
- `docs/nix/supply-chain-security.md` — source pinning, hash verification.

Record a one-paragraph summary of the package, its build system, and the
intended `packages.<system>.<name>` output path.

### 2. Design the builder

Choose the builder per `docs/nix/packaging-recipes.md` and the
`.agents/skills/nix-packaging-recipes/SKILL.md` skill:

- `buildNpmPackage` — Node.js projects with a `package-lock.json`.
- `buildPythonApplication` — Python applications with a setup.
- `buildGoModule` — Go modules with a `vendorHash`.
- Bun compile — Bun-based projects.
- `pip install --target` — Python packages without a build system.
- Custom `mkDerivation` — when no higher-level builder fits.

Decide whether the derivation is a fixed-output derivation (FOD) and which
hashes need computing. Decide `meta` attributes (`description`, `homepage`,
`license`, `maintainers`, `platforms`).

### 3. Write the derivation

Implement `nix/packages/<name>.nix` applying the purity rules from the
`constraint-nix-purity` and `constraint-nix-sandbox-safety` skills: no network
access in the sandbox, pinned source with hash, deterministic store path.
Wire the derivation into `flake.nix` `packages` output per the
`nix-flake-anatomy` skill.

### 4. Compute FOD hashes

```sh
just update-hashes
```

`just update-hashes` computes and updates fixed-output derivation hashes
(`vendorHash`, `outputHash`, `cargoHash`). Confirm the source is pinned before
accepting a new hash; do not blindly accept a hash without verifying the
source.

### 5. Test the package

Build the package and confirm it produces a working binary:

```sh
nix build .#<name>
```

Add a `passthru.tests` entry or `checks` output per `docs/nix/testing.md` and
the `.agents/skills/nix-testing/SKILL.md` skill. Cover the build path and a
smoke test of the binary's `--help` or `--version`.

### 6. Run checks

Run the gates in this exact order; stop and fix before proceeding if a gate
fails:

```sh
nix build .#<name>
just update-hashes
nix flake check
just lint-nix
just verify
```

If any gate fails, fix the root cause (do not suppress with `lib.fake` or
skip phases). Re-run from `nix build`.

### 7. Surface for human review

On pass, surface the package for human review. The evaluator cannot verify
runtime correctness of the packaged binary — a human must confirm the binary
behaves as expected at runtime (not just that it builds).

## Docs consulted

- `docs/nix/packaging-recipes.md`
- `docs/nix/derivations-and-builds.md`
- `docs/nix/flake-anatomy.md`
- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/supply-chain-security.md`

## Skills loaded

- `.agents/skills/nix-packaging-recipes/SKILL.md`
- `.agents/skills/nix-derivations/SKILL.md`
- `.agents/skills/nix-flake-anatomy/SKILL.md`
- `.agents/skills/constraint-nix-purity/SKILL.md`
- `.agents/skills/constraint-nix-sandbox-safety/SKILL.md`
- `.agents/skills/nix-testing/SKILL.md`

## Commands run (in order)

```sh
nix build .#<name>
just update-hashes
nix flake check
just lint-nix
just verify
```

## Evidence to report

- One-paragraph summary of the package, build system, and output path (from
  step 1).
- The builder chosen and why (from step 2).
- List of files added/modified (`nix/packages/<name>.nix`, `flake.nix`).
- The FOD hashes computed and the source pinning verification (from step 4).
- The `passthru.tests` or `checks` entry added and what it covers (from step
  5).
- Raw output (or pass/fail) of each command in step 6.
- Explicit statement that the package builds and passes gates, with a
  handoff to human review for runtime correctness.

## When human judgment is needed

- Choosing a builder when the upstream build system is non-standard (custom
  `mkDerivation` vs. patching an existing builder).
- Deciding whether a fixed-output derivation is justified or whether the
  derivation can be a normal build.
- Verifying that a computed hash corresponds to a legitimate upstream release
  rather than a compromised source (escalate to supply-chain review).
- Deciding whether a `meta.license` is correct when upstream is ambiguous.
- Approving a sandbox escape for a package that requires network access
  during build (purity decisions belong to the repo owner).

## Avoiding scope creep

- Package only the upstream project described in step 1. If a related package
  appears, record it as a follow-up rather than folding it in.
- Do not refactor unrelated derivations that the new package touches only at a
  call site; restrict changes to the new package and its flake wiring.
- Do not add flake inputs, overlays, or options beyond what the new package
  requires; if a flake change is needed, keep it minimal and note it in the
  report.
- Do not modify the upstream source to make it build; patch in the derivation
  via `patches` and document why.
- If a gate fails because of a pre-existing package, fix only the new
  package's contribution and surface the pre-existing issue as a follow-up.
