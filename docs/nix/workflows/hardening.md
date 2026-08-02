---
type: Workflow
resource: https://nix.dev/
title: Nix Hardening Workflow
description: Step-by-step workflow for applying a production-hardening pass to the Nix flake configuration.
tags: [nix, workflow, hardening]
timestamp: 2026-07-24T00:00:00Z
---

# Nix Hardening Workflow

## Purpose

Process for applying a production-hardening pass to the ai-workbench flake —
eval-time purity, build-time sandbox safety, store hygiene, secret hygiene,
and supply chain. The workflow enforces a deterministic order: scope the
hardening gaps, plan the edits, implement them, validate, and verify. Every
recommendation must cite a verbatim config key and source doc.

## When to use

Use this workflow when hardening the flake for production: tightening sandbox
policy, adding store hygiene, enforcing secret indirection, pinning inputs,
or adding supply-chain gates. Do not use it for adding new packages (use
`packaging.md`), reviewing a diff (use `code-review.md`), or running the
standalone validation suite (use `validation.md`). On pass, chain to
`workflow-rust-validation-00-orchestration` for the cross-language final gate.

## Order of operations

### 1. Scope the hardening gaps

Assess the current flake against the hardening baseline and identify gaps tied
to verbatim config keys. Read the relevant topic docs:

- `docs/nix/purity-and-sandboxing.md` — `sandbox`, `__noChroot` policy.
- `docs/nix/store-hygiene-and-gc.md` — `auto-optimise-store`, GC roots.
- `docs/nix/supply-chain-security.md` — input pinning, hash verification.
- `docs/nix/secrets-and-sops.md` — `sops`/`agenix` indirection.
- `docs/nix/deployment-and-runtime.md` — runtime flags, `nix.conf`.
- `docs/nix/caching-and-binary-caches.md` — `substituters`, `trusted-public-keys`.

Record a one-paragraph summary of the gaps and the files to touch:
`flake.nix`, `nix/devshells/default.nix`, `nix/packages/agentctl.nix`, and
optionally a new `nix.conf`.

### 2. Plan the edits

List the hardening edits, each tied to a verbatim config key + source doc. For
example:

- `sandbox = true` from `docs/nix/purity-and-sandboxing.md`.
- `auto-optimise-store = true` from `docs/nix/store-hygiene-and-gc.md`.
- `substituters` + `trusted-public-keys` from
  `docs/nix/caching-and-binary-caches.md`.

For any recommendation that cannot be traced to a verbatim config key in a
source doc, mark it `[WORKESTRATOR NOTE]` or `[INFERRED]` and flag for human
review. Do not present an inferred recommendation as a documented rule.

### 3. Implement the edits

Apply the planned edits to `flake.nix`, `nix/devshells/default.nix`,
`nix/packages/agentctl.nix`, and optionally a new `nix.conf` under the active
constraints:

- `.agents/skills/constraint-nix-purity/SKILL.md` — eval-time purity.
- `.agents/skills/constraint-nix-sandbox-safety/SKILL.md` — build-time
  sandbox safety.
- `.agents/skills/constraint-nix-store-hygiene/SKILL.md` — store hygiene.
- `.agents/skills/constraint-nix-secret-hygiene/SKILL.md` — secret hygiene.
- `.agents/skills/constraint-nix-reproducibility/SKILL.md` — reproducibility.

Preserve `os.environ/`-style secret indirection and in-memory constraints
where applicable.

### 4. Validate

```sh
just lint-nix
just store-audit
nix flake check --no-build
```

`just lint-nix` confirms conventions; `just store-audit` confirms no
`result*` symlink leaks and clean GC roots; `nix flake check --no-build`
confirms eval-time purity without a full build. If any gate fails, fix the
root cause and re-run.

### 5. Verify

```sh
nix build .#workestrate
just verify-full
```

Confirm no `result*` symlink leaks after the build. Confirm the hardened
flake still produces the expected store path. On pass, chain to
`workflow-rust-validation-00-orchestration` for the cross-language final
gate.

## Docs consulted

- `docs/nix/purity-and-sandboxing.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/supply-chain-security.md`
- `docs/nix/secrets-and-sops.md`
- `docs/nix/deployment-and-runtime.md`
- `docs/nix/caching-and-binary-caches.md`

## Skills loaded

- `.agents/skills/constraint-nix-purity/SKILL.md`
- `.agents/skills/constraint-nix-sandbox-safety/SKILL.md`
- `.agents/skills/constraint-nix-store-hygiene/SKILL.md`
- `.agents/skills/constraint-nix-secret-hygiene/SKILL.md`
- `.agents/skills/constraint-nix-reproducibility/SKILL.md`

## Commands run (in order)

```sh
just lint-nix
just store-audit
nix flake check --no-build
nix build .#workestrate
just verify-full
```

Confirm no `result*` symlink leaks after the build.

## Evidence to report

- One-paragraph summary of the hardening gaps and files touched (from step 1).
- The planned edits, each tied to a verbatim config key + source doc (from
  step 2).
- List of files modified (`flake.nix`, `nix/devshells/default.nix`,
  `nix/packages/agentctl.nix`, `nix.conf`).
- Raw output (or pass/fail) of each command in steps 4 and 5.
- Explicit confirmation that no `result*` symlink leaks remain.
- Any `[WORKESTRATOR NOTE]` or `[INFERRED]` recommendations flagged for human
  review.
- Handoff to `workflow-rust-validation-00-orchestration` on pass.

## When human judgment is needed

- Deciding whether a `sandbox = true` setting breaks a derivation that
  requires network access (escalate to packaging or implementation).
- Judging whether a `substituters` change is safe for the deployment
  environment (consult `docs/nix/caching-and-binary-caches.md`).
- Weighing an `auto-optimise-store` setting against disk I/O tradeoffs.
- Deciding whether a secret indirection (`sops` vs `agenix`) is correct for the
  repo's deployment model.
- Approving a `nix.conf` change that affects the host Nix daemon (host policy
  decisions belong to the repo owner).

## Anti-hallucination policy

- Every recommendation must cite a verbatim config key and the source doc it
  comes from. For example: `sandbox` from `docs/nix/purity-and-sandboxing.md`;
  `auto-optimise-store` from `docs/nix/store-hygiene-and-gc.md`.
- If a recommendation cannot be traced to a verbatim config key in a source
  doc, mark it `[WORKESTRATOR NOTE]` or `[INFERRED]` and flag for human review.
  Do not present an inferred recommendation as a documented rule.
- Do not invent config keys, env vars, or builder attributes that do not
  appear in the source docs or skills.
- Do not claim a hardening edit is "best practice" without a source doc
  citation; cite the doc or mark it inferred.
- Keep the report honest about what was verified vs. what was inferred.
