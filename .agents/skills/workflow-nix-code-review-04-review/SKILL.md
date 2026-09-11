---
name: workflow-nix-code-review-04-review
description: |
  Use only for the review phase of the Nix code-review workflow. Perform the
  human-judgment review using per-doc checklists across purity, reproducibility,
  sandbox safety, scope discipline, secret hygiene, and store hygiene. Do not
  use for scoping, analysis, running gates, or issuing a verdict.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-code-review
  org.phase: review
  org.phase_order: "04"
---

# Phase 04: review (Nix code review)

## Phase purpose

Perform the human-judgment review using the per-doc checklists across purity,
reproducibility, sandbox safety, scope discipline, secret hygiene, and store
hygiene. Walk each topic doc's `## Review checklist` section explicitly and
cite items rather than paraphrasing.

## Steps to perform

1. **Purity / source filters** — consult `docs/nix/purity-and-sandboxing.md`
   and the `constraint-nix-purity` skill. Look for `src = ./.` without a
   filter, `builtins.path`/`cleanSourceWith` without `filter =`,
   `builtins.getFlake` + `toString`, `--impure` invocations, and unfiltered
   path copies that churn the store (the 29 GB incident class).
2. **Reproducibility / pinning** — consult `docs/nix/supply-chain-security.md`
   and the `constraint-nix-reproducibility` skill. Look for unpinned flake
   inputs, mutable refs (`github:owner/repo` without rev), FOD hash mismatches
   or missing `outputHash`, `nix flake update` in CI, and stale lockfile
   entries.
3. **Sandbox safety** — consult `docs/nix/purity-and-sandboxing.md` and the
   `constraint-nix-sandbox-safety` skill. Look for network access in
   buildPhase/installPhase, `__noChroot` without justification, undeclared
   build dependencies, `HOME` not set to `$TMPDIR`, and system file
   dependencies.
4. **Scope discipline** — consult `docs/nix/conventions-and-style.md` and the
   `constraint-nix-scope-discipline` skill. Look for `with` in large scopes,
   `rec` when `let` suffices, `<nixpkgs>` channel references in flake code,
   missing `lib.` prefixes, and overly broad `let` bindings.
5. **Secret hygiene** — consult `docs/nix/secrets-and-sops.md` and the
   `constraint-nix-secret-hygiene` skill. Look for embedded secrets in Nix
   expressions, secrets resolved at eval-time instead of runtime, missing
   SOPS/age usage, and `with-secrets` wrapper not used for secret injection.
6. **Store hygiene / GC roots** — consult `docs/nix/store-hygiene-and-gc.md`
   and the `constraint-nix-store-hygiene` skill. Look for stale `result*`
   symlinks, `nix profile install` in scripts, `buildLayeredImage` where
   `streamLayeredImage` should be used, missing `--no-link --print-out-paths`
   in CI, and devshell gcroot not re-pinned after `flake.lock` changes.
7. Walk each topic doc's `## Review checklist` section explicitly; record
   pass/fail/N-A per item. **Cite checklist items; do not paraphrase them.**

## Docs to consult

- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/supply-chain-security.md`
- `docs/nix/conventions-and-style.md`
- `docs/nix/secrets-and-sops.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/derivations-and-builds.md`

## Operational skills to load

- `constraint-nix-purity`
- `constraint-nix-reproducibility`
- `constraint-nix-sandbox-safety`
- `constraint-nix-scope-discipline`
- `constraint-nix-secret-hygiene`
- `constraint-nix-store-hygiene`
- `nix-derivations` (if a derivation is present)
- `nix-flake-anatomy` (if flake.nix is present)

## Constraints to apply

- `constraint-nix-purity` — enforce source-filter discipline; flag `src = ./.`
  without filter, `builtins.path`/`cleanSourceWith` without `filter =`, and
  `--impure` invocations.
- `constraint-nix-reproducibility` — flag unpinned inputs, mutable refs, FOD
  hash mismatches, and `nix flake update` in CI.
- `constraint-nix-sandbox-safety` — flag network in build phases,
  `__noChroot` without justification, undeclared deps, and `HOME` not set to
  `$TMPDIR`.
- `constraint-nix-scope-discipline` — flag `with` in large scopes, `rec` when
  `let` suffices, `<nixpkgs>` in flake code, and missing `lib.` prefixes.
- `constraint-nix-secret-hygiene` — flag embedded secrets, eval-time secret
  resolution, and missing SOPS/age usage.
- `constraint-nix-store-hygiene` — flag stale `result*` symlinks,
  `buildLayeredImage` misuse, missing `--no-link`, and stale gcroots.

## Validations to run

None — this is a manual review phase that uses per-doc checklists. Automated
validations ran in phase 03 (workflow-nix-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-nix-code-review-00-orchestration`. Set:

- `outcome`: `pass` if no findings; `partial` if findings exist but none are
  blockers; `fail` if a blocker-level finding was identified.
- `constraints_applied`: all constraints listed above that applied.
- `risks`: each finding with file:line, the checklist item or doc rule
  violated (cited), and a concrete suggested fix. Severity classification
  happens in phase 05-verdict; here record the raw findings.
- `assumptions`: any N-A checklist items and why.
- `next_phase`: `05-verdict`.
- `next_workflow`: `null`.
