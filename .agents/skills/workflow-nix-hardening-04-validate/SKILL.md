---
name: workflow-nix-hardening-04-validate
description: |
  Use only for the validate phase of the Nix hardening workflow. Run
  `just lint-nix`, `just store-audit`, and `nix flake check --no-build` against
  the edited flake. Do not use for scoping, planning, implementation, or final
  verification.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*) Bash(python3:*) Bash(bash:*)
metadata:
  org.kind: workflow-phase
  org.workflow: nix-hardening
  org.phase: validate
  org.phase_order: "04"
---

# Phase 04: validate (Nix hardening)

## Phase purpose

Run the config-level gates — `just lint-nix`, `just store-audit`, and
`nix flake check --no-build` — against the edited Nix flake configuration.
This is the config-level gate; it does not do a full build. A failure here
stops the workflow and returns to phase 03 for a fix.

## Steps to perform

1. Read the implement-phase handoff (the edited file paths).
2. Run the gates in this order, stopping and surfacing a failure if any fails:
   - `just lint-nix` — the purity guard (scripts/check-nix-paths.sh, checks
     1–5). Confirms no `--impure`, no bare `builtins.path`, no impure path
     references.
   - `just store-audit` — the store hygiene scan
     (scripts/store-audit.py --warn-if-source-over 50; informational).
     Surfaces excessive source-tree leakage into the store.
   - `nix flake check --no-build` — flake eval + checks without a full build.
     Confirms the `nixConfig` block and all outputs evaluate cleanly.
3. If any gate fails, capture the exact error, stop, and surface a handoff with
   `outcome: fail` and `next_phase: 03-implement` so the root cause can be
   fixed. Do not attempt the fix in this phase.
4. If all gates pass, record the raw output and proceed to phase 05.

## Docs to consult

- `docs/nix/purity-and-sandboxing.md` — for interpreting lint-nix failures.
- `docs/nix/store-hygiene-and-gc.md` — for interpreting store-audit failures.
- `docs/nix/testing.md` — for interpreting `nix flake check` failures.
- `.agents/skills/nix-usage/SKILL.md` — for the purity rules behind the gates.

## Operational skills to load

None mandatory. The validation commands are run directly as validations
(below), not as operational skills.

## Constraints to apply

There are no Nix-specific constraint skills in the registry. Reference the
docs/nix/ corpus rules directly:

- Purity rules from `docs/nix/purity-and-sandboxing.md` — a `lint-nix` failure
  is a purity violation (e.g. `--impure`, bare `builtins.path`); route back to
  phase 03, do not paper over it.
- Store-hygiene rules from `docs/nix/store-hygiene-and-gc.md` — a
  `store-audit` failure (source-over threshold exceeded) is a store-hygiene
  violation; route back to phase 03.
- Supply-chain rules from `docs/nix/supply-chain-security.md` — a
  `nix flake check` failure on a FOD hash is a supply-chain violation; route
  back to phase 03.

## Validations to run

- `just lint-nix` — the purity guard (scripts/check-nix-paths.sh). This is the
  first gate.
- `just store-audit` — the store hygiene scan (scripts/store-audit.py
  --warn-if-source-over 50; informational). This is the second gate.
- `nix flake check --no-build` — flake eval + checks without a full build.
  This is the third gate.

## Handoff output

Return the handoff YAML schema defined in
`workflow-nix-hardening-00-orchestration`. Set:

- `outcome` to `pass` only when all three gates pass.
- `constraints_applied` to include the purity rules from
  `docs/nix/purity-and-sandboxing.md`, store-hygiene rules from
  `docs/nix/store-hygiene-and-gc.md`, and supply-chain rules from
  `docs/nix/supply-chain-security.md`.
- `validations_run` to list each gate with pass/fail.
- `evidence` to include the raw gate output.
- `next_phase: 05-verify` on pass; `next_phase: 03-implement` on fail.
- `blockers: []` on pass; the failure detail on fail.

```yaml
outcome: pass|fail|partial
files_touched: []
constraints_applied:
  - purity-rules (docs/nix/purity-and-sandboxing.md)
  - store-hygiene-rules (docs/nix/store-hygiene-and-gc.md)
  - supply-chain-rules (docs/nix/supply-chain-security.md)
assumptions:
  - ...
risks:
  - ...
tests_run:
  - name: just lint-nix
    covers: "purity guard (no --impure, no bare builtins.path)"
  - name: just store-audit
    covers: "store hygiene (source-over threshold)"
  - name: nix flake check --no-build
    covers: "flake eval + checks without full build"
tests_needed:
  - "nix build .#workestrate (phase 05)"
  - "just verify-full (phase 05)"
next_phase: 05-verify
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers: []
validations_run:
  - validation: just lint-nix
    gate: "scripts/check-nix-paths.sh (checks 1-5)"
    result: pass|fail
  - validation: just store-audit
    gate: "scripts/store-audit.py --warn-if-source-over 50 (informational scan)"
    result: pass|fail
  - validation: nix flake check --no-build
    gate: "nix flake check --no-build"
    result: pass|fail
constraints_checked:
  - purity-rules (docs/nix/purity-and-sandboxing.md)
  - store-hygiene-rules (docs/nix/store-hygiene-and-gc.md)
  - supply-chain-rules (docs/nix/supply-chain-security.md)
evidence:
  - gate: "just lint-nix"
    output: ...
  - gate: "just store-audit"
    output: ...
  - gate: "nix flake check --no-build"
    output: ...
failures: []
not_fully_checkable: []
```
