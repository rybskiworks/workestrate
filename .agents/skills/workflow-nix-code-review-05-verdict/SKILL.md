---
name: workflow-nix-code-review-05-verdict
description: |
  Use only for the verdict phase of the Nix code-review workflow. Classify
  every finding under a severity level and issue a final verdict. Do not use
  for scoping, analysis, running gates, or manual review.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-code-review
  org.phase: verdict
  org.phase_order: "05"
---

# Phase 05: verdict (Nix code review)

## Phase purpose

Classify every finding under a severity level, record the checklist items
walked and their status, and issue a final verdict. If the review surfaces a
defect, hand off to `workflow-nix-debugging`; do not fix in the review pass.

## Steps to perform

1. Classify every finding under exactly one severity level:
   - **blocker** — must fix before merge: purity violation (`--impure`,
     unfiltered `src = ./.`, `builtins.path` without `filter =`), eval/build
     failure, FOD hash mismatch, embedded secret in a Nix expression, sandbox
     escape (network in buildPhase), `nix flake update` in CI, stale gcroot
     pinning the wrong closure.
   - **major** — should fix before merge but may be deferred with owner
     approval: unpinned flake input, mutable ref without rev, `__noChroot`
     without justification, undeclared build dependency, `buildLayeredImage`
     where `streamLayeredImage` should be used, missing `--no-link` in CI
     scripts, `with` in a large scope.
   - **minor** — fix encouraged but not blocking: `rec` when `let` suffices,
     missing `lib.` prefix, `HOME` not set to `$TMPDIR` in a non-critical
     derivation, naming inconsistency, redundant `let` binding.
   - **nit** — optional polish: comment wording, import ordering, example
     simplification, style inconsistency the formatter missed.
   - **praise** — positive callout: notably tight source filter, excellent FOD
     hash discipline, idiomatic `lib.` usage, clean `streamLayeredImage`
     migration.
2. For each finding record: `file:line`, `severity`, the checklist item or
   doc rule violated (cited, not paraphrased), and a concrete suggested fix.
3. State explicitly which `## Review checklist` items were walked and their
   pass/fail/N-A status.
4. Issue a final verdict: `approve`, `request changes`, or `reject`.
5. If the review surfaces a defect, set `next_workflow: workflow-nix-debugging`
   (do not fix in the review pass). Otherwise set `next_workflow: null`.

## Docs to consult

None new. Findings reference the docs cited in phase 04-review.

## Operational skills to load

None new.

## Constraints to apply

- `constraint-nix-scope-discipline` — findings cover only the diff; pre-existing
  issues in untouched code are recorded as separate follow-ups, not as review
  findings.

## Validations to run

None — this is a reporting phase. Automated validations ran in phase 03
(workflow-nix-code-review-03-check).

## Handoff output

Return the handoff YAML block per the schema in
`workflow-nix-code-review-00-orchestration`, including the verification-phase
extra fields. Set:

- `outcome`: `pass` if verdict is `approve`; `partial` if `request changes`;
  `fail` if `reject`.
- `constraints_applied`: `constraint-nix-scope-discipline` (plus any
  constraints applied in phase 04 that produced findings).
- `risks`: the final classified findings list (file:line, severity, cited
  checklist item, suggested fix).
- `blockers`: every finding classified `blocker`.
- `next_phase`: `null` (this is the final phase).
- `next_workflow`: `workflow-nix-debugging` if a defect was found, else
  `null`.

Include the verification-phase extra fields:

```yaml
validations_run:
  - validation-nix-lint
  - validation-nix-flake-check
  - validation-nix-build
  - validation-nix-test
  - validation-nix-supply-chain
constraints_checked:
  - constraint-nix-purity
  - constraint-nix-reproducibility
  - constraint-nix-sandbox-safety
  - constraint-nix-scope-discipline
  - constraint-nix-secret-hygiene
  - constraint-nix-store-hygiene
evidence:
  - ...
failures:
  - ...
not_fully_checkable:
  - ...
```

`not_fully_checkable` records any checklist item or gate that could not be
fully resolved mechanically (e.g. `nix flake check` skipped because nix is not
on the host, or a FOD hash whose correctness requires a network fetch the
sandbox disallows), with a reason and a suggested escalation path.
