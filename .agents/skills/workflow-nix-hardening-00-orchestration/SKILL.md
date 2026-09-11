---
name: workflow-nix-hardening-00-orchestration
description: |
  Use only to orchestrate the Nix production-hardening workflow. Use when
  applying a production-hardening pass to the ai-workbench Nix flake
  configuration — eval-time purity, build-time sandbox safety, store hygiene,
  secret hygiene, and supply chain. Do not use for adding new packages, reviewing
  a diff, or running the standalone validation gate suite (use
  workflow-rust-validation).
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*) Bash(python3:*) Bash(bash:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: nix-hardening
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Nix Hardening (Orchestration)

This skill orchestrates the Nix production-hardening workflow. It is the
entry point for a hardening pass on the ai-workbench Nix flake configuration
that requires scoping against a hardening baseline, planning verbatim-keyed
edits, applying them under constraints, and validating/verifying the result.
Each phase is a separate public skill (Shape A) loaded by exact name; the
orchestrator loads them in order and applies the continuation policy between
phases.

## Phase routing

The workflow runs five phases in order. Each phase is a public skill at the
same level as this orchestrator.

| Order | Phase skill | Purpose |
| --- | --- | --- |
| 01 | `workflow-nix-hardening-01-scope` | Assess current flake config against the hardening baseline (load `nix-usage`); identify gaps tied to verbatim config keys. |
| 02 | `workflow-nix-hardening-02-plan` | List the hardening edits, each tied to a verbatim config key + source. |
| 03 | `workflow-nix-hardening-03-implement` | Apply the edits to `flake.nix`, `nix/devshells/default.nix`, `nix/packages/agentctl.nix`, and optionally a new `nix.conf`; apply constraints. |
| 04 | `workflow-nix-hardening-04-validate` | Run `just lint-nix`, `just store-audit`, `nix flake check --no-build`. |
| 05 | `workflow-nix-hardening-05-verify` | Run `nix build .#workestrate` + `just verify-full` + confirm no `result*` symlink leaks; report. |

### Phase skipping conditions

Phases should run in full. Skipping is permitted only under the conditions
below; if a phase is skipped, record the skip and the reason in the handoff.

- `workflow-nix-hardening-01-scope` — **cannot be skipped.** The gap
  assessment against the hardening baseline is required so later phases can be
  checked against it.
- `workflow-nix-hardening-02-plan` — **cannot be skipped.** Every edit must
  be tied to a verbatim config key + source before implementation.
- `workflow-nix-hardening-03-implement` — **cannot be skipped.** This is
  the core config-editing phase.
- `workflow-nix-hardening-04-validate` — **cannot be skipped.** Config
  validation is the minimum gate after any config change.
- `workflow-nix-hardening-05-verify` — **cannot be skipped.** Build +
  full verify gate + symlink-leak check are the verification evidence for the
  workflow.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each
phase skill's "Constraints to apply" and "Validations to run" sections. **This
orchestrator does not re-enumerate them.** (Constraints/validations are owned
by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator
hands off to the next workflow per the SDLC chain. The terminal-phase handoff's
`next_workflow` field is `null` (end of *this* workflow); the chaining decision
below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-nix-hardening-05-verify` returns `outcome: pass` | `workflow-rust-validation-00-orchestration` |

This workflow's place in the overall SDLC: a hardening pass is followed by the
standalone Rust validation gate suite to confirm no regressions across the full
build surface.

## Handoff format

Each phase returns a handoff in this YAML schema. The orchestrator reads
`outcome`, `blockers`, and `next_phase` to decide whether to continue.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - purity-rules (docs/nix/purity-and-sandboxing.md)
  - ...
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: 04-validate
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

The `workflow-nix-hardening-05-verify` phase additionally returns the
verification-phase extra fields: `validations_run`, `constraints_checked`,
`evidence`, `failures`, `not_fully_checkable`. See
`workflow-nix-hardening-05-verify`.

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is
decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation policy

The orchestrator uses `conditional-next` between phases. After a phase returns
its handoff, proceed to `next_phase` only when the outcome is `pass` and there
are no blockers; otherwise stop and surface the handoff for human review.

```yaml
continuation_policy: conditional-next
condition: outcome == pass && blockers == []
on_pass: proceed_to_next_phase
on_fail: stop
fallback: stop
```

## Anti-hallucination policy

Every hardening recommendation in every phase MUST cite a verbatim config key
(a `nix.conf` setting, a `flake.nix` attribute, or a justfile recipe) plus the
docs page that documents it (e.g. `sandbox` from `docs/nix/purity-and-sandboxing.md`;
`auto-optimise-store` from `docs/nix/store-hygiene-and-gc.md`). Recommendations
not traceable to a verbatim key + source MUST be marked `[WORKESTRATOR NOTE]` or
`[INFERRED]`. Workestrator-specific recommendations (deployment choices that go
beyond upstream docs, e.g. introducing a project `nix.conf` or a flake
`nixConfig` block) MUST be marked `[WORKESTRATOR NOTE]`.
