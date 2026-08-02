---
name: workflow-okf-knowledge-base-creation-00-orchestration
description: |
  Use only to orchestrate the OKF knowledge base creation workflow. Use when
  creating a new OKF-compliant knowledge base bundle from source material —
  scoping the domain, designing the bundle structure, implementing topic docs
  and index files, validating conformance, and verifying corpus integration.
  Do not use for reviewing an existing bundle (use a config-review-style
  workflow), fixing a broken bundle (use a debugging-style workflow), or
  adding a single concept to an existing bundle (do that directly with
  okf-author).
allowed-tools: Read Write Edit Bash(git:*) Bash(python3:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: okf-knowledge-base-creation
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: OKF Knowledge Base Creation (Orchestration)

This skill orchestrates the OKF knowledge base creation workflow. It is the
entry point for creating a new OKF-compliant knowledge base bundle from source
material that requires scoping, design, implementation, validation, and
verification. Each phase is a separate public skill (Shape A) loaded by exact
name; the orchestrator loads them in order and applies the continuation policy
between phases.

## Phase routing

The workflow runs five phases in order. Each phase is a public skill at the
same level as this orchestrator.

| Order | Phase skill | Purpose |
| --- | --- | --- |
| 01 | `workflow-okf-knowledge-base-creation-01-scope` | Understand requirements: knowledge domain, sources, depth targets. Record intended corpus structure. |
| 02 | `workflow-okf-knowledge-base-creation-02-design` | Design bundle directory structure, OKF type taxonomy, cross-reference graph, skill derivation map, workflow map, index.md structure. |
| 03 | `workflow-okf-knowledge-base-creation-03-implement` | Create bundle directory structure, crawl files, topic docs, index.md files, source-map.md, log.md. |
| 04 | `workflow-okf-knowledge-base-creation-04-validate` | Run okf-validate, check cross-reference integrity, provenance completeness, skill coverage, depth targets. |
| 05 | `workflow-okf-knowledge-base-creation-05-verify` | Run full OKF validation, verify corpus integration, skill registration, workflow chaining, clean up temporary files. |

### Phase skipping conditions

Phases should run in full. Skipping is permitted only under the conditions
below; if a phase is skipped, record the skip and the reason in the handoff.

- `workflow-okf-knowledge-base-creation-01-scope` — **cannot be skipped.**
  Capturing domain, sources, and depth targets is required.
- `workflow-okf-knowledge-base-creation-02-design` — **may be skipped only
  for trivial single-concept additions** to an existing bundle with no new
  type taxonomy, no cross-references, and no new directory structure. If the
  bundle introduces new types, cross-refs, or directory layout, the phase
  must run.
- `workflow-okf-knowledge-base-creation-03-implement` — **cannot be
  skipped.** This is the core file-creation phase.
- `workflow-okf-knowledge-base-creation-04-validate` — **cannot be
  skipped.** Every bundle must pass OKF conformance and integrity checks.
- `workflow-okf-knowledge-base-creation-05-verify` — **cannot be
  skipped.** Final verification and cleanup are the validation evidence for
  the workflow.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns `outcome: pass`, the orchestrator hands off to the next workflow per the knowledge lifecycle chain. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-okf-knowledge-base-creation-05-verify` returns `outcome: pass` | `workflow-okf-validation-00-orchestration` (if the bundle needs a standalone full OKF validation gate) OR `workflow-nix-validation-00-orchestration` (if the bundle is part of a Nix-managed workspace needing the nix validation gate). Default: `workflow-okf-validation-00-orchestration`. |

This workflow's place in the overall knowledge lifecycle: the created bundle is a self-contained OKF distribution unit that may be consumed by any OKF reader.

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
next_phase: 04-validate
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

The `workflow-okf-knowledge-base-creation-05-verify` phase additionally returns the
verification-phase extra fields: `validations_run`, `constraints_checked`,
`evidence`, `failures`, `not_fully_checkable`. See
`workflow-okf-knowledge-base-creation-05-verify`.

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is decided by this orchestrator's Workflow chaining section, not by the handoff.

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
