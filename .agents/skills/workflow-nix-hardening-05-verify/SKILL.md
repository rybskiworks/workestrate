---
name: workflow-nix-hardening-05-verify
description: |
  Use only for the verify phase of the Nix hardening workflow. Run
  `nix build .#workestrate`, `just verify-full`, and confirm no `result*`
  symlink leaks, then report. Do not use for scoping, planning, implementation,
  or config-check validation.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*) Bash(python3:*) Bash(bash:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-hardening
  org.phase: verify
  org.phase_order: "05"
---

# Phase 05: verify (Nix hardening)

## Phase purpose

Run the verification gate suite — canonical build, full pre-merge gate, and
symlink-leak check — and report the root-cause-to-verification evidence chain.
This is the terminal phase; it does not introduce new behavior.

## Steps to perform

1. Read the validate-phase handoff (lint-nix, store-audit, flake check passed).
2. Run the gates in this order, stopping and surfacing a failure if any fails:
   - `nix build .#workestrate` — the canonical package build. Confirms no
     regressions across the full build surface. Must use `--no-link` (or the
     existing `result` symlink must be cleaned) to avoid store leaks.
   - `just verify-full` — the full pre-merge gate: toolchain-check, check,
     test, spec-examples, golden-check, schema-check,
     scaffold-check, lint-nix, store-audit, + `nix build .#workestrate`.
     Confirms the entire workspace is green.
   - Confirm no `result*` symlink leaks after the build. The build must use
     `--no-link` or any `result` symlink must be cleaned. Check with
     `ls result* 2>/dev/null` — no output means no leak.
3. Apply the active constraints (below) to interpret each gate result.
4. Report: the scope-phase gap table (which gaps were closed), files modified,
   raw output/pass-fail of each gate, and any deferred decisions (e.g. E6/E7).

## Docs to consult

- `docs/nix/store-hygiene-and-gc.md` — for interpreting symlink-leak and
  store-audit results.
- `docs/nix/purity-and-sandboxing.md` — for interpreting lint-nix results.
- `docs/nix/supply-chain-security.md` — for interpreting FOD hash results.
- `docs/nix/testing.md` — for interpreting `nix flake check` results.
- `.agents/skills/nix-usage/SKILL.md` — for the gcroot pin and build conventions.

## Operational skills to load

None mandatory. The verification commands are run directly as validations
(below).

## Constraints to apply

There are no Nix-specific constraint skills in the registry. Reference the
docs/nix/ corpus rules directly:

- Purity rules from `docs/nix/purity-and-sandboxing.md` — a `lint-nix` failure
  within `verify-full` is a purity regression; route back to phase 03.
- Supply-chain rules from `docs/nix/supply-chain-security.md` — a build
  failure on a FOD hash is a supply-chain violation; route back to phase 03.
- Store-hygiene rules from `docs/nix/store-hygiene-and-gc.md` — a
  `result*` symlink leak is a store-hygiene violation; the build must use
  `--no-link`.
- Secret hygiene from `docs/nix/secrets-and-sops.md` — confirm no hardcoded
  secrets were introduced; all secrets remain `SOPS_AGE_KEY_FILE` indirection.

## Validations to run

Run these verification gates in order. Each maps to a runtime check.

- `nix build .#workestrate` — the canonical package build; confirms no
  regressions.
- `just verify-full` — the full pre-merge gate (toolchain-check, check, test,
  spec-examples, golden-check, schema-check, scaffold-check,
  lint-nix, store-audit, + nix build).
- `result*` symlink-leak check — confirm no `result*` symlinks after build
  (build must use `--no-link`).

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-hardening-00-orchestration`, extended with the
verification-phase extra fields. Set:

- `outcome` to `pass` only when all three gates pass.
- `constraints_applied` to include the purity, supply-chain, store-hygiene,
  and secret-hygiene rules listed above.
- `validations_run` to list each gate with its pass/fail.
- `constraints_checked` to list each constraint verified.
- `evidence` to include raw output of each gate.
- `failures` to list any gate that failed (empty on full pass).
- `not_fully_checkable` to list anything that could not be fully validated
  automatically and why (e.g. `nix build .#workestrate` could not run because
  KVM/nix daemon is unavailable in the verify environment — single-user Nix,
  no daemon).
- `next_phase: null` and `next_workflow: null` — this is the terminal phase.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: flake.nix
    change: "hardened nixConfig block (E1–E8 applied; E6/E7 per decision)"
  - path: nix/packages/tempest.nix
    change: "replaced placeholder npmDepsHash (E9)"
  - path: justfile
    change: "wired nix flake check --no-build into verify-full (E10)"
constraints_applied:
  - purity-rules (docs/nix/purity-and-sandboxing.md)
  - supply-chain-rules (docs/nix/supply-chain-security.md)
  - store-hygiene-rules (docs/nix/store-hygiene-and-gc.md)
  - secret-hygiene (docs/nix/secrets-and-sops.md)
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: nix build .#workestrate
    covers: "canonical package build; no regressions"
  - name: just verify-full
    covers: "full pre-merge gate (toolchain, check, test, lint-nix, store-audit, nix build)"
  - name: result-symlink-leak-check
    covers: "no result* symlinks after build (--no-link)"
tests_needed: []
next_phase: null
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: nix build .#workestrate
    gate: "nix build .#workestrate --no-link"
    result: pass|fail
  - validation: just verify-full
    gate: "just verify-full"
    result: pass|fail
  - validation: result-symlink-leak-check
    gate: "ls result* 2>/dev/null (no output = no leak)"
    result: pass|fail
constraints_checked:
  - purity-rules (docs/nix/purity-and-sandboxing.md)
  - supply-chain-rules (docs/nix/supply-chain-security.md)
  - store-hygiene-rules (docs/nix/store-hygiene-and-gc.md)
  - secret-hygiene (docs/nix/secrets-and-sops.md)
evidence:
  - gate: "nix build .#workestrate"
    output: ...
  - gate: "just verify-full"
    output: ...
  - gate: "result-symlink-leak-check"
    output: ...
failures: []
not_fully_checkable: []
```
