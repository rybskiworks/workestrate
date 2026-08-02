---
name: workflow-nix-validation-00-orchestration
description: |
  Use only to orchestrate the Nix validation workflow. Use when running the
  full validation gate suite (flake check, eval, lint, test, format, docs,
  supply chain) as the final gate before merge/release or as a periodic
  workspace health gate. This workflow IS the validation — it runs all gates
  and reports; it does not modify code or config. Do not use as a substitute
  for implementation, refactoring, debugging, or code-review workflows; run it
  after one of those has produced code or config changes.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: nix-validation
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Nix Validation (Orchestration)

Run the full validation gate suite on Nix flake outputs, scripts, and config
and report pass/fail per gate with evidence. This workflow IS the validation —
it runs all gates and reports. It does not modify code or config. If a gate
fails, hand off to the appropriate fixing workflow; do not fix in the
validation pass.

## Phase routing

Phases are flat public phase skills loaded by exact name. Run them in order:

| Order | Phase Skill | Purpose |
|-------|-------------|---------|
| 01 | `workflow-nix-validation-01-scope` | Identify what changed (git diff); determine which gates apply (nix on host? formatter configured? store-delta-check applicable?). Record applicable vs not-applicable gates. |
| 02 | `workflow-nix-validation-02-compile` | Run `validation-nix-flake-check` (`nix flake check`) and `validation-nix-eval` (`nix eval .#workestrate.meta.description`, `nix flake show`). Record pass/fail + evidence. |
| 03 | `workflow-nix-validation-03-lint-test` | Run `validation-nix-lint` (`just lint-nix`), `validation-nix-test` (`nix build .#checks.x86_64-linux.<name>`), and `validation-nix-supply-chain` (`just store-audit`). Record pass/fail + evidence. |
| 04 | `workflow-nix-validation-04-format-doc` | Run `validation-nix-format` (`nix fmt --check` if configured) and verify docs cross-refs. Record pass/fail + evidence per gate. |
| 05 | `workflow-nix-validation-05-report` | Aggregate all gate results into a per-gate table; report overall verdict; hand off to fixing workflow if any applicable gate is red. |

### Phase skipping conditions

- **01-scope**: Never skip. Determines which gates are applicable vs not-applicable, and whether the HOST-GATE (nix on host) is satisfied.
- **02-compile** / **03-lint-test** / **04-format-doc**: Run all applicable gates. A gate marked not-applicable in phase 01 is skipped within its phase, but the phase still runs for its other gates. If a gate fails, continue to the remaining gates to give a full picture (overall result is fail). Gates requiring nix on the host are marked `not_fully_checkable` when nix is absent.
- **05-report**: Never skip. Produces the final verdict and handoff.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-nix-validation-05-report` returns `outcome: pass` (all applicable gates green) | `stop` |

This workflow's place in the overall SDLC: see docs/languages.md "Workflows map" for the full graph.

## Handoff format

Each phase returns this YAML handoff:

```yaml
outcome: pass|fail|partial
files_touched: []
constraints_applied:
  - constraint-nix-scope-discipline
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: 04-format-doc
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

Phase 05-report additionally returns:

```yaml
validations_run:
  - validation-nix-flake-check
  - validation-nix-eval
  - validation-nix-lint
  - validation-nix-test
  - validation-nix-format
  - validation-nix-supply-chain
constraints_checked:
  - constraint-nix-scope-discipline
evidence:
  - ...
failures:
  - ...
not_fully_checkable:
  - ...
```

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation policy

**stop**: after the report (phase 05), no further action. If any applicable gate
failed, hand off to the appropriate fixing workflow — do NOT fix in the
validation pass:

- Defect → `docs/nix/error-handling-and-debugging.md`
- Missing tests/checks → `docs/nix/testing.md`
- Structural cleanup → `docs/nix/conventions-and-style.md`

Then re-run validation after the fix.

```yaml
continuation: stop
next_phase: null
fallback: stop
```

## Docs Consulted

- `docs/nix/validation.md` (source workflow)
- `docs/nix/testing.md`
- `docs/nix/flake-anatomy.md`
- `docs/nix/store-hygiene-and-gc.md`
- `docs/nix/supply-chain-security.md`
- `docs/nix/conventions-and-style.md`
- `docs/nix/error-handling-and-debugging.md`
- `docs/nix-purity.md`
