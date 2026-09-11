---
name: workflow-nix-packaging-00-orchestration
description: |
  Use when onboarding a new package/dependency into the ai-workbench Nix flake —
  writing a derivation in `nix/packages/<name>.nix`, wiring it into flake
  outputs, computing FOD hashes, and verifying purity. Do NOT use for ordinary
  Rust/Elixir/Gleam implementation or for editing devshell-only config.
allowed-tools: Read Write Edit Bash(nix:*) Bash(just:*) Bash(git:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: nix-packaging
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: Nix Packaging (Orchestration)

This skill orchestrates the Nix packaging workflow. It is the entry point for
onboarding a new package or dependency into the ai-workbench Nix flake —
writing a derivation in `nix/packages/<name>.nix`, wiring it into `flake.nix`
outputs, computing fixed-output-derivation (FOD) hashes, and verifying purity.
Each phase is a separate public skill (Shape A) loaded by exact name; the
orchestrator loads them in order and applies the continuation policy between
phases.

## Phase routing

The workflow runs five phases in order. Each phase is a public skill at the
same level as this orchestrator.

| Order | Phase skill | Purpose |
| --- | --- | --- |
| 01 | `workflow-nix-packaging-01-scope` | Identify the software to package: language/ecosystem, dependencies, build system, source location. |
| 02 | `workflow-nix-packaging-02-design` | Choose the right builder (buildNpmPackage, buildPythonApplication, buildGoModule, bun compile, pip install --target, custom mkDerivation), design the derivation, plan FOD strategy. |
| 03 | `workflow-nix-packaging-03-implement` | Write the derivation in `nix/packages/<name>.nix`, add to flake outputs, run `just update-hashes` for FOD hashes. |
| 04 | `workflow-nix-packaging-04-test` | `nix build .#<name>`, verify output, test in devshell. |
| 05 | `workflow-nix-packaging-05-verify` | `nix flake check`, `just lint-nix`, `just verify`, confirm purity. |

## Per-phase constraint and validation ownership

Each phase owns its own constraints and validations. The orchestrator does not
enumerate them here — consult each phase skill for the constraints it applies
and the validations it runs. Scope discipline (stay within the requirements
captured in phase 01) applies to all phases.

## Workflow chaining

When phase 05-verify returns `outcome == pass`, there is no dedicated Nix
code-review workflow to chain to. Packaging changes require human review of
the derivation recipe and `flake.nix` wiring before merge — the Nix
evaluator cannot verify runtime correctness of the packaged binary. Surface
the handoff for manual review.

```yaml
chaining_policy: conditional-next-workflow
condition: phase_05_verify.outcome == pass && blockers == []
on_pass: surface_for_human_review
on_fail: stop
note: |
  No dedicated nix code-review workflow exists yet. Human review of the
  derivation recipe and flake.nix wiring is recommended before merge; the
  evaluator cannot verify runtime correctness of the packaged binary.
```

## Handoff format

Each phase returns a handoff in this YAML schema. The orchestrator reads
`outcome`, `blockers`, and `next_phase` to decide whether to continue.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: ...
    change: ...
constraints_applied:
  - ...
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: 04-test
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

The `workflow-nix-packaging-05-verify` phase additionally returns the
verification-phase extra fields: `validations_run`, `constraints_checked`,
`evidence`, `failures`, `not_fully_checkable`. See
`workflow-nix-packaging-05-verify`.

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
