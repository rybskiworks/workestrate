---
name: workflow-rust-validation-00-orchestration
description: |
  Use only to orchestrate the Rust validation workflow. Use when running the
  full validation gate suite (compile, clippy, tests, doctests, format, doc
  build, supply chain) as the final gate before merge/release or as a periodic
  workspace health gate. This workflow IS the validation — it runs all gates
  and reports; it does not modify code. Do not use as a substitute for
  implementation, refactoring, debugging, or code-review workflows; run it
  after one of those has produced code.
allowed-tools: Read Write Edit Bash(cargo:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: rust-validation
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Rust Validation (Orchestration)

Run the full validation gate suite on Rust code and report pass/fail per gate
with evidence. This workflow IS the validation — it runs all gates and reports.
It does not modify code. If a gate fails, hand off to the appropriate fixing
workflow; do not fix in the validation pass.

## Phase routing

Phases are flat public phase skills loaded by exact name. Run them in order:

| Order | Phase Skill | Purpose |
|-------|-------------|---------|
| 01 | `workflow-rust-validation-01-scope` | Identify what changed (git diff); determine which gates apply (unsafe present? coverage configured? cargo-deny configured?). Record applicable vs not-applicable gates. |
| 02 | `workflow-rust-validation-02-compile` | Run `validation-rust-compile` (cargo check --all-targets) and `validation-rust-clippy` (cargo clippy --workspace --all-targets -- -D warnings). Record pass/fail + evidence. |
| 03 | `workflow-rust-validation-03-lint-test` | Run `validation-rust-test` (cargo test --all-features + cargo test --doc). Record pass/fail + evidence. |
| 04 | `workflow-rust-validation-04-format-doc` | Run `validation-rust-format`, `validation-rust-docs`, `validation-rust-supply-chain`. Record pass/fail + evidence per gate. |
| 05 | `workflow-rust-validation-05-report` | Aggregate all gate results into a per-gate table; report overall verdict; hand off to fixing workflow if any applicable gate is red. |

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
| `workflow-rust-validation-05-report` returns `outcome: pass` (all applicable gates green) | `stop` |

This workflow's place in the overall SDLC: see docs/languages.md "Workflows map" for the full graph.

## Handoff format

Each phase returns this YAML handoff:

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - constraint-rust-scope-discipline
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
  - validation-rust-compile
  - validation-rust-clippy
  - validation-rust-test
  - validation-rust-format
  - validation-rust-docs
  - validation-rust-supply-chain
constraints_checked:
  - constraint-rust-scope-discipline
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

- Defect → `docs/rust/workflows/debugging.md`
- Missing tests/docs → `docs/rust/workflows/implementation.md`
- Structural cleanup → `docs/rust/workflows/refactoring.md`

Then re-run validation after the fix.

```yaml
continuation: stop
next_phase: null
fallback: stop
```

## Docs Consulted

- `docs/rust/workflows/validation.md` (source workflow)
- `docs/rust/lints-clippy.md`
- `docs/rust/style-formatting.md`
- `docs/rust/testing.md`
- `docs/rust/cargo-dependencies.md`
- `docs/rust/supply-chain-security.md`
- `docs/rust/editions-tooling.md`
- `docs/rust/unsafe-security.md`
- `docs/rust/documentation-guidelines.md`
