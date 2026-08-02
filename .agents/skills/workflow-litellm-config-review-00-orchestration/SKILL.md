---
name: workflow-litellm-config-review-00-orchestration
description: |
  Use only to orchestrate the LiteLLM config-review workflow. Use when reviewing a
  `config.yaml` diff/change (read-only; does NOT fix). Do not use for implementing
  config changes (use workflow-litellm-config-change), or fixing a defect (use
  workflow-litellm-config-change). If the review surfaces a defect, hand off to
  workflow-litellm-config-change; do not fix in the review pass.
allowed-tools: Read Bash(git:*) Bash(python:*)
metadata:
  org.kind: workflow-orchestration
  org.workflow: litellm-config-review
  org.phase: orchestration
  org.phase_order: "00"
---

# Workflow: LiteLLM Config Review (Orchestration)

This is the orchestration entry point for the LiteLLM config-review workflow.
Each phase is a separate public skill (Shape A) loaded by exact name. Do not
execute review work directly from this file; load the phase skill for the
current phase and follow it.

The workflow enforces a deterministic order: understand the diff, categorize it
by dimension and load the matching regular skill, run the automated config
checks, perform the human-judgment review against the 8 constraints, and report
findings with severity levels. This workflow is **read-only** — it reviews a
`config.yaml` diff/change and does NOT fix defects. If the review surfaces a
defect, hand off to `workflow-litellm-config-change`; do not fix in the review
pass.

## Phase routing

| Order | Phase skill | Purpose |
| --- | --- | --- |
| 01 | `workflow-litellm-config-review-01-scope` | Capture the diff's stated intent + risk areas; identify top-level sections + `model_name` aliases touched. |
| 02 | `workflow-litellm-config-review-02-analyze` | Categorize the diff by dimension (provider prefix, api_base rules, routing/fallback, secret hygiene, DB/Redis keys, deprecations); load relevant regular skill. |
| 03 | `workflow-litellm-config-review-03-check` | Run `validation-litellm-config-check` (checks a–i) + `validation-litellm-no-hardcoded-secrets`; record pass/fail. |
| 04 | `workflow-litellm-config-review-04-review` | Human-judgment review against the 8 constraints; flag violations with severity. |
| 05 | `workflow-litellm-config-review-05-verdict` | Classify findings (blocker/major/minor/nit); issue verdict (approve / request-changes / reject); hand off defects to `workflow-litellm-config-change`. |

Phases run strictly in order. Each phase skill returns a handoff YAML block; the
orchestrator reads `next_phase` to decide which phase skill to load next.

### Phase skipping conditions

- `workflow-litellm-config-review-01-scope` — cannot be skipped. The diff and
  its stated intent must be captured before any judgment is applied.
- `workflow-litellm-config-review-02-analyze` — cannot be skipped. Regular-skill
  loading is driven by the dimension categorization; skipping risks reviewing
  provider-prefix or api_base issues without the right regular skill loaded.
- `workflow-litellm-config-review-03-check` — may be skipped **only if** both
  validations (`validation-litellm-config-check`, `validation-litellm-no-hardcoded-secrets`)
  were already run green in CI and the evidence (command, commit SHA, CI run URL
  or log excerpt) is attached to the handoff. If either validation was not run or
  its evidence is missing, run the checks here.
- `workflow-litellm-config-review-04-review` — cannot be skipped. Automated
  checks do not substitute for the 8-constraint human-judgment review.
- `workflow-litellm-config-review-05-verdict` — cannot be skipped. A review
  without a recorded verdict and severity-classified findings is incomplete.

When a phase is skipped, record the skip reason and the attached evidence in the
handoff `assumptions` and `tests_run` fields.

## Per-phase constraint and validation ownership

Each phase applies its own constraints and runs its own validations — see each phase skill's "Constraints to apply" and "Validations to run" sections. **This orchestrator does not re-enumerate them.** (Constraints/validations are owned by phases; the orchestrator only routes and chains.)

## Workflow chaining

After this workflow's terminal phase returns its verdict, the orchestrator hands off to the next workflow per the chain below. The terminal-phase handoff's `next_workflow` field is `null` (end of *this* workflow); the chaining decision below is made by this orchestrator.

| Condition | Next workflow |
| --- | --- |
| `workflow-litellm-config-review-05-verdict` returns verdict `approve` | `null` (end — review passed) |
| `workflow-litellm-config-review-05-verdict` returns verdict `request-changes` (defect in config) | `workflow-litellm-config-change-00-orchestration` |
| `workflow-litellm-config-review-05-verdict` returns verdict `reject` | `null` (end — rejected; do not auto-fix) |

This workflow's place in the overall SDLC: see `docs/litellm/00-index.md` for the
full LiteLLM workflow map.

## Handoff format

Each phase returns a handoff YAML block using this schema. The orchestrator reads
`next_phase` to advance; `next_phase: null` ends the workflow.

```yaml
outcome: pass|fail|partial
files_touched:
  - path: infra/litellm/config.yaml
    change: ...
constraints_applied:
  - constraint-litellm-config-schema
  - ...
assumptions:
  - ...
risks:
  - ...
tests_run:
  - ...
tests_needed:
  - ...
next_phase: 04-review
next_workflow: null
handoff_requires_hil: false
hil_reason: null
blockers:
  - ...
```

Phase `workflow-litellm-config-review-05-verdict` additionally returns the
verification-phase extra fields: `validations_run`, `constraints_checked`,
`evidence`, `failures`, `not_fully_checkable`.

Set `next_workflow: null` in the terminal phase; cross-workflow chaining is decided by this orchestrator's Workflow chaining section, not by the handoff.

## Continuation policy

This workflow uses `conditional-next` between phases. After a phase returns its
handoff, proceed to `next_phase` only when the outcome is `pass` and there are
no blockers; otherwise stop and surface the handoff for human review.

```yaml
continuation_policy: conditional-next
condition: outcome == pass && blockers == []
on_pass: proceed_to_next_phase
on_fail: stop
fallback: stop
```
