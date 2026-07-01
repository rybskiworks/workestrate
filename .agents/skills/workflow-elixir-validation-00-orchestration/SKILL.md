---
name: workflow-elixir-validation-00-orchestration
description: |
  Use only to orchestrate the Elixir validation workflow. Use when running the
  full validation gate suite (compile, credo, dialyzer, tests, format) as the
  final gate before merge/release or as a periodic workspace health gate. This
  workflow IS the validation — it runs all gates and reports; it does not modify
  code. Do not use as a substitute for implementation, refactoring, debugging,
  or code-review workflows; run it after one of those has produced code.
allowed-tools: Read Write Edit Bash(mix:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: elixir-validation
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Elixir Validation (Orchestration)

Run the full validation gate suite on Elixir code and report pass/fail per gate with evidence. This workflow IS the validation — it runs all gates and reports. It does not modify code. If a gate fails, hand off to the appropriate fixing workflow; do not fix in the validation pass.

## Phase routing

Phases are flat public phase skills loaded by exact name. Run them in order:

| Order | Phase Skill | Purpose |
|-------|-------------|---------|
| 01 | `workflow-elixir-validation-01-scope` | Identify what changed (git diff); determine which gates apply (OTP code? coverage configured? deps audit available?). Record applicable vs not-applicable gates. |
| 02 | `workflow-elixir-validation-02-compile` | Run `validation-elixir-compile` (`mix compile --warnings-as-errors`) and `validation-elixir-credo` (`mix credo --strict`). Record pass/fail + evidence. |
| 03 | `workflow-elixir-validation-03-lint-test` | Run `validation-elixir-dialyzer` (`mix dialyzer`) and `validation-elixir-test` (`mix test`). Record pass/fail + evidence. |
| 04 | `workflow-elixir-validation-04-format-doc` | Run `validation-elixir-format` (`mix format --check-formatted`). Record pass/fail + evidence. |
| 05 | `workflow-elixir-validation-05-report` | Aggregate all gate results into a per-gate table; report overall verdict; hand off to fixing workflow if any applicable gate is red. |

### Phase skipping conditions

- **01-scope**: Never skip. Determines which gates are applicable vs not-applicable.
- **02-compile** / **03-lint-test** / **04-format-doc**: Run all applicable gates. A gate marked not-applicable in phase 01 is skipped within its phase, but the phase still runs for its other gates. If a gate fails, continue to the remaining gates to give a full picture (overall result is fail).
- **05-report**: Never skip. Produces the final verdict and handoff.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the SDLC chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-elixir-validation-05-report` returns `outcome: pass` (all applicable gates green) | `stop` |

This workflow's place in the overall SDLC: see docs/languages.md "Workflows map" for the full graph.

## Handoff format

Each phase returns this YAML handoff:

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-elixir-style
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
  - validation-elixir-compile
  - validation-elixir-credo
  - validation-elixir-dialyzer
  - validation-elixir-test
  - validation-elixir-format
constraints_checked:
  - constraint-elixir-style
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

- Defect → `docs/elixir/workflows/debugging.md`
- Missing tests/docs → `docs/elixir/workflows/implementation.md`
- Structural cleanup → `docs/elixir/workflows/refactoring.md`

Then re-run validation after the fix.

```yaml
continuation: stop
next_phase: null
fallback: stop
```

## Docs Consulted

- `docs/elixir/workflows/validation.md` (source workflow)
- `docs/elixir/static-analysis-credo.md`
- `docs/elixir/naming-conventions.md`
- `docs/elixir/testing-exunit.md`
- `docs/elixir/mix-project-structure.md`
- `docs/elixir/dependencies-and-packages.md`
- `docs/beam/validation.md`
- `docs/elixir/documentation-and-publishing.md`
- `docs/elixir/typespecs-and-dialyzer.md`
