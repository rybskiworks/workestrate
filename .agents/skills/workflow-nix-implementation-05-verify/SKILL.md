---
name: workflow-nix-implementation-05-verify
description: |
  Use only for the verify phase of the Nix implementation workflow. Run the
  full validation suite (nix flake check, nix build, just lint-nix, just
  verify) and report evidence. Do not use for scoping, design, implementation,
  or test writing.
allowed-tools: Read Write Edit Bash(nix:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-implementation
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: verify (Nix implementation)

## Phase purpose

Run the full validation suite (`nix flake check`, `nix build`, `just lint-nix`,
`just verify`) and report evidence. This is the validation phase; it does not
introduce new behavior.

## Steps to perform

1. Run the gates in this exact order, stopping and fixing at the root cause if
   any fails. After a fix, re-run from `just lint-nix`:
   - `just lint-nix` (in-container purity guard; no nix required)
   - `nix eval .#<attr>.drvPath` (fast eval sanity for the new attribute)
   - `nix flake check --no-build` (eval all outputs; fast)
   - `nix flake check` (build all `checks` derivations)
   - `nix build .#<name> --no-link --print-out-paths` (build the new output;
     `--no-link` avoids pinning a `result` symlink / GC root)
2. Run `just verify` (the full project gate suite: toolchain-check, check,
   test, lint-nix, store-audit, etc.). This is the project-level acceptance
   gate; the new Nix code must not regress it.
3. Run `just store-audit` to confirm the new code did not introduce impure
   `*-source` store paths exceeding the 50M threshold.
4. Clean up out-links: remove any `result*` symlinks and `/tmp/*.tar.gz`
   out-links created during verification; they pin closures forever.
5. Apply `constraint-nix-store-hygiene` to verify no GC-root leaks were
   introduced.
6. Report: the intended-behavior summary (from phase 01), files
   added/modified, checks added with coverage, raw output/pass-fail of each
   gate plus the store-audit, and any deferred policy decisions.

## Docs to consult

- `docs/nix/testing.md`
- `docs/nix-purity.md`
- `docs/nix/store-hygiene-and-gc.md`

## Operational skills to load

- `nix-testing` — for `nix flake check` gate interpretation.
- `nix-store-gc` — for store-audit / out-link cleanup interpretation.

## Constraints to apply

- `constraint-nix-purity` — verify the new code passes the purity guard and
  introduces no impure `*-source` store paths.
- `constraint-nix-store-hygiene` — verify no `result*` symlinks or out-links
  were left behind; verify the store-audit threshold is not exceeded.
- `constraint-nix-reproducibility` — verify all flake inputs remain pinned via
  `flake.lock` (run `nix flake metadata` if a lock change was made).
- `constraint-nix-scope-discipline` — fix only the new code's contribution to
  any gate failure. If a gate fails because of pre-existing code, fix only the
  new code's contribution and surface the pre-existing issue as a follow-up.

## Validations to run

Run these validation skills in gate order. Each maps to a command or build
step.

- `validation-nix-lint` — `just lint-nix` (in-container purity guard)
- `validation-nix-eval` — `nix eval .#<attr>.drvPath` (eval sanity)
- `validation-nix-flake-check` — `nix flake check` (eval + build all checks)
- `validation-nix-build` — `nix build .#<name> --no-link --print-out-paths`
- `validation-nix-test` — `nix build .#checks.x86_64-linux.<name>` (run checks)
- `validation-nix-supply-chain` — `nix flake metadata` (inputs pinned) — run
  only if `flake.lock` changed.
- `validation-nix-format` — documented gate; this project does NOT configure a
  Nix formatter, so this gate is `not_fully_checkable` (manual review only).

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-implementation-00-orchestration`, extended with the
verification-phase extra fields. Set:

- `outcome` to `pass` only when every gate and the store-audit pass.
- `constraints_applied` to include `constraint-nix-purity`,
  `constraint-nix-store-hygiene`, `constraint-nix-reproducibility`, and
  `constraint-nix-scope-discipline`.
- `validations_run` to list each validation skill with its pass/fail.
- `constraints_checked` to list each constraint verified.
- `evidence` to include raw output or pass/fail of each gate and the
  store-audit.
- `failures` to list any gate that failed (empty on full pass).
- `not_fully_checkable` to list anything that could not be fully validated
  automatically and why (e.g. `validation-nix-format` — no formatter
  configured; `nix build` if nix/KVM is unavailable on the host).
- `next_phase: null` and `next_workflow: null` — this is the terminal phase.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-nix-purity
  - constraint-nix-store-hygiene
  - constraint-nix-reproducibility
  - constraint-nix-scope-discipline
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: ...
    covers: ...
tests_needed:
  - ...
next_phase: null
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: validation-nix-lint
    gate: just lint-nix
    result: pass|fail
  - validation: validation-nix-eval
    gate: nix eval .#<attr>.drvPath
    result: pass|fail
  - validation: validation-nix-flake-check
    gate: nix flake check
    result: pass|fail
  - validation: validation-nix-build
    gate: nix build .#<name> --no-link --print-out-paths
    result: pass|fail
  - validation: validation-nix-test
    gate: nix build .#checks.x86_64-linux.<name>
    result: pass|fail
  - validation: validation-nix-supply-chain
    gate: nix flake metadata
    result: pass|fail|skipped
  - validation: validation-nix-format
    gate: (no formatter configured — manual review)
    result: not_fully_checkable
constraints_checked:
  - constraint-nix-purity
  - constraint-nix-store-hygiene
  - constraint-nix-reproducibility
  - constraint-nix-scope-discipline
evidence:
  - gate: just lint-nix
    output: ...
  - gate: nix eval .#<attr>.drvPath
    output: ...
  - gate: nix flake check
    output: ...
  - gate: nix build .#<name> --no-link --print-out-paths
    output: ...
  - gate: nix build .#checks.x86_64-linux.<name>
    output: ...
  - gate: just store-audit
    output: ...
failures: []
not_fully_checkable:
  - validation-nix-format (no Nix formatter configured — manual review only)
```
